-- 讨论主题表
CREATE TABLE discussions (
    id BIGINT PRIMARY KEY,
    title VARCHAR(1000) NOT NULL DEFAULT '',
    category VARCHAR(100) NOT NULL, -- 讨论分类
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP, -- 最后一次被回复的时间
    user_id BIGINT NOT NULL REFERENCES users,
    state VARCHAR NOT NULL DEFAULT 'open', -- 'open', 'locked'
    pinnde BOOLEAN NOT NULL DEFAULT false, 
    deleted BOOLEAN NOT NULL DEFAULT false, -- 软删除
    deleted_at TIMESTAMPTZ -- 删除时间
);

-- 帖子表
CREATE TABLE posts (
    id BIGINT PRIMARY KEY,
    discussion_id BIGINT NOT NULL REFERENCES discussions(id), -- 关联 discussions 表
    floor_number BIGINT NOT NULL, -- 楼层号（每个discussion独立）
    content VARCHAR(65536) NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_id BIGINT NOT NULL REFERENCES users,
    replied_to BIGINT NULL, -- 允许为 NULL
    deleted BOOLEAN NOT NULL DEFAULT false, -- 软删除
    deleted_at TIMESTAMPTZ, -- 删除时间

    UNIQUE (discussion_id, floor_number) -- 确保每个讨论的楼层号唯一
);


-- 假设 discussion_id 为 1，用户 ID 为 1
DO $$
DECLARE
    new_floor_number BIGINT;
    discussion_id BIGINT := 1; -- 替换为你的讨论 ID
BEGIN
    -- 获取当前讨论的最大楼层号
    SELECT COALESCE(MAX(floor_number), 0) + 1 INTO new_floor_number
    FROM posts
    WHERE discussion_id = discussion_id;

    -- 插入新帖子
    INSERT INTO posts (
        id,
        discussion_id,
        floor_number,
        content,
        created_at,
        updated_at,
        user_id,
        replied_to
    ) VALUES (
        (discussion_id * 1000 + new_floor_number), -- 生成唯一 ID
        discussion_id,
        new_floor_number, -- 使用计算出的楼层号
        '这是第 ' || new_floor_number || ' 楼的内容', -- 帖子内容
        NOW(), -- 创建时间
        NOW(), -- 更新时间
        1, -- 假设用户 ID 为 1
        NULL -- 假设没有回复
    );
END $$;