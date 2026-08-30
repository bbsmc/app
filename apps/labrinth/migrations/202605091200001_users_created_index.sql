-- 为按时间分桶查询注册趋势提供索引支持
CREATE INDEX IF NOT EXISTS idx_users_created ON users (created);
