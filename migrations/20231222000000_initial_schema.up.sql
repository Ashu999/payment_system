CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    balance DECIMAL(19,2) NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- ENUM types for transaction types and status
CREATE TYPE transaction_type AS ENUM ('SENT', 'RECEIVED');
CREATE TYPE transaction_status AS ENUM ('SUCCESS', 'FAILURE');

CREATE TABLE transactions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    transaction_type transaction_type NOT NULL,
    amount DECIMAL(19,2) NOT NULL,
    status transaction_status NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX idx_transactions_user_id ON transactions(user_id);