-- 网盘链接展示方式：default 直接跳转 / qrcode 纯二维码 / both 二维码+跳转链接
ALTER TABLE disk_urls ADD COLUMN display varchar(20) NOT NULL DEFAULT 'default';

ALTER TABLE disk_urls
    ADD CONSTRAINT disk_urls_display_check CHECK (display IN ('default', 'qrcode', 'both'));
