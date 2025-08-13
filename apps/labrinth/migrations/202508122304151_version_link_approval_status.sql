-- 添加版本链接审核状态字段
ALTER TABLE version_link_version
ADD COLUMN approval_status VARCHAR(20) NOT NULL DEFAULT 'pending';

-- 创建索引以提高查询性能
CREATE INDEX idx_version_link_approval ON version_link_version(approval_status);

-- 将所有现有的版本链接设置为已审核通过
UPDATE version_link_version SET approval_status = 'approved';

-- 添加注释
COMMENT ON COLUMN version_link_version.approval_status IS '版本链接审核状态: pending=审核中, approved=审核通过';