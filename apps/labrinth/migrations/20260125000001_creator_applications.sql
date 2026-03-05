-- 高级创作者申请表
CREATE TABLE creator_applications (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    real_name       VARCHAR(100) NOT NULL,              -- 真实姓名
    contact_info    VARCHAR(255) NOT NULL,              -- 联系方式（手机/QQ/微信）
    id_card_number  VARCHAR(100),                       -- 身份证号（加密存储）
    portfolio_links TEXT,                               -- 作品链接（JSON数组）
    application_reason TEXT,                            -- 申请理由/自我介绍
    status          VARCHAR(20) DEFAULT 'pending' NOT NULL,  -- pending/approved/rejected
    reviewer_id     BIGINT REFERENCES users(id),        -- 审核人
    review_note     TEXT,                               -- 审核备注
    created_at      TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    reviewed_at     TIMESTAMPTZ
);

-- 索引：按用户ID查询
CREATE INDEX idx_creator_applications_user_id ON creator_applications(user_id);

-- 索引：按状态查询（管理员审核列表）
CREATE INDEX idx_creator_applications_status ON creator_applications(status);

-- 索引：按创建时间排序
CREATE INDEX idx_creator_applications_created_at ON creator_applications(created_at DESC);

-- 在 users 表添加高级创作者标记
ALTER TABLE users ADD COLUMN is_premium_creator BOOLEAN DEFAULT FALSE NOT NULL;
ALTER TABLE users ADD COLUMN creator_verified_at TIMESTAMPTZ;

-- 索引：快速查询高级创作者
CREATE INDEX idx_users_is_premium_creator ON users(is_premium_creator) WHERE is_premium_creator = TRUE;

