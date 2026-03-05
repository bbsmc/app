-- 添加价格和有效期范围约束
-- 价格范围：1-1000 元（与 API 验证一致）
-- 有效期范围：1-3650 天（与 API 验证一致）

-- 删除原有的宽松约束并添加更严格的约束
ALTER TABLE project_pricing DROP CONSTRAINT IF EXISTS project_pricing_price_check;
ALTER TABLE project_pricing ADD CONSTRAINT project_pricing_price_check CHECK (price >= 1 AND price <= 1000);

ALTER TABLE project_pricing DROP CONSTRAINT IF EXISTS project_pricing_validity_days_check;
ALTER TABLE project_pricing ADD CONSTRAINT project_pricing_validity_days_check CHECK (validity_days IS NULL OR (validity_days >= 1 AND validity_days <= 3650));

COMMENT ON COLUMN project_pricing.price IS '价格，单位为人民币元，范围 1-1000';
COMMENT ON COLUMN project_pricing.validity_days IS '授权有效期天数（1-3650），NULL 表示永久有效';
