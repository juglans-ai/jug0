// src/providers/llm/claude_code.rs
//
// Claude Code CLI as an LLM provider — spawns `claude` subprocess,
// communicates via NDJSON stdin/stdout protocol.
// Internal tool calls (bash, edit_file, etc.) are auto-allowed;
// only text content is streamed back to jug0.

use super::{ChatStreamChunk, LlmProvider, TokenUsage};
use crate::entities::messages;
use crate::handlers::chat::MessagePart;
use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use serde::Deserialize;
use serde_json::{json, Value};
use std::pin::Pin;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

// ---------------------------------------------------------------------------
// NDJSON protocol types (mirrors juglans/src/ui/tui/claude_code.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum StreamLine {
    #[serde(rename = "stream_event")]
    StreamEvent {
        event: StreamInnerEvent,
        #[allow(dead_code)]
        #[serde(default)]
        session_id: Option<String>,
    },
    #[serde(rename = "assistant")]
    Assistant { message: Value },
    #[serde(rename = "result")]
    ResultLine {
        #[allow(dead_code)]
        #[serde(default)]
        result: Option<String>,
        #[allow(dead_code)]
        #[serde(default)]
        total_cost_usd: Option<f64>,
        #[allow(dead_code)]
        #[serde(default)]
        duration_ms: Option<u64>,
        #[allow(dead_code)]
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        usage: Option<UsagePayload>,
    },
    #[serde(rename = "control_request")]
    ControlRequest {
        request_id: String,
        request: ControlRequestBody,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "subtype")]
enum ControlRequestBody {
    #[serde(rename = "can_use_tool")]
    CanUseTool {
        #[allow(dead_code)]
        tool_name: String,
        #[allow(dead_code)]
        #[serde(default)]
        input: Value,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum StreamInnerEvent {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        #[allow(dead_code)]
        index: Option<u32>,
        delta: Delta,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Delta {
    #[serde(rename = "text_delta")]
    Text { text: String },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
struct UsagePayload {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

// ---------------------------------------------------------------------------
// Internal event enum for the channel
// ---------------------------------------------------------------------------

enum InternalEvent {
    TextDelta(String),
    AssistantSnapshot(String),
    Result {
        input_tokens: u64,
        output_tokens: u64,
    },
    PermissionRequest {
        request_id: String,
    },
    Error(String),
    Done,
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

pub struct ClaudeCodeProvider {
    claude_bin: String,
    cwd: String,
}

impl ClaudeCodeProvider {
    pub fn new() -> Self {
        let claude_bin = std::env::var("CLAUDE_BIN").unwrap_or_else(|_| "claude".to_string());
        let cwd = std::env::var("CLAUDE_CODE_CWD").unwrap_or_else(|_| ".".to_string());
        Self { claude_bin, cwd }
    }

    /// Map jug0 model name to claude CLI short name
    fn map_model(model: &str) -> &str {
        let m = model.to_lowercase();
        if m.contains("opus") {
            "opus"
        } else if m.contains("haiku") {
            "haiku"
        } else {
            "sonnet"
        }
    }
}

#[async_trait]
impl LlmProvider for ClaudeCodeProvider {
    async fn stream_chat(
        &self,
        model: &str,
        system_prompt: Option<String>,
        history: Vec<messages::Model>,
        _tools: Option<Vec<Value>>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>>> {
        // Extract last user message
        let user_message = history
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| {
                if let Ok(parts) = serde_json::from_value::<Vec<MessagePart>>(m.parts.clone()) {
                    parts
                        .first()
                        .and_then(|p| p.content.clone())
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            })
            .unwrap_or_default();

        // Prepend system prompt if present
        let prompt = if let Some(sp) = system_prompt.filter(|s| !s.trim().is_empty()) {
            format!(
                "[System Instructions]\n{}\n\n[User Message]\n{}",
                sp, user_message
            )
        } else {
            user_message
        };

        // Resolve cwd (expand ~)
        let real_cwd = if let Some(stripped) = self.cwd.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                format!("{}/{}", home.to_string_lossy(), stripped)
            } else {
                self.cwd.clone()
            }
        } else {
            self.cwd.clone()
        };

        let cli_model = Self::map_model(model);

        let mut cmd = Command::new(&self.claude_bin);
        cmd.arg("--output-format")
            .arg("stream-json")
            .arg("--input-format")
            .arg("stream-json")
            .arg("--dangerously-skip-permissions")
            .arg("--verbose")
            .arg("--model")
            .arg(cli_model)
            .current_dir(&real_cwd)
            .env_remove("CLAUDECODE") // prevent nested session error
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            anyhow::anyhow!("Failed to spawn claude CLI at `{}`: {}", self.claude_bin, e)
        })?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture claude stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture claude stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture claude stderr"))?;

        // Send init + user message
        let init_msg = json!({
            "type": "control_request",
            "request_id": "init_1",
            "request": {
                "subtype": "initialize",
                "hooks": null,
                "agents": null
            }
        });
        stdin
            .write_all(format!("{}\n", init_msg).as_bytes())
            .await?;
        stdin.flush().await?;

        let user_msg = json!({
            "type": "user",
            "session_id": "",
            "message": {
                "role": "user",
                "content": prompt
            },
            "parent_tool_use_id": null
        });
        stdin
            .write_all(format!("{}\n", user_msg).as_bytes())
            .await?;
        stdin.flush().await?;

        let (tx, rx) = mpsc::unbounded_channel::<InternalEvent>();

        // Task 1: parse stdout NDJSON
        let tx_stdout = tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                match serde_json::from_str::<StreamLine>(trimmed) {
                    Ok(parsed) => {
                        let events = translate(parsed);
                        for ev in events {
                            if tx_stdout.send(ev).is_err() {
                                return;
                            }
                        }
                    }
                    Err(_) => {} // skip unparseable lines
                }
            }
            let _ = tx_stdout.send(InternalEvent::Done);
        });

        // Task 2: collect stderr
        let tx_stderr = tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut buf = String::new();
            while let Ok(Some(line)) = lines.next_line().await {
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(&line);
            }
            if !buf.is_empty() {
                let _ = tx_stderr.send(InternalEvent::Error(buf));
            }
        });

        // Task 3: handle permission requests (auto-allow) — needs stdin writer
        let (perm_tx, mut perm_rx) = mpsc::unbounded_channel::<String>();

        // Forward permission request_ids to the stdin writer task
        let _tx_main = tx;
        let perm_tx_clone = perm_tx;

        // Consume rx and produce ChatStreamChunk stream
        let (chunk_tx, chunk_rx) = mpsc::unbounded_channel::<Result<ChatStreamChunk>>();

        tokio::spawn(async move {
            let mut internal_rx = rx;
            while let Some(ev) = internal_rx.recv().await {
                match ev {
                    InternalEvent::TextDelta(text) => {
                        let _ = chunk_tx.send(Ok(ChatStreamChunk {
                            content: Some(text),
                            tool_calls: vec![],
                            usage: None,
                            finish_reason: None,
                        }));
                    }
                    InternalEvent::AssistantSnapshot(text) => {
                        let _ = chunk_tx.send(Ok(ChatStreamChunk {
                            content: Some(text),
                            tool_calls: vec![],
                            usage: None,
                            finish_reason: None,
                        }));
                    }
                    InternalEvent::Result {
                        input_tokens,
                        output_tokens,
                    } => {
                        let _ = chunk_tx.send(Ok(ChatStreamChunk {
                            content: None,
                            tool_calls: vec![],
                            usage: Some(TokenUsage {
                                input_tokens: input_tokens as i64,
                                output_tokens: output_tokens as i64,
                                total_tokens: (input_tokens + output_tokens) as i64,
                            }),
                            finish_reason: Some("stop".to_string()),
                        }));
                    }
                    InternalEvent::PermissionRequest { request_id } => {
                        let _ = perm_tx_clone.send(request_id);
                    }
                    InternalEvent::Error(msg) => {
                        tracing::warn!("claude-code stderr: {}", msg);
                    }
                    InternalEvent::Done => {
                        break;
                    }
                }
            }
            // Stream is done
            drop(chunk_tx);
        });

        // Task 4: stdin writer for permission responses
        tokio::spawn(async move {
            while let Some(request_id) = perm_rx.recv().await {
                let response = json!({
                    "type": "control_response",
                    "response": {
                        "subtype": "success",
                        "request_id": request_id,
                        "response": {
                            "behavior": "allow"
                        }
                    }
                });
                if stdin
                    .write_all(format!("{}\n", response).as_bytes())
                    .await
                    .is_err()
                {
                    break;
                }
                let _ = stdin.flush().await;
            }
            // Keep child alive until stream ends, then kill
            drop(stdin);
            let _ = child.wait().await;
        });

        let stream = UnboundedReceiverStream::new(chunk_rx);
        Ok(Box::pin(stream))
    }
}

