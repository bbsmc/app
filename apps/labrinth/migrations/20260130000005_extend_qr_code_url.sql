-- 扩展 qr_code_url 字段长度
-- 支付宝返回的二维码 URL 可能是 base64 编码的图片数据，长度可能超过 512 字符

ALTER TABLE payment_orders
    ALTER COLUMN qr_code_url TYPE TEXT;

COMMENT ON COLUMN payment_orders.qr_code_url IS '支付二维码 URL 或 base64 数据';
