-- schema.sql
-- jug0 complete database schema (source of truth)
-- This file represents the CURRENT state of the database after all migrations.
-- Use this to bootstrap a fresh database: psql -f db/schema.sql

-- ============================================
-- Extensions
-- ============================================

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS vector;

-- ============================================
-- Tables
-- ============================================

-- 0. Organizations
CREATE TABLE IF NOT EXISTS organizations (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    api_key_hash VARCHAR(255) NOT NULL,
    public_key TEXT,
    key_algorithm VARCHAR(20) DEFAULT 'RS256',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 1. Users
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    org_id VARCHAR(50) REFERENCES organizations(id),
    external_id VARCHAR(255),
    email VARCHAR(255),
    password_hash VARCHAR(255),
    name VARCHAR(255),
    username VARCHAR(50) UNIQUE,
    role VARCHAR(50) DEFAULT 'user',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uniq_org_external_id UNIQUE NULLS NOT DISTINCT (org_id, external_id)
);

-- 2. Prompts
CREATE TABLE IF NOT EXISTS prompts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    slug VARCHAR(255) NOT NULL,
    org_id VARCHAR(50) REFERENCES organizations(id),
    user_id UUID,
    name VARCHAR(255),
    content TEXT NOT NULL,
    input_variables JSONB DEFAULT '[]',
    type VARCHAR(50) DEFAULT 'user',
    tags JSONB DEFAULT '[]',
    allowed_agent_slugs JSONB DEFAULT '["*"]',
    is_public BOOLEAN DEFAULT FALSE,
    is_system BOOLEAN DEFAULT FALSE,
    usage_count INT DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uniq_org_user_slug UNIQUE NULLS NOT DISTINCT (org_id, user_id, slug)
);

-- 3. Agents
CREATE TABLE IF NOT EXISTS agents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    slug VARCHAR(255) NOT NULL,
    org_id VARCHAR(50) REFERENCES organizations(id),
    user_id UUID,
    name VARCHAR(255),
    description TEXT,
    system_prompt_id UUID REFERENCES prompts(id),
    allowed_models JSONB DEFAULT '["gpt-4o"]',
    default_model VARCHAR(100) DEFAULT 'gpt-4o',
    temperature FLOAT DEFAULT 0.7,
    mcp_config JSONB DEFAULT '[]',
    skills JSONB DEFAULT '[]',
    fork_from_id UUID REFERENCES agents(id),
    workflow_id UUID,
    is_public BOOLEAN DEFAULT FALSE,
    username VARCHAR(50),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uniq_org_slug UNIQUE NULLS NOT DISTINCT (org_id, slug)
);

-- 4. Workflows
CREATE TABLE IF NOT EXISTS workflows (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    slug VARCHAR(255) NOT NULL,
    org_id VARCHAR(50) REFERENCES organizations(id),
    user_id UUID,
    name VARCHAR(255),
    description TEXT,
    endpoint_url TEXT,
    trigger_config JSONB,
    definition JSONB,
    is_active BOOLEAN DEFAULT TRUE,
    is_public BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Add FK for agents.workflow_id after workflows table exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'agents_workflow_id_fkey'
    ) THEN
        ALTER TABLE agents ADD CONSTRAINT agents_workflow_id_fkey
            FOREIGN KEY (workflow_id) REFERENCES workflows(id);
    END IF;
END $$;

-- 5. Chats
CREATE TABLE IF NOT EXISTS chats (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    org_id VARCHAR(50) REFERENCES organizations(id),
    user_id UUID,
    agent_id UUID REFERENCES agents(id),
    external_id VARCHAR(255),
    title VARCHAR(255),
    model VARCHAR(100),
    last_message_id INTEGER DEFAULT 0,
    metadata JSONB,
    incognito BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 6. Messages
CREATE TABLE IF NOT EXISTS messages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    chat_id UUID NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    message_id INTEGER NOT NULL,
    role VARCHAR(50) NOT NULL,
    message_type VARCHAR(50) NOT NULL DEFAULT 'chat',
    state VARCHAR(50) NOT NULL DEFAULT 'context_visible',
    parts JSONB NOT NULL DEFAULT '[]',
    tool_calls JSONB,
    tool_call_id TEXT,
    ref_message_id INTEGER,
    metadata JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT unique_chat_message_id UNIQUE(chat_id, message_id),
    CONSTRAINT valid_role CHECK (role IN ('user', 'assistant', 'tool', 'system')),
    CONSTRAINT valid_message_type CHECK (message_type IN (
        'chat', 'command', 'command_result', 'tool_call', 'tool_result', 'system'
    ))
);

-- 7. API Keys
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES users(id),
    name VARCHAR(255),
    prefix VARCHAR(50),
    key_hash VARCHAR(255),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,
    last_used_at TIMESTAMP
);

