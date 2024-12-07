-- WIKI 页面 数据表
CREATE TABLE IF NOT EXISTS wikis
(
    id             BIGINT PRIMARY KEY,
    mod_id         BIGINT       NOT NULL REFERENCES mods,
    -- 将sort_order字段类型修改为SERIAL，使其具备自增功能，同时去掉原来的默认值定义，因为SERIAL会自动处理默认值
    sort_order     SERIAL,
    title          VARCHAR(255) NOT NULL DEFAULT '',
    body          varchar(65536) NOT NULL DEFAULT '',
    slug          VARCHAR(255) NOT NULL,
    -- 添加parent_wiki_id字段，用于设置父类wiki页面，若无父类则值为0，有则对应其他wiki的id主键
    parent_wiki_id BIGINT       NOT NULL DEFAULT 0,
    featured       BOOLEAN      NOT NULL DEFAULT FALSE,
    created        timestamptz           DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated        timestamptz           DEFAULT CURRENT_TIMESTAMP NOT NULL,
    draft          BOOLEAN      NOT NULL DEFAULT TRUE,
    -- 添加外键约束（可选，如果希望建立和自身主键的关联约束）
    CONSTRAINT fk_parent_wiki
        FOREIGN KEY (parent_wiki_id)
            REFERENCES wikis (id)
);