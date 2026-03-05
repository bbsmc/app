use super::DatabaseError;
use super::ids::*;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 订单状态
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Pending,  // 待支付
    Paid,     // 已支付
    Failed,   // 支付失败
    Refunded, // 已退款
    Expired,  // 已过期（未支付）
}

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Paid => "paid",
            Self::Failed => "failed",
            Self::Refunded => "refunded",
            Self::Expired => "expired",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "paid" => Self::Paid,
            "failed" => Self::Failed,
            "refunded" => Self::Refunded,
            "expired" => Self::Expired,
            _ => Self::Pending,
        }
    }
}

/// 支付方式
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Alipay,
    Wechat,
}

impl PaymentMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Alipay => "alipay",
            Self::Wechat => "wechat",
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "alipay" => Some(Self::Alipay),
            "wechat" => Some(Self::Wechat),
            _ => None,
        }
    }
}

/// 支付订单
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentOrder {
    pub id: PaymentOrderId,
    pub order_no: String,
    pub external_order_no: Option<String>,
    pub user_id: UserId,
    pub project_id: ProjectId,
    pub seller_id: UserId,
    pub amount: Decimal,
    pub platform_fee: Decimal,
    pub seller_amount: Decimal,
    pub status: OrderStatus,
    pub payment_method: Option<PaymentMethod>,
    pub qr_code_url: Option<String>,
    pub validity_days: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// 平台服务费率 (2.5%)
const PLATFORM_FEE_RATE: f64 = 0.025;

/// 订单过期时间（分钟）
const ORDER_EXPIRE_MINUTES: i64 = 30;

impl PaymentOrder {
    /// 生成订单号
    /// 格式: BB{4位随机大写字母}{yyyyMMddHHmmss}{4位随机数}
    /// 例如: BBABCD2026013012345601234
    /// 随机性: 26^4 × 10000 ≈ 4.5亿种组合
    pub fn generate_order_no() -> String {
        let now = Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S").to_string();

        // 生成4位不重复的随机大写字母
        let mut letters = Vec::with_capacity(4);
        let mut used = [false; 26];
        while letters.len() < 4 {
            let idx = rand::random::<usize>() % 26;
            if !used[idx] {
                used[idx] = true;
                letters.push((b'A' + idx as u8) as char);
            }
        }
        let letter_str: String = letters.into_iter().collect();

        // 生成4位随机数
        let random: u32 = rand::random::<u32>() % 10000;

        format!("BB{}{}{:04}", letter_str, timestamp, random)
    }

    /// 计算平台服务费
    pub fn calculate_platform_fee(amount: Decimal) -> Decimal {
        let fee = amount
            * Decimal::try_from(PLATFORM_FEE_RATE).unwrap_or(Decimal::ZERO);
        fee.round_dp(2)
    }

    /// 创建订单
    pub async fn create(
        user_id: UserId,
        project_id: ProjectId,
        seller_id: UserId,
        amount: Decimal,
        validity_days: Option<i32>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Self, DatabaseError> {
        let id = generate_payment_order_id(&mut *transaction).await?;
        let order_no = Self::generate_order_no();
        let now = Utc::now();
        let expires_at = now + Duration::minutes(ORDER_EXPIRE_MINUTES);

        let platform_fee = Self::calculate_platform_fee(amount);
        let seller_amount = amount - platform_fee;

        sqlx::query!(
            "
            INSERT INTO payment_orders (
                id, order_no, user_id, project_id, seller_id,
                amount, platform_fee, seller_amount, status,
                validity_days, created_at, expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending', $9, $10, $11)
            ",
            id.0,
            &order_no,
            user_id.0,
            project_id.0,
            seller_id.0,
            amount,
            platform_fee,
            seller_amount,
            validity_days,
            now,
            expires_at,
        )
        .execute(&mut **transaction)
        .await?;

        Ok(Self {
            id,
            order_no,
            external_order_no: None,
            user_id,
            project_id,
            seller_id,
            amount,
            platform_fee,
            seller_amount,
            status: OrderStatus::Pending,
            payment_method: None,
            qr_code_url: None,
            validity_days,
            created_at: now,
            paid_at: None,
            expires_at: Some(expires_at),
        })
    }

    /// 根据订单号获取订单
    pub async fn get_by_order_no<'a, E>(
        order_no: &str,
        executor: E,
    ) -> Result<Option<Self>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            "
            SELECT id, order_no, external_order_no, user_id, project_id, seller_id,
                   amount, platform_fee, seller_amount, status, payment_method,
                   qr_code_url, validity_days, created_at, paid_at, expires_at
            FROM payment_orders
            WHERE order_no = $1
            ",
            order_no,
        )
        .fetch_optional(executor)
        .await?;

        Ok(result.map(|row| Self {
            id: PaymentOrderId(row.id),
            order_no: row.order_no,
            external_order_no: row.external_order_no,
            user_id: UserId(row.user_id),
            project_id: ProjectId(row.project_id),
            seller_id: UserId(row.seller_id),
            amount: row.amount,
            platform_fee: row.platform_fee.unwrap_or(Decimal::ZERO),
            seller_amount: row.seller_amount,
            status: OrderStatus::from_string(&row.status),
            payment_method: row
                .payment_method
                .and_then(|m| PaymentMethod::from_string(&m)),
            qr_code_url: row.qr_code_url,
            validity_days: row.validity_days,
            created_at: row.created_at,
            paid_at: row.paid_at,
            expires_at: row.expires_at,
        }))
    }

    /// 获取用户对某项目的待支付订单
    pub async fn get_pending_by_user_project<'a, E>(
        user_id: UserId,
        project_id: ProjectId,
        executor: E,
    ) -> Result<Option<Self>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            "
            SELECT id, order_no, external_order_no, user_id, project_id, seller_id,
                   amount, platform_fee, seller_amount, status, payment_method,
                   qr_code_url, validity_days, created_at, paid_at, expires_at
            FROM payment_orders
            WHERE user_id = $1 AND project_id = $2 AND status = 'pending'
                  AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY created_at DESC
            LIMIT 1
            ",
            user_id.0,
            project_id.0,
        )
        .fetch_optional(executor)
        .await?;

        Ok(result.map(|row| Self {
            id: PaymentOrderId(row.id),
            order_no: row.order_no,
            external_order_no: row.external_order_no,
            user_id: UserId(row.user_id),
            project_id: ProjectId(row.project_id),
            seller_id: UserId(row.seller_id),
            amount: row.amount,
            platform_fee: row.platform_fee.unwrap_or(Decimal::ZERO),
            seller_amount: row.seller_amount,
            status: OrderStatus::from_string(&row.status),
            payment_method: row
                .payment_method
                .and_then(|m| PaymentMethod::from_string(&m)),
            qr_code_url: row.qr_code_url,
            validity_days: row.validity_days,
            created_at: row.created_at,
            paid_at: row.paid_at,
            expires_at: row.expires_at,
        }))
    }

    /// 更新订单支付信息（不存储二维码 URL，每次请求时重新生成）
    pub async fn update_payment_info(
        order_no: &str,
        external_order_no: &str,
        payment_method: PaymentMethod,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query!(
            "
            UPDATE payment_orders
            SET external_order_no = $2, payment_method = $3
            WHERE order_no = $1 AND status = 'pending'
            ",
            order_no,
            external_order_no,
            payment_method.as_str(),
        )
        .execute(&mut **transaction)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 删除用户对某项目的过期待支付订单
    ///
    /// 用于解决唯一约束冲突问题：当存在过期的 pending 订单时，
    /// 先将其删除，然后才能创建新订单
    pub async fn delete_expired_pending_orders<'a, E>(
        user_id: UserId,
        project_id: ProjectId,
        executor: E,
    ) -> Result<u64, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            "
            DELETE FROM payment_orders
            WHERE user_id = $1 AND project_id = $2
                  AND status = 'pending'
                  AND expires_at IS NOT NULL
                  AND expires_at <= NOW()
            ",
            user_id.0,
            project_id.0,
        )
        .execute(executor)
        .await?;

        Ok(result.rows_affected())
    }

    /// 标记订单为已支付
    pub async fn mark_as_paid(
        order_no: &str,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Option<Self>, DatabaseError> {
        let now = Utc::now();

        let result = sqlx::query!(
            "
            UPDATE payment_orders
            SET status = 'paid', paid_at = $2
            WHERE order_no = $1 AND status = 'pending'
            RETURNING id, order_no, external_order_no, user_id, project_id, seller_id,
                      amount, platform_fee, seller_amount, status, payment_method,
                      qr_code_url, validity_days, created_at, paid_at, expires_at
            ",
            order_no,
            now,
        )
        .fetch_optional(&mut **transaction)
        .await?;

        Ok(result.map(|row| Self {
            id: PaymentOrderId(row.id),
            order_no: row.order_no,
            external_order_no: row.external_order_no,
            user_id: UserId(row.user_id),
            project_id: ProjectId(row.project_id),
            seller_id: UserId(row.seller_id),
            amount: row.amount,
            platform_fee: row.platform_fee.unwrap_or(Decimal::ZERO),
            seller_amount: row.seller_amount,
            status: OrderStatus::from_string(&row.status),
            payment_method: row
                .payment_method
                .and_then(|m| PaymentMethod::from_string(&m)),
            qr_code_url: row.qr_code_url,
            validity_days: row.validity_days,
            created_at: row.created_at,
            paid_at: row.paid_at,
            expires_at: row.expires_at,
        }))
    }

    /// 删除超过 12 小时未付款的待支付订单
    pub async fn delete_stale_pending_orders(
        pool: &sqlx::PgPool,
    ) -> Result<u64, DatabaseError> {
        let result = sqlx::query!(
            "
            DELETE FROM payment_orders
            WHERE status = 'pending'
              AND created_at < NOW() - INTERVAL '12 hours'
            "
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// 获取用户的订单列表
    pub async fn get_user_orders<'a, E>(
        user_id: UserId,
        executor: E,
    ) -> Result<Vec<Self>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let results = sqlx::query!(
            "
            SELECT id, order_no, external_order_no, user_id, project_id, seller_id,
                   amount, platform_fee, seller_amount, status, payment_method,
                   qr_code_url, validity_days, created_at, paid_at, expires_at
            FROM payment_orders
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
            user_id.0,
        )
        .fetch_all(executor)
        .await?;

        Ok(results
            .into_iter()
            .map(|row| Self {
                id: PaymentOrderId(row.id),
                order_no: row.order_no,
                external_order_no: row.external_order_no,
                user_id: UserId(row.user_id),
                project_id: ProjectId(row.project_id),
                seller_id: UserId(row.seller_id),
                amount: row.amount,
                platform_fee: row.platform_fee.unwrap_or(Decimal::ZERO),
                seller_amount: row.seller_amount,
                status: OrderStatus::from_string(&row.status),
                payment_method: row
                    .payment_method
                    .and_then(|m| PaymentMethod::from_string(&m)),
                qr_code_url: row.qr_code_url,
                validity_days: row.validity_days,
                created_at: row.created_at,
                paid_at: row.paid_at,
                expires_at: row.expires_at,
            })
            .collect())
    }
}
