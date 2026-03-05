use crate::auth::email::send_email;
use crate::auth::validate::get_user_record_from_bearer_token;
use crate::auth::{AuthProvider, AuthenticationError, get_user_from_headers};
use crate::database::models::flow_item::Flow;
use crate::database::redis::RedisPool;
use crate::file_hosting::FileHost;
use crate::models::ids::base62_impl::{parse_base62, to_base62};
use crate::models::ids::random_base62_rng;
use crate::models::pats::Scopes;
use crate::models::users::{Badges, Role};
use crate::queue::session::AuthQueue;
use crate::queue::socket::ActiveSockets;
use crate::routes::ApiError;
use crate::routes::internal::session::issue_session;
use crate::util::captcha::check_hcaptcha;
use crate::util::env::parse_strings_from_var;
use crate::util::ext::get_image_ext;
use crate::util::img::upload_image_optimized;
use crate::util::phone::send_phone_number_code;
use crate::util::validate::{RE_URL_SAFE, validation_errors_to_string};
use actix_web::web::{Data, Payload, Query, ServiceConfig, scope};
use actix_web::{HttpRequest, HttpResponse, delete, get, patch, post, web};
use actix_ws::Closed;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::Engine;
use chrono::{Duration, Utc};
use rand::Rng;
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::SeedableRng;
use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator::Validate;

pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("auth")
            .service(ws_init)
            .service(init)
            .service(auth_callback)
            .service(delete_auth_provider)
            .service(create_account_with_password)
            .service(login_password)
            .service(login_2fa)
            .service(begin_2fa_flow)
            .service(finish_2fa_flow)
            .service(remove_2fa)
            .service(reset_password_begin)
            .service(change_password)
            .service(resend_verify_email)
            .service(set_email)
            .service(verify_email)
            .service(subscribe_newsletter)
            .service(phone_number_code)
            .service(phone_number_bind),
    );
}

#[derive(Debug)]
pub struct TempUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,

    pub avatar_url: Option<String>,
    pub bio: Option<String>,

    pub country: Option<String>,
}

