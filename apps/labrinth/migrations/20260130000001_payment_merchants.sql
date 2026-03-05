-- 支付商户表（卖家支付平台配置）
CREATE TABLE payment_merchants (
    user_id         BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    sid             INTEGER NOT NULL,              -- 支付平台店铺ID
    secret_key      VARCHAR(200) NOT NULL,         -- 支付平台密钥（AES-256-GCM 加密存储）
    verified        BOOLEAN DEFAULT FALSE,         -- 是否已验证
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_payment_merchants_sid ON payment_merchants(sid);
CREATE INDEX idx_payment_merchants_verified ON payment_merchants(verified);

COMMENT ON TABLE payment_merchants IS '支付商户表，存储卖家支付平台配置';
COMMENT ON COLUMN payment_merchants.secret_key IS '支付平台密钥，使用 AES-256-GCM 加密存储';
