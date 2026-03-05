CREATE TABLE user_profile_reviews (
    id bigint PRIMARY KEY,
    user_id bigint NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    review_type varchar(32) NOT NULL,  -- 'avatar', 'username', 'bio'
    old_value text,                     -- 旧值（username/bio 为文本，avatar 为 JSON）
    new_value text NOT NULL,            -- 新值（username/bio 为文本，avatar 为 JSON）
    risk_labels text NOT NULL,          -- 风控标签
    status varchar(32) NOT NULL DEFAULT 'pending',  -- pending/approved/rejected/cancelled
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reviewed_by bigint REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at timestamptz,
    review_notes text
);

CREATE INDEX user_profile_reviews_user_idx ON user_profile_reviews (user_id, status);
CREATE INDEX user_profile_reviews_status_idx ON user_profile_reviews (status, created_at DESC);
CREATE UNIQUE INDEX user_profile_reviews_pending_idx
    ON user_profile_reviews (user_id, review_type) WHERE status = 'pending';
