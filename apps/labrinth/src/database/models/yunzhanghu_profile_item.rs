//! 云账户(Yunzhanghu)用户资料数据库模型
//!
//! 与 `users` 表 1:1 关系，按需创建（用户首次填实名/绑定支付宝时插入）。
//! 姓名、身份证号、手机号、支付宝账号通过 [`crate::util::encrypt`] 用 AES-256-GCM 加密存储；
//! 仅在调用云账户支付/签约接口前在内存里临时解密。

use crate::database::models::{DatabaseError, UserId};
use crate::util::encrypt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, PgPool};

/// 用户与云账户的签约状态机
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum YzhSignStatus {
    /// 未签约：还没发起过签约（默认）
    Unsigned,
    /// 签约中：已发起 H5 签约但用户尚未完成 / 回调未到
    Signing,
    /// 已签约：可发起提现
    Signed,
    /// 已解约：用户主动解约或被云账户解约，需要重新签约
    Terminated,
}

impl YzhSignStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            YzhSignStatus::Unsigned => "unsigned",
            YzhSignStatus::Signing => "signing",
            YzhSignStatus::Signed => "signed",
            YzhSignStatus::Terminated => "terminated",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "signing" => YzhSignStatus::Signing,
            "signed" => YzhSignStatus::Signed,
            "terminated" => YzhSignStatus::Terminated,
            _ => YzhSignStatus::Unsigned,
        }
    }
}

impl std::fmt::Display for YzhSignStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

fn encrypt_profile_field(
    label: &str,
    plaintext: &str,
) -> Result<String, DatabaseError> {
    encrypt::encrypt(plaintext).map_err(|e| {
        DatabaseError::SchemaError(format!("{}加密失败: {}", label, e))
    })
}

fn decrypt_profile_field(
    label: &str,
    encrypted: Option<&str>,
    legacy_plaintext: Option<String>,
) -> Result<Option<String>, DatabaseError> {
    match encrypted {
        Some(ciphertext) => {
            encrypt::decrypt(ciphertext).map(Some).map_err(|e| {
                DatabaseError::SchemaError(format!("{}解密失败: {}", label, e))
            })
        }
        None => Ok(legacy_plaintext),
    }
}

