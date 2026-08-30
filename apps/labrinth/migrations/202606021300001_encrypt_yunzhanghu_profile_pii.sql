-- Add encrypted storage columns for Yunzhanghu profile PII.
--
-- The legacy plaintext columns remain for compatibility with already-migrated
-- databases. Application startup backfills those values into these encrypted
-- columns and clears the plaintext columns.
ALTER TABLE user_yunzhanghu_profiles
    ADD COLUMN real_name_encrypted text,
    ADD COLUMN phone_encrypted text,
    ADD COLUMN alipay_account_encrypted text;
