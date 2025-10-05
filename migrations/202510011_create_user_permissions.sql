CREATE TABLE user_permissions (
    pubkey TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    permission TEXT NOT NULL,
    PRIMARY KEY (pubkey, endpoint)
);
