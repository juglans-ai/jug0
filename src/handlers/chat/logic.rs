// src/handlers/chat/logic.rs
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, Set, ActiveModelTrait,
    DatabaseConnection, QuerySelect
};
use serde_json::{json, Value};
use std::collections::HashMap;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::time::Duration;
use uuid::Uuid;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use dashmap::DashMap;
use scopeguard::defer;

use crate::entities::{messages, chats};
use crate::entities::messages::message_types;
use crate::services::mcp::{McpClient, McpTool};
use crate::providers::{ChatStreamChunk, TokenUsage, ProviderFactory};
use crate::services::memory::service::MemoryService;
use crate::errors::AppError;
use crate::auth::AuthUser;
use super::types::{InternalStreamEvent, ToolCallAccumulator, MessagePart};
use super::helpers::{merge_tools, try_repair_json};

pub fn run_chat_stream(
    db: DatabaseConnection,
    active_requests: DashMap<Uuid, CancellationToken>,
    mcp_client: McpClient,
    provider_factory: ProviderFactory,
    memory_service: MemoryService,
    user: AuthUser,
    chat_id: Uuid,
    mut last_message_id: i32,  // 当前最新的 message_id
    user_message_uuid: Uuid,   // 用户消息的 UUID
    model_to_use: String,
    final_system_prompt: Option<String>,
    client_tools_def: Option<Vec<Value>>,
    server_tools: Vec<McpTool>,
    should_use_memory: bool,
    history_override: Option<Value>,   // 上下文控制：true/false/自定义数组
    message_state: String,             // 消息状态
    tool_result_rx: Option<mpsc::Receiver<Vec<super::types::ToolResultPayload>>>,
    tool_result_channels: DashMap<Uuid, mpsc::Sender<Vec<super::types::ToolResultPayload>>>,
) -> impl Stream<Item = Result<InternalStreamEvent, AppError>> {

    let cancel_token = CancellationToken::new();
    active_requests.insert(chat_id, cancel_token.clone());

    let mut tool_result_rx = tool_result_rx;

    async_stream::stream! {
        defer! {
            active_requests.remove(&chat_id);
            tool_result_channels.remove(&chat_id);
        }

        // last_message_id 此时等于 user_message_id（由 mod.rs 传入）
        let user_message_id = last_message_id;
        let stream_start = std::time::Instant::now();

        yield Ok(InternalStreamEvent::Meta { chat_id, user_message_id, user_message_uuid: Some(user_message_uuid) });

        let mut loop_count = 0;
        let max_loops = 50;

        loop {
            // 每轮循环分配新的 assistant message_id
            last_message_id += 1;
            let assistant_message_id = last_message_id;
            if loop_count >= max_loops {
                yield Ok(InternalStreamEvent::Error("Max interaction loops exceeded".to_string()));
                break;
            }
            loop_count += 1;

            // 根据 history 参数决定上下文加载策略
            let history_from_db = match &history_override {
                // history=false → 仅当前交互（user msg + tool results）
                Some(Value::Bool(false)) => {
                    messages::Entity::find()
                        .filter(messages::Column::ChatId.eq(chat_id))
                        .filter(messages::Column::MessageId.gte(user_message_id))
                        .order_by_asc(messages::Column::MessageId)
                        .all(&db)
                        .await
                        .map_err(AppError::Database)?
                },
                // history=[...] → 自定义上下文 + 当前交互
                Some(Value::Array(custom)) => {
                    let mut history = parse_custom_history(custom);
                    let current = messages::Entity::find()
                        .filter(messages::Column::ChatId.eq(chat_id))
                        .filter(messages::Column::MessageId.gte(user_message_id))
                        .order_by_asc(messages::Column::MessageId)
                        .all(&db)
                        .await
                        .map_err(AppError::Database)?;
                    history.extend(current);
                    history
                },
                // 默认(true/null) → 历史上下文(按state过滤) + 当前交互(始终包含)
                // 合并为单次查询: (state IN (...) AND id < user) OR (id >= user)
                _ => {
                    messages::Entity::find()
                        .filter(messages::Column::ChatId.eq(chat_id))
                        .filter(
                            Condition::any()
                                .add(
                                    Condition::all()
                                        .add(messages::Column::State.is_in([
                                            messages::states::CONTEXT_VISIBLE,
                                            messages::states::CONTEXT_HIDDEN,
                                        ]))
                                        .add(messages::Column::MessageId.lt(user_message_id))
                                )
                                .add(messages::Column::MessageId.gte(user_message_id))
                        )
                        .order_by_asc(messages::Column::MessageId)
                        .all(&db)
                        .await
                        .map_err(AppError::Database)?
                }
            };

            tracing::info!("⏱ [Stream] History loaded: {}ms (count: {})", stream_start.elapsed().as_millis(), history_from_db.len());

            // 过滤孤立 tool 消息：role="tool" 必须有前置 assistant 的 tool_calls 包含匹配的 id
            let history_from_db = {
                let mut valid_tool_call_ids = std::collections::HashSet::new();
                for msg in &history_from_db {
                    if msg.role == "assistant" {
                        if let Some(ref tc) = msg.tool_calls {
                            if let Some(arr) = tc.as_array() {
                                for call in arr {
                                    if let Some(id) = call["id"].as_str() {
                                        valid_tool_call_ids.insert(id.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                history_from_db.into_iter().filter(|msg| {
                    if msg.role == "tool" {
                        msg.tool_call_id.as_ref()
                            .map(|id| valid_tool_call_ids.contains(id))
                            .unwrap_or(false)
                    } else {
                        true
                    }
                }).collect::<Vec<_>>()
            };

            let all_tools = merge_tools(&client_tools_def, &server_tools);
            let provider = provider_factory.get_provider(&model_to_use);

            let mut stream: Pin<Box<dyn Stream<Item = anyhow::Result<ChatStreamChunk>> + Send>> =
                match provider.stream_chat(&model_to_use, final_system_prompt.clone(), history_from_db, all_tools).await {
                    Ok(s) => s,
                    Err(e) => {
                        yield Ok(InternalStreamEvent::Error(e.to_string()));
                        break;
                    }
                };

            tracing::info!("⏱ [Stream] Provider stream opened: {}ms (model: {})", stream_start.elapsed().as_millis(), model_to_use);

            let mut full_response = String::new();
            let mut tool_acc_map: HashMap<i32, ToolCallAccumulator> = HashMap::new();
            let mut is_stream_cancelled = false;
            let mut accumulated_usage: Option<TokenUsage> = None;
            let mut first_token_logged = false;

            while let Some(result) = stream.next().await {
                if cancel_token.is_cancelled() { is_stream_cancelled = true; break; }
                match result {
                    Ok(chunk) => {
                        if let Some(content) = chunk.content {
                            if !content.is_empty() {
                                if !first_token_logged {
                                    tracing::info!("⏱ [Stream] First token: {}ms", stream_start.elapsed().as_millis());
                                    first_token_logged = true;
                                }
                                full_response.push_str(&content);
                                yield Ok(InternalStreamEvent::Content(content));
                            }
                        }
                        for tc in chunk.tool_calls {
                            let entry = tool_acc_map.entry(tc.index).or_default();
                            if let Some(id) = tc.id { entry.id = id; }
                            if let Some(name) = tc.name { entry.name.push_str(&name); }
                            if let Some(args) = tc.arguments { entry.arguments.push_str(&args); }
                        }
                        // Accumulate usage from final chunk
                        if chunk.usage.is_some() {
                            accumulated_usage = chunk.usage;
                        }
                    },
                    Err(e) => {
                        yield Ok(InternalStreamEvent::Error(e.to_string()));
                        is_stream_cancelled = true;
                        break;
                    }
                }
            }
            tracing::info!("⏱ [Stream] Generation done: {}ms (tokens: {} chars)", stream_start.elapsed().as_millis(), full_response.len());
            if is_stream_cancelled { break; }

            if !tool_acc_map.is_empty() {
                let mut server_side_calls = Vec::new();
                let mut client_side_calls = Vec::new();
                let mut all_calls_json = Vec::new();

                for (_, acc) in tool_acc_map {
                    let tool_name = acc.name.clone();
                    let args_str = try_repair_json(&acc.arguments);
                    let args_val: Value = serde_json::from_str(&args_str).unwrap_or(json!({}));

                    let call_info = json!({
                        "id": acc.id,
                        "type": "function",
                        "function": { "name": tool_name, "arguments": args_str }
                    });
                    all_calls_json.push(call_info);

                    if let Some(mcp_tool) = server_tools.iter().find(|t| t.name == tool_name) {
                        server_side_calls.push((acc.id, mcp_tool.clone(), args_val));
                    } else {
                        client_side_calls.push(json!({ "id": acc.id, "name": tool_name, "arguments": args_str }));
                    }
                }

                // 保存 assistant 消息（带 tool_calls）
                let assistant_uuid = Uuid::new_v4();
                let assistant_msg = messages::ActiveModel {
                    id: Set(assistant_uuid),
                    chat_id: Set(chat_id),
                    message_id: Set(assistant_message_id),
                    role: Set("assistant".to_string()),
                    message_type: Set(message_types::TOOL_CALL.to_string()),
                    state: Set(message_state.clone()),
                    parts: Set(if full_response.trim().is_empty() {
                        json!([])
                    } else {
                        json!([{ "type": "text", "content": full_response }])
                    }),
                    tool_calls: Set(Some(json!(all_calls_json))),
                    metadata: Set(Some(json!({
                        "model": model_to_use,
                        "usage": accumulated_usage,
                    }))),
                    ..Default::default()
                };
                if let Err(e) = assistant_msg.insert(&db).await {
                    tracing::error!("Failed to save assistant msg: {:?}", e);
                    break;
                }

                // 更新 chat.last_message_id
                let _ = update_chat_last_message_id(&db, chat_id, assistant_message_id).await;

                if !server_side_calls.is_empty() {
                    for (call_id, tool, args) in server_side_calls {
                        // 为每个 tool result 分配 message_id
                        last_message_id += 1;
                        let tool_message_id = last_message_id;

                        match mcp_client.execute_tool(&tool, args).await {
                            Ok(result) => {
                                let tool_msg = messages::ActiveModel {
                                    id: Set(Uuid::new_v4()),
                                    chat_id: Set(chat_id),
                                    message_id: Set(tool_message_id),
                                    role: Set("tool".to_string()),
                                    message_type: Set(message_types::TOOL_RESULT.to_string()),
                                    state: Set(message_state.clone()),
                                    parts: Set(json!([{ "type": "tool_result", "content": result }])),
                                    tool_call_id: Set(Some(call_id)),
                                    ref_message_id: Set(Some(assistant_message_id)),
                                    ..Default::default()
                                };
                                tool_msg.insert(&db).await.ok();
                            },
                            Err(e) => {
                                let err_msg = format!("Error executing {}: {}", tool.name, e);
                                let tool_msg = messages::ActiveModel {
                                    id: Set(Uuid::new_v4()),
                                    chat_id: Set(chat_id),
                                    message_id: Set(tool_message_id),
                                    role: Set("tool".to_string()),
                                    message_type: Set(message_types::TOOL_RESULT.to_string()),
                                    state: Set(message_state.clone()),
                                    parts: Set(json!([{ "type": "tool_result", "content": err_msg }])),
                                    tool_call_id: Set(Some(call_id)),
                                    ref_message_id: Set(Some(assistant_message_id)),
                                    ..Default::default()
                                };
                                tool_msg.insert(&db).await.ok();
                            }
                        }

                        // 更新 chat.last_message_id
                        let _ = update_chat_last_message_id(&db, chat_id, tool_message_id).await;
                    }

                    continue; // 循环顶部会分配新的 assistant_message_id
                }

                if !client_side_calls.is_empty() {
                    yield Ok(InternalStreamEvent::ToolCall { message_id: assistant_message_id, tools: client_side_calls });

                    if let Some(ref mut rx) = tool_result_rx {
                        // 等待前端返回 tool result（5分钟超时 + 取消支持）
                        let results = tokio::select! {
                            results = rx.recv() => match results {
                                Some(r) => r,
                                None => {
                                    yield Ok(InternalStreamEvent::Error("Tool result channel closed".into()));
                                    break;
                                }
                            },
                            _ = tokio::time::sleep(Duration::from_secs(300)) => {
                                yield Ok(InternalStreamEvent::Error("Tool result timeout (5min)".into()));
                                break;
                            },
                            _ = cancel_token.cancelled() => { break; }
                        };

                        // 保存每个 tool result 为消息
                        for result in &results {
                            last_message_id += 1;
                            let tool_msg = messages::ActiveModel {
                                id: Set(Uuid::new_v4()),
                                chat_id: Set(chat_id),
                                message_id: Set(last_message_id),
                                role: Set("tool".to_string()),
                                message_type: Set(message_types::TOOL_RESULT.to_string()),
                                state: Set(message_state.clone()),
                                parts: Set(json!([{ "type": "tool_result", "content": result.content }])),
                                tool_call_id: Set(Some(result.tool_call_id.clone())),
                                ref_message_id: Set(Some(assistant_message_id)),
                                ..Default::default()
                            };
                            tool_msg.insert(&db).await.ok();
                            let _ = update_chat_last_message_id(&db, chat_id, last_message_id).await;
                        }

                        continue; // 继续循环，循环顶部会分配新的 assistant_message_id
                    } else {
                        break; // 无 channel（非流式模式），保持原有行为
                    }
                }

            } else {
                // 保存纯文本 assistant 消息
                let assistant_uuid = Uuid::new_v4();
                let parts_json = json!([{ "type": "text", "content": full_response }]);
                let assistant_msg = messages::ActiveModel {
                    id: Set(assistant_uuid),
                    chat_id: Set(chat_id),
                    message_id: Set(assistant_message_id),
                    role: Set("assistant".to_string()),
                    message_type: Set(message_types::CHAT.to_string()),
                    state: Set(message_state.clone()),
                    parts: Set(parts_json.clone()),
                    metadata: Set(Some(json!({
                        "model": model_to_use,
                        "usage": accumulated_usage,
                    }))),
                    ..Default::default()
                };
                let saved_assistant = assistant_msg.insert(&db).await.ok();

                // 更新 chat.last_message_id
                let _ = update_chat_last_message_id(&db, chat_id, assistant_message_id).await;

                if should_use_memory && saved_assistant.is_some() {
                    let chat_record = chats::Entity::find_by_id(chat_id).one(&db).await.ok().flatten();
                    let agent_id = chat_record.and_then(|c| c.agent_id);

                    let last_messages: Vec<messages::Model> = messages::Entity::find()
                        .filter(messages::Column::ChatId.eq(chat_id))
                        .filter(messages::Column::State.is_in([messages::states::CONTEXT_VISIBLE, messages::states::CONTEXT_HIDDEN]))
                        .order_by_desc(messages::Column::MessageId)
                        .limit(2)
                        .all(&db)
                        .await
                        .unwrap_or_default();

                    let mut mem_messages = Vec::new();
                    for msg in last_messages.into_iter().rev() {
                        if let Ok(parts) = serde_json::from_value::<Vec<MessagePart>>(msg.parts) {
                            for part in parts {
                                if part.part_type == "text" {
                                    mem_messages.push(MessagePart {
                                        role: Some(msg.role.clone()),
                                        content: part.content,
                                        part_type: "text".to_string(),
                                        data: None,
                                        tool_call_id: None,
                                    });
                                }
                            }
                        }
                    }

                    if !mem_messages.is_empty() {
                        let ms = memory_service.clone();
                        let uid = user.id.to_string();
                        let aid = agent_id.map(|id| id.to_string());
                        tokio::spawn(async move {
                            let _ = ms.add_memory(mem_messages, Some(uid), aid, None, None).await;
                        });
                    }
                }

                // 发送完成事件
                yield Ok(InternalStreamEvent::Done { message_id: assistant_message_id, assistant_message_uuid: Some(assistant_uuid) });
                break;
            }
        }
    }
}

/// 将自定义 JSON 数组转为 Vec<messages::Model>
fn parse_custom_history(items: &[Value]) -> Vec<messages::Model> {
    items.iter().filter_map(|item| {
        let role = item["role"].as_str()?.to_string();
        let parts = item.get("parts").cloned().unwrap_or(json!([]));
        Some(messages::Model {
            id: Uuid::nil(),
            chat_id: Uuid::nil(),
            message_id: 0,
            role,
            message_type: "chat".to_string(),
            state: messages::states::CONTEXT_VISIBLE.to_string(),
            parts,
            tool_calls: item.get("tool_calls").cloned(),
            tool_call_id: item.get("tool_call_id").and_then(|v| v.as_str()).map(String::from),
            ref_message_id: None,
            metadata: None,
            created_at: None,
            updated_at: None,
        })
    }).collect()
}

/// 更新 chat.last_message_id
pub async fn update_chat_last_message_id(
    db: &DatabaseConnection,
    chat_id: Uuid,
    message_id: i32,
) -> Result<(), AppError> {
    chats::Entity::update_many()
        .col_expr(chats::Column::LastMessageId, sea_orm::sea_query::Expr::value(message_id))
        .col_expr(
            chats::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(chrono::Utc::now().naive_utc()),
        )
        .filter(chats::Column::Id.eq(chat_id))
        .exec(db)
        .await?;

    Ok(())
}
