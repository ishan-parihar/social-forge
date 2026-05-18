-- Add auth_method to distinguish how an integration was connected
ALTER TABLE integrations ADD COLUMN IF NOT EXISTS auth_method TEXT NOT NULL DEFAULT 'oauth';
