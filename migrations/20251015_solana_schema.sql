-- Solana blockchain API schema
-- Created: 2025-10-15
-- Tables: solana_transactions, users, user_permissions

-- Extension for UUID support
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users table: stores Solana wallet public keys and their roles
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    pubkey VARCHAR(44) NOT NULL UNIQUE, -- Solana base58 pubkey (32 bytes = 44 chars)
    role VARCHAR(50) NOT NULL DEFAULT 'user', -- user, admin, service
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for fast pubkey lookups
CREATE INDEX IF NOT EXISTS idx_users_pubkey ON users(pubkey);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);

-- User permissions table: endpoint-level access control
CREATE TABLE IF NOT EXISTS user_permissions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    pubkey VARCHAR(44) NOT NULL REFERENCES users(pubkey) ON DELETE CASCADE,
    endpoint VARCHAR(255) NOT NULL, -- e.g. "/api/transactions", "/api/users"
    permission VARCHAR(50) NOT NULL, -- read, write, admin
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(pubkey, endpoint)
);

-- Indexes for permissions
CREATE INDEX IF NOT EXISTS idx_user_permissions_pubkey ON user_permissions(pubkey);
CREATE INDEX IF NOT EXISTS idx_user_permissions_endpoint ON user_permissions(endpoint);

-- Solana transactions table: stores indexed blockchain transactions
CREATE TABLE IF NOT EXISTS solana_transactions (
    signature VARCHAR(88) PRIMARY KEY, -- Solana signature (64 bytes = 88 chars base58)
    slot BIGINT NOT NULL,
    from_pubkey VARCHAR(44), -- Source wallet (can be NULL for some tx types)
    to_pubkey VARCHAR(44),   -- Destination wallet (can be NULL for some tx types)
    lamports BIGINT,         -- Amount transferred in lamports (1 SOL = 1B lamports)
    program_ids TEXT[],      -- Array of program IDs involved in the transaction
    instructions JSONB DEFAULT '[]'::jsonb, -- Transaction instructions as JSON
    block_time BIGINT,       -- Unix timestamp (seconds) when block was produced
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for solana_transactions
CREATE INDEX IF NOT EXISTS idx_solana_tx_slot ON solana_transactions(slot);
CREATE INDEX IF NOT EXISTS idx_solana_tx_from_to ON solana_transactions(from_pubkey, to_pubkey);
CREATE INDEX IF NOT EXISTS idx_solana_tx_block_time ON solana_transactions(block_time DESC);

-- GIN indexes for array and JSONB columns (for efficient filtering)
CREATE INDEX IF NOT EXISTS idx_solana_tx_program_ids_gin ON solana_transactions USING GIN(program_ids);
CREATE INDEX IF NOT EXISTS idx_solana_tx_instructions_gin ON solana_transactions USING GIN(instructions jsonb_path_ops);

-- Comments for documentation
COMMENT ON TABLE users IS 'Registered users (Solana wallet public keys)';
COMMENT ON TABLE user_permissions IS 'Endpoint-level access control for users';
COMMENT ON TABLE solana_transactions IS 'Indexed Solana blockchain transactions';
COMMENT ON COLUMN solana_transactions.signature IS 'Unique transaction signature (base58)';
COMMENT ON COLUMN solana_transactions.program_ids IS 'Array of Solana program IDs involved';
COMMENT ON COLUMN solana_transactions.instructions IS 'Transaction instructions in JSON format';

