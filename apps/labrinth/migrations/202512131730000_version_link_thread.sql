-- 为版本链接添加thread支持
-- 用于翻译者和原作者之间的沟通

-- 添加thread_id字段到version_link_version表
ALTER TABLE version_link_version 
ADD COLUMN thread_id BIGINT REFERENCES threads(id) ON DELETE SET NULL;

-- 创建索引以提高查询性能
CREATE INDEX idx_version_link_thread ON version_link_version(thread_id) WHERE thread_id IS NOT NULL;

-- 添加注释
COMMENT ON COLUMN version_link_version.thread_id IS '关联的消息线程ID，用于翻译者和原作者之间的沟通';