-- 006_add_persistent_handle_chat_index.sql
-- Add index for persistent @handle conversations (Telegram-style DM)
-- Enables efficient lookup: find user's existing chat with a specific agent

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE indexname = 'idx_chats_org_user_agent'
    ) THEN
        CREATE INDEX idx_chats_org_user_agent
        ON chats(org_id, user_id, agent_id)
        WHERE agent_id IS NOT NULL;

        RAISE NOTICE 'Created index idx_chats_org_user_agent for persistent @handle conversations';
    ELSE
        RAISE NOTICE 'Index idx_chats_org_user_agent already exists';
    END IF;
END $$;
