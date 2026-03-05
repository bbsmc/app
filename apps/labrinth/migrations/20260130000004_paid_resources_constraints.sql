-- 付费资源表添加 CHECK 约束
-- 确保数据完整性和一致性

-- 1. 金额字段非负约束
ALTER TABLE user_purchases
    ADD CONSTRAINT check_user_purchases_amount_non_negative
    CHECK (amount >= 0);

ALTER TABLE payment_orders
    ADD CONSTRAINT check_payment_orders_amount_non_negative
    CHECK (amount >= 0);

ALTER TABLE payment_orders
    ADD CONSTRAINT check_payment_orders_platform_fee_non_negative
    CHECK (platform_fee >= 0);

ALTER TABLE payment_orders
    ADD CONSTRAINT check_payment_orders_seller_amount_non_negative
    CHECK (seller_amount >= 0);

-- 2. 金额一致性约束
ALTER TABLE payment_orders
    ADD CONSTRAINT check_payment_orders_amount_consistency
    CHECK (seller_amount = amount - platform_fee);

-- 3. 状态字段合法值约束
ALTER TABLE user_purchases
    ADD CONSTRAINT check_user_purchases_status_valid
    CHECK (status IN ('active', 'expired', 'refunded'));

ALTER TABLE payment_orders
    ADD CONSTRAINT check_payment_orders_status_valid
    CHECK (status IN ('pending', 'paid', 'failed', 'refunded', 'expired'));

-- 4. 支付方式合法值约束
ALTER TABLE payment_orders
    ADD CONSTRAINT check_payment_orders_payment_method_valid
    CHECK (payment_method IS NULL OR payment_method IN ('alipay', 'wechat'));

-- 5. 有效期范围约束 (与 API 验证一致)
ALTER TABLE project_pricing
    ADD CONSTRAINT check_project_pricing_validity_days_range
    CHECK (validity_days IS NULL OR (validity_days >= 1 AND validity_days <= 3650));

ALTER TABLE payment_orders
    ADD CONSTRAINT check_payment_orders_validity_days_range
    CHECK (validity_days IS NULL OR (validity_days >= 1 AND validity_days <= 3650));

-- 6. 删除冗余索引 (order_no 已有 UNIQUE 约束会自动创建索引)
DROP INDEX IF EXISTS idx_payment_orders_order_no;

-- 7. 添加防止同一用户对同一项目存在多个 pending 订单的部分唯一索引
CREATE UNIQUE INDEX IF NOT EXISTS idx_payment_orders_pending_unique
    ON payment_orders(user_id, project_id)
    WHERE status = 'pending';
