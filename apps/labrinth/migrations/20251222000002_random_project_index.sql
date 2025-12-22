-- 为 random_projects 查询添加索引
-- 这个索引大幅降低了 random_projects_get 查询的成本
-- 从约 354.04..363.39 降至约 171.33..180.68（约 2 倍改进）

-- 检查索引是否已存在，避免重复创建
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_indexes
        WHERE indexname = 'mods_status'
    ) THEN
        CREATE INDEX mods_status ON mods(status);
    END IF;
END
$$;
