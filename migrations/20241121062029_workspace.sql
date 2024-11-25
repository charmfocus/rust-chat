-- Add migration script here
CREATE TABLE IF NOT EXISTS workspaces (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(32) NOT NULL UNIQUE,
    owner_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_workspace_name ON workspaces (name);
CREATE INDEX IF NOT EXISTS idx_owner_id ON workspaces (owner_id);

-- alter users table to add workspace_id
ALTER TABLE users ADD COLUMN workspace_id BIGINT NOT NULL;

-- alter chats table to add workspace_id
ALTER TABLE chats ADD COLUMN workspace_id BIGINT NOT NULL;

-- add index for users for workspace_id
CREATE INDEX IF NOT EXISTS idx_workspace_id ON users (workspace_id);

-- add index for chats for workspace_id
CREATE INDEX IF NOT EXISTS idx_workspace_id ON chats (workspace_id);