/// 云账户资料模型。
///
/// `real_name`、`phone`、`alipay_account` 是从密文字段解密后的内存值。
/// 旧明文字段只作为历史数据兼容回退，不再写入新值。
#[derive(Clone)]
pub struct YunzhanghuProfile {
    pub user_id: UserId,
    pub real_name: Option<String>,
    /// AES-GCM 密文 base64，存储格式由 [`crate::util::encrypt`] 决定
    pub id_card_encrypted: Option<String>,
    pub id_card_last4: Option<String>,
    pub phone: Option<String>,
    pub alipay_account: Option<String>,
    pub sign_status: YzhSignStatus,
    pub sign_url: Option<String>,
    pub sign_nonce: Option<String>,
    pub signed_at: Option<DateTime<Utc>>,
    pub terminated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl YunzhanghuProfile {
    /// 用 [`crate::util::encrypt::decrypt`] 把存储的身份证密文还原成明文。
    /// 返回 `None` 表示用户尚未填写身份证号。
    pub fn decrypt_id_card(
        &self,
    ) -> Result<Option<String>, encrypt::EncryptionError> {
        self.id_card_encrypted
            .as_deref()
            .map(encrypt::decrypt)
            .transpose()
    }

    pub async fn get<'a, E>(
        user_id: UserId,
        exec: E,
    ) -> Result<Option<YunzhanghuProfile>, DatabaseError>
    where
        E: PgExecutor<'a>,
    {
        let row = sqlx::query!(
            "
            SELECT user_id, real_name, real_name_encrypted, id_card_encrypted, id_card_last4,
                   phone, phone_encrypted, alipay_account, alipay_account_encrypted,
                   sign_status, sign_url, sign_nonce, signed_at, terminated_at, created_at,
                   updated_at
            FROM user_yunzhanghu_profiles
            WHERE user_id = $1
            ",
            user_id.0
        )
        .fetch_optional(exec)
        .await?;

        let Some(r) = row else {
            return Ok(None);
        };

        let real_name = decrypt_profile_field(
            "真实姓名",
            r.real_name_encrypted.as_deref(),
            r.real_name,
        )?;
        let phone = decrypt_profile_field(
            "手机号",
            r.phone_encrypted.as_deref(),
            r.phone,
        )?;
        let alipay_account = decrypt_profile_field(
            "支付宝账号",
            r.alipay_account_encrypted.as_deref(),
            r.alipay_account,
        )?;

        Ok(Some(YunzhanghuProfile {
            user_id: UserId(r.user_id),
            real_name,
            id_card_encrypted: r.id_card_encrypted,
            id_card_last4: r.id_card_last4,
            phone,
            alipay_account,
            sign_status: YzhSignStatus::parse(&r.sign_status),
            sign_url: r.sign_url,
            sign_nonce: r.sign_nonce,
            signed_at: r.signed_at,
            terminated_at: r.terminated_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    /// 无损回填旧明文字段到新密文字段，并清空旧明文字段。
    ///
    /// 纯 SQL migration 无法调用应用侧 AES-GCM 密钥，启动时用本函数处理
    /// 已有历史行，避免旧数据长期以姓名/手机号/支付宝明文留在库里。
    pub async fn backfill_legacy_plaintext(
        pool: &PgPool,
    ) -> Result<u64, DatabaseError> {
        let mut updated = 0;

        loop {
            let rows = sqlx::query!(
                "
                SELECT user_id, real_name, phone, alipay_account
                FROM user_yunzhanghu_profiles
                WHERE real_name IS NOT NULL
                   OR phone IS NOT NULL
                   OR alipay_account IS NOT NULL
                ORDER BY user_id
                LIMIT 500
                "
            )
            .fetch_all(pool)
            .await?;

            if rows.is_empty() {
                break;
            }

            for row in rows {
                let real_name_cipher = row
                    .real_name
                    .as_deref()
                    .map(|value| encrypt_profile_field("真实姓名", value))
                    .transpose()?;
                let phone_cipher = row
                    .phone
                    .as_deref()
                    .map(|value| encrypt_profile_field("手机号", value))
                    .transpose()?;
                let alipay_account_cipher = row
                    .alipay_account
                    .as_deref()
                    .map(|value| encrypt_profile_field("支付宝账号", value))
                    .transpose()?;

                let result = sqlx::query!(
                    "
                    UPDATE user_yunzhanghu_profiles
                    SET real_name_encrypted = COALESCE($2, real_name_encrypted),
                        phone_encrypted = COALESCE($3, phone_encrypted),
                        alipay_account_encrypted = COALESCE($4, alipay_account_encrypted),
                        real_name = NULL,
                        phone = NULL,
                        alipay_account = NULL,
                        updated_at = NOW()
                    WHERE user_id = $1
                    ",
                    row.user_id,
                    real_name_cipher,
                    phone_cipher,
                    alipay_account_cipher,
                )
                .execute(pool)
                .await?;

                updated += result.rows_affected();
            }
        }

        Ok(updated)
    }

    /// 创建或更新实名 + 支付宝账号字段（部分字段）。
    ///
    /// 调用方需要在身份要素或收款账号变更时调用
    /// [`Self::reset_sign_status_for_kyc_change`]，强制用户重新签约。
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_kyc(
        executor: impl PgExecutor<'_>,
        user_id: UserId,
        real_name: &str,
        id_card_plain: &str,
        phone: &str,
        alipay_account: &str,
    ) -> Result<(), DatabaseError> {
        let id_card_cipher = encrypt::encrypt(id_card_plain).map_err(|e| {
            DatabaseError::SchemaError(format!("身份证加密失败: {}", e))
        })?;
        let real_name_cipher = encrypt_profile_field("真实姓名", real_name)?;
        let phone_cipher = encrypt_profile_field("手机号", phone)?;
        let alipay_account_cipher =
            encrypt_profile_field("支付宝账号", alipay_account)?;

        let id_card_chars: Vec<char> = id_card_plain.chars().collect();
        let last4: String = if id_card_chars.len() >= 4 {
            id_card_chars[id_card_chars.len() - 4..].iter().collect()
        } else {
            id_card_plain.to_string()
        };

        sqlx::query!(
            "
            INSERT INTO user_yunzhanghu_profiles (
                user_id, real_name, real_name_encrypted, id_card_encrypted, id_card_last4,
                phone, phone_encrypted, alipay_account, alipay_account_encrypted
            )
            VALUES ($1, NULL, $2, $3, $4, NULL, $5, NULL, $6)
            ON CONFLICT (user_id) DO UPDATE SET
                real_name = NULL,
                real_name_encrypted = EXCLUDED.real_name_encrypted,
                id_card_encrypted = EXCLUDED.id_card_encrypted,
                id_card_last4 = EXCLUDED.id_card_last4,
                phone = NULL,
                phone_encrypted = EXCLUDED.phone_encrypted,
                alipay_account = NULL,
                alipay_account_encrypted = EXCLUDED.alipay_account_encrypted,
                updated_at = NOW()
            ",
            user_id.0,
            real_name_cipher,
            id_card_cipher,
            last4,
            phone_cipher,
            alipay_account_cipher,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    /// 更新签约状态（含 sign_url、signed_at、terminated_at 等关联时间戳）
    pub async fn update_sign_status(
        executor: impl PgExecutor<'_>,
        user_id: UserId,
        status: YzhSignStatus,
        sign_url: Option<&str>,
        sign_nonce: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let (signed_at, terminated_at) = match status {
            YzhSignStatus::Signed => (Some(Utc::now()), None),
            YzhSignStatus::Terminated => (None, Some(Utc::now())),
            _ => (None, None),
        };

        sqlx::query!(
            "
            UPDATE user_yunzhanghu_profiles
            SET sign_status = $2,
                sign_url = CASE
                    WHEN $2 = 'signing' THEN COALESCE($3, sign_url)
                    ELSE NULL
                END,
                sign_nonce = CASE
                    WHEN $2 = 'signing' THEN COALESCE($4, sign_nonce)
                    ELSE NULL
                END,
                signed_at = $5,
                terminated_at = $6,
                updated_at = NOW()
            WHERE user_id = $1
            ",
            user_id.0,
            status.as_str(),
            sign_url,
            sign_nonce,
            signed_at,
            terminated_at,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    /// 身份要素或收款账号变更后，清空签约状态与历史签约 URL。
    pub async fn reset_sign_status_for_kyc_change(
        executor: impl PgExecutor<'_>,
        user_id: UserId,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "
            UPDATE user_yunzhanghu_profiles
            SET sign_status = 'unsigned',
                sign_url = NULL,
                sign_nonce = NULL,
                signed_at = NULL,
                terminated_at = NULL,
                updated_at = NOW()
            WHERE user_id = $1
            ",
            user_id.0,
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
