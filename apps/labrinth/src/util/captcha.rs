use crate::database::redis::RedisPool;
use crate::routes::ApiError;
use serde::Deserialize;

/// 一次性 token 的保留时长（秒），覆盖 TAC token 的有效期即可
const CAPTCHA_TOKEN_TTL: i64 = 300;

pub async fn check_hcaptcha(
    challenge: &String,
    redis: &RedisPool,
) -> Result<bool, ApiError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| ApiError::Turnstile)?;

    #[derive(Deserialize, Debug)]
    struct Response {
        code: i32,
        msg: String,
        // data: Option<serde_json::Value>,
        success: bool,
    }

    let url = dotenvy::var("TAC_URL")?;

    // 安全加固: challenge 由客户端提供，会被拼进 TAC_URL 路径末尾。
    // 若其中含路径分隔符（/ \）或百分号编码（%），可把请求重定向到同主机的其他接口
    // （如 .../secondaryValidation/../gen），从而绕过二次校验。
    // 这里拒绝这些字符和空白/控制字符，并限制长度；合法 token 不含这些字符。
    if challenge.is_empty()
        || challenge.len() > 512
        || challenge.chars().any(|c| {
            matches!(c, '/' | '\\' | '%') || c.is_whitespace() || c.is_control()
        })
    {
        return Err(ApiError::Turnstile);
    }

    // 拼接 url 和 challenge
    let url_challenge = format!("{}{}", url, challenge);
    let val: Response = client
        .post(url_challenge)
        .send()
        .await
        .map_err(|_| ApiError::Turnstile)?
        .json()
        .await
        .map_err(|_| ApiError::Turnstile)?;

    if val.code == 200 && val.success {
        // 安全加固: 一次性使用。校验成功后用 Redis 原子标记该 token，
        // 重复提交同一 token 直接拒绝（防重放）。
        // Redis 不可用时放行（fail-open），避免给验证码新增一个硬依赖导致登录中断。
        match redis.connect().await {
            Ok(mut conn) => {
                match conn
                    .set_if_not_exists(
                        "captcha_used",
                        challenge,
                        CAPTCHA_TOKEN_TTL,
                    )
                    .await
                {
                    Ok(true) => Ok(true),
                    Ok(false) => Err(ApiError::InvalidInput(
                        "验证码已被使用，请重新验证。".to_string(),
                    )),
                    Err(e) => {
                        tracing::warn!(
                            "验证码一次性标记写入失败，已放行: {e:?}"
                        );
                        Ok(true)
                    }
                }
            }
            Err(e) => {
                tracing::warn!("验证码一次性标记无法连接 Redis，已放行: {e:?}");
                Ok(true)
            }
        }
    } else {
        Err(ApiError::InvalidInput(val.msg))
    }
}