-- 8. Models
CREATE TABLE IF NOT EXISTS models (
    id VARCHAR(100) PRIMARY KEY,
    provider VARCHAR(50) NOT NULL,
    name VARCHAR(200),
    owned_by VARCHAR(100),
    context_length INTEGER,
    capabilities JSONB DEFAULT '{}',
    pricing JSONB,
    raw_data JSONB,
    is_available BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 9. Model Sync Log
CREATE TABLE IF NOT EXISTS model_sync_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    provider VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL,
    model_count INTEGER,
    error_message TEXT,
    synced_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 10. Handles
CREATE TABLE IF NOT EXISTS handles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id VARCHAR NOT NULL,
    handle VARCHAR(50) NOT NULL,
    target_type VARCHAR(20) NOT NULL,
    target_id UUID NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================
-- Indexes
-- ============================================

-- Users
CREATE INDEX IF NOT EXISTS idx_users_external_id ON users(external_id);
CREATE INDEX IF NOT EXISTS idx_users_org_id ON users(org_id);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username_unique ON users(username) WHERE username IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_org_id_external_id ON users(org_id, external_id);

-- Prompts
CREATE INDEX IF NOT EXISTS idx_prompts_slug ON prompts(slug);
CREATE INDEX IF NOT EXISTS idx_prompts_user_id ON prompts(user_id);
CREATE INDEX IF NOT EXISTS idx_prompts_org_id_user_id ON prompts(org_id, user_id);
CREATE INDEX IF NOT EXISTS idx_prompts_org_id_is_public ON prompts(org_id, is_public);
CREATE INDEX IF NOT EXISTS idx_prompts_usage_count ON prompts(usage_count DESC);
CREATE INDEX IF NOT EXISTS idx_prompts_created_at ON prompts(created_at DESC);

-- Agents
CREATE INDEX IF NOT EXISTS idx_agents_slug ON agents(slug);
CREATE INDEX IF NOT EXISTS idx_agents_user_id ON agents(user_id);
CREATE INDEX IF NOT EXISTS idx_agents_org_id_user_id ON agents(org_id, user_id);
CREATE INDEX IF NOT EXISTS idx_agents_org_id_is_public ON agents(org_id, is_public);
CREATE INDEX IF NOT EXISTS idx_agents_created_at ON agents(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_agents_username ON agents(org_id, username) WHERE username IS NOT NULL;

-- Workflows
CREATE INDEX IF NOT EXISTS idx_workflows_user_id ON workflows(user_id);
CREATE INDEX IF NOT EXISTS idx_workflows_org_id_user_id ON workflows(org_id, user_id);
CREATE INDEX IF NOT EXISTS idx_workflows_org_id_is_public ON workflows(org_id, is_public);
CREATE INDEX IF NOT EXISTS idx_workflows_created_at ON workflows(created_at DESC);

-- Chats
CREATE INDEX IF NOT EXISTS idx_chats_user_id ON chats(user_id);
CREATE INDEX IF NOT EXISTS idx_chats_updated_at ON chats(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_chats_org_user_agent ON chats(org_id, user_id, agent_id) WHERE agent_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_chats_org_external_id ON chats(org_id, external_id) WHERE external_id IS NOT NULL;

-- Messages
CREATE INDEX IF NOT EXISTS idx_messages_chat_id ON messages(chat_id);
CREATE INDEX IF NOT EXISTS idx_messages_chat_message_id ON messages(chat_id, message_id);
CREATE INDEX IF NOT EXISTS idx_messages_chat_state ON messages(chat_id, state);
CREATE INDEX IF NOT EXISTS idx_messages_message_type ON messages(message_type);
CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at);

-- Models
CREATE INDEX IF NOT EXISTS idx_models_provider ON models(provider);
CREATE INDEX IF NOT EXISTS idx_models_is_available ON models(is_available);

-- Model Sync Log
CREATE INDEX IF NOT EXISTS idx_model_sync_log_provider ON model_sync_log(provider);
CREATE INDEX IF NOT EXISTS idx_model_sync_log_synced_at ON model_sync_log(synced_at DESC);

-- Handles
CREATE UNIQUE INDEX IF NOT EXISTS idx_handles_org_handle ON handles(org_id, handle);
CREATE INDEX IF NOT EXISTS idx_handles_target ON handles(target_type, target_id);

-- ============================================
-- Functions
-- ============================================

CREATE OR REPLACE FUNCTION next_message_id(p_chat_id UUID)
RETURNS INTEGER AS $$
DECLARE
    next_id INTEGER;
BEGIN
    SELECT COALESCE(last_message_id, 0) + 1 INTO next_id
    FROM chats
    WHERE id = p_chat_id;
    RETURN COALESCE(next_id, 1);
END;
$$ LANGUAGE plpgsql;