// ---------------------------------------------------------------------------
// translate — convert parsed NDJSON to internal events
// ---------------------------------------------------------------------------

fn translate(line: StreamLine) -> Vec<InternalEvent> {
    match line {
        StreamLine::StreamEvent { event, .. } => match event {
            StreamInnerEvent::ContentBlockDelta { delta, .. } => match delta {
                Delta::Text { text } => vec![InternalEvent::TextDelta(text)],
                Delta::Unknown => vec![],
            },
            StreamInnerEvent::MessageStop => vec![],
            StreamInnerEvent::Unknown => vec![],
        },
        StreamLine::ResultLine { usage, .. } => {
            vec![InternalEvent::Result {
                input_tokens: usage.as_ref().and_then(|u| u.input_tokens).unwrap_or(0),
                output_tokens: usage.as_ref().and_then(|u| u.output_tokens).unwrap_or(0),
            }]
        }
        StreamLine::ControlRequest {
            request_id,
            request,
        } => match request {
            ControlRequestBody::CanUseTool { .. } => {
                vec![InternalEvent::PermissionRequest { request_id }]
            }
            ControlRequestBody::Unknown => vec![],
        },
        StreamLine::Assistant { message } => {
            // Extract text from assistant snapshot
            let mut full_text = String::new();
            if let Some(content) = message.get("content").and_then(|c| c.as_array()) {
                for block in content {
                    if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                        if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                            if !text.is_empty() {
                                full_text.push_str(text);
                            }
                        }
                    }
                }
            }
            if full_text.is_empty() {
                vec![]
            } else {
                vec![InternalEvent::AssistantSnapshot(full_text)]
            }
        }
        StreamLine::Unknown => vec![],
    }
}
