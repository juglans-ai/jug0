-- 005_add_handles.sql
-- Create handles table for @username mapping

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'handles') THEN
        CREATE TABLE handles (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            org_id VARCHAR NOT NULL,
            handle VARCHAR(50) NOT NULL,
            target_type VARCHAR(20) NOT NULL,  -- 'agent' | 'user'
            target_id UUID NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        );

        CREATE UNIQUE INDEX idx_handles_org_handle ON handles(org_id, handle);
        CREATE INDEX idx_handles_target ON handles(target_type, target_id);

        RAISE NOTICE 'Created handles table';
    ELSE
        RAISE NOTICE 'handles table already exists';
    END IF;
END $$;

-- Add username column to agents table if not exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'agents' AND column_name = 'username'
    ) THEN
        ALTER TABLE agents ADD COLUMN username VARCHAR(50);
        CREATE INDEX idx_agents_username ON agents(org_id, username) WHERE username IS NOT NULL;
        RAISE NOTICE 'Added username column to agents table';
    ELSE
        RAISE NOTICE 'agents.username column already exists';
    END IF;
END $$;

-- Create handles for existing users in their orgs
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT u.id as user_id, u.username, u.org_id
        FROM users u
        WHERE u.username IS NOT NULL
          AND u.org_id IS NOT NULL
          AND NOT EXISTS (
              SELECT 1 FROM handles h
              WHERE h.org_id = u.org_id
                AND h.handle = u.username
          )
    LOOP
        INSERT INTO handles (org_id, handle, target_type, target_id)
        VALUES (r.org_id, r.username, 'user', r.user_id);
        RAISE NOTICE 'Created handle @% for user % in org %', r.username, r.user_id, r.org_id;
    END LOOP;
END $$;
