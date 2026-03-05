-- 添加 MC百科 和 九域资源社区 链接平台

-- 先修复序列值，防止自增 id 冲突
SELECT setval('link_platforms_id_seq', COALESCE((SELECT MAX(id) FROM link_platforms), 1));

-- 添加 MC百科 (mcmod)
INSERT INTO link_platforms (name, donation)
VALUES ('mcmod', true)
ON CONFLICT (name) DO UPDATE SET donation = EXCLUDED.donation;

-- 添加 九域资源社区 (mc9y)
INSERT INTO link_platforms (name, donation)
VALUES ('mc9y', true)
ON CONFLICT (name) DO UPDATE SET donation = EXCLUDED.donation;

SELECT setval('mods_links_id_seq', COALESCE((SELECT MAX(id) FROM mods_links), 1));
