-- 为项目添加汉化追踪字段
ALTER TABLE mods ADD COLUMN translation_tracking BOOLEAN NOT NULL DEFAULT FALSE;