impl TempUser {
    async fn create_account(
        self,
        provider: AuthProvider,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        client: &PgPool,
        file_host: &Arc<dyn FileHost + Send + Sync>,
        redis: &RedisPool,
    ) -> Result<crate::database::models::UserId, AuthenticationError> {
        if let Some(email) = &self.email
            && crate::database::models::User::get_email(email, client)
                .await?
                .is_some()
        {
            return Err(AuthenticationError::DuplicateUser);
        }

        let user_id =
            crate::database::models::generate_user_id(transaction).await?;

        let mut username_increment: i32 = 0;
        let mut username = None;

        while username.is_none() {
            let test_username = format!(
                "{}{}",
                self.username,
                if username_increment > 0 {
                    username_increment.to_string()
                } else {
                    "".to_string()
                }
            );

            let new_id = crate::database::models::User::get(
                &test_username,
                client,
                redis,
            )
            .await?;

            if new_id.is_none() {
                username = Some(test_username);
            } else {
                username_increment += 1;
            }
        }

        // 延迟创建头像审核记录（等用户 INSERT 后才能满足外键约束）
        let mut avatar_risk_info: Option<AvatarRiskInfo> = None;

        let (avatar_url, raw_avatar_url) = if let Some(avatar_url) =
            self.avatar_url
        {
            let res = reqwest::get(&avatar_url).await?;
            let headers = res.headers().clone();

            let img_data = if let Some(content_type) = headers
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|ct| ct.to_str().ok())
            {
                get_image_ext(content_type)
            } else {
                avatar_url.rsplit('.').next()
            };

            if let Some(ext) = img_data {
                let bytes = res.bytes().await?;

                let upload_result = upload_image_optimized(
                    &format!(
                        "user/{}",
                        crate::models::users::UserId::from(user_id)
                    ),
                    bytes,
                    ext,
                    Some(96),
                    Some(1.0),
                    &**file_host,
                    crate::util::img::UploadImagePos {
                        pos: "注册头像".to_string(),
                        url: format!("/user/{}", &self.username),
                        username: self.username.clone(),
                    },
                    redis,
                    true, // 跳过内置风控，下面手动检查
                )
                .await;

                if let Ok(upload_result) = upload_result {
                    // 手动风控检查，区分涉政（删除）和非涉政 REVIEW（进审核）
                    let risk_result =
                        crate::util::risk::check_image_risk_with_labels(
                            &upload_result.url,
                            &format!("/user/{}", &self.username),
                            &self.username,
                            "注册头像",
                            redis,
                        )
                        .await;

                    match risk_result {
                        Ok(result) if !result.passed => {
                            let is_political =
                                crate::util::risk::contains_political_labels(
                                    &result.labels,
                                );
                            avatar_risk_info = Some(AvatarRiskInfo {
                                image_url: upload_result.url.clone(),
                                raw_image_url: upload_result.raw_url.clone(),
                                frame_url: result.frame_url,
                                labels: result.labels,
                                is_political,
                            });
                            if is_political {
                                // 涉政：立即删除 S3 图片，头像置空
                                // 审计记录延迟到用户 INSERT 后在事务内创建
                                if let Err(e) =
                                    crate::util::img::delete_old_images(
                                        Some(upload_result.url),
                                        Some(upload_result.raw_url),
                                        &**file_host,
                                    )
                                    .await
                                {
                                    log::warn!("涉政头像删除S3文件失败: {}", e);
                                }
                                (None, None)
                            } else {
                                // 非涉政 REVIEW：保留图片（S3），但用户记录中不设头像，等审核通过后再更新
                                (None, None)
                            }
                        }
                        _ => {
                            // 通过或风控 API 异常：正常使用头像
                            (
                                Some(upload_result.url),
                                Some(upload_result.raw_url),
                            )
                        }
                    }
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // OAuth 注册用户名风控检测：不通过则用随机用户名替代
        if let Some(ref uname) = username {
            let risk_passed = crate::util::risk::check_text_risk(
                uname,
                uname,
                &format!("/auth/{:?}", provider),
                "OAuth注册用户名",
                redis,
            )
            .await
            .unwrap_or(true);
            if !risk_passed {
                log::warn!(
                    "OAuth 注册用户名风控未通过，使用随机用户名替代: {}",
                    uname
                );
                username = Some(format!(
                    "user_{}",
                    crate::models::ids::random_base62(6)
                ));
            }
        }

        if let Some(username) = username {
            crate::database::models::User {
                id: user_id,
                github_id: if provider == AuthProvider::GitHub {
                    Some(
                        self.id.clone().parse().map_err(|_| {
                            AuthenticationError::InvalidCredentials
                        })?,
                    )
                } else {
                    None
                },
                discord_id: if provider == AuthProvider::Discord {
                    Some(
                        self.id.parse().map_err(|_| {
                            AuthenticationError::InvalidCredentials
                        })?,
                    )
                } else {
                    None
                },
                gitlab_id: if provider == AuthProvider::GitLab {
                    Some(
                        self.id.parse().map_err(|_| {
                            AuthenticationError::InvalidCredentials
                        })?,
                    )
                } else {
                    None
                },
                google_id: if provider == AuthProvider::Google {
                    Some(self.id.clone())
                } else {
                    None
                },
                steam_id: if provider == AuthProvider::Steam {
                    Some(
                        self.id.parse().map_err(|_| {
                            AuthenticationError::InvalidCredentials
                        })?,
                    )
                } else {
                    None
                },
                microsoft_id: if provider == AuthProvider::Microsoft {
                    Some(self.id.clone())
                } else {
                    None
                },
                bilibili_id: if provider == AuthProvider::Bilibili {
                    Some(self.id.clone())
                } else {
                    None
                },
                qq_id: if provider == AuthProvider::QQ {
                    Some(self.id.clone())
                } else {
                    None
                },
                password: None,
                paypal_id: if provider == AuthProvider::PayPal {
                    Some(self.id)
                } else {
                    None
                },
                paypal_country: self.country,
                paypal_email: if provider == AuthProvider::PayPal {
                    self.email.clone()
                } else {
                    None
                },
                venmo_handle: None,
                stripe_customer_id: None,
                totp_secret: None,
                username,
                email: self.email,
                email_verified: true,
                avatar_url,
                raw_avatar_url,
                bio: self.bio,
                created: Utc::now(),
                role: Role::Developer.to_string(),
                badges: Badges::default(),
                wiki_ban_time: Default::default(),
                wiki_overtake_count: 0,
                phone_number: None,
                is_premium_creator: false,
                creator_verified_at: None,
                active_bans: vec![],
                pending_profile_reviews: vec![],
            }
            .insert(transaction)
            .await?;

            // 用户已插入（同一事务），在事务内创建头像审核记录（满足外键约束）
            if let Some(risk_info) = avatar_risk_info {
                let review_id = crate::models::ids::random_base62(8) as i64;
                let status = if risk_info.is_political {
                    "auto_deleted"
                } else {
                    "pending"
                };
                let old_value = serde_json::json!({
                    "avatar_url": null,
                    "raw_avatar_url": null,
                })
                .to_string();
                let new_value = serde_json::json!({
                    "avatar_url": &risk_info.image_url,
                    "raw_avatar_url": &risk_info.raw_image_url,
                    "frame_url": &risk_info.frame_url,
                })
                .to_string();
                if let Err(e) = sqlx::query!(
                    "INSERT INTO user_profile_reviews (id, user_id, review_type, old_value, new_value, risk_labels, status)
                     VALUES ($1, $2, 'avatar', $3, $4, $5, $6)",
                    review_id,
                    user_id.0,
                    &old_value,
                    &new_value,
                    &risk_info.labels,
                    status,
                )
                .execute(&mut **transaction)
                .await
                {
                    log::error!("创建头像审核记录失败: {}", e);
                }
            }

            Ok(user_id)
        } else {
            Err(AuthenticationError::InvalidCredentials)
        }
    }
}

/// OAuth 注册时头像风控检查的延迟信息
struct AvatarRiskInfo {
    image_url: String,
    raw_image_url: String,
    frame_url: Option<String>,
    labels: String,
    is_political: bool,
}

impl AuthProvider {
    pub fn get_redirect_url(
        &self,
        state: String,
    ) -> Result<String, AuthenticationError> {
        let self_addr = dotenvy::var("SELF_ADDR")?;
        let raw_redirect_uri = format!("{}/v2/auth/callback", self_addr);
        let redirect_uri = urlencoding::encode(&raw_redirect_uri);

        Ok(match self {
            AuthProvider::GitHub => {
                let client_id = dotenvy::var("GITHUB_CLIENT_ID")?;

                format!(
                    "https://github.com/login/oauth/authorize?client_id={}&prompt=select_account&state={}&scope=read%3Auser%20user%3Aemail&redirect_uri={}",
                    client_id, state, redirect_uri,
                )
            }
            AuthProvider::Discord => {
                let client_id = dotenvy::var("DISCORD_CLIENT_ID")?;

                format!(
                    "https://discord.com/api/oauth2/authorize?client_id={}&state={}&response_type=code&scope=identify%20email&redirect_uri={}",
                    client_id, state, redirect_uri
                )
            }
            AuthProvider::Microsoft => {
                let client_id = dotenvy::var("MICROSOFT_CLIENT_ID")?;

                format!(
                    "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id={}&response_type=code&scope=user.read&state={}&prompt=select_account&redirect_uri={}",
                    client_id, state, redirect_uri
                )
            }
            AuthProvider::GitLab => {
                let client_id = dotenvy::var("GITLAB_CLIENT_ID")?;

                format!(
                    "https://gitlab.com/oauth/authorize?client_id={}&state={}&scope=read_user+profile+email&response_type=code&redirect_uri={}",
                    client_id, state, redirect_uri,
                )
            }
            AuthProvider::Google => {
                let client_id = dotenvy::var("GOOGLE_CLIENT_ID")?;

                format!(
                    "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&state={}&scope={}&response_type=code&redirect_uri={}",
                    client_id,
                    state,
                    urlencoding::encode(
                        "https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile"
                    ),
                    redirect_uri,
                )
            }
            AuthProvider::Steam => {
                format!(
                    "https://steamcommunity.com/openid/login?openid.ns={}&openid.mode={}&openid.return_to={}{}{}&openid.realm={}&openid.identity={}&openid.claimed_id={}",
                    urlencoding::encode("http://specs.openid.net/auth/2.0"),
                    "checkid_setup",
                    redirect_uri,
                    urlencoding::encode("?state="),
                    state,
                    self_addr,
                    "http://specs.openid.net/auth/2.0/identifier_select",
                    "http://specs.openid.net/auth/2.0/identifier_select",
                )
            }
            AuthProvider::PayPal => {
                let api_url = dotenvy::var("PAYPAL_API_URL")?;
                let client_id = dotenvy::var("PAYPAL_CLIENT_ID")?;

                let auth_url = if api_url.contains("sandbox") {
                    "sandbox.paypal.com"
                } else {
                    "paypal.com"
                };

                format!(
                    "https://{auth_url}/connect?flowEntry=static&client_id={client_id}&scope={}&response_type=code&redirect_uri={redirect_uri}&state={state}",
                    urlencoding::encode(
                        "openid email address https://uri.paypal.com/services/paypalattributes"
                    ),
                )
            }
            AuthProvider::Bilibili => {
                let client_id = dotenvy::var("BILIBILI_CLIENT_ID")?;
                let raw_gourl =
                    format!("{}/v2/auth/callback", dotenvy::var("SELF_ADDR")?);
                let gourl = urlencoding::encode(&raw_gourl);

                format!(
                    "https://account.bilibili.com/pc/account-pc/auth/oauth?client_id={}&gourl={}&state={}",
                    client_id, gourl, state,
                )
            }
            AuthProvider::QQ => {
                let client_id = dotenvy::var("QQ_CLIENT_ID")?;

                format!(
                    "https://graph.qq.com/oauth2.0/authorize?response_type=code&client_id={}&redirect_uri={}&state={}&scope=get_user_info",
                    client_id, redirect_uri, state,
                )
            }
        })
    }

    pub async fn get_token(
        &self,
        query: HashMap<String, String>,
    ) -> Result<String, AuthenticationError> {
        let redirect_uri =
            format!("{}/v2/auth/callback", dotenvy::var("SELF_ADDR")?);

        #[derive(Deserialize)]
        struct AccessToken {
            pub access_token: String,
        }

        let res = match self {
            AuthProvider::GitHub => {
                let code = query
                    .get("code")
                    .ok_or_else(|| AuthenticationError::InvalidCredentials)?;
                let client_id = dotenvy::var("GITHUB_CLIENT_ID")?;
                let client_secret = dotenvy::var("GITHUB_CLIENT_SECRET")?;

                let url = format!(
                    "https://github.com/login/oauth/access_token?client_id={}&client_secret={}&code={}&redirect_uri={}",
                    client_id, client_secret, code, redirect_uri
                );

                let token: AccessToken = reqwest::Client::new()
                    .post(&url)
                    .header(reqwest::header::ACCEPT, "application/json")
                    .send()
                    .await?
                    .json()
                    .await?;

                token.access_token
            }
            AuthProvider::Discord => {
                let code = query
                    .get("code")
                    .ok_or_else(|| AuthenticationError::InvalidCredentials)?;
                let client_id = dotenvy::var("DISCORD_CLIENT_ID")?;
                let client_secret = dotenvy::var("DISCORD_CLIENT_SECRET")?;

                let mut map = HashMap::new();
                map.insert("client_id", &*client_id);
                map.insert("client_secret", &*client_secret);
                map.insert("code", code);
                map.insert("grant_type", "authorization_code");
                map.insert("redirect_uri", &redirect_uri);

                let token: AccessToken = reqwest::Client::new()
                    .post("https://discord.com/api/v10/oauth2/token")
                    .header(reqwest::header::ACCEPT, "application/json")
                    .form(&map)
                    .send()
                    .await?
                    .json()
                    .await?;

                token.access_token
            }
            AuthProvider::Microsoft => {
                let code = query
                    .get("code")
                    .ok_or_else(|| AuthenticationError::InvalidCredentials)?;
                let client_id = dotenvy::var("MICROSOFT_CLIENT_ID")?;
                let client_secret = dotenvy::var("MICROSOFT_CLIENT_SECRET")?;

                let mut map = HashMap::new();
                map.insert("client_id", &*client_id);
                map.insert("client_secret", &*client_secret);
                map.insert("code", code);
                map.insert("grant_type", "authorization_code");
                map.insert("redirect_uri", &redirect_uri);

                let token: AccessToken = reqwest::Client::new()
                    .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
                    .header(reqwest::header::ACCEPT, "application/json")
                    .form(&map)
                    .send()
                    .await?
                    .json()
                    .await?;

                token.access_token
            }
            AuthProvider::GitLab => {
                let code = query
                    .get("code")
                    .ok_or_else(|| AuthenticationError::InvalidCredentials)?;
                let client_id = dotenvy::var("GITLAB_CLIENT_ID")?;
                let client_secret = dotenvy::var("GITLAB_CLIENT_SECRET")?;

                let mut map = HashMap::new();
                map.insert("client_id", &*client_id);
                map.insert("client_secret", &*client_secret);
                map.insert("code", code);
                map.insert("grant_type", "authorization_code");
                map.insert("redirect_uri", &redirect_uri);

                let token: AccessToken = reqwest::Client::new()
                    .post("https://gitlab.com/oauth/token")
                    .header(reqwest::header::ACCEPT, "application/json")
                    .form(&map)
                    .send()
                    .await?
                    .json()
                    .await?;

                token.access_token
            }
            AuthProvider::Google => {
                let code = query
                    .get("code")
                    .ok_or_else(|| AuthenticationError::InvalidCredentials)?;
                let client_id = dotenvy::var("GOOGLE_CLIENT_ID")?;
                let client_secret = dotenvy::var("GOOGLE_CLIENT_SECRET")?;

                let mut map = HashMap::new();
                map.insert("client_id", &*client_id);
                map.insert("client_secret", &*client_secret);
                map.insert("code", code);
                map.insert("grant_type", "authorization_code");
                map.insert("redirect_uri", &redirect_uri);

                let token: AccessToken = reqwest::Client::new()
                    .post("https://oauth2.googleapis.com/token")
                    .header(reqwest::header::ACCEPT, "application/json")
                    .form(&map)
                    .send()
                    .await?
                    .json()
                    .await?;

                token.access_token
            }
            AuthProvider::Steam => {
                let mut form = HashMap::new();

                let signed = query
                    .get("openid.signed")
                    .ok_or_else(|| AuthenticationError::InvalidCredentials)?;
                form.insert(
                    "openid.assoc_handle".to_string(),
                    &**query.get("openid.assoc_handle").ok_or_else(|| {
                        AuthenticationError::InvalidCredentials
                    })?,
                );
                form.insert("openid.signed".to_string(), &**signed);
                form.insert(
                    "openid.sig".to_string(),
                    &**query.get("openid.sig").ok_or_else(|| {
                        AuthenticationError::InvalidCredentials
                    })?,
                );
                form.insert(
                    "openid.ns".to_string(),
                    "http://specs.openid.net/auth/2.0",
                );
                form.insert("openid.mode".to_string(), "check_authentication");

                for val in signed.split(',') {
                    if let Some(arr_val) = query.get(&format!("openid.{}", val))
                    {
                        form.insert(format!("openid.{}", val), &**arr_val);
                    }
                }

                let res = reqwest::Client::new()
                    .post("https://steamcommunity.com/openid/login")
                    .header("Accept-language", "en")
                    .form(&form)
                    .send()
                    .await?
                    .text()
                    .await?;

                if res.contains("is_valid:true") {
                    let identity =
                        query.get("openid.identity").ok_or_else(|| {
                            AuthenticationError::InvalidCredentials
                        })?;

                    identity
                        .rsplit('/')
                        .next()
                        .ok_or_else(|| AuthenticationError::InvalidCredentials)?
                        .to_string()
                } else {
                    return Err(AuthenticationError::InvalidCredentials);
                }
            }
            AuthProvider::PayPal => {
                let code = query
                    .get("code")
                    .ok_or_else(|| AuthenticationError::InvalidCredentials)?;
                let api_url = dotenvy::var("PAYPAL_API_URL")?;
                let client_id = dotenvy::var("PAYPAL_CLIENT_ID")?;
                let client_secret = dotenvy::var("PAYPAL_CLIENT_SECRET")?;

                let mut map = HashMap::new();
                map.insert("code", code.as_str());
                map.insert("grant_type", "authorization_code");

                let token: AccessToken = reqwest::Client::new()
                    .post(format!("{api_url}oauth2/token"))
                    .header(reqwest::header::ACCEPT, "application/json")
                    .header(
                        AUTHORIZATION,
                        format!(
                            "Basic {}",
                            base64::engine::general_purpose::STANDARD
                                .encode(format!("{client_id}:{client_secret}"))
                        ),
                    )
                    .form(&map)
                    .send()
                    .await?
                    .json()
                    .await?;

                token.access_token
            }
            AuthProvider::Bilibili => {
                let code = query
                    .get("code")
                    .ok_or_else(|| AuthenticationError::InvalidCredentials)?;
                let client_id = dotenvy::var("BILIBILI_CLIENT_ID")?;
                let client_secret = dotenvy::var("BILIBILI_CLIENT_SECRET")?;

                let url = format!(
                    "https://api.bilibili.com/x/account-oauth2/v1/token?client_id={}&client_secret={}&grant_type=authorization_code&code={}",
                    client_id, client_secret, code
                );

                let raw_resp = reqwest::Client::new()
                    .post(&url)
                    .header(
                        reqwest::header::CONTENT_TYPE,
                        "application/x-www-form-urlencoded",
                    )
                    .send()
                    .await?
                    .text()
                    .await?;

                log::debug!(
                    "Bilibili token response length: {}",
                    raw_resp.len()
                );

                #[derive(Deserialize)]
                struct BiliTokenData {
                    pub access_token: String,
                }
                #[derive(Deserialize)]
                struct BiliTokenResp {
                    pub code: i64,
                    pub data: Option<BiliTokenData>,
                }

                let resp: BiliTokenResp = serde_json::from_str(&raw_resp)?;

                if resp.code != 0 {
                    log::warn!(
                        "Bilibili token exchange failed with code: {}",
                        resp.code
                    );
                    return Err(AuthenticationError::InvalidCredentials);
                }

                resp.data
                    .ok_or(AuthenticationError::InvalidCredentials)?
                    .access_token
            }
            AuthProvider::QQ => {
                let code = query
                    .get("code")
                    .ok_or_else(|| AuthenticationError::InvalidCredentials)?;
                let client_id = dotenvy::var("QQ_CLIENT_ID")?;
                let client_secret = dotenvy::var("QQ_CLIENT_SECRET")?;

                // QQ 换 token 返回的是 URL query string 格式
                let url = format!(
                    "https://graph.qq.com/oauth2.0/token?grant_type=authorization_code&client_id={}&client_secret={}&code={}&redirect_uri={}&fmt=json",
                    client_id, client_secret, code, redirect_uri,
                );

                let raw_resp = reqwest::Client::new()
                    .get(&url)
                    .send()
                    .await?
                    .text()
                    .await?;

                log::debug!("QQ token response length: {}", raw_resp.len());

                #[derive(Deserialize)]
                struct QQTokenResp {
                    pub access_token: Option<String>,
                }

                let resp: QQTokenResp = serde_json::from_str(&raw_resp)?;

                resp.access_token
                    .ok_or(AuthenticationError::InvalidCredentials)?
            }
        };

        Ok(res)
    }

    pub async fn get_user(
        &self,
        token: &str,
    ) -> Result<TempUser, AuthenticationError> {
        let res = match self {
            AuthProvider::GitHub => {
                let response = reqwest::Client::new()
                    .get("https://api.github.com/user")
                    .header(reqwest::header::USER_AGENT, "Modrinth")
                    .header(AUTHORIZATION, format!("token {token}"))
                    .send()
                    .await?;

                if token.starts_with("gho_") {
                    let client_id = response
                        .headers()
                        .get("x-oauth-client-id")
                        .and_then(|x| x.to_str().ok());

                    if client_id
                        != Some(&*dotenvy::var("GITHUB_CLIENT_ID").unwrap())
                    {
                        return Err(AuthenticationError::InvalidClientId);
                    }
                }

                #[derive(Serialize, Deserialize, Debug)]
                pub struct GitHubUser {
                    pub login: String,
                    pub id: u64,
                    pub avatar_url: String,
                    pub name: Option<String>,
                    pub email: Option<String>,
                    pub bio: Option<String>,
                }

                let github_user: GitHubUser = response.json().await?;

                TempUser {
                    id: github_user.id.to_string(),
                    username: github_user.login,
                    email: github_user.email,
                    avatar_url: Some(github_user.avatar_url),
                    bio: github_user.bio,
                    country: None,
                }
            }
            AuthProvider::Discord => {
                #[derive(Serialize, Deserialize, Debug)]
                pub struct DiscordUser {
                    pub username: String,
                    pub id: String,
                    pub avatar: Option<String>,
                    pub global_name: Option<String>,
                    pub email: Option<String>,
                }

                let discord_user: DiscordUser = reqwest::Client::new()
                    .get("https://discord.com/api/v10/users/@me")
                    .header(reqwest::header::USER_AGENT, "Modrinth")
                    .header(AUTHORIZATION, format!("Bearer {token}"))
                    .send()
                    .await?
                    .json()
                    .await?;

                let id = discord_user.id.clone();
                TempUser {
                    id: discord_user.id,
                    username: discord_user.username,
                    email: discord_user.email,
                    avatar_url: discord_user.avatar.map(|x| {
                        format!(
                            "https://cdn.discordapp.com/avatars/{}/{}.webp",
                            id, x
                        )
                    }),
                    bio: None,
                    country: None,
                }
            }
            AuthProvider::Microsoft => {
                #[derive(Deserialize, Debug)]
                #[serde(rename_all = "camelCase")]
                pub struct MicrosoftUser {
                    pub id: String,
                    pub mail: Option<String>,
                    pub user_principal_name: String,
                }

                let microsoft_user: MicrosoftUser = reqwest::Client::new()
                    .get("https://graph.microsoft.com/v1.0/me?$select=id,displayName,mail,userPrincipalName")
                    .header(reqwest::header::USER_AGENT, "Modrinth")
                    .header(AUTHORIZATION, format!("Bearer {token}"))
                    .send()
                    .await?.json().await?;

                TempUser {
                    id: microsoft_user.id,
                    username: microsoft_user
                        .user_principal_name
                        .split('@')
                        .next()
                        .unwrap_or_default()
                        .to_string(),
                    email: microsoft_user.mail,
                    avatar_url: None,
                    bio: None,
                    country: None,
                }
            }
            AuthProvider::GitLab => {
                #[derive(Serialize, Deserialize, Debug)]
                pub struct GitLabUser {
                    pub id: i32,
                    pub username: String,
                    pub email: Option<String>,
                    pub avatar_url: Option<String>,
                    pub name: Option<String>,
                    pub bio: Option<String>,
                }

                let gitlab_user: GitLabUser = reqwest::Client::new()
                    .get("https://gitlab.com/api/v4/user")
                    .header(reqwest::header::USER_AGENT, "Modrinth")
                    .header(AUTHORIZATION, format!("Bearer {token}"))
                    .send()
                    .await?
                    .json()
                    .await?;

                TempUser {
                    id: gitlab_user.id.to_string(),
                    username: gitlab_user.username,
                    email: gitlab_user.email,
                    avatar_url: gitlab_user.avatar_url,
                    bio: gitlab_user.bio,
                    country: None,
                }
            }
            AuthProvider::Google => {
                #[derive(Deserialize, Debug)]
                pub struct GoogleUser {
                    pub id: String,
                    pub email: String,
                    pub picture: Option<String>,
                }

                let google_user: GoogleUser = reqwest::Client::new()
                    .get("https://www.googleapis.com/userinfo/v2/me")
                    .header(reqwest::header::USER_AGENT, "Modrinth")
                    .header(AUTHORIZATION, format!("Bearer {token}"))
                    .send()
                    .await?
                    .json()
                    .await?;

                TempUser {
                    id: google_user.id,
                    username: google_user
                        .email
                        .split('@')
                        .next()
                        .unwrap_or_default()
                        .to_string(),
                    email: Some(google_user.email),
                    avatar_url: google_user.picture,
                    bio: None,
                    country: None,
                }
            }
            AuthProvider::Steam => {
                let api_key = dotenvy::var("STEAM_API_KEY")?;

                #[derive(Deserialize)]
                struct SteamResponse {
                    response: Players,
                }

                #[derive(Deserialize)]
                struct Players {
                    players: Vec<Player>,
                }

                #[derive(Deserialize)]
                struct Player {
                    steamid: String,
                    profileurl: String,
                    avatar: Option<String>,
                }

                let response: String = reqwest::get(
                    &format!(
                        "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={}&steamids={}",
                        api_key,
                        token
                    )
                )
                    .await?
                    .text()
                    .await?;

                let mut response: SteamResponse =
                    serde_json::from_str(&response)?;

                if let Some(player) = response.response.players.pop() {
                    let username = player
                        .profileurl
                        .trim_matches('/')
                        .rsplit('/')
                        .next()
                        .unwrap_or(&player.steamid)
                        .to_string();
                    TempUser {
                        id: player.steamid,
                        username,
                        email: None,
                        avatar_url: player.avatar,
                        bio: None,
                        country: None,
                    }
                } else {
                    return Err(AuthenticationError::InvalidCredentials);
                }
            }
            AuthProvider::PayPal => {
                #[derive(Deserialize, Debug)]
                pub struct PayPalUser {
                    pub payer_id: String,
                    pub email: String,
                    pub picture: Option<String>,
                    pub address: PayPalAddress,
                }

                #[derive(Deserialize, Debug)]
                pub struct PayPalAddress {
                    pub country: String,
                }

                let api_url = dotenvy::var("PAYPAL_API_URL")?;

                let paypal_user: PayPalUser = reqwest::Client::new()
                    .get(format!(
                        "{api_url}identity/openidconnect/userinfo?schema=openid"
                    ))
                    .header(reqwest::header::USER_AGENT, "Modrinth")
                    .header(AUTHORIZATION, format!("Bearer {token}"))
                    .send()
                    .await?
                    .json()
                    .await?;

                TempUser {
                    id: paypal_user.payer_id,
                    username: paypal_user
                        .email
                        .split('@')
                        .next()
                        .unwrap_or_default()
                        .to_string(),
                    email: Some(paypal_user.email),
                    avatar_url: paypal_user.picture,
                    bio: None,
                    country: Some(paypal_user.address.country),
                }
            }
            AuthProvider::Bilibili => {
                use hmac::{Hmac, Mac};
                use sha2::Sha256;

                let client_id = dotenvy::var("BILIBILI_CLIENT_ID")?;
                let client_secret = dotenvy::var("BILIBILI_CLIENT_SECRET")?;

                let ts = Utc::now().timestamp().to_string();
                let nonce = Utc::now()
                    .timestamp_nanos_opt()
                    .unwrap_or_else(|| Utc::now().timestamp_millis())
                    .to_string();

                // GET 请求 body 为空，content-md5 = md5("")
                let content_md5 = format!("{:x}", md5::compute(b""));

                let headers_map = std::collections::BTreeMap::from([
                    ("x-bili-accesskeyid", client_id.as_str()),
                    ("x-bili-content-md5", content_md5.as_str()),
                    ("x-bili-signature-method", "HMAC-SHA256"),
                    ("x-bili-signature-nonce", nonce.as_str()),
                    ("x-bili-signature-version", "2.0"),
                    ("x-bili-timestamp", ts.as_str()),
                ]);

                let string_to_sign = headers_map
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect::<Vec<_>>()
                    .join("\n");

                let mut mac =
                    Hmac::<Sha256>::new_from_slice(client_secret.as_bytes())
                        .map_err(|_| AuthenticationError::InvalidCredentials)?;
                mac.update(string_to_sign.as_bytes());
                let signature = hex::encode(mac.finalize().into_bytes());

                let raw_resp = reqwest::Client::new()
                    .get("https://member.bilibili.com/arcopen/fn/user/account/info")
                    .header(reqwest::header::ACCEPT, "application/json")
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .header("access-token", token)
                    .header("x-bili-accesskeyid", &client_id)
                    .header("x-bili-content-md5", &content_md5)
                    .header("x-bili-signature-method", "HMAC-SHA256")
                    .header("x-bili-signature-nonce", &nonce)
                    .header("x-bili-signature-version", "2.0")
                    .header("x-bili-timestamp", &ts)
                    .header(AUTHORIZATION, &signature)
                    .send()
                    .await?
                    .text()
                    .await?;

                log::debug!(
                    "Bilibili user info response length: {}",
                    raw_resp.len()
                );

                #[derive(Deserialize, Debug)]
                struct BiliUserData {
                    pub name: String,
                    pub face: String,
                    pub openid: String,
                }
                #[derive(Deserialize, Debug)]
                struct BiliUserResp {
                    pub code: i64,
                    pub data: Option<BiliUserData>,
                }

                let bili_user: BiliUserResp = serde_json::from_str(&raw_resp)?;

                if bili_user.code != 0 {
                    log::warn!(
                        "Bilibili user info failed with code: {}",
                        bili_user.code
                    );
                    return Err(AuthenticationError::InvalidCredentials);
                }

                let user_data = bili_user
                    .data
                    .ok_or(AuthenticationError::InvalidCredentials)?;

                TempUser {
                    id: user_data.openid,
                    username: user_data.name,
                    email: None,
                    avatar_url: Some(user_data.face),
                    bio: None,
                    country: None,
                }
            }
            AuthProvider::QQ => {
                let client_id = dotenvy::var("QQ_CLIENT_ID")?;

                // 第一步：获取 openid
                let me_url = format!(
                    "https://graph.qq.com/oauth2.0/me?access_token={}&fmt=json",
                    token
                );

                #[derive(Deserialize, Debug)]
                struct QQMeResp {
                    pub openid: Option<String>,
                    pub client_id: Option<String>,
                }

                let me_resp: QQMeResp = reqwest::Client::new()
                    .get(&me_url)
                    .send()
                    .await?
                    .json()
                    .await?;

                let openid = me_resp
                    .openid
                    .ok_or(AuthenticationError::InvalidCredentials)?;

                // 验证 client_id 匹配
                if me_resp.client_id.as_deref() != Some(&client_id) {
                    log::warn!(
                        "QQ client_id mismatch: expected={}, got={:?}",
                        client_id,
                        me_resp.client_id
                    );
                    return Err(AuthenticationError::InvalidClientId);
                }

                // 第二步：获取用户信息
                let user_info_url = format!(
                    "https://graph.qq.com/user/get_user_info?access_token={}&oauth_consumer_key={}&openid={}",
                    token, client_id, openid
                );

                #[derive(Deserialize, Debug)]
                struct QQUserInfo {
                    pub ret: i64,
                    pub nickname: Option<String>,
                    #[serde(rename = "figureurl_qq_2")]
                    pub avatar_url: Option<String>,
                }

                let user_info: QQUserInfo = reqwest::Client::new()
                    .get(&user_info_url)
                    .send()
                    .await?
                    .json()
                    .await?;

                if user_info.ret != 0 {
                    log::warn!(
                        "QQ get_user_info failed with ret: {}",
                        user_info.ret
                    );
                    return Err(AuthenticationError::InvalidCredentials);
                }

                TempUser {
                    id: openid,
                    username: user_info
                        .nickname
                        .unwrap_or_else(|| "qq_user".to_string()),
                    email: None,
                    avatar_url: user_info.avatar_url,
                    bio: None,
                    country: None,
                }
            }
        };

        Ok(res)
    }

    pub async fn get_user_id<'a, 'b, E>(
        &self,
        id: &str,
        executor: E,
    ) -> Result<Option<crate::database::models::UserId>, AuthenticationError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        Ok(match self {
            AuthProvider::GitHub => {
                let value = sqlx::query!(
                    "SELECT id FROM users WHERE github_id = $1",
                    id.parse::<i64>()
                        .map_err(|_| AuthenticationError::InvalidCredentials)?
                )
                .fetch_optional(executor)
                .await?;

                value.map(|x| crate::database::models::UserId(x.id))
            }
            AuthProvider::Discord => {
                let value = sqlx::query!(
                    "SELECT id FROM users WHERE discord_id = $1",
                    id.parse::<i64>()
                        .map_err(|_| AuthenticationError::InvalidCredentials)?
                )
                .fetch_optional(executor)
                .await?;

                value.map(|x| crate::database::models::UserId(x.id))
            }
            AuthProvider::Microsoft => {
                let value = sqlx::query!(
                    "SELECT id FROM users WHERE microsoft_id = $1",
                    id
                )
                .fetch_optional(executor)
                .await?;

                value.map(|x| crate::database::models::UserId(x.id))
            }
            AuthProvider::GitLab => {
                let value = sqlx::query!(
                    "SELECT id FROM users WHERE gitlab_id = $1",
                    id.parse::<i64>()
                        .map_err(|_| AuthenticationError::InvalidCredentials)?
                )
                .fetch_optional(executor)
                .await?;

                value.map(|x| crate::database::models::UserId(x.id))
            }
            AuthProvider::Google => {
                let value = sqlx::query!(
                    "SELECT id FROM users WHERE google_id = $1",
                    id
                )
                .fetch_optional(executor)
                .await?;

                value.map(|x| crate::database::models::UserId(x.id))
            }
            AuthProvider::Steam => {
                let value = sqlx::query!(
                    "SELECT id FROM users WHERE steam_id = $1",
                    id.parse::<i64>()
                        .map_err(|_| AuthenticationError::InvalidCredentials)?
                )
                .fetch_optional(executor)
                .await?;

                value.map(|x| crate::database::models::UserId(x.id))
            }
            AuthProvider::PayPal => {
                let value = sqlx::query!(
                    "SELECT id FROM users WHERE paypal_id = $1",
                    id
                )
                .fetch_optional(executor)
                .await?;

                value.map(|x| crate::database::models::UserId(x.id))
            }
            AuthProvider::Bilibili => {
                let value = sqlx::query!(
                    "SELECT id FROM users WHERE bilibili_id = $1",
                    id
                )
                .fetch_optional(executor)
                .await?;

                value.map(|x| crate::database::models::UserId(x.id))
            }
            AuthProvider::QQ => {
                let value =
                    sqlx::query!("SELECT id FROM users WHERE qq_id = $1", id)
                        .fetch_optional(executor)
                        .await?;

                value.map(|x| crate::database::models::UserId(x.id))
            }
        })
    }

    pub async fn update_user_id(
        &self,
        user_id: crate::database::models::UserId,
        id: Option<&str>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), AuthenticationError> {
        match self {
            AuthProvider::GitHub => {
                sqlx::query!(
                    "
                    UPDATE users
                    SET github_id = $2
                    WHERE (id = $1)
                    ",
                    user_id as crate::database::models::UserId,
                    id.and_then(|x| x.parse::<i64>().ok())
                )
                .execute(&mut **transaction)
                .await?;
            }
            AuthProvider::Discord => {
                sqlx::query!(
                    "
                    UPDATE users
                    SET discord_id = $2
                    WHERE (id = $1)
                    ",
                    user_id as crate::database::models::UserId,
                    id.and_then(|x| x.parse::<i64>().ok())
                )
                .execute(&mut **transaction)
                .await?;
            }
            AuthProvider::Microsoft => {
                sqlx::query!(
                    "
                    UPDATE users
                    SET microsoft_id = $2
                    WHERE (id = $1)
                    ",
                    user_id as crate::database::models::UserId,
                    id,
                )
                .execute(&mut **transaction)
                .await?;
            }
            AuthProvider::GitLab => {
                sqlx::query!(
                    "
                    UPDATE users
                    SET gitlab_id = $2
                    WHERE (id = $1)
                    ",
                    user_id as crate::database::models::UserId,
                    id.and_then(|x| x.parse::<i64>().ok())
                )
                .execute(&mut **transaction)
                .await?;
            }
            AuthProvider::Google => {
                sqlx::query!(
                    "
                    UPDATE users
                    SET google_id = $2
                    WHERE (id = $1)
                    ",
                    user_id as crate::database::models::UserId,
                    id,
                )
                .execute(&mut **transaction)
                .await?;
            }
            AuthProvider::Steam => {
                sqlx::query!(
                    "
                    UPDATE users
                    SET steam_id = $2
                    WHERE (id = $1)
                    ",
                    user_id as crate::database::models::UserId,
                    id.and_then(|x| x.parse::<i64>().ok())
                )
                .execute(&mut **transaction)
                .await?;
            }
            AuthProvider::PayPal => {
                if id.is_none() {
                    sqlx::query!(
                        "
                        UPDATE users
                        SET paypal_country = NULL, paypal_email = NULL, paypal_id = NULL
                        WHERE (id = $1)
                        ",
                        user_id as crate::database::models::UserId,
                    )
                    .execute(&mut **transaction)
                    .await?;
                } else {
                    sqlx::query!(
                        "
                        UPDATE users
                        SET paypal_id = $2
                        WHERE (id = $1)
                        ",
                        user_id as crate::database::models::UserId,
                        id,
                    )
                    .execute(&mut **transaction)
                    .await?;
                }
            }
            AuthProvider::Bilibili => {
                sqlx::query!(
                    "
                    UPDATE users
                    SET bilibili_id = $2
                    WHERE (id = $1)
                    ",
                    user_id as crate::database::models::UserId,
                    id,
                )
                .execute(&mut **transaction)
                .await?;
            }
            AuthProvider::QQ => {
                sqlx::query!(
                    "
                    UPDATE users
                    SET qq_id = $2
                    WHERE (id = $1)
                    ",
                    user_id as crate::database::models::UserId,
                    id,
                )
                .execute(&mut **transaction)
                .await?;
            }
        }

        Ok(())
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AuthProvider::GitHub => "GitHub",
            AuthProvider::Discord => "Discord",
            AuthProvider::Microsoft => "Microsoft",
            AuthProvider::GitLab => "GitLab",
            AuthProvider::Google => "Google",
            AuthProvider::Steam => "Steam",
            AuthProvider::PayPal => "PayPal",
            AuthProvider::Bilibili => "Bilibili",
            AuthProvider::QQ => "QQ",
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct AuthorizationInit {
    pub url: String,
    #[serde(default)]
    pub provider: AuthProvider,
    pub token: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct Authorization {
    pub code: String,
    pub state: String,
}

// Init link takes us to GitHub API and calls back to callback endpoint with a code and state
// http://localhost:8000/auth/init?url=https://bbsmc.net
#[get("init")]
pub async fn init(
    req: HttpRequest,
    Query(info): Query<AuthorizationInit>, // callback url
    client: Data<PgPool>,
    redis: Data<RedisPool>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, AuthenticationError> {
    let url =
        url::Url::parse(&info.url).map_err(|_| AuthenticationError::Url)?;

    let allowed_callback_urls =
        parse_strings_from_var("ALLOWED_CALLBACK_URLS").unwrap_or_default();
    let domain = url.host_str().ok_or(AuthenticationError::Url)?;
    if !allowed_callback_urls.iter().any(|x| domain.ends_with(x))
        && domain != "bbsmc.net"
    {
        return Err(AuthenticationError::Url);
    }

    let user_id = if let Some(token) = info.token {
        let (_, user) = get_user_record_from_bearer_token(
            &req,
            Some(&token),
            &**client,
            &redis,
            &session_queue,
        )
        .await?
        .ok_or_else(|| AuthenticationError::InvalidCredentials)?;

        Some(user.id)
    } else {
        None
    };

    let state = Flow::OAuth {
        user_id,
        url: Some(info.url),
        provider: info.provider,
    }
    .insert(Duration::minutes(30), &redis)
    .await?;

    let url = info.provider.get_redirect_url(state)?;
    Ok(HttpResponse::TemporaryRedirect()
        .append_header(("Location", &*url))
        .json(serde_json::json!({ "url": url })))
}

#[derive(Serialize, Deserialize)]
pub struct WsInit {
    pub provider: AuthProvider,
}

#[get("ws")]
pub async fn ws_init(
    req: HttpRequest,
    Query(info): Query<WsInit>,
    body: Payload,
    db: Data<RwLock<ActiveSockets>>,
    redis: Data<RedisPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let (res, session, _msg_stream) = actix_ws::handle(&req, body)?;

    async fn sock(
        mut ws_stream: actix_ws::Session,
        info: WsInit,
        db: Data<RwLock<ActiveSockets>>,
        redis: Data<RedisPool>,
    ) -> Result<(), Closed> {
        let flow = Flow::OAuth {
            user_id: None,
            url: None,
            provider: info.provider,
        }
        .insert(Duration::minutes(30), &redis)
        .await;

        if let Ok(state) = flow
            && let Ok(url) = info.provider.get_redirect_url(state.clone())
        {
            ws_stream
                .text(serde_json::json!({ "url": url }).to_string())
                .await?;

            let db = db.write().await;
            db.auth_sockets.insert(state, ws_stream);
        }

        Ok(())
    }

    let _ = sock(session, info, db, redis).await;

    Ok(res)
}

#[get("callback")]
pub async fn auth_callback(
    req: HttpRequest,
    Query(query): Query<HashMap<String, String>>,
    active_sockets: Data<RwLock<ActiveSockets>>,
    client: Data<PgPool>,
    file_host: Data<Arc<dyn FileHost + Send + Sync>>,
    redis: Data<RedisPool>,
) -> Result<HttpResponse, crate::auth::templates::ErrorPage> {
    let state_string = query
        .get("state")
        .ok_or_else(|| AuthenticationError::InvalidCredentials)?
        .clone();

    let sockets = active_sockets.clone();
    let state = state_string.clone();
    let res: Result<HttpResponse, AuthenticationError> = async move {

        let flow = Flow::get(&state, &redis).await?;

        // Extract cookie header from request
        if let Some(Flow::OAuth {
                        user_id,
                        provider,
                        url,
                    }) = flow
        {
            Flow::remove(&state, &redis).await?;

            let token = provider.get_token(query).await?;
            let oauth_user = provider.get_user(&token).await?;

            let user_id_opt = provider.get_user_id(&oauth_user.id, &**client).await?;

            let mut transaction = client.begin().await?;
            if let Some(id) = user_id {
                if user_id_opt.is_some() {
                    return Err(AuthenticationError::DuplicateUser);
                }

                provider
                    .update_user_id(id, Some(&oauth_user.id), &mut transaction)
                    .await?;

                let user = crate::database::models::User::get_id(id, &**client, &redis).await?;

                if provider == AuthProvider::PayPal  {
                    sqlx::query!(
                        "
                        UPDATE users
                        SET paypal_country = $1, paypal_email = $2, paypal_id = $3
                        WHERE (id = $4)
                        ",
                        oauth_user.country,
                        oauth_user.email,
                        oauth_user.id,
                        id as crate::database::models::ids::UserId,
                    )
                        .execute(&mut *transaction)
                        .await?;
                } else if let Some(email) = user.and_then(|x| x.email) {
                    send_email(
                        email,
                        "已添加身份验证方法",
                        &format!("您现在可以使用 {} 身份验证提供程序登录 BBSMC。", provider.as_str()),
                        "如果不是您进行的更改，请立即发送电子邮件 (support@bbsmc.net) 联系我们。",
                        None,
                    )?;
                }

                transaction.commit().await?;
                crate::database::models::User::clear_caches(&[(id, None)], &redis).await?;

                if let Some(url) = url {
                    Ok(HttpResponse::TemporaryRedirect()
                        .append_header(("Location", &*url))
                        .json(serde_json::json!({ "url": url })))
                } else {
                    Err(AuthenticationError::InvalidCredentials)
                }
            } else {
                let user_id = if let Some(user_id) = user_id_opt {
                    let user = crate::database::models::User::get_id(user_id, &**client, &redis)
                        .await?
                        .ok_or_else(|| AuthenticationError::InvalidCredentials)?;

                    if user.totp_secret.is_some() {
                        let flow = Flow::Login2FA { user_id: user.id }
                            .insert(Duration::minutes(30), &redis)
                            .await?;

                        if let Some(url) = url {
                            let redirect_url = format!(
                                "{}{}error=2fa_required&flow={}",
                                url,
                                if url.contains('?') { "&" } else { "?" },
                                flow
                            );

                            return Ok(HttpResponse::TemporaryRedirect()
                                .append_header(("Location", &*redirect_url))
                                .json(serde_json::json!({ "url": redirect_url })));
                        } else {
                            let mut ws_conn = {
                                let db = sockets.read().await;

                                let mut x = db
                                    .auth_sockets
                                    .get_mut(&state)
                                    .ok_or_else(|| AuthenticationError::SocketError)?;

                                x.value_mut().clone()
                            };

                            ws_conn
                                .text(
                                    serde_json::json!({
                                        "error": "2fa_required",
                                        "flow": flow,
                                    }).to_string()
                                )
                                .await.map_err(|_| AuthenticationError::SocketError)?;

                            let _ = ws_conn.close(None).await;

                            return Ok(crate::auth::templates::Success {
                                icon: user.avatar_url.as_deref().unwrap_or("https://cdn.bbsmc.net/raw/placeholder.svg"),
                                name: &user.username,
                            }.render());
                        }
                    }

                    user_id
                } else {
                    oauth_user.create_account(provider, &mut transaction, &client, &file_host, &redis).await?
                };

                let session = issue_session(req, user_id, &mut transaction, &redis).await?;
                transaction.commit().await?;

                if let Some(url) = url {
                    let redirect_url = format!(
                        "{}{}code={}{}",
                        url,
                        if url.contains('?') { '&' } else { '?' },
                        session.session,
                        if user_id_opt.is_none() {
                            "&new_account=true"
                        } else {
                            ""
                        }
                    );

                    Ok(HttpResponse::TemporaryRedirect()
                        .append_header(("Location", &*redirect_url))
                        .json(serde_json::json!({ "url": redirect_url })))
                } else {
                    let user = crate::database::models::user_item::User::get_id(
                        user_id,
                        &**client,
                        &redis,
                    )
                        .await?.ok_or_else(|| AuthenticationError::InvalidCredentials)?;

                    let mut ws_conn = {
                        let db = sockets.read().await;

                        let mut x = db
                            .auth_sockets
                            .get_mut(&state)
                            .ok_or_else(|| AuthenticationError::SocketError)?;

                        x.value_mut().clone()
                    };

                    ws_conn
                        .text(
                            serde_json::json!({
                                        "code": session.session,
                                    }).to_string()
                        )
                        .await.map_err(|_| AuthenticationError::SocketError)?;
                    let _ = ws_conn.close(None).await;

                    Ok(crate::auth::templates::Success {
                        icon: user.avatar_url.as_deref().unwrap_or("https://cdn.bbsmc.net/raw/placeholder.svg"),
                        name: &user.username,
                    }.render())
                }
            }
        } else {
            Err::<HttpResponse, AuthenticationError>(AuthenticationError::InvalidCredentials)
        }
    }.await;

    // Because this is callback route, if we have an error, we need to ensure we close the original socket if it exists
    if let Err(ref e) = res {
        let db = active_sockets.read().await;
        let mut x = db.auth_sockets.get_mut(&state_string);

        if let Some(x) = x.as_mut() {
            let mut ws_conn = x.value_mut().clone();

            ws_conn
                .text(
                    serde_json::json!({
                                "error": &e.error_name(),
                                "description": &e.to_string(),
                            }        )
                    .to_string(),
                )
                .await
                .map_err(|_| AuthenticationError::SocketError)?;
            let _ = ws_conn.close(None).await;
        }
    }

    Ok(res?)
}

#[derive(Deserialize)]
pub struct DeleteAuthProvider {
    pub provider: AuthProvider,
}

#[delete("provider")]
pub async fn delete_auth_provider(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    delete_provider: web::Json<DeleteAuthProvider>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_AUTH_WRITE]),
    )
    .await?
    .1;

    if !user.auth_providers.map(|x| x.len() > 1).unwrap_or(false)
        && !user.has_password.unwrap_or(false)
    {
        return Err(ApiError::InvalidInput(
            "您必须为该帐户添加另一种身份验证方法！".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;

    delete_provider
        .provider
        .update_user_id(user.id.into(), None, &mut transaction)
        .await?;

    if delete_provider.provider != AuthProvider::PayPal
        && let Some(email) = user.email
    {
        send_email(
            email,
            "身份验证方法已移除",
            &format!(
                "您现在无法使用 {} 身份验证提供程序登录 BBSMC",
                delete_provider.provider.as_str()
            ),
            "如果不是您进行的更改，请立即通过电子邮件 (support@bbsmc.net) 联系我们。",
            None,
        )?;
    }

    transaction.commit().await?;
    crate::database::models::User::clear_caches(
        &[(user.id.into(), None)],
        &redis,
    )
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

// pub async fn sign_up_beehiiv(email: &str) -> Result<(), AuthenticationError> {
//     let id = dotenvy::var("BEEHIIV_PUBLICATION_ID")?;
//     let api_key = dotenvy::var("BEEHIIV_API_KEY")?;
//     let site_url = dotenvy::var("SITE_URL")?;
//
//     let client = reqwest::Client::new();
//     client
//         .post(format!(
//             "https://api.beehiiv.com/v2/publications/{id}/subscriptions"
//         ))
//         .header(AUTHORIZATION, format!("Bearer {}", api_key))
//         .json(&serde_json::json!({
//             "email": email,
//             "utm_source": "modrinth",
//             "utm_medium": "account_creation",
//             "referring_site": site_url,
//         }))
//         .send()
//         .await?
//         .error_for_status()?
//         .text()
//         .await?;
//
//     Ok(())
// }

#[derive(Deserialize, Validate)]
pub struct NewAccount {
    #[validate(length(min = 1, max = 39), regex(path = *RE_URL_SAFE))]
    pub username: String,
    #[validate(length(min = 8, max = 256))]
    pub password: String,
    #[validate(email)]
    pub email: String,
    pub challenge: String,
    pub sign_up_newsletter: Option<bool>,
}

#[post("create")]
pub async fn create_account_with_password(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    new_account: web::Json<NewAccount>,
) -> Result<HttpResponse, ApiError> {
    new_account.0.validate().map_err(|err| {
        ApiError::InvalidInput(validation_errors_to_string(err, None))
    })?;

    if !check_hcaptcha(&new_account.challenge).await? {
        return Err(ApiError::Turnstile);
    }

    if crate::database::models::User::get(
        &new_account.username,
        &**pool,
        &redis,
    )
    .await?
    .is_some()
    {
        return Err(ApiError::InvalidInput("用户名已被使用".to_string()));
    }

    // 用户名风控检测
    let risk_passed = crate::util::risk::check_text_risk(
        &new_account.username,
        &new_account.username,
        "/auth/create",
        "注册用户名",
        &redis,
    )
    .await?;
    if !risk_passed {
        return Err(ApiError::InvalidInput(
            "用户名包含违规内容，请更换用户名".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;
    let user_id =
        crate::database::models::generate_user_id(&mut transaction).await?;

    let new_account = new_account.0;

    let score = zxcvbn::zxcvbn(
        &new_account.password,
        &[&new_account.username, &new_account.email],
    );

    if score.score() < zxcvbn::Score::Three {
        return Err(ApiError::InvalidInput(
            if let Some(feedback) = score.feedback().and_then(|x| x.warning()) {
                format!("密码太弱 {}", feedback)
            } else {
                "密码强度太弱！请提高其强度。".to_string()
            },
        ));
    }

    let hasher = Argon2::default();
    let salt = SaltString::generate(&mut ChaCha20Rng::from_entropy());
    let password_hash = hasher
        .hash_password(new_account.password.as_bytes(), &salt)?
        .to_string();

    if crate::database::models::User::get_email(&new_account.email, &**pool)
        .await?
        .is_some()
    {
        return Err(ApiError::InvalidInput("该邮箱已被注册过".to_string()));
    }

    crate::database::models::User {
        id: user_id,
        github_id: None,
        discord_id: None,
        gitlab_id: None,
        google_id: None,
        steam_id: None,
        microsoft_id: None,
        bilibili_id: None,
        qq_id: None,
        password: Some(password_hash),
        paypal_id: None,
        paypal_country: None,
        paypal_email: None,
        venmo_handle: None,
        stripe_customer_id: None,
        totp_secret: None,
        username: new_account.username.clone(),
        email: Some(new_account.email.clone()),
        email_verified: false,
        avatar_url: None,
        raw_avatar_url: None,
        bio: None,
        created: Utc::now(),
        role: Role::Developer.to_string(),
        badges: Badges::default(),
        wiki_ban_time: Default::default(),
        wiki_overtake_count: 0,
        phone_number: None,
        is_premium_creator: false,
        creator_verified_at: None,
        active_bans: vec![],
        pending_profile_reviews: vec![],
    }
    .insert(&mut transaction)
    .await?;

    let session = issue_session(req, user_id, &mut transaction, &redis).await?;
    let res = crate::models::sessions::Session::from(session, true, None);

    let flow = Flow::ConfirmEmail {
        user_id,
        confirm_email: new_account.email.clone(),
    }
    .insert(Duration::hours(24), &redis)
    .await?;

    send_email_verify(
        new_account.email.clone(),
        flow,
        &format!("欢迎加入 BBSMC 资源社区, {}!", new_account.username),
    )?;

    if new_account.sign_up_newsletter.unwrap_or(false) {
        // sign_up_beehiiv(&new_account.email).await?;
    }

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(res))
}

#[derive(Deserialize, Validate)]
pub struct Login {
    pub username: String,
    pub password: String,
    pub challenge: String,
}

// #[derive(Deserialize, Validate)]
// pub struct Challenge {
//     pub captcha_id: String,
//     pub captcha_output: String,
//     pub gen_time: String,
//     pub lot_number: String,
//     pub pass_token: String,
// }

#[post("login")]
pub async fn login_password(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    login: web::Json<Login>,
) -> Result<HttpResponse, ApiError> {
    if !check_hcaptcha(&login.challenge).await? {
        return Err(ApiError::Turnstile);
    }

    let user = if let Some(user) =
        crate::database::models::User::get(&login.username, &**pool, &redis)
            .await?
    {
        user
    } else {
        let user =
            crate::database::models::User::get_email(&login.username, &**pool)
                .await?
                .ok_or_else(|| AuthenticationError::InvalidCredentials)?;

        crate::database::models::User::get_id(user, &**pool, &redis)
            .await?
            .ok_or_else(|| AuthenticationError::InvalidCredentials)?
    };

    let hasher = Argon2::default();
    hasher
        .verify_password(
            login.password.as_bytes(),
            &PasswordHash::new(
                &user
                    .password
                    .ok_or_else(|| AuthenticationError::InvalidCredentials)?,
            )?,
        )
        .map_err(|_| AuthenticationError::InvalidCredentials)?;

    if user.totp_secret.is_some() {
        let flow = Flow::Login2FA { user_id: user.id }
            .insert(Duration::minutes(30), &redis)
            .await?;

        Ok(HttpResponse::Ok().json(serde_json::json!({
            "error": "2fa_required",
            "description": "需要 2FA 才能完成此操作。",
            "flow": flow,
        })))
    } else {
        let mut transaction = pool.begin().await?;
        let session =
            issue_session(req, user.id, &mut transaction, &redis).await?;
        let res = crate::models::sessions::Session::from(session, true, None);
        transaction.commit().await?;

        Ok(HttpResponse::Ok().json(res))
    }
}

#[derive(Deserialize, Validate)]
pub struct PhoneNumberCode {
    pub phone_number: String,
    pub challenge: String,
}

#[post("phone_number_code")]
pub async fn phone_number_code(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    phone_number_code: web::Json<PhoneNumberCode>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    if !check_hcaptcha(&phone_number_code.challenge).await? {
        return Err(ApiError::Turnstile);
    }
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_AUTH_WRITE]),
    )
    .await?
    .1;

    if user.has_phonenumber.unwrap_or(false) {
        let user_ = if let Some(user_) =
            crate::database::models::User::get(&user.username, &**pool, &redis)
                .await?
        {
            user_
        } else {
            let user_ = crate::database::models::User::get_email(
                &user.username,
                &**pool,
            )
            .await?
            .ok_or_else(|| AuthenticationError::InvalidCredentials)?;

            crate::database::models::User::get_id(user_, &**pool, &redis)
                .await?
                .ok_or_else(|| AuthenticationError::InvalidCredentials)?
        };
        if user_.phone_number == Some(phone_number_code.phone_number.clone()) {
            return Err(ApiError::InvalidInput(
                "两次修改的手机号相同，请重新输入".to_string(),
            ));
        }
    }

    // 判断 90秒内是否发送过短信
    let mut conn = redis.connect().await?;
    let namespace = "phone_number_user_last_send".to_string();
    let value = conn.get(&namespace, &user.id.to_string()).await?;
    if value.is_some() {
        return Err(ApiError::InvalidInput("90秒内已发送过短信".to_string()));
    }

    // 随机生成 6位数字
    let code = rand::thread_rng().gen_range(100000..999999);

    // 生成UUID作为key
    let token = uuid::Uuid::new_v4().to_string();

    conn.set("phone_number_code", &token, &code.to_string(), Some(300))
        .await?;

    conn.set(
        "phone_number_cache",
        &token,
        &phone_number_code.phone_number,
        Some(300),
    )
    .await?;
    send_phone_number_code(&phone_number_code.phone_number, &code.to_string())
        .await?;

    //  记录90秒内发送过短信
    conn.set(&namespace, &user.id.to_string(), &token, Some(90))
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "token": token,
    })))
}

#[derive(Deserialize, Validate)]
pub struct PhoneNumberBind {
    pub phone_number: String,
    pub code: String,
    pub token: String,
}

#[post("phone_number_bind")]
pub async fn phone_number_bind(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    phone_number_bind: web::Json<PhoneNumberBind>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_AUTH_WRITE]),
    )
    .await?
    .1;
    let mut conn = redis.connect().await?;
    let namespace_code = "phone_number_code".to_string();
    let namespace_cache = "phone_number_cache".to_string();
    let value_code =
        conn.get(&namespace_code, &phone_number_bind.token).await?;
    let value_cache =
        conn.get(&namespace_cache, &phone_number_bind.token).await?;
    if value_code.is_none() || value_cache.is_none() {
        return Err(ApiError::InvalidInput("验证码已超时".to_string()));
    }

    if value_code.unwrap() != phone_number_bind.code {
        return Err(ApiError::InvalidInput("验证码错误".to_string()));
    }

    if value_cache.unwrap() != phone_number_bind.phone_number {
        return Err(ApiError::InvalidInput("手机号错误".to_string()));
    }

    let mut transaction = pool.begin().await?;

    sqlx::query!(
        "
        UPDATE users
        SET phone_number = $1
        WHERE (id = $2)
        ",
        phone_number_bind.phone_number,
        user.id.0 as i64,
    )
    .execute(&mut *transaction)
    .await?;

    if let Some(user_email) = user.email {
        send_email(
            user_email,
            "手机号已绑定",
            &format!(
                "您的账户手机号已更新为 {}。",
                phone_number_bind.phone_number
            ),
            "如果不是您进行的更改，请立即通过我们的电子邮件 (support@bbsmc.net) 联系我们。",
            None,
        )?;
    }

    transaction.commit().await?;

    crate::database::models::User::clear_caches(
        &[(user.id.into(), None)],
        &redis,
    )
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize, Validate)]
pub struct Login2FA {
    pub code: String,
    pub flow: String,
}

async fn validate_2fa_code(
    input: String,
    secret: String,
    allow_backup: bool,
    user_id: crate::database::models::UserId,
    redis: &RedisPool,
    pool: &PgPool,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<bool, AuthenticationError> {
    let totp = totp_rs::TOTP::new(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        totp_rs::Secret::Encoded(secret)
            .to_bytes()
            .map_err(|_| AuthenticationError::InvalidCredentials)?,
    )
    .map_err(|_| AuthenticationError::InvalidCredentials)?;

    const TOTP_NAMESPACE: &str = "used_totp";
    let mut conn = redis.connect().await?;

    // Check if TOTP has already been used
    if conn
        .get(TOTP_NAMESPACE, &format!("{}-{}", input, user_id.0))
        .await?
        .is_some()
    {
        return Err(AuthenticationError::InvalidCredentials);
    }

    // 使用 check_current 进行时间偏移容错验证
    if totp
        .check_current(input.as_str())
        .map_err(|_| AuthenticationError::InvalidCredentials)?
    {
        conn.set(
            TOTP_NAMESPACE,
            &format!("{}-{}", input, user_id.0),
            "",
            Some(60),
        )
        .await?;

        Ok(true)
    } else if allow_backup {
        let backup_codes =
            crate::database::models::User::get_backup_codes(user_id, pool)
                .await?;

        if !backup_codes.contains(&input) {
            Ok(false)
        } else {
            let code = parse_base62(&input).unwrap_or_default();

            sqlx::query!(
                "
                    DELETE FROM user_backup_codes
                    WHERE user_id = $1 AND code = $2
                    ",
                user_id as crate::database::models::ids::UserId,
                code as i64,
            )
            .execute(&mut **transaction)
            .await?;

            crate::database::models::User::clear_caches(
                &[(user_id, None)],
                redis,
            )
            .await?;

            Ok(true)
        }
    } else {
        Err(AuthenticationError::InvalidCredentials)
    }
}

#[post("login/2fa")]
pub async fn login_2fa(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    login: web::Json<Login2FA>,
) -> Result<HttpResponse, ApiError> {
    let flow = Flow::get(&login.flow, &redis)
        .await?
        .ok_or_else(|| AuthenticationError::InvalidCredentials)?;

    if let Flow::Login2FA { user_id } = flow {
        let user =
            crate::database::models::User::get_id(user_id, &**pool, &redis)
                .await?
                .ok_or_else(|| AuthenticationError::InvalidCredentials)?;

        let mut transaction = pool.begin().await?;
        if !validate_2fa_code(
            login.code.clone(),
            user.totp_secret
                .ok_or_else(|| AuthenticationError::InvalidCredentials)?,
            true,
            user.id,
            &redis,
            &pool,
            &mut transaction,
        )
        .await?
        {
            return Err(ApiError::Authentication(
                AuthenticationError::InvalidCredentials,
            ));
        }
        Flow::remove(&login.flow, &redis).await?;

        let session =
            issue_session(req, user_id, &mut transaction, &redis).await?;
        let res = crate::models::sessions::Session::from(session, true, None);
        transaction.commit().await?;

        Ok(HttpResponse::Ok().json(res))
    } else {
        Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ))
    }
}

