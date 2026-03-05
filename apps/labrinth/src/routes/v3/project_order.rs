//! 项目订单路由（买家端）
//!
//! 提供用户购买付费项目的功能。
//! 包括创建订单、查询订单状态、获取支付二维码等。

use actix_web::{HttpRequest, HttpResponse, web};
use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration as StdDuration;
use validator::Validate;

use crate::auth::get_user_from_headers;
use crate::database::models::UserId as DBUserId;
use crate::database::models::ids::ProjectId as DbProjectId;
use crate::database::models::payment_merchant_item::PaymentMerchant;
use crate::database::models::payment_order_item::{
    OrderStatus, PaymentMethod, PaymentOrder,
};
use crate::database::models::project_item::Project;
use crate::database::models::project_pricing_item::ProjectPricing;
use crate::database::models::team_item::TeamMember;
use crate::database::models::user_purchase_item::UserPurchase;
use crate::database::redis::RedisPool;
use crate::models::ids::ProjectId;
use crate::models::pats::Scopes;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use crate::util::validate::validation_errors_to_string;

/// 支付平台 API 请求超时时间（秒）
const PAYMENT_API_TIMEOUT_SECS: u64 = 15;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("order")
            .route("", web::post().to(create_order))
            .route("", web::get().to(list_user_orders))
            .route("/{order_no}", web::get().to(get_order))
            .route("/{order_no}/status", web::get().to(query_order_status)),
    );
}

// ==================== 请求/响应结构 ====================

/// 创建订单请求
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateOrderRequest {
    /// 项目 ID（slug 或 base62 ID）
    #[validate(length(min = 1, max = 64, message = "项目 ID 无效"))]
    pub project_id: String,
    /// 支付方式: "alipay" 或 "wechat"
    #[validate(custom(function = "validate_payment_method"))]
    pub payment_method: String,
}

fn validate_payment_method(
    method: &str,
) -> Result<(), validator::ValidationError> {
    match method {
        "alipay" | "wechat" => Ok(()),
        _ => {
            let mut err =
                validator::ValidationError::new("invalid_payment_method");
            err.message = Some("支付方式必须是 alipay 或 wechat".into());
            Err(err)
        }
    }
}

/// 订单创建响应
#[derive(Debug, Clone, Serialize)]
pub struct CreateOrderResponse {
    /// 订单号
    pub order_no: String,
    /// 支付金额（元）
    pub amount: Decimal,
    /// 支付二维码 URL（base64 图片或链接）
    pub qr_code_url: Option<String>,
    /// 订单过期时间
    pub expires_at: Option<chrono::DateTime<Utc>>,
    /// 支付方式
    pub payment_method: String,
    /// 项目信息
    pub project: OrderProjectInfo,
}

/// 订单中的项目信息
#[derive(Debug, Clone, Serialize)]
pub struct OrderProjectInfo {
    pub id: String,
    pub title: String,
    pub slug: String,
}

/// 订单详情响应
#[derive(Debug, Clone, Serialize)]
pub struct OrderDetailResponse {
    pub order_no: String,
    pub amount: Decimal,
    pub status: String,
    pub payment_method: Option<String>,
    pub qr_code_url: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub paid_at: Option<chrono::DateTime<Utc>>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub project: OrderProjectInfo,
}

/// 订单状态查询响应
#[derive(Debug, Clone, Serialize)]
pub struct OrderStatusResponse {
    pub order_no: String,
    pub status: String,
    pub paid_at: Option<chrono::DateTime<Utc>>,
}

/// 用户订单列表响应
#[derive(Debug, Clone, Serialize)]
pub struct UserOrdersResponse {
    pub orders: Vec<OrderSummary>,
}

/// 订单摘要（用于列表）
#[derive(Debug, Clone, Serialize)]
pub struct OrderSummary {
    pub order_no: String,
    pub amount: Decimal,
    pub status: String,
    pub payment_method: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub paid_at: Option<chrono::DateTime<Utc>>,
    pub project_id: String,
    pub project_title: Option<String>,
}

/// 支付接口创建订单响应
#[derive(Debug, Clone, Deserialize)]
struct PaymentApiCreateOrderResponse {
    code: i32,
    msg: Option<String>,
    data: Option<PaymentApiCreateOrderData>,
}

#[derive(Debug, Clone, Deserialize)]
struct PaymentApiCreateOrderData {
    img: Option<String>,
}

