-- 003_add_message_id.sql
-- Add fine-grained message tracking: message_id, message_type, stateless, ref_message_id, metadata

-- ============================================
-- Part 1: Add new columns to messages table
-- ============================================

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'messages' AND column_name = 'message_id') THEN
        ALTER TABLE messages ADD COLUMN message_id INTEGER;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'messages' AND column_name = 'message_type') THEN
        ALTER TABLE messages ADD COLUMN message_type VARCHAR(50) DEFAULT 'chat';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'messages' AND column_name = 'stateless') THEN
        ALTER TABLE messages ADD COLUMN stateless BOOLEAN DEFAULT FALSE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'messages' AND column_name = 'ref_message_id') THEN
        ALTER TABLE messages ADD COLUMN ref_message_id INTEGER;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'messages' AND column_name = 'metadata') THEN
        ALTER TABLE messages ADD COLUMN metadata JSONB;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'messages' AND column_name = 'updated_at') THEN
        ALTER TABLE messages ADD COLUMN updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP;
    END IF;
END $$;

-- ============================================
-- Part 2: Add last_message_id and metadata to chats
-- ============================================

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'chats' AND column_name = 'last_message_id') THEN
        ALTER TABLE chats ADD COLUMN last_message_id INTEGER DEFAULT 0;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'chats' AND column_name = 'metadata') THEN
        ALTER TABLE chats ADD COLUMN metadata JSONB;
    END IF;
END $$;

-- ============================================
-- Part 3: Backfill message_id for existing messages
-- ============================================

WITH ranked AS (
    SELECT id, chat_id,
           ROW_NUMBER() OVER (PARTITION BY chat_id ORDER BY created_at, id) as rn
    FROM messages
    WHERE message_id IS NULL
)
UPDATE messages m
SET message_id = r.rn
FROM ranked r
WHERE m.id = r.id;

UPDATE chats c
SET last_message_id = COALESCE(
    (SELECT MAX(message_id) FROM messages m WHERE m.chat_id = c.id),
    0
);

-- ============================================
-- Part 4: Set NOT NULL constraints (after backfill)
-- ============================================

DO $$
BEGIN
    UPDATE messages SET message_id = 1 WHERE message_id IS NULL;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'messages' AND column_name = 'message_id' AND is_nullable = 'YES') THEN
        ALTER TABLE messages ALTER COLUMN message_id SET NOT NULL;
    END IF;
END $$;

DO $$
BEGIN
    UPDATE messages SET message_type = 'chat' WHERE message_type IS NULL;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'messages' AND column_name = 'message_type' AND is_nullable = 'YES') THEN
        ALTER TABLE messages ALTER COLUMN message_type SET NOT NULL;
        ALTER TABLE messages ALTER COLUMN message_type SET DEFAULT 'chat';
    END IF;
END $$;

DO $$
BEGIN
    UPDATE messages SET stateless = FALSE WHERE stateless IS NULL;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'messages' AND column_name = 'stateless' AND is_nullable = 'YES') THEN
        ALTER TABLE messages ALTER COLUMN stateless SET NOT NULL;
        ALTER TABLE messages ALTER COLUMN stateless SET DEFAULT FALSE;
    END IF;
END $$;

-- ============================================
-- Part 5: Add constraints
-- ============================================

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'unique_chat_message_id') THEN
        ALTER TABLE messages ADD CONSTRAINT unique_chat_message_id UNIQUE(chat_id, message_id);
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'valid_role') THEN
        ALTER TABLE messages ADD CONSTRAINT valid_role CHECK (role IN ('user', 'assistant', 'tool', 'system'));
    END IF;
EXCEPTION
    WHEN check_violation THEN
        UPDATE messages SET role = 'user' WHERE role NOT IN ('user', 'assistant', 'tool', 'system');
        ALTER TABLE messages ADD CONSTRAINT valid_role CHECK (role IN ('user', 'assistant', 'tool', 'system'));
END $$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'valid_message_type') THEN
        ALTER TABLE messages ADD CONSTRAINT valid_message_type
        CHECK (message_type IN ('chat', 'command', 'command_result', 'tool_call', 'tool_result', 'system'));
    END IF;
EXCEPTION
    WHEN check_violation THEN
        UPDATE messages SET message_type = 'chat'
        WHERE message_type NOT IN ('chat', 'command', 'command_result', 'tool_call', 'tool_result', 'system');
        ALTER TABLE messages ADD CONSTRAINT valid_message_type
        CHECK (message_type IN ('chat', 'command', 'command_result', 'tool_call', 'tool_result', 'system'));
END $$;

-- ============================================
-- Part 6: Create indexes
-- ============================================

CREATE INDEX IF NOT EXISTS idx_messages_chat_message_id ON messages(chat_id, message_id);
CREATE INDEX IF NOT EXISTS idx_messages_chat_stateless ON messages(chat_id, stateless);
CREATE INDEX IF NOT EXISTS idx_messages_message_type ON messages(message_type);

-- ============================================
-- Part 7: Helper function for next message_id
-- ============================================

CREATE OR REPLACE FUNCTION next_message_id(p_chat_id UUID)
RETURNS INTEGER AS $$
DECLARE
    next_id INTEGER;
BEGIN
    SELECT COALESCE(last_message_id, 0) + 1 INTO next_id
    FROM chats
    WHERE id = p_chat_id;
    IF next_id IS NULL THEN
        next_id := 1;
    END IF;
    RETURN next_id;
END;
$$ LANGUAGE plpgsql;
