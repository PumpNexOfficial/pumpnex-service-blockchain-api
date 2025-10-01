CREATE TABLE solana_transactions (
    id UUID PRIMARY KEY,
    signature TEXT NOT NULL,
    from_pubkey TEXT NOT NULL,
    to_pubkey TEXT,
    instructions JSONB NOT NULL,
    lamports BIGINT,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    slot BIGINT
);