/// 支付接口订单查询响应
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaymentApiQueryOrderResponse {
    code: i32,
    msg: Option<String>,
    data: Option<PaymentApiQueryOrderData>,
}

/// 支付接口订单查询数据
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaymentApiQueryOrderData {
    /// 交易状态: SUCCESS=成功, NOTPAY=未支付, CLOSED=已关闭, REFUND=已退款
    trade_state: Option<String>,
    /// 支付平台订单号
    #[allow(dead_code)]
    order_id: Option<String>,
}

// ==================== 路由处理 ====================

/// 创建订单
///
/// POST /v3/order
///
/// 用户发起购买请求，创建订单并返回支付二维码
pub async fn create_order(
    req: HttpRequest,
    body: web::Json<CreateOrderRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(|err| {
        ApiError::Validation(validation_errors_to_string(err, None))
    })?;

    // 1. 验证用户登录
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?
    .1;

    let db_user_id = DBUserId(user.id.0 as i64);

    // 2. 获取项目信息
    let project = Project::get(&body.project_id, &**pool, &redis)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("项目不存在".to_string()))?;

    let project_id = project.inner.id.0;

    // 3. 验证项目是否是付费项目
    if !project.inner.is_paid {
        return Err(ApiError::InvalidInput(
            "该项目不是付费项目，无需购买".to_string(),
        ));
    }

    // 4. 检查用户是否已购买（且未过期）
    let has_access = UserPurchase::check_access(
        db_user_id,
        DbProjectId(project_id),
        &**pool,
    )
    .await?;

    if has_access {
        return Err(ApiError::InvalidInput(
            "您已经购买过该项目，无需重复购买".to_string(),
        ));
    }

    // 5. 获取项目定价信息
    let pricing = ProjectPricing::get(DbProjectId(project_id), &**pool)
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("该项目尚未设置定价".to_string())
        })?;

    // 6. 获取卖家的商户配置
    // 获取团队拥有者作为卖家
    let team_members =
        TeamMember::get_from_team_full(project.inner.team_id, &**pool, &redis)
            .await?;

    let owner = team_members.iter().find(|m| m.is_owner).ok_or_else(|| {
        ApiError::InvalidInput("无法找到项目所有者".to_string())
    })?;

    let seller_user_id = owner.user_id;

    let merchant = PaymentMerchant::get_by_user(seller_user_id, &**pool)
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput(
                "卖家尚未配置收款账户，暂时无法购买".to_string(),
            )
        })?;

    if !merchant.verified {
        return Err(ApiError::InvalidInput(
            "卖家收款账户尚未验证，暂时无法购买".to_string(),
        ));
    }

    // 7. 解析支付方式
    let payment_method = match body.payment_method.as_str() {
        "alipay" => PaymentMethod::Alipay,
        "wechat" => PaymentMethod::Wechat,
        _ => {
            return Err(ApiError::InvalidInput("不支持的支付方式".to_string()));
        }
    };

    // 8. 先删除过期的待支付订单（避免唯一约束冲突）
    let deleted = PaymentOrder::delete_expired_pending_orders(
        db_user_id,
        DbProjectId(project_id),
        &**pool,
    )
    .await?;

    if deleted > 0 {
        log::info!(
            "已删除 {} 个过期订单: user_id={}, project_id={}",
            deleted,
            db_user_id.0,
            project_id
        );
    }

    // 9. 检查是否已有待支付订单（复用已有订单，避免重复创建）
    let existing_order = PaymentOrder::get_pending_by_user_project(
        db_user_id,
        DbProjectId(project_id),
        &**pool,
    )
    .await?;

    let order = if let Some(existing) = existing_order {
        log::info!(
            "复用已有待支付订单: order_no={}, user_id={}, project_id={}",
            existing.order_no,
            db_user_id.0,
            project_id
        );
        existing
    } else {
        // 创建新订单（处理竞态条件：如果唯一约束冲突，则获取已存在的订单）
        let mut transaction = pool.begin().await?;
        let create_result = PaymentOrder::create(
            db_user_id,
            DbProjectId(project_id),
            seller_user_id,
            pricing.price,
            pricing.validity_days,
            &mut transaction,
        )
        .await;

        match create_result {
            Ok(new_order) => {
                // 更新订单的支付信息
                let external_order_no = format!("7Y{}", new_order.order_no);
                PaymentOrder::update_payment_info(
                    &new_order.order_no,
                    &external_order_no,
                    payment_method.clone(),
                    &mut transaction,
                )
                .await?;

                transaction.commit().await?;
                new_order
            }
            Err(e) => {
                // 检查是否是唯一约束冲突（竞态条件导致）
                let error_str = format!("{:?}", e);
                if error_str.contains("idx_payment_orders_pending_unique")
                    || error_str.contains("duplicate key")
                {
                    // 回滚事务并获取已存在的订单
                    drop(transaction);
                    log::info!(
                        "订单创建冲突，获取已存在订单: user_id={}, project_id={}",
                        db_user_id.0,
                        project_id
                    );
                    PaymentOrder::get_pending_by_user_project(
                        db_user_id,
                        DbProjectId(project_id),
                        &**pool,
                    )
                    .await?
                    .ok_or_else(|| {
                        ApiError::InvalidInput(
                            "订单创建失败，请重试".to_string(),
                        )
                    })?
                } else {
                    return Err(e.into());
                }
            }
        }
    };

    // 9. 调用支付接口创建支付订单（每次都重新生成二维码）
    let qr_code_url = create_payment_order(
        &order.order_no,
        merchant.sid,
        &merchant.secret_key,
        &project.inner.name,
        &user.username,
        pricing.price,
        &payment_method,
    )
    .await?;

    // 10. 构建响应
    let response = CreateOrderResponse {
        order_no: order.order_no,
        amount: pricing.price,
        qr_code_url: Some(qr_code_url),
        expires_at: order.expires_at,
        payment_method: payment_method.as_str().to_string(),
        project: OrderProjectInfo {
            id: crate::models::ids::ProjectId::from(DbProjectId(project_id))
                .to_string(),
            title: project.inner.name,
            slug: project.inner.slug.clone().unwrap_or_default(),
        },
    };

    Ok(HttpResponse::Ok().json(response))
}

/// 获取用户订单列表
///
/// GET /v3/order
///
/// 返回当前用户的所有订单
pub async fn list_user_orders(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 验证用户登录
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?
    .1;

    let db_user_id = DBUserId(user.id.0 as i64);

    // 获取用户的所有订单
    let orders = PaymentOrder::get_user_orders(db_user_id, &**pool).await?;

    // 获取所有相关项目的信息
    let project_ids: Vec<DbProjectId> =
        orders.iter().map(|o| o.project_id).collect();
    let projects = Project::get_many_ids(&project_ids, &**pool, &redis).await?;

    // 构建项目 ID 到名称的映射
    let project_map: std::collections::HashMap<i64, String> = projects
        .into_iter()
        .map(|p| (p.inner.id.0, p.inner.name))
        .collect();

    // 构建响应
    let order_summaries: Vec<OrderSummary> = orders
        .into_iter()
        .map(|order| OrderSummary {
            order_no: order.order_no,
            amount: order.amount,
            status: order.status.as_str().to_string(),
            payment_method: order
                .payment_method
                .map(|m| m.as_str().to_string()),
            created_at: order.created_at,
            paid_at: order.paid_at,
            project_id: ProjectId::from(order.project_id).to_string(),
            project_title: project_map.get(&order.project_id.0).cloned(),
        })
        .collect();

    Ok(HttpResponse::Ok().json(UserOrdersResponse {
        orders: order_summaries,
    }))
}

/// 获取订单详情
///
/// GET /v3/order/{order_no}
pub async fn get_order(
    req: HttpRequest,
    path: web::Path<String>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let order_no = path.into_inner();

    // 验证用户登录
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?
    .1;

    let db_user_id = DBUserId(user.id.0 as i64);

    // 获取订单
    let order = PaymentOrder::get_by_order_no(&order_no, &**pool)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("订单不存在".to_string()))?;

    // 验证订单归属
    if order.user_id != db_user_id {
        return Err(ApiError::InvalidInput("无权查看此订单".to_string()));
    }

    // 获取项目信息
    let project = Project::get_id(order.project_id, &**pool, &redis)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("项目不存在".to_string()))?;

    let response = OrderDetailResponse {
        order_no: order.order_no,
        amount: order.amount,
        status: order.status.as_str().to_string(),
        payment_method: order.payment_method.map(|m| m.as_str().to_string()),
        qr_code_url: order.qr_code_url,
        created_at: order.created_at,
        paid_at: order.paid_at,
        expires_at: order.expires_at,
        project: OrderProjectInfo {
            id: ProjectId::from(order.project_id).to_string(),
            title: project.inner.name,
            slug: project.inner.slug.unwrap_or_default(),
        },
    };

    Ok(HttpResponse::Ok().json(response))
}

/// 查询订单支付状态
///
/// GET /v3/order/{order_no}/status
///
/// 前端轮询使用，检查订单是否已支付。
/// 如果订单状态为 pending，会主动调用支付接口查询支付状态。
pub async fn query_order_status(
    req: HttpRequest,
    path: web::Path<String>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let order_no = path.into_inner();

    // 验证用户登录
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?
    .1;

    let db_user_id = DBUserId(user.id.0 as i64);

    // 获取订单
    let mut order = PaymentOrder::get_by_order_no(&order_no, &**pool)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("订单不存在".to_string()))?;

    // 验证订单归属
    if order.user_id != db_user_id {
        return Err(ApiError::InvalidInput("无权查看此订单".to_string()));
    }

    // 如果订单仍为 pending 状态，主动查询支付接口
    if order.status == OrderStatus::Pending {
        // 获取卖家的商户配置
        if let Ok(Some(merchant)) =
            PaymentMerchant::get_by_user(order.seller_id, &**pool).await
        {
            // 调用支付接口查询订单状态
            if let Ok(Some(trade_state)) = query_payment_order_status(
                &order.order_no,
                merchant.sid,
                &merchant.secret_key,
            )
            .await
            {
                log::info!(
                    "支付接口查询订单状态: order_no={}, trade_state={}",
                    order.order_no,
                    trade_state
                );

                // 如果支付成功，更新订单状态
                if trade_state == "SUCCESS" {
                    if let Err(e) = handle_payment_success(
                        &order,
                        &pool,
                        &redis,
                        merchant.sid,
                        &merchant.secret_key,
                    )
                    .await
                    {
                        log::error!(
                            "处理支付成功失败: order_no={}, error={:?}",
                            order.order_no,
                            e
                        );
                    } else {
                        // 重新获取更新后的订单
                        if let Ok(Some(updated)) =
                            PaymentOrder::get_by_order_no(&order_no, &**pool)
                                .await
                        {
                            order = updated;
                        }
                    }
                }
            }
        }
    }

    let response = OrderStatusResponse {
        order_no: order.order_no,
        status: order.status.as_str().to_string(),
        paid_at: order.paid_at,
    };

    Ok(HttpResponse::Ok().json(response))
}

// ==================== 支付接口调用 ====================