#[post("2fa/get_secret")]
pub async fn begin_2fa_flow(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_AUTH_WRITE]),
    )
    .await?
    .1;

    if !user.has_totp.unwrap_or(false) {
        let string = totp_rs::Secret::generate_secret();
        let encoded = string.to_encoded();

        let flow = Flow::Initialize2FA {
            user_id: user.id.into(),
            secret: encoded.to_string(),
        }
        .insert(Duration::minutes(30), &redis)
        .await?;

        Ok(HttpResponse::Ok().json(serde_json::json!({
            "secret": encoded.to_string(),
            "flow": flow,
        })))
    } else {
        Err(ApiError::InvalidInput(
            "用户已在其帐户上启用 2FA！".to_string(),
        ))
    }
}

#[post("2fa")]
pub async fn finish_2fa_flow(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    login: web::Json<Login2FA>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let flow = Flow::get(&login.flow, &redis)
        .await?
        .ok_or_else(|| AuthenticationError::InvalidCredentials)?;

    if let Flow::Initialize2FA { user_id, secret } = flow {
        let user = get_user_from_headers(
            &req,
            &**pool,
            &redis,
            &session_queue,
            Some(&[Scopes::USER_AUTH_WRITE]),
        )
        .await?
        .1;

        if user.id != user_id.into() {
            return Err(ApiError::Authentication(
                AuthenticationError::InvalidCredentials,
            ));
        }

        let mut transaction = pool.begin().await?;

        if !validate_2fa_code(
            login.code.clone(),
            secret.clone(),
            false,
            user.id.into(),
            &redis,
            &pool,
            &mut transaction,
        )
        .await?
        {
            return Err(ApiError::Authentication(
                AuthenticationError::InvalidCredentials,
            ));
        }

        Flow::remove(&login.flow, &redis).await?;

        sqlx::query!(
            "
            UPDATE users
            SET totp_secret = $1
            WHERE (id = $2)
            ",
            secret,
            user_id as crate::database::models::ids::UserId,
        )
        .execute(&mut *transaction)
        .await?;

        sqlx::query!(
            "
            DELETE FROM user_backup_codes
            WHERE user_id = $1
            ",
            user_id as crate::database::models::ids::UserId,
        )
        .execute(&mut *transaction)
        .await?;

        let mut codes = Vec::new();

        for _ in 0..6 {
            let mut rng = ChaCha20Rng::from_entropy();
            let val = random_base62_rng(&mut rng, 11);

            sqlx::query!(
                "
                INSERT INTO user_backup_codes (
                    user_id, code
                )
                VALUES (
                    $1, $2
                )
                ",
                user_id as crate::database::models::ids::UserId,
                val as i64,
            )
            .execute(&mut *transaction)
            .await?;

            codes.push(to_base62(val));
        }

        if let Some(email) = user.email {
            send_email(
                email,
                "已启用双因素身份验证",
                "登录 BBSMC 时，您现在可以在输入常用的电子邮件地址和密码后，输入由身份验证应用生成的代码。",
                "如果不是您进行的更改，请立即通过我们的电子邮件(support@bbsmc.net)联系我们。",
                None,
            )?;
        }

        transaction.commit().await?;
        crate::database::models::User::clear_caches(
            &[(user.id.into(), None)],
            &redis,
        )
        .await?;

        Ok(HttpResponse::Ok().json(serde_json::json!({
            "backup_codes": codes,
        })))
    } else {
        Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ))
    }
}

#[derive(Deserialize)]
pub struct Remove2FA {
    pub code: String,
}

#[delete("2fa")]
pub async fn remove_2fa(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    login: web::Json<Remove2FA>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (scopes, user) = get_user_record_from_bearer_token(
        &req,
        None,
        &**pool,
        &redis,
        &session_queue,
    )
    .await?
    .ok_or_else(|| AuthenticationError::InvalidCredentials)?;

    if !scopes.contains(Scopes::USER_AUTH_WRITE) {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

    let mut transaction = pool.begin().await?;

    if !validate_2fa_code(
        login.code.clone(),
        user.totp_secret.ok_or_else(|| {
            ApiError::InvalidInput("该账户未启用双因素身份验证！".to_string())
        })?,
        true,
        user.id,
        &redis,
        &pool,
        &mut transaction,
    )
    .await?
    {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

    sqlx::query!(
        "
        UPDATE users
        SET totp_secret = NULL
        WHERE (id = $1)
        ",
        user.id as crate::database::models::ids::UserId,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query!(
        "
        DELETE FROM user_backup_codes
        WHERE user_id = $1
        ",
        user.id as crate::database::models::ids::UserId,
    )
    .execute(&mut *transaction)
    .await?;

    if let Some(email) = user.email {
        send_email(
            email,
            "双因素身份验证已移除",
            "登录 BBSMC 时，您不再需要双因素身份验证即可访问。",
            "如果不是您进行的更改，请立即通过电子邮件 (support@bbsmc.net) 联系我们。",
            None,
        )?;
    }

    transaction.commit().await?;
    crate::database::models::User::clear_caches(&[(user.id, None)], &redis)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize)]
pub struct ResetPassword {
    pub username: String,
    pub challenge: String,
}

#[post("password/reset")]
pub async fn reset_password_begin(
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    reset_password: web::Json<ResetPassword>,
) -> Result<HttpResponse, ApiError> {
    if !check_hcaptcha(&reset_password.challenge).await? {
        return Err(ApiError::Turnstile);
    }

    let user = if let Some(user_id) = crate::database::models::User::get_email(
        &reset_password.username,
        &**pool,
    )
    .await?
    {
        crate::database::models::User::get_id(user_id, &**pool, &redis).await?
    } else {
        crate::database::models::User::get(
            &reset_password.username,
            &**pool,
            &redis,
        )
        .await?
    };

    if let Some(user) = user {
        let flow = Flow::ForgotPassword { user_id: user.id }
            .insert(Duration::hours(24), &redis)
            .await?;

        if let Some(email) = user.email {
            send_email(
                email,
                "重置您的密码",
                "请访问以下链接以重置您的密码。如果按钮无法使用，您可以复制链接并将其粘贴到浏览器中。",
                "如果您没有请求重置密码，您可以放心忽略此邮件。",
                Some((
                    "重置密码",
                    &format!(
                        "{}/{}?flow={}",
                        dotenvy::var("SITE_URL")?,
                        dotenvy::var("SITE_RESET_PASSWORD_PATH")?,
                        flow
                    ),
                )),
            )?;
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[derive(Deserialize, Validate)]
pub struct ChangePassword {
    pub flow: Option<String>,
    pub old_password: Option<String>,
    pub new_password: Option<String>,
}

#[patch("password")]
pub async fn change_password(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    change_password: web::Json<ChangePassword>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = if let Some(flow) = &change_password.flow {
        let flow = Flow::get(flow, &redis).await?;

        if let Some(Flow::ForgotPassword { user_id }) = flow {
            let user =
                crate::database::models::User::get_id(user_id, &**pool, &redis)
                    .await?
                    .ok_or_else(|| AuthenticationError::InvalidCredentials)?;

            Some(user)
        } else {
            None
        }
    } else {
        None
    };

    let user = if let Some(user) = user {
        user
    } else {
        let (scopes, user) = get_user_record_from_bearer_token(
            &req,
            None,
            &**pool,
            &redis,
            &session_queue,
        )
        .await?
        .ok_or_else(|| AuthenticationError::InvalidCredentials)?;

        if !scopes.contains(Scopes::USER_AUTH_WRITE) {
            return Err(ApiError::Authentication(
                AuthenticationError::InvalidCredentials,
            ));
        }

        if let Some(pass) = user.password.as_ref() {
            let old_password =
                change_password.old_password.as_ref().ok_or_else(|| {
                    ApiError::CustomAuthentication(
                        "修改密码时必须提供旧密码！".to_string(),
                    )
                })?;

            let hasher = Argon2::default();
            hasher.verify_password(
                old_password.as_bytes(),
                &PasswordHash::new(pass)?,
            )?;
        }

        user
    };

    let mut transaction = pool.begin().await?;

    let update_password =
        if let Some(new_password) = &change_password.new_password {
            let score = zxcvbn::zxcvbn(
                new_password,
                &[&user.username, &user.email.clone().unwrap_or_default()],
            );

            if score.score() < zxcvbn::Score::Three {
                return Err(ApiError::InvalidInput(
                    if let Some(feedback) =
                        score.feedback().and_then(|x| x.warning())
                    {
                        format!("密码强度太弱：{}", feedback)
                    } else {
                        "密码强度太弱！请提高密码复杂度。".to_string()
                    },
                ));
            }

            let hasher = Argon2::default();
            let salt = SaltString::generate(&mut ChaCha20Rng::from_entropy());
            let password_hash = hasher
                .hash_password(new_password.as_bytes(), &salt)?
                .to_string();

            Some(password_hash)
        } else {
            if !(user.github_id.is_some()
                || user.gitlab_id.is_some()
                || user.microsoft_id.is_some()
                || user.google_id.is_some()
                || user.steam_id.is_some()
                || user.discord_id.is_some())
            {
                return Err(ApiError::InvalidInput(
                    "移除密码登录前，必须先添加其他身份验证方式！".to_string(),
                ));
            }

            None
        };

    sqlx::query!(
        "
        UPDATE users
        SET password = $1
        WHERE (id = $2)
        ",
        update_password,
        user.id as crate::database::models::ids::UserId,
    )
    .execute(&mut *transaction)
    .await?;

    if let Some(flow) = &change_password.flow {
        Flow::remove(flow, &redis).await?;
    }

    if let Some(email) = user.email {
        let changed = if update_password.is_some() {
            "修改完成"
        } else {
            "已删除"
        };

        send_email(
            email,
            &format!("密码{}", changed),
            &format!("您的账户密码已{}。", changed),
            "如果不是您进行的更改，请立即通过我们的电子邮件 (support@bbsmc.net) 联系我们。",
            None,
        )?;
    }

    transaction.commit().await?;
    crate::database::models::User::clear_caches(&[(user.id, None)], &redis)
        .await?;

    Ok(HttpResponse::Ok().finish())
}

#[derive(Deserialize, Validate)]
pub struct SetEmail {
    #[validate(email)]
    pub email: String,
}

#[patch("email")]
pub async fn set_email(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    email: web::Json<SetEmail>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    email.0.validate().map_err(|err| {
        ApiError::InvalidInput(validation_errors_to_string(err, None))
    })?;

    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_AUTH_WRITE]),
    )
    .await?
    .1;

    let mut transaction = pool.begin().await?;

    sqlx::query!(
        "
        UPDATE users
        SET email = $1, email_verified = FALSE
        WHERE (id = $2)
        ",
        email.email,
        user.id.0 as i64,
    )
    .execute(&mut *transaction)
    .await?;

    if let Some(user_email) = user.email {
        send_email(
            user_email,
            "邮箱已更改",
            &format!("您的账户邮箱已更新为 {}。", email.email),
            "如果不是您进行的更改，请立即通过我们的电子邮件 (support@bbsmc.net) 联系我们。",
            None,
        )?;
    }

    let flow = Flow::ConfirmEmail {
        user_id: user.id.into(),
        confirm_email: email.email.clone(),
    }
    .insert(Duration::hours(24), &redis)
    .await?;

    send_email_verify(email.email.clone(), flow, "我们需要验证您的邮箱地址。")?;

    transaction.commit().await?;
    crate::database::models::User::clear_caches(
        &[(user.id.into(), None)],
        &redis,
    )
    .await?;

    Ok(HttpResponse::Ok().finish())
}

#[post("email/resend_verify")]
pub async fn resend_verify_email(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_AUTH_WRITE]),
    )
    .await?
    .1;

    if let Some(email) = user.email {
        if user.email_verified.unwrap_or(false) {
            return Err(ApiError::InvalidInput("用户邮箱已验证！".to_string()));
        }

        let flow = Flow::ConfirmEmail {
            user_id: user.id.into(),
            confirm_email: email.clone(),
        }
        .insert(Duration::hours(24), &redis)
        .await?;

        send_email_verify(email, flow, "我们需要验证您的邮箱地址。")?;

        Ok(HttpResponse::NoContent().finish())
    } else {
        Err(ApiError::InvalidInput("该账户未设置电子邮箱".to_string()))
    }
}

#[derive(Deserialize)]
pub struct VerifyEmail {
    pub flow: String,
}

#[post("email/verify")]
pub async fn verify_email(
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    email: web::Json<VerifyEmail>,
) -> Result<HttpResponse, ApiError> {
    let flow = Flow::get(&email.flow, &redis).await?;

    if let Some(Flow::ConfirmEmail {
        user_id,
        confirm_email,
    }) = flow
    {
        let user =
            crate::database::models::User::get_id(user_id, &**pool, &redis)
                .await?
                .ok_or_else(|| AuthenticationError::InvalidCredentials)?;

        if user.email != Some(confirm_email) {
            return Err(ApiError::InvalidInput(
                "邮箱地址与待验证邮箱不匹配，请重新获取验证链接。".to_string(),
            ));
        }

        let mut transaction = pool.begin().await?;

        sqlx::query!(
            "
            UPDATE users
            SET email_verified = TRUE
            WHERE (id = $1)
            ",
            user.id as crate::database::models::ids::UserId,
        )
        .execute(&mut *transaction)
        .await?;

        Flow::remove(&email.flow, &redis).await?;
        transaction.commit().await?;
        crate::database::models::User::clear_caches(&[(user.id, None)], &redis)
            .await?;

        Ok(HttpResponse::NoContent().finish())
    } else {
        Err(ApiError::InvalidInput(
            "验证流程不存在或已过期，请重新获取验证链接。".to_string(),
        ))
    }
}

#[post("email/subscribe")]
pub async fn subscribe_newsletter(
    req: HttpRequest,
    pool: Data<PgPool>,
    redis: Data<RedisPool>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_AUTH_WRITE]),
    )
    .await?
    .1;

    if let Some(email) = user.email {
        // sign_up_beehiiv(&email).await?;
        println!("Signed up {} for newsletter", email);
        Ok(HttpResponse::NoContent().finish())
    } else {
        Err(ApiError::InvalidInput(
            "该账户没有设置电子邮箱地址".to_string(),
        ))
    }
}

fn send_email_verify(
    email: String,
    flow: String,
    opener: &str,
) -> Result<(), crate::auth::email::MailError> {
    send_email(
        email,
        "验证您的邮箱",
        opener,
        "请点击下面的链接以验证您的邮箱。如果按钮无法使用，您可以复制链接并粘贴到浏览器中。该链接将在 24 小时后失效。",
        Some((
            "验证邮箱",
            &format!(
                "{}/{}?flow={}",
                dotenvy::var("SITE_URL")?,
                dotenvy::var("SITE_VERIFY_EMAIL_PATH")?,
                flow
            ),
        )),
    )
}
