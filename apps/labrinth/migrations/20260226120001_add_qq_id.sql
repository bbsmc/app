ALTER TABLE users ADD COLUMN qq_id text;
CREATE INDEX users_qq_id ON users (qq_id);