/// 调用支付接口创建支付订单
async fn create_payment_order(
    order_no: &str,
    sid: i32,
    secret_key: &str,
    title: &str,
    user_display_name: &str,
    amount: Decimal,
    payment_method: &PaymentMethod,
) -> Result<String, ApiError> {
    // 获取支付平台 API 配置
    let api_url = dotenvy::var("SEVENPAY_API_URL").unwrap_or_default();
    let create_order_path =
        dotenvy::var("SEVENPAY_CREATE_ORDER_PATH").unwrap_or_default();

    if api_url.is_empty() || create_order_path.is_empty() {
        return Err(ApiError::InvalidInput(
            "支付平台配置未完成，请联系管理员".to_string(),
        ));
    }

    // 将金额转换为分（整数）
    let amount_fen_decimal = (amount * Decimal::from(100)).round();
    let amount_fen: i64 = amount_fen_decimal
        .try_into()
        .map_err(|_| ApiError::InvalidInput("金额转换失败".to_string()))?;
    let amount_fen = amount_fen.to_string();

    // 支付类型: 1=微信, 2=支付宝
    let pay_type = match payment_method {
        PaymentMethod::Wechat => "1",
        PaymentMethod::Alipay => "2",
    };

    // 生成签名
    let sign_raw = format!(
        "{}{}{}{}{}{}{}",
        order_no,
        sid,
        title,
        pay_type,
        user_display_name,
        amount_fen,
        secret_key
    );
    let sign = format!("{:x}", md5::compute(&sign_raw));

    // URL 编码用户名和标题
    let encoded_title = urlencoding::encode(title);
    let encoded_user = urlencoding::encode(user_display_name);

    // 构建请求 URL
    let request_url = format!(
        "{}{}?orderNo={}&sid={}&title={}&payType={}&userDisplayName={}&money={}",
        api_url,
        create_order_path,
        order_no,
        sid,
        encoded_title,
        pay_type,
        encoded_user,
        amount_fen
    );

    log::info!("调用支付接口创建订单: order_no={}, sid={}", order_no, sid);

    // 发送请求
    let client = reqwest::Client::new();
    let response = client
        .get(&request_url)
        .header("Authorization", &sign)
        .timeout(StdDuration::from_secs(PAYMENT_API_TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| {
            log::error!("调用支付接口创建订单失败: {}", e);
            ApiError::InvalidInput("无法连接支付平台，请稍后重试".to_string())
        })?;

    if !response.status().is_success() {
        log::error!("支付接口返回错误状态: {}", response.status());
        return Err(ApiError::InvalidInput(format!(
            "支付平台返回错误: {}",
            response.status()
        )));
    }

    let body: PaymentApiCreateOrderResponse =
        response.json().await.map_err(|e| {
            log::error!("解析支付接口响应失败: {}", e);
            ApiError::InvalidInput("支付平台响应格式错误".to_string())
        })?;

    if body.code != 200 {
        let msg = body.msg.unwrap_or_else(|| "未知错误".to_string());
        log::error!("支付接口创建订单失败: code={}, msg={}", body.code, msg);
        return Err(ApiError::InvalidInput(format!(
            "创建支付订单失败: {}",
            msg
        )));
    }

    // 获取二维码 URL
    let qr_code_url = body.data.and_then(|d| d.img).ok_or_else(|| {
        log::error!("支付接口响应中没有二维码数据");
        ApiError::InvalidInput("支付平台未返回支付二维码".to_string())
    })?;

    log::info!(
        "支付接口创建订单成功: order_no={}, qr_code_len={}",
        order_no,
        qr_code_url.len()
    );

    Ok(qr_code_url)
}

// ==================== 签名生成工具函数 ====================

/// 生成支付接口 API 请求签名
pub fn generate_payment_auth_sign(sid: i32, secret_key: &str) -> String {
    let sign_raw = format!("{}{}", sid, secret_key);
    format!("{:x}", md5::compute(&sign_raw))
}

/// 调用支付接口查询订单状态
///
/// 主动查询支付平台的订单状态，不依赖回调
async fn query_payment_order_status(
    order_no: &str,
    sid: i32,
    secret_key: &str,
) -> Result<Option<String>, ApiError> {
    let api_url = dotenvy::var("SEVENPAY_API_URL").unwrap_or_default();
    let query_order_path =
        dotenvy::var("SEVENPAY_QUERY_ORDER_PATH").unwrap_or_default();

    if api_url.is_empty() || query_order_path.is_empty() {
        log::warn!("支付接口 API 配置不完整，跳过主动查询");
        return Ok(None);
    }

    // 生成签名
    let sign_raw = format!("{}{}{}", order_no, sid, secret_key);
    let sign = format!("{:x}", md5::compute(&sign_raw));

    let request_url = format!(
        "{}{}?orderNo={}&sid={}",
        api_url, query_order_path, order_no, sid
    );

    log::debug!(
        "调用支付接口查询订单状态: order_no={}, sid={}",
        order_no,
        sid
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&request_url)
        .header("Authorization", &sign)
        .timeout(StdDuration::from_secs(5)) // 查询超时 5 秒
        .send()
        .await
        .map_err(|e| {
            log::warn!("调用支付接口查询订单状态失败: {}", e);
            ApiError::InvalidInput("查询支付状态失败".to_string())
        })?;

    if !response.status().is_success() {
        log::warn!("支付接口查询返回错误状态: {}", response.status());
        return Ok(None);
    }

    let body: PaymentApiQueryOrderResponse =
        response.json().await.map_err(|e| {
            log::warn!("解析支付接口查询响应失败: {}", e);
            ApiError::InvalidInput("解析支付状态失败".to_string())
        })?;

    if body.code != 200 {
        log::warn!(
            "支付接口查询订单状态失败: code={}, msg={:?}",
            body.code,
            body.msg
        );
        return Ok(None);
    }

    // 返回交易状态
    Ok(body.data.and_then(|d| d.trade_state))
}

/// 处理支付成功，更新订单状态并创建购买记录
///
/// 主动查询支付成功时调用，会同时通知支付平台订单已发货
async fn handle_payment_success(
    order: &PaymentOrder,
    pool: &PgPool,
    redis: &RedisPool,
    sid: i32,
    secret_key: &str,
) -> Result<(), ApiError> {
    use crate::database::models::user_purchase_item::UserPurchase;
    use chrono::Duration;

    // 开始事务
    let mut transaction = pool.begin().await.map_err(|e| {
        log::error!("开始事务失败: {}", e);
        ApiError::InvalidInput("处理支付失败".to_string())
    })?;

    // 标记订单为已支付
    let paid_order =
        PaymentOrder::mark_as_paid(&order.order_no, &mut transaction)
            .await
            .map_err(|e| {
                log::error!("更新订单状态失败: {}", e);
                ApiError::InvalidInput("更新订单状态失败".to_string())
            })?;

    let paid_order = match paid_order {
        Some(o) => o,
        None => {
            // 订单可能已被处理（幂等性）
            log::info!("订单已处理过: order_no={}", order.order_no);
            return Ok(());
        }
    };

    // 计算购买有效期
    let expires_at = paid_order
        .validity_days
        .map(|days| Utc::now() + Duration::days(days as i64));

    // 创建用户购买记录
    let purchase = UserPurchase::create(
        paid_order.user_id,
        paid_order.project_id,
        Some(order.order_no.clone()),
        paid_order.amount,
        expires_at,
        &mut transaction,
    )
    .await
    .map_err(|e| {
        log::error!("创建购买记录失败: {}", e);
        ApiError::InvalidInput("创建购买记录失败".to_string())
    })?;

    // 提交事务
    transaction.commit().await.map_err(|e| {
        log::error!("提交事务失败: {}", e);
        ApiError::InvalidInput("处理支付失败".to_string())
    })?;

    // 更新 Redis 缓存
    if let Err(e) = UserPurchase::add_to_user_purchase_cache(
        paid_order.user_id,
        paid_order.project_id,
        redis,
    )
    .await
    {
        log::warn!("更新购买缓存失败: {:?}", e);
    }

    log::info!(
        "主动查询支付成功处理完成: order_no={}, user_id={}, project_id={}, purchase_id={}",
        order.order_no,
        paid_order.user_id.0,
        paid_order.project_id.0,
        purchase.id.0
    );

    // 通知支付平台订单已发货（主动查询时需要手动触发）
    if let Err(e) = notify_order_shipped(&order.order_no, sid, secret_key).await
    {
        log::warn!(
            "通知订单发货失败: order_no={}, error={}",
            order.order_no,
            e
        );
        // 不影响主流程，仅记录警告
    }

    Ok(())
}

/// 通知支付平台订单已发货
///
/// 调用支付平台的 /shipOrder 接口，将订单状态从已支付(201)更新为已发货(301)
async fn notify_order_shipped(
    order_no: &str,
    sid: i32,
    secret_key: &str,
) -> Result<(), String> {
    let api_url = dotenvy::var("SEVENPAY_API_URL").unwrap_or_default();
    let ship_order_path =
        dotenvy::var("SEVENPAY_SHIP_ORDER_PATH").unwrap_or_default();

    if api_url.is_empty() || ship_order_path.is_empty() {
        return Err("支付平台发货接口未配置".to_string());
    }

    // 生成签名
    // 7yPay 签名算法：将所有参数值按 URL 参数顺序拼接 + key，然后 MD5
    // URL 参数顺序：orderNo, sid, other
    // 签名原文：orderNo值 + sid值 + other值 + key
    let sign_raw = format!("{}{}{}{}", order_no, sid, "true", secret_key);
    let sign = format!("{:x}", md5::compute(&sign_raw));

    let request_url = format!(
        "{}{}?orderNo={}&sid={}&other=true",
        api_url, ship_order_path, order_no, sid
    );

    log::info!("通知支付平台订单发货: order_no={}, sid={}", order_no, sid);

    let client = reqwest::Client::new();
    let response = client
        .get(&request_url)
        .header("Authorization", &sign)
        .timeout(StdDuration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP 状态错误: {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if body.get("code").and_then(|c| c.as_i64()) == Some(200) {
        log::info!("订单发货通知成功: order_no={}", order_no);
        Ok(())
    } else {
        let msg = body
            .get("msg")
            .and_then(|m| m.as_str())
            .unwrap_or("未知错误");
        Err(format!("发货失败: {}", msg))
    }
}
