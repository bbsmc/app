-- 解耦项目删除与图片审核记录
-- 当项目被删除时，image_content_reviews.project_id 设置为 NULL 而不是阻止删除
-- 审核记录用于历史/审计（违规图 URL、风控标签、复审人员），即使项目已删也应保留
-- 参考 20251222000001_decouple_threads_from_projects.sql 的相同思路

ALTER TABLE image_content_reviews
DROP CONSTRAINT IF EXISTS image_content_reviews_project_id_fkey,
ADD CONSTRAINT image_content_reviews_project_id_fkey
FOREIGN KEY (project_id) REFERENCES mods(id)
ON DELETE SET NULL;
