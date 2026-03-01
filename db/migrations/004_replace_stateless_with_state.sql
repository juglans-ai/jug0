-- 004_replace_stateless_with_state.sql
-- Replace stateless BOOLEAN with fine-grained state VARCHAR(50):
--   context_visible (default): AI context visible + SSE output
--   context_hidden: AI context visible, no SSE
--   display_only: SSE output only, not in AI context
--   silent: neither

-- 1. Add state column
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'messages' AND column_name = 'state'
    ) THEN
        ALTER TABLE messages ADD COLUMN state VARCHAR(50) DEFAULT 'context_visible';
    END IF;
END $$;

-- 2. Migrate data: stateless=true -> silent, stateless=false -> context_visible
UPDATE messages SET state = CASE
    WHEN stateless = true THEN 'silent'
    ELSE 'context_visible'
END
WHERE state IS NULL OR state = 'context_visible';

-- 3. Set NOT NULL
ALTER TABLE messages ALTER COLUMN state SET NOT NULL;
ALTER TABLE messages ALTER COLUMN state SET DEFAULT 'context_visible';

-- 4. Drop old column
ALTER TABLE messages DROP COLUMN IF EXISTS stateless;

-- 5. Update indexes
DROP INDEX IF EXISTS idx_messages_chat_stateless;
CREATE INDEX IF NOT EXISTS idx_messages_chat_state ON messages(chat_id, state);
