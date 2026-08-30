//! 从环境/文件加载云账户凭据，并在进程内缓存。

use std::fmt;
use std::sync::OnceLock;

use super::error::YzhError;

/// 进程级凭据快照。
#[derive(Clone)]
pub struct YzhCredentials {
    pub api_url: String,
    pub dealer_id: String,
    pub broker_id: String,
    pub app_key: String,
    pub des_key: String,
    /// PKCS#8 PEM 字符串（含 `-----BEGIN/END PRIVATE KEY-----`）
    pub dealer_private_key_pem: String,
    /// X.509 PEM 字符串（含 `-----BEGIN/END PUBLIC KEY-----`）
    pub platform_public_key_pem: String,
}

impl fmt::Debug for YzhCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("YzhCredentials")
            .field("api_url", &self.api_url)
            .field("dealer_id", &self.dealer_id)
            .field("broker_id", &self.broker_id)
            .field("app_key", &"<redacted>")
            .field("des_key", &"<redacted>")
            .field("dealer_private_key_pem", &"<redacted>")
            .field("platform_public_key_pem", &"<redacted>")
            .finish()
    }
}

static CREDENTIALS: OnceLock<YzhCredentials> = OnceLock::new();

/// 加载并缓存凭据。首次调用会读环境变量和密钥文件，后续直接返回缓存。
pub fn load() -> Result<&'static YzhCredentials, YzhError> {
    if let Some(c) = CREDENTIALS.get() {
        return Ok(c);
    }
    let c = load_uncached()?;
    let _ = CREDENTIALS.set(c);
    Ok(CREDENTIALS.get().expect("just set"))
}

fn env(name: &'static str) -> Result<String, YzhError> {
    dotenvy::var(name).map_err(|_| YzhError::MissingEnv(name))
}

fn read_pem_file(env_key: &'static str) -> Result<String, YzhError> {
    let path = env(env_key)?;
    std::fs::read_to_string(&path)
        .map_err(|source| YzhError::KeyFileRead { path, source })
}

fn load_uncached() -> Result<YzhCredentials, YzhError> {
    let des_key = env("YUNZHANGHU_3DES_KEY")?;
    if des_key.len() != 24 {
        return Err(YzhError::InvalidDesKeyLength(des_key.len()));
    }

    Ok(YzhCredentials {
        api_url: env("YUNZHANGHU_API_URL")?.trim_end_matches('/').to_string(),
        dealer_id: env("YUNZHANGHU_DEALER_ID")?,
        broker_id: env("YUNZHANGHU_BROKER_ID")?,
        app_key: env("YUNZHANGHU_APP_KEY")?,
        des_key,
        dealer_private_key_pem: read_pem_file(
            "YUNZHANGHU_DEALER_PRIVATE_KEY_PATH",
        )?,
        platform_public_key_pem: read_pem_file(
            "YUNZHANGHU_PLATFORM_PUBLIC_KEY_PATH",
        )?,
    })
}
