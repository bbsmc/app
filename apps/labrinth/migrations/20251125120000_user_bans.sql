-- 用户封禁系统

-- 1. 封禁主表
CREATE TABLE user_bans (
    id bigint PRIMARY KEY,
    user_id bigint NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ban_type varchar(64) NOT NULL,  -- global, resource, forum
    reason text NOT NULL,
    internal_reason text,
    banned_by bigint NOT NULL REFERENCES users(id),
    banned_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at timestamptz,
    is_active boolean NOT NULL DEFAULT true,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb
);

-- 确保同一用户的同一类型封禁只有一个活跃记录（部分唯一索引）
CREATE UNIQUE INDEX user_bans_unique_active_idx ON user_bans (user_id, ban_type) WHERE is_active = true;
CREATE INDEX user_bans_user_active_idx ON user_bans (user_id, is_active, expires_at) WHERE is_active = true;
CREATE INDEX user_bans_type_idx ON user_bans (ban_type, is_active);
CREATE INDEX user_bans_expires_idx ON user_bans (expires_at) WHERE expires_at IS NOT NULL AND is_active = true;
CREATE INDEX user_bans_active_idx ON user_bans (is_active, banned_at DESC);
CREATE INDEX user_bans_banned_by_idx ON user_bans (banned_by, banned_at DESC);

-- 2. 封禁操作历史表
CREATE TABLE user_ban_history (
    id bigint PRIMARY KEY,
    ban_id bigint NOT NULL REFERENCES user_bans(id) ON DELETE CASCADE,
    action varchar(32) NOT NULL,  -- created, modified, revoked
    operator_id bigint NOT NULL REFERENCES users(id),
    operated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    old_data jsonb,
    new_data jsonb NOT NULL,
    reason text NOT NULL
);

CREATE INDEX user_ban_history_ban_idx ON user_ban_history (ban_id, operated_at DESC);
CREATE INDEX user_ban_history_operator_idx ON user_ban_history (operator_id, operated_at DESC);

-- 3. 封禁申诉表
CREATE TABLE user_ban_appeals (
    id bigint PRIMARY KEY,
    ban_id bigint NOT NULL UNIQUE REFERENCES user_bans(id) ON DELETE CASCADE,
    user_id bigint NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason text NOT NULL,
    status varchar(32) NOT NULL DEFAULT 'pending',  -- pending, approved, rejected
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reviewed_by bigint REFERENCES users(id),
    reviewed_at timestamptz,
    review_notes text,
    thread_id bigint
);

CREATE INDEX user_ban_appeals_user_idx ON user_ban_appeals (user_id, created_at DESC);
CREATE INDEX user_ban_appeals_status_idx ON user_ban_appeals (status, created_at DESC);
CREATE INDEX user_ban_appeals_reviewed_idx ON user_ban_appeals (reviewed_by, reviewed_at DESC) WHERE reviewed_by IS NOT NULL;

-- 4. 扩展 threads 表支持封禁申诉
ALTER TABLE threads ADD COLUMN ban_appeal_id bigint REFERENCES user_ban_appeals(id) ON DELETE SET NULL;
CREATE INDEX threads_ban_appeal_idx ON threads (ban_appeal_id) WHERE ban_appeal_id IS NOT NULL;

-- 5. 添加申诉表的线程外键（循环引用需要延迟添加）
ALTER TABLE user_ban_appeals ADD CONSTRAINT user_ban_appeals_thread_fkey FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE SET NULL;
