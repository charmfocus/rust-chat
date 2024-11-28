-- Add migration script here

-- create user table
CREATE TABLE IF NOT EXISTS users (
    id BIGSERIAL PRIMARY KEY,
    fullname VARCHAR(64) NOT NULL UNIQUE,
    email VARCHAR(64) NOT NULL UNIQUE,
    -- password is stored as a hash argon2
    password_hash VARCHAR(97) NOT NULL,
    workspace_id BIGINT NOT NULL,
    -- workspace name
    workspace varchar(32) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- create unique index for users for email
CREATE UNIQUE INDEX IF NOT EXISTS idx_email ON users (email);

-- add index for users for workspace_id
CREATE INDEX IF NOT EXISTS idx_workspace_id ON users (workspace_id);

-- workspace for users
CREATE TABLE IF NOT EXISTS workspaces (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(32) NOT NULL UNIQUE,
    owner_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_workspace_name ON workspaces (name);
CREATE INDEX IF NOT EXISTS idx_owner_id ON workspaces (owner_id);

-- create chat type: single group privete_channel public_channel
CREATE TYPE chat_type AS ENUM ('single', 'group', 'private_channel', 'public_channel');

-- create chat table
CREATE TABLE IF NOT EXISTS chats (
    id BIGSERIAL PRIMARY KEY,
    workspace_id BIGINT NOT NULL,
    name VARCHAR(64),
    type chat_type NOT NULL,
    members BIGINT[] NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- add index for chats for workspace_id
CREATE INDEX IF NOT EXISTS idx_workspace_id ON chats (workspace_id);

-- create message table
CREATE TABLE IF NOT EXISTS messages (
    id BIGSERIAL PRIMARY KEY,
    chat_id BIGINT NOT NULL,
    sender_id BIGINT NOT NULL,
    content TEXT NOT NULL,
    files TEXT[] DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- create index for messages for chat_id and crated_at order by desc
CREATE INDEX IF NOT EXISTS idx_chat_id_created_at ON messages (chat_id, created_at DESC);

-- create index for message for sender_id
CREATE INDEX IF NOT EXISTS idx_sender_id ON messages (sender_id, created_at DESC);
