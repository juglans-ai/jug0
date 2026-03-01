-- 002_add_username.sql
-- Add GitHub-style username field to users table

-- 1. Add username column if not exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'users' AND column_name = 'username'
    ) THEN
        ALTER TABLE users ADD COLUMN username VARCHAR(50);
    END IF;
END $$;

-- 2. Generate usernames for existing users (based on name or email)
UPDATE users
SET username = LOWER(
    REGEXP_REPLACE(
        COALESCE(name, SPLIT_PART(email, '@', 1), 'user'),
        '[^a-z0-9]', '-', 'g'
    )
) || '-' || SUBSTRING(id::text, 1, 4)
WHERE username IS NULL OR username = '';

-- 3. Handle duplicates by appending random suffix
WITH duplicates AS (
    SELECT id, username,
           ROW_NUMBER() OVER (PARTITION BY username ORDER BY created_at) as rn
    FROM users
)
UPDATE users u
SET username = u.username || '-' || SUBSTRING(gen_random_uuid()::text, 1, 4)
FROM duplicates d
WHERE u.id = d.id AND d.rn > 1;

-- 4. Add unique constraint if not exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'users_username_key'
    ) THEN
        ALTER TABLE users ADD CONSTRAINT users_username_key UNIQUE (username);
    END IF;
END $$;

-- 5. Create index for username lookups
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- 6. Create official system user (if not exists)
INSERT INTO users (id, org_id, username, name, role, created_at)
VALUES (
    uuid_generate_v4(),
    'juglans_official',
    'juglans',
    'Juglans Official',
    'system',
    NOW()
)
ON CONFLICT (username) DO NOTHING;

-- 7. Update official prompts/agents to have system user_id
UPDATE prompts
SET user_id = (SELECT id FROM users WHERE username = 'juglans' LIMIT 1)
WHERE org_id = 'juglans_official' AND user_id IS NULL;

UPDATE agents
SET user_id = (SELECT id FROM users WHERE username = 'juglans' LIMIT 1)
WHERE org_id = 'juglans_official' AND user_id IS NULL;

-- 8. Add user_id and endpoint_url to workflows if not exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'workflows' AND column_name = 'user_id'
    ) THEN
        ALTER TABLE workflows ADD COLUMN user_id UUID;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'workflows' AND column_name = 'endpoint_url'
    ) THEN
        ALTER TABLE workflows ADD COLUMN endpoint_url TEXT;
    END IF;
END $$;
