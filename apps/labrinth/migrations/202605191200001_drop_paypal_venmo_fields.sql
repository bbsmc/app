-- 下线 PayPal / Venmo / Tremendous 提现通道：删除用户表中相关字段
-- Tremendous 不在 users 表存数据（按 BBSMC email 发送），无需 SQL 改动

ALTER TABLE users
    DROP COLUMN IF EXISTS paypal_id,
    DROP COLUMN IF EXISTS paypal_country,
    DROP COLUMN IF EXISTS paypal_email,
    DROP COLUMN IF EXISTS venmo_handle;
