//! 支付商户配置路由（卖家端）
//!
//! 提供卖家配置支付账户的功能。
//! 只有 is_premium_creator=true 的用户才能配置。

use actix_web::{HttpRequest, HttpResponse, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::Validate;

use std::time::Duration;

use crate::auth::get_user_from_headers;
use crate::database::models::UserId as DBUserId;

/// 支付平台 API 请求超时时间（秒）
const PAYMENT_API_TIMEOUT_SECS: u64 = 10;
use crate::database::models::payment_merchant_item::{
    PaymentMerchant, PaymentMerchantBuilder,
};
use crate::database::redis::RedisPool;
use crate::models::pats::Scopes;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use crate::util::validate::validation_errors_to_string;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("payment/merchant")
            .route("", web::post().to(create_or_update_merchant))
            .route("", web::get().to(get_merchant))
            .route("", web::delete().to(delete_merchant))
            .route("/verify", web::get().to(verify_merchant)),
    );
}

// ==================== 请求/响应结构 ====================

/// 创建/更新商户配置请求
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateMerchantRequest {
    /// 支付平台店铺 ID (SID)
    #[validate(range(min = 1, message = "店铺 ID 必须是正整数"))]
    pub sid: i32,
    /// 支付平台密钥
    #[validate(length(
        min = 8,
        max = 128,
        message = "密钥长度必须在 8-128 个字符之间"
    ))]
    pub secret_key: String,
}

/// 商户配置响应（不返回密钥）
#[derive(Debug, Clone, Serialize)]
pub struct MerchantResponse {
    pub sid: i32,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<PaymentMerchant> for MerchantResponse {
    fn from(merchant: PaymentMerchant) -> Self {
        Self {
            sid: merchant.sid,
            verified: merchant.verified,
            created_at: merchant.created_at,
            updated_at: merchant.updated_at,
        }
    }
}

/// 验证结果响应
#[derive(Debug, Clone, Serialize)]
pub struct VerifyResponse {
    pub success: bool,
    pub message: String,
}

// ==================== 路由处理 ====================

/// 创建或更新商户配置
///
/// POST /v3/user/payment/merchant
pub async fn create_or_update_merchant(
    req: HttpRequest,
    body: web::Json<CreateMerchantRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(|err| {
        ApiError::Validation(validation_errors_to_string(err, None))
    })?;

    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_WRITE]),
    )
    .await?
    .1;

    let db_user_id = DBUserId(user.id.0 as i64);

    // 检查用户是否是高级创作者
    let db_user =
        crate::database::models::User::get_id(db_user_id, &**pool, &redis)
            .await?
            .ok_or_else(|| ApiError::InvalidInput("用户不存在".to_string()))?;

    if !db_user.is_premium_creator {
        return Err(ApiError::InvalidInput(
            "只有高级创作者才能配置支付商户".to_string(),
        ));
    }

    // 检查 SID 是否已被其他用户使用
    if PaymentMerchant::is_sid_used_by_other(body.sid, db_user_id, &**pool)
        .await?
    {
        return Err(ApiError::InvalidInput(
            "该店铺 ID 已被其他用户使用".to_string(),
        ));
    }

    // 先验证商户配置是否有效（调用支付平台 API）
    let verify_result =
        verify_merchant_on_platform(body.sid, &body.secret_key).await?;
    if !verify_result.success {
        return Err(ApiError::InvalidInput(verify_result.message));
    }

    // 创建或更新商户配置（验证通过后直接设置 verified = true）
    let mut transaction = pool.begin().await?;

    let builder = PaymentMerchantBuilder::new(
        db_user_id,
        body.sid,
        body.secret_key.trim().to_string(),
    );

    builder.upsert(&mut transaction).await?;

    // 验证通过，设置 verified = true
    PaymentMerchant::set_verified(db_user_id, true, &mut transaction).await?;

    transaction.commit().await?;

    // 获取更新后的配置
    let merchant = PaymentMerchant::get_by_user(db_user_id, &**pool)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("配置保存失败".to_string()))?;

    Ok(HttpResponse::Ok().json(MerchantResponse::from(merchant)))
}

/// 获取当前用户的商户配置
///
/// GET /v3/user/payment/merchant
pub async fn get_merchant(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_READ]),
    )
    .await?
    .1;

    let db_user_id = DBUserId(user.id.0 as i64);

    let merchant = PaymentMerchant::get_by_user(db_user_id, &**pool).await?;

    match merchant {
        Some(m) => Ok(HttpResponse::Ok().json(MerchantResponse::from(m))),
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "not_found",
            "description": "您还没有配置支付商户"
        }))),
    }
}

