-- Issues系统数据库表
-- 参考GitHub Issues设计

-- 标签表 (用于issues标签分类)
CREATE TABLE issue_labels (
    id SERIAL PRIMARY KEY, -- 标签的唯一标识符
    name VARCHAR(100) NOT NULL, -- 标签名称
    color VARCHAR(7) NOT NULL DEFAULT '#7c7c7c', -- 标签显示的颜色（十六进制格式）
    description VARCHAR(500) DEFAULT '', -- 标签的详细描述
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL -- 标签创建时间
);

-- Issues主表
CREATE TABLE issues (
    id BIGINT PRIMARY KEY, -- Issue的唯一标识符
    mod_id BIGINT NOT NULL REFERENCES mods ON UPDATE CASCADE, -- 关联的模组ID
    title VARCHAR(1000) NOT NULL, -- Issue的标题
    body VARCHAR(65536) DEFAULT '' NOT NULL, -- Issue的详细内容
    state VARCHAR(20) DEFAULT 'open' NOT NULL, -- Issue的状态：open(开放)或closed(关闭)
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL, -- Issue创建时间
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL, -- Issue最后更新时间
    closed_at TIMESTAMP WITH TIME ZONE NULL, -- Issue关闭时间
    author_id BIGINT NOT NULL REFERENCES users ON UPDATE CASCADE, -- Issue创建者ID
    locked BOOLEAN DEFAULT FALSE NOT NULL, -- 是否锁定讨论（防止新回复）
    deleted BOOLEAN DEFAULT FALSE NOT NULL, -- 是否已删除
    deleted_at TIMESTAMP WITH TIME ZONE NULL -- 删除时间
);

-- Issues回复表
CREATE TABLE issue_comments (
    id BIGINT PRIMARY KEY, -- 评论的唯一标识符
    issue_id BIGINT NOT NULL REFERENCES issues ON UPDATE CASCADE, -- 关联的Issue ID
    author_id BIGINT NOT NULL REFERENCES users ON UPDATE CASCADE, -- 评论作者ID
    body VARCHAR(65536) NOT NULL, -- 评论内容
    comment_type VARCHAR(20) DEFAULT 'reply' NOT NULL, -- 评论类型：reply(回复)或notification(通知)
    reply_to_id BIGINT NULL REFERENCES issue_comments(id) ON UPDATE CASCADE, -- 回复的上一级评论ID
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL, -- 评论创建时间
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL, -- 评论最后更新时间
    deleted BOOLEAN DEFAULT FALSE NOT NULL, -- 是否已删除
    deleted_at TIMESTAMP WITH TIME ZONE NULL -- 删除时间
);

-- Issues标签关联表
CREATE TABLE issue_label_associations (
    issue_id BIGINT NOT NULL REFERENCES issues ON UPDATE CASCADE, -- 关联的Issue ID
    label_id INTEGER NOT NULL REFERENCES issue_labels ON UPDATE CASCADE, -- 关联的标签ID
    PRIMARY KEY (issue_id, label_id)
);

-- Issues指派人关联表 (支持多人指派)
CREATE TABLE issue_assignees (
    issue_id BIGINT NOT NULL REFERENCES issues ON UPDATE CASCADE, -- 关联的Issue ID
    user_id BIGINT NOT NULL REFERENCES users ON UPDATE CASCADE, -- 被指派用户的ID
    assigned_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL, -- 指派时间
    assigned_by BIGINT NOT NULL REFERENCES users ON UPDATE CASCADE, -- 执行指派操作的用户ID
    PRIMARY KEY (issue_id, user_id)
);

-- 添加索引以提高查询性能
CREATE INDEX idx_issues_mod_id ON issues(mod_id);
CREATE INDEX idx_issues_author_id ON issues(author_id);
CREATE INDEX idx_issues_state ON issues(state);
CREATE INDEX idx_issues_created_at ON issues(created_at);
CREATE INDEX idx_issue_comments_issue_id ON issue_comments(issue_id);
CREATE INDEX idx_issue_comments_author_id ON issue_comments(author_id);
CREATE INDEX idx_issue_comments_reply_to_id ON issue_comments(reply_to_id);
CREATE INDEX idx_issue_comments_type ON issue_comments(comment_type);
CREATE INDEX idx_issue_assignees_user_id ON issue_assignees(user_id);
CREATE INDEX idx_issue_assignees_assigned_by ON issue_assignees(assigned_by);

-- 插入一些默认标签
INSERT INTO issue_labels (name, color, description) VALUES 
('错误', '#dc3545', '错误或缺陷'),
('改进', '#0d6efd', '新功能或改进'),
('问题', '#6f42c1', '问题求助'),
('文档', '#0dcaf0', '文档相关'),
('求助', '#198754', '寻求帮助'),
('重复', '#6c757d', '重复的问题'),
('无需修复', '#495057', '无需修复'),
('已修复', '#28a745', '问题已解决'),
('已改进', '#20c997', '功能已改进'),
('暂缓', '#fd7e14', '暂时搁置'),
('计划', '#6610f2', '计划中的功能'); 