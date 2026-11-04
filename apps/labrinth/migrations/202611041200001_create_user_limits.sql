-- 上游 f466470d0 (#4430) + ddb131a70 (#7591)：每用户每日发布限额。
-- 本 fork 只移植"每日限额"，不包含上游的项目/组织/收藏总量上限；
-- 且"每日新建项目数"改为按创建者（mods.created_by）计数，而非上游的按团队成员身份计数。
--
-- user_id 为 NULL 的那一行是全局默认值；给某个用户单独插一行即可覆盖他的额度
-- （没写的列取下面的默认值）。管理员（role = admin）在代码层不受限。
-- 手工加覆盖行时：user_id 填数字 id（i64，不是 base62 字符串），并先确认该用户存在——
-- 没有外键，填错不会报错，只会悄悄不生效。
-- "一天"按北京时间自然日计（00:00 重置），见 src/models/v3/user_limits.rs。
--
-- 部署安全（新 pod 启动时自动执行迁移，滚动发布期间新旧 pod 并存）：
-- 1. 只用一个文件 = 一个事务：拿不到锁超时时整体回滚，库保持原样，旧版本 pod 重启/回滚都不受影响
--    （若拆成多个文件，部分提交后旧二进制会因 VersionMissing 无法启动）。
-- 2. 唯一的外部锁是 mods 上 ADD COLUMN 需要的 ACCESS EXCLUSIVE（只改元数据，拿到即完成）。
--    lock_timeout 3 秒：有上传在途占着 mods 时快速失败，k8s 重启新 pod 重试，旧 pod 照常服务；
--    每次尝试最多让 mods 排队约 3 秒。建议在上传较少的时段发布。
--    idle_in_transaction_session_timeout 10 秒：万一执行迁移的 pod 所在节点在 ALTER 拿到锁之后、COMMIT 之前失联
--    （半开连接），服务端最多 10 秒就断开会话、释放 mods 上的锁，而不是等 TCP keepalive（可能数小时）。
-- 3. 不建外键：外键会让迁移对 users 加 SHARE ROW EXCLUSIVE 锁，与在途写 users 的事务互等。
--    用户 id 随机生成（8 位 base62，约 2.15e14 种），实际上不会复用；账户注销后残留的 user_limits 行 /
--    created_by 值几乎不可能匹配到别人，即便撞上影响也可忽略。
-- 4. 不建索引：计数查询在当前规模下顺序扫描代价很小（mods 数百行、versions 数万行），
--    而在本事务里建索引会额外持有 versions/mods 的锁。若 versions 将来很大，
--    可在单独一次低峰发布里用 CREATE INDEX CONCURRENTLY 补 versions (author_id, date_published)。
SET LOCAL lock_timeout = '3s';
SET LOCAL idle_in_transaction_session_timeout = '10s';

CREATE TABLE user_limits (
    user_id              BIGINT  UNIQUE,
    projects_per_day     INTEGER NOT NULL DEFAULT 10    CHECK (projects_per_day >= 0),
    versions_per_project INTEGER NOT NULL DEFAULT 10000 CHECK (versions_per_project >= 0),
    versions_per_day     INTEGER NOT NULL DEFAULT 50    CHECK (versions_per_day >= 0)
);

-- 全局默认行（user_id IS NULL）至多一行。用部分唯一索引代替 UNIQUE NULLS NOT DISTINCT，
-- 以兼容 PostgreSQL 15 以下版本。（新建的空表上建索引，不涉及其它表的锁。）
CREATE UNIQUE INDEX user_limits_single_default
    ON user_limits ((user_id IS NULL))
    WHERE user_id IS NULL;

INSERT INTO user_limits (user_id, projects_per_day, versions_per_project, versions_per_day)
VALUES (NULL, 10, 10000, 50);

-- 记录项目创建者，供"今日新建项目数"按创建者计数：在组织名下建项目、被邀请进别人的项目、
-- 项目转让都不影响计数。由用户创建项目时写入（routes/v3/project_creation.rs）；
-- 系统任务（如汉化追踪）创建的项目为 NULL；历史项目为 NULL，不影响当日计数。
ALTER TABLE mods ADD COLUMN created_by BIGINT;
