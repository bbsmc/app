ALTER TABLE users ADD COLUMN wechat_id text;
CREATE INDEX users_wechat_id ON users (wechat_id);
