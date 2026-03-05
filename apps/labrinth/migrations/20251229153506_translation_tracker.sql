-- 为开启追踪的项目添加汉化包 slug 存储字段
-- 当 translation_tracking = true 时，translation_tracker 存储对应汉化包的 slug
ALTER TABLE mods ADD COLUMN translation_tracker VARCHAR(64) DEFAULT NULL;
