-- 发起签约时生成的一次性回调 nonce，绑定 pending sign 记录。
-- 回调 URL 必须带回同一个 nonce，避免有效回调体被重放到其他 user_id。
ALTER TABLE user_yunzhanghu_profiles
    ADD COLUMN sign_nonce text;
