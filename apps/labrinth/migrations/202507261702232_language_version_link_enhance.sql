-- 为版本语言绑定表添加额外字段和索引
ALTER TABLE version_link_version 
ADD COLUMN link_type varchar(50) DEFAULT 'translation' NOT NULL,
ADD COLUMN language_code varchar(10) DEFAULT 'zh_CN' NOT NULL,
ADD COLUMN created_at timestamptz DEFAULT CURRENT_TIMESTAMP NOT NULL,
ADD COLUMN description text;

-- 添加注释说明
COMMENT ON TABLE version_link_version IS '版本语言绑定关系表，用于关联汉化版本与原版本';
COMMENT ON COLUMN version_link_version.version_id IS '汉化版本ID';
COMMENT ON COLUMN version_link_version.joining_version_id IS '原版本ID（被汉化的版本）';
COMMENT ON COLUMN version_link_version.link_type IS '链接类型：translation（翻译）, resource_pack（资源包）, addon（附加包）等';
COMMENT ON COLUMN version_link_version.language_code IS '目标语言代码，如 zh_CN（简体中文）, zh_TW（繁体中文）等';
COMMENT ON COLUMN version_link_version.description IS '绑定关系说明，如汉化说明等';

-- 创建索引以优化查询
CREATE INDEX idx_version_link_joining ON version_link_version(joining_version_id);
CREATE INDEX idx_version_link_type ON version_link_version(link_type);
CREATE INDEX idx_version_link_language ON version_link_version(language_code);
CREATE INDEX idx_version_link_created ON version_link_version(created_at DESC);

-- 创建复合索引优化常见查询
CREATE INDEX idx_version_link_joining_type_lang ON version_link_version(joining_version_id, link_type, language_code);