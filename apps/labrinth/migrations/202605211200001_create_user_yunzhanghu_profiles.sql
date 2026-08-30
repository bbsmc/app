-- 用户云账户(Yunzhanghu)实名 + 收款账号 + 签约信息
--
-- 与 users 1:1 关系（用户尝试提现时按需创建）；身份证号通过 AES-256-GCM
-- 加密存储（密钥由 ID_CARD_ENCRYPTION_KEY env 提供），明文绝不落盘。
CREATE TABLE user_yunzhanghu_profiles (
    user_id              bigint      PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,

    real_name            text,
    -- AES-GCM 密文，base64(nonce(12B) || ciphertext+tag) 由 util::encrypt 生成
    id_card_encrypted    text,
    -- 身份证号末 4 位，仅用于前端脱敏展示
    id_card_last4        text,
    phone                text,
    -- 支付宝账号（手机号或邮箱）
    alipay_account       text,

    -- 签约状态：unsigned(未签约) / signing(签约中) / signed(已签约) / terminated(已解约)
    sign_status          text        NOT NULL DEFAULT 'unsigned',
    sign_url             text,
    signed_at            timestamptz,
    terminated_at        timestamptz,

    created_at           timestamptz NOT NULL DEFAULT NOW(),
    updated_at           timestamptz NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_yzh_profiles_sign_status
    ON user_yunzhanghu_profiles (sign_status);