/// 删除商户配置
///
/// DELETE /v3/payment/merchant
pub async fn delete_merchant(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_WRITE]),
    )
    .await?
    .1;

    let db_user_id = DBUserId(user.id.0 as i64);

    // TODO: 在删除前检查是否有关联的付费插件或未完成订单
    // 如果有付费插件正在销售，应阻止删除并提示用户先下架
    // let has_paid_projects = check_user_has_paid_projects(db_user_id, &**pool).await?;
    // if has_paid_projects {
    //     return Err(ApiError::InvalidInput(
    //         "您有付费插件正在销售，请先下架所有付费插件再删除商户配置".to_string(),
    //     ));
    // }

    let mut transaction = pool.begin().await?;

    let deleted = PaymentMerchant::delete(db_user_id, &mut transaction).await?;

    transaction.commit().await?;

    if deleted {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "not_found",
            "description": "您还没有配置支付商户"
        })))
    }
}

/// 支付平台验证响应
#[derive(Debug, Clone, Deserialize)]
struct PaymentPlatformVerifyResponse {
    code: i32,
    data: Option<PaymentPlatformVerifyData>,
    msg: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaymentPlatformVerifyData {
    exists: bool,
    alipay_bound: bool,
    #[allow(dead_code)]
    wechat_bound: Option<bool>,
    server_name: Option<String>,
    #[allow(dead_code)]
    message: Option<String>,
}

/// 调用支付平台验证商户配置
///
/// 返回验证结果，不更新数据库状态
async fn verify_merchant_on_platform(
    sid: i32,
    secret_key: &str,
) -> Result<VerifyResponse, ApiError> {
    // 获取支付平台 API 配置
    let api_url = dotenvy::var("SEVENPAY_API_URL").unwrap_or_default();
    let verify_path =
        dotenvy::var("SEVENPAY_VERIFY_MERCHANT_PATH").unwrap_or_default();

    if api_url.is_empty() || verify_path.is_empty() {
        return Ok(VerifyResponse {
            success: false,
            message: "支付平台配置未完成".to_string(),
        });
    }

    // 生成签名
    let sid_str = sid.to_string();
    let sign_raw = format!("{}{}", sid_str, secret_key);
    let sign = format!("{:x}", md5::compute(&sign_raw));

    // 调用支付平台验证接口
    let verify_url = format!("{}{}?sid={}", api_url, verify_path, sid);

    let client = reqwest::Client::new();
    let platform_response = client
        .get(&verify_url)
        .header("Authorization", &sign)
        .timeout(Duration::from_secs(PAYMENT_API_TIMEOUT_SECS))
        .send()
        .await;

    match platform_response {
        Ok(resp) => {
            if !resp.status().is_success() {
                return Ok(VerifyResponse {
                    success: false,
                    message: format!("支付平台返回错误: {}", resp.status()),
                });
            }

            let body: Result<PaymentPlatformVerifyResponse, _> =
                resp.json().await;
            match body {
                Ok(platform_data) => {
                    if platform_data.code != 200 {
                        return Ok(VerifyResponse {
                            success: false,
                            message: platform_data
                                .msg
                                .unwrap_or_else(|| "签名验证失败".to_string()),
                        });
                    }

                    if let Some(data) = platform_data.data {
                        if !data.exists {
                            return Ok(VerifyResponse {
                                success: false,
                                message: "商户不存在，请检查店铺 ID 是否正确"
                                    .to_string(),
                            });
                        }

                        if !data.alipay_bound {
                            return Ok(VerifyResponse {
                                success: false,
                                message: "商户未绑定支付宝商户号，请先在支付平台完成绑定"
                                    .to_string(),
                            });
                        }

                        return Ok(VerifyResponse {
                            success: true,
                            message: format!(
                                "验证成功！商户名称: {}",
                                data.server_name
                                    .unwrap_or_else(|| "未知".to_string())
                            ),
                        });
                    }

                    Ok(VerifyResponse {
                        success: false,
                        message: "支付平台返回数据异常".to_string(),
                    })
                }
                Err(e) => {
                    log::error!("解析支付平台响应失败: {}", e);
                    Ok(VerifyResponse {
                        success: false,
                        message: "支付平台响应格式错误".to_string(),
                    })
                }
            }
        }
        Err(e) => {
            log::error!("调用支付平台验证接口失败: {}", e);
            Ok(VerifyResponse {
                success: false,
                message: "无法连接支付平台，请稍后重试".to_string(),
            })
        }
    }
}

/// 验证商户配置
///
/// GET /v3/user/payment/merchant/verify
///
/// 通过调用支付平台 API 验证配置是否正确
pub async fn verify_merchant(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_WRITE]),
    )
    .await?
    .1;

    let db_user_id = DBUserId(user.id.0 as i64);

    let merchant = PaymentMerchant::get_by_user(db_user_id, &**pool)
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("您还没有配置支付商户".to_string())
        })?;

    // 调用支付平台验证
    let verify_result =
        verify_merchant_on_platform(merchant.sid, &merchant.secret_key).await?;

    // 根据验证结果更新 verified 状态
    let mut transaction = pool.begin().await?;
    PaymentMerchant::set_verified(
        db_user_id,
        verify_result.success,
        &mut transaction,
    )
    .await?;
    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(verify_result))
}
