-- 高级创作者申请 Thread 支持
-- 参考封禁申诉的设计模式

-- 1. 为 threads 表添加 creator_application_id 列
ALTER TABLE threads ADD COLUMN creator_application_id bigint REFERENCES creator_applications(id) ON DELETE SET NULL;
CREATE INDEX threads_creator_application_idx ON threads (creator_application_id) WHERE creator_application_id IS NOT NULL;

-- 2. 为 creator_applications 表添加 thread_id 列
ALTER TABLE creator_applications ADD COLUMN thread_id bigint;

-- 3. 添加外键约束（循环引用需要延迟添加）
ALTER TABLE creator_applications ADD CONSTRAINT creator_applications_thread_fkey FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE SET NULL;

-- 4. 修改身份证号字段 - 存储加密后的数据，增加长度以容纳加密内容
-- 加密后的 base64 字符串会比原文长，需要增加字段长度
ALTER TABLE creator_applications ALTER COLUMN id_card_number TYPE VARCHAR(512);

-- 5. 添加注释
COMMENT ON COLUMN threads.creator_application_id IS '关联的高级创作者申请ID';
COMMENT ON COLUMN creator_applications.thread_id IS '关联的消息线程ID，用于申请人和管理员之间的沟通';
COMMENT ON COLUMN creator_applications.id_card_number IS '加密存储的身份证号';

