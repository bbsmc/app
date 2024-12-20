
CREATE TABLE IF NOT EXISTS wiki_cache (
    id             BIGINT PRIMARY KEY,
    mod_id         BIGINT       NOT NULL REFERENCES mods,
    user_id        BIGINT       NOT NULL REFERENCES users,
    created        timestamptz           DEFAULT CURRENT_TIMESTAMP NOT NULL,
    status        VARCHAR(255) NOT NULL DEFAULT 'draft',
--  status -> draft, review , success, reject
    old          jsonb        NOT NULL DEFAULT '[]'::jsonb, -- 旧的内容
    caches          jsonb        NOT NULL DEFAULT '[]'::jsonb, -- 新的内容
    message          jsonb        NOT NULL DEFAULT '[]'::jsonb,  -- 如果被驳回，驳回的原因和来回重新提交修改的内容
    again_count      bigint NOT NULL DEFAULT 0,
    again_time timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP
)