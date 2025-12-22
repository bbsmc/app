-- 解耦项目删除与线程删除
-- 当项目被删除时，线程的 mod_id 设置为 NULL 而不是删除线程
-- 这样线程可以继续存在，用于历史记录和审计

ALTER TABLE threads
DROP CONSTRAINT IF EXISTS threads_mod_id_fkey,
ADD CONSTRAINT threads_mod_id_fkey
FOREIGN KEY (mod_id) REFERENCES mods(id)
ON DELETE SET NULL;
