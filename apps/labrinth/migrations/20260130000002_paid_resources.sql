-- 付费资源系统数据库迁移

-- 1. mods 表添加付费标记
-- 注意：is_paid 一旦设置为 true 就不能改为 false（通过应用层逻辑控制）
ALTER TABLE mods ADD COLUMN is_paid BOOLEAN DEFAULT FALSE NOT NULL;

-- 索引：快速查询付费资源
CREATE INDEX idx_mods_is_paid ON mods(is_paid) WHERE is_paid = TRUE;

-- 2. 项目定价表
CREATE TABLE project_pricing (
    project_id      BIGINT PRIMARY KEY REFERENCES mods(id) ON DELETE CASCADE,
    price           DECIMAL(10, 2) NOT NULL CHECK (price > 0),  -- 价格（单位：元）
    validity_days   INTEGER CHECK (validity_days IS NULL OR validity_days > 0),  -- 有效期天数，NULL 表示永久
    created_at      TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at      TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

COMMENT ON TABLE project_pricing IS '项目定价表，存储付费资源的价格和有效期配置';
COMMENT ON COLUMN project_pricing.price IS '价格，单位为人民币元';
COMMENT ON COLUMN project_pricing.validity_days IS '授权有效期天数，NULL 表示永久有效';

-- 3. files 表添加私有存储标记
-- is_private = true 的文件存储在私有桶，需要购买后才能访问
ALTER TABLE files ADD COLUMN is_private BOOLEAN DEFAULT FALSE NOT NULL;

-- 索引：快速查询私有文件
CREATE INDEX idx_files_is_private ON files(is_private) WHERE is_private = TRUE;

-- 4. 用户购买记录表
CREATE TABLE user_purchases (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id      BIGINT NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    order_no        VARCHAR(64) UNIQUE,           -- 订单号
    amount          DECIMAL(10, 2) NOT NULL,      -- 支付金额
    purchased_at    TIMESTAMPTZ DEFAULT NOW() NOT NULL,  -- 购买时间
    expires_at      TIMESTAMPTZ,                  -- 过期时间，NULL 表示永久
    status          VARCHAR(20) DEFAULT 'active' NOT NULL,  -- active/expired/refunded

    -- 复合唯一约束：同一用户对同一项目只能有一条有效记录
    UNIQUE (user_id, project_id)
);

-- 索引
CREATE INDEX idx_user_purchases_user_id ON user_purchases(user_id);
CREATE INDEX idx_user_purchases_project_id ON user_purchases(project_id);
CREATE INDEX idx_user_purchases_status ON user_purchases(status);
CREATE INDEX idx_user_purchases_expires_at ON user_purchases(expires_at) WHERE expires_at IS NOT NULL;

COMMENT ON TABLE user_purchases IS '用户购买记录表，记录用户的付费资源购买历史';
COMMENT ON COLUMN user_purchases.expires_at IS '授权过期时间，NULL 表示永久有效';

-- 5. 支付订单表
CREATE TABLE payment_orders (
    id              BIGSERIAL PRIMARY KEY,
    order_no        VARCHAR(64) UNIQUE NOT NULL,  -- 平台订单号
    external_order_no VARCHAR(64),                -- 支付平台订单号
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id      BIGINT NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    seller_id       BIGINT NOT NULL REFERENCES users(id),  -- 卖家用户ID
    amount          DECIMAL(10, 2) NOT NULL,      -- 订单金额
    platform_fee    DECIMAL(10, 2) DEFAULT 0,     -- 平台服务费（2.5%）
    seller_amount   DECIMAL(10, 2) NOT NULL,      -- 卖家实际收到金额
    status          VARCHAR(20) DEFAULT 'pending' NOT NULL,  -- pending/paid/failed/refunded/expired
    payment_method  VARCHAR(20),                  -- alipay/wechat
    qr_code_url     VARCHAR(512),                 -- 支付二维码 URL
    validity_days   INTEGER,                      -- 购买的有效期天数
    created_at      TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    paid_at         TIMESTAMPTZ,                  -- 支付完成时间
    expires_at      TIMESTAMPTZ                   -- 订单过期时间（未支付的订单）
);

-- 索引
CREATE INDEX idx_payment_orders_user_id ON payment_orders(user_id);
CREATE INDEX idx_payment_orders_project_id ON payment_orders(project_id);
CREATE INDEX idx_payment_orders_seller_id ON payment_orders(seller_id);
CREATE INDEX idx_payment_orders_status ON payment_orders(status);
CREATE INDEX idx_payment_orders_order_no ON payment_orders(order_no);
CREATE INDEX idx_payment_orders_external_order_no ON payment_orders(external_order_no) WHERE external_order_no IS NOT NULL;

COMMENT ON TABLE payment_orders IS '支付订单表，记录所有支付订单';
COMMENT ON COLUMN payment_orders.platform_fee IS '平台服务费，按 2.5% 计算';
COMMENT ON COLUMN payment_orders.seller_amount IS '卖家实际收到的金额 = amount - platform_fee';
