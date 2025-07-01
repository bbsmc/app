-- 0  只用链接里设置的 1 全都用  3 关闭issues
ALTER TABLE mods
    ADD COLUMN issues_type int NOT NULL DEFAULT 0;