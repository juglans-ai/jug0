-- 007_add_external_id.sql
-- Add external_id column for arbitrary string chat identifiers (e.g. Feishu group "oc_xxx")

ALTER TABLE chats ADD COLUMN IF NOT EXISTS external_id VARCHAR(255);
CREATE UNIQUE INDEX IF NOT EXISTS idx_chats_org_external_id
  ON chats(org_id, external_id) WHERE external_id IS NOT NULL;
