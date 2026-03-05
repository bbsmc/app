use crate::file_hosting::FileHostingError;
use crate::routes::analytics::{page_view_ingest, playtime_ingest};
use crate::util::cors::default_cors;
use crate::util::env::parse_strings_from_var;
use actix_cors::Cors;
use actix_files::Files;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, web};
use futures::FutureExt;

pub mod internal;
pub mod v2;
pub mod v3;

pub mod v2_reroute;

mod analytics;
mod index;
mod maven;
mod not_found;
mod updates;

pub use self::not_found::not_found;

pub fn root_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("maven")
            .wrap(default_cors())
            .configure(maven::config),
    );
    cfg.service(
        web::scope("updates")
            .wrap(default_cors())
            .configure(updates::config),
    );
    cfg.service(
        web::scope("analytics")
            .wrap(
                Cors::default()
                    .allowed_origin_fn(|origin, _req_head| {
                        let allowed_origins =
                            parse_strings_from_var("ANALYTICS_ALLOWED_ORIGINS")
                                .unwrap_or_default();

                        allowed_origins.contains(&"*".to_string())
                            || allowed_origins.contains(
                                &origin
                                    .to_str()
                                    .unwrap_or_default()
                                    .to_string(),
                            )
                    })
                    .allowed_methods(vec!["GET", "POST"])
                    .allowed_headers(vec![
                        actix_web::http::header::AUTHORIZATION,
                        actix_web::http::header::ACCEPT,
                        actix_web::http::header::CONTENT_TYPE,
                    ])
                    .max_age(3600),
            )
            .service(page_view_ingest)
            .service(playtime_ingest),
    );
    cfg.service(
        web::scope("api/v1")
            .wrap(default_cors())
            .wrap_fn(|req, _srv| {
                async {
                    Ok(req.into_response(
                        HttpResponse::Gone()
                            .content_type("application/json")
                            .body(r#"{"error":"api_deprecated","description":"您正在使用一个过时版本 API 的应用程序。请更新应用程序或切换到其他应用。"}"#)
                    ))
                }.boxed_local()
            })
    );
    cfg.service(
        web::scope("")
            .wrap(default_cors())
            .service(index::index_get)
            .service(Files::new("/", "assets/")),
    );
}

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("运行环境错误")]
    Env(#[from] dotenvy::Error),
    #[error("您已被封禁：{0}")]
    Banned(String),
    #[error("上传文件时出错: {0}")]
    FileHosting(#[from] FileHostingError),
    #[error("数据库错误: {0}")]
    Database(#[from] crate::database::models::DatabaseError),
    #[error("数据库错误: {0}")]
    SqlxDatabase(#[from] sqlx::Error),
    #[error("Clickhouse数据库错误: {0}")]
    Clickhouse(#[from] clickhouse::error::Error),
    #[error("服务器内部错误: {0}")]
    Xml(String),
    #[error("反序列化错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("身份验证错误: {0}")]
    Authentication(#[from] crate::auth::AuthenticationError),
    #[error("认证失败: {0}")]
    CustomAuthentication(String),
    #[error("无效输入: {0}")]
    InvalidInput(String),
    #[error("验证输入时出错: {0}")]
    Validation(String),
    #[error("您已被禁止申请编辑，恢复时间: {0}")]
    WikiBan(String),
    #[error("搜索出错: {0}")]
    Search(#[from] meilisearch_sdk::errors::Error),
    #[error("索引错误: {0}")]
    Indexing(#[from] crate::search::indexing::IndexingError),
    #[error("付款错误: {0}")]
    Payments(String),
    #[error("Discord 错误: {0}")]
    Discord(String),
    #[error("验证码错误。请尝试重新提交。")]
    Turnstile,
    #[error("解码 Base62 时出错: {0}")]
    Decoding(#[from] crate::models::ids::DecodingError),
    #[error("图片解析错误: {0}")]
    ImageParse(#[from] image::ImageError),
    #[error("密码哈希错误: {0}")]
    PasswordHashing(#[from] argon2::password_hash::Error),
    #[error("{0}")]
    Mail(#[from] crate::auth::email::MailError),
    #[error("重新路由请求时出错: {0}")]
    Reroute(#[from] reqwest::Error),
    #[error("无法读取 Zip 存档: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("资源未找到")]
    NotFound,
    #[error("该资源已存在")]
    ISExists,
    #[error(
        "该资源正在被 {0} 修改百科页面，请等待其他用户修改完并且被审核完成后再进行提交修改"
    )]
    ISConflict(String),
    #[error("您的请求过于频繁，请等待 {0} 毫秒后重试。剩余配额: 0/{1}")]
    RateLimitError(u128, u32),
    #[error("与支付处理器交互时出错: {0}")]
    Stripe(#[from] stripe::StripeError),
    #[error("阿里云短信服务错误: {0}")]
    AliyunSms(#[from] alibaba_cloud_sdk_rust::error::AliyunSDKError),
    #[error("您已达到上传图片的限制 ({0}/{1})")]
    ImageLimit(u32, u32),
    #[error(
        "由于您多次上传文本被识别为风险违规，您已达到上传文本/图片的限制，解除时间: {0}"
    )]
    RiskLimit(String),
}

impl ApiError {
    pub fn as_api_error<'a>(&self) -> crate::models::error::ApiError<'a> {
        crate::models::error::ApiError {
            error: match self {
                ApiError::Env(..) => "environment_error",
                ApiError::Banned(..) => "user_banned",
                ApiError::SqlxDatabase(..) => "database_error",
                ApiError::Database(..) => "database_error",
                ApiError::Authentication(..) => "unauthorized",
                ApiError::CustomAuthentication(..) => "unauthorized",
                ApiError::Xml(..) => "xml_error",
                ApiError::WikiBan(..) => "wiki_ban",
                ApiError::Json(..) => "json_error",
                ApiError::Search(..) => "search_error",
                ApiError::Indexing(..) => "indexing_error",
                ApiError::FileHosting(..) => "file_hosting_error",
                ApiError::InvalidInput(..) => "invalid_input",
                ApiError::Validation(..) => "invalid_input",
                ApiError::Payments(..) => "payments_error",
                ApiError::Discord(..) => "discord_error",
                ApiError::Turnstile => "turnstile_error",
                ApiError::Decoding(..) => "decoding_error",
                ApiError::ImageParse(..) => "invalid_image",
                ApiError::PasswordHashing(..) => "password_hashing_error",
                ApiError::Mail(..) => "mail_error",
                ApiError::Clickhouse(..) => "clickhouse_error",
                ApiError::Reroute(..) => "reroute_error",
                ApiError::NotFound => "not_found",
                ApiError::ISExists => "is_exists",
                ApiError::ISConflict(..) => "is_conflict",
                ApiError::Zip(..) => "zip_error",
                ApiError::Io(..) => "io_error",
                ApiError::RateLimitError(..) => "ratelimit_error",
                ApiError::Stripe(..) => "stripe_error",
                ApiError::AliyunSms(..) => "aliyun_sms_error",
                ApiError::ImageLimit(..) => "image_limit",
                ApiError::RiskLimit(..) => "risk_limit",
            },
            description: self.to_string(),
        }
    }
}

impl actix_web::ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Env(..) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Banned(..) => StatusCode::FORBIDDEN,
            ApiError::Database(..) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::SqlxDatabase(..) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Clickhouse(..) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Authentication(..) => StatusCode::UNAUTHORIZED,
            ApiError::CustomAuthentication(..) => StatusCode::UNAUTHORIZED,
            ApiError::Xml(..) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Json(..) => StatusCode::BAD_REQUEST,
            ApiError::Search(..) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Indexing(..) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::FileHosting(..) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::InvalidInput(..) => StatusCode::BAD_REQUEST,
            ApiError::Validation(..) => StatusCode::BAD_REQUEST,
            ApiError::Payments(..) => StatusCode::FAILED_DEPENDENCY,
            ApiError::Discord(..) => StatusCode::FAILED_DEPENDENCY,
            ApiError::Turnstile => StatusCode::BAD_REQUEST,
            ApiError::Decoding(..) => StatusCode::BAD_REQUEST,
            ApiError::ImageParse(..) => StatusCode::BAD_REQUEST,
            ApiError::PasswordHashing(..) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Mail(..) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Reroute(..) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::ISExists => StatusCode::BAD_REQUEST,
            ApiError::ISConflict(..) => StatusCode::BAD_REQUEST,
            ApiError::Zip(..) => StatusCode::BAD_REQUEST,
            ApiError::Io(..) => StatusCode::BAD_REQUEST,
            ApiError::RateLimitError(..) => StatusCode::TOO_MANY_REQUESTS,
            ApiError::Stripe(..) => StatusCode::FAILED_DEPENDENCY,
            ApiError::AliyunSms(..) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::WikiBan(..) => StatusCode::BAD_REQUEST,
            ApiError::ImageLimit(..) => StatusCode::BAD_REQUEST,
            ApiError::RiskLimit(..) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(self.as_api_error())
    }
}
