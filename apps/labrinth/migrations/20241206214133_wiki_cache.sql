
CREATE TABLE IF NOT EXISTS wiki_cache (
    id             BIGINT PRIMARY KEY,
    mod_id         BIGINT       NOT NULL REFERENCES mods,
    user_id        BIGINT       NOT NULL REFERENCES users,
    created        timestamptz           DEFAULT CURRENT_TIMESTAMP NOT NULL,
    status        VARCHAR(255) NOT NULL DEFAULT 'draft',
    caches          jsonb        NOT NULL DEFAULT '[]'::jsonb
)