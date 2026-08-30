use crate::models::ids::{Base62Id, UserId};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct PayoutId(pub u64);

#[derive(Serialize, Deserialize, Clone)]
pub struct Payout {
    pub id: PayoutId,
    pub user_id: UserId,
    pub status: PayoutStatus,
    pub created: DateTime<Utc>,
    #[serde(with = "rust_decimal::serde::float")]
    pub amount: Decimal,

    #[serde(with = "rust_decimal::serde::float_option")]
    pub fee: Option<Decimal>,
    pub method: Option<PayoutMethodType>,
    /// 提现接收地址脱敏值（如支付宝手机号/邮箱等，原值只在后端支付流程中使用）
    pub method_address: Option<String>,
    pub platform_id: Option<String>,
    /// 用户可见的退回/失败原因。
    pub reject_reason: Option<String>,
    /// 云账户成功订单回填的用户侧实际服务费/税费信息。
    pub yunzhanghu_details: Option<PayoutYunzhanghuDetails>,
}

impl Payout {
    pub fn from(data: crate::database::models::payout_item::Payout) -> Self {
        Self::from_with_yunzhanghu_details(data, None)
    }

    pub fn from_with_yunzhanghu_details(
        data: crate::database::models::payout_item::Payout,
        yunzhanghu_details: Option<PayoutYunzhanghuDetails>,
    ) -> Self {
        Self {
            id: data.id.into(),
            user_id: data.user_id.into(),
            status: data.status,
            created: data.created,
            amount: data.amount,
            fee: data.fee,
            method: data.method,
            method_address: mask_payout_method_address(
                data.method,
                data.method_address.as_deref(),
            ),
            platform_id: data.platform_id,
            reject_reason: match data.status {
                PayoutStatus::Cancelled | PayoutStatus::Failed => {
                    data.admin_reject_reason
                }
                _ => None,
            },
            yunzhanghu_details,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PayoutYunzhanghuDetails {
    #[serde(with = "rust_decimal::serde::float")]
    pub received_amount: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub service_fee: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub tax: Decimal,
}

fn mask_payout_method_address(
    method: Option<PayoutMethodType>,
    address: Option<&str>,
) -> Option<String> {
    let address = address?.trim();
    if address.is_empty() {
        return None;
    }

    Some(match method {
        Some(PayoutMethodType::YunzhanghuAlipay) => {
            mask_alipay_account(address)
        }
        _ => mask_generic_identifier(address),
    })
}

fn mask_alipay_account(account: &str) -> String {
    if account.chars().all(|c| c.is_ascii_digit())
        && account.chars().count() >= 7
    {
        return mask_keep_edges(account, 3, 4);
    }

    if let Some((local, domain)) = account.split_once('@') {
        let first = local.chars().next().unwrap_or('*');
        return format!("{}***@{}", first, domain);
    }

    mask_generic_identifier(account)
}

fn mask_generic_identifier(value: &str) -> String {
    let len = value.chars().count();
    if len <= 4 {
        return "*".repeat(len.max(1));
    }
    mask_keep_edges(value, 2, 2)
}

fn mask_keep_edges(
    value: &str,
    prefix_len: usize,
    suffix_len: usize,
) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= prefix_len + suffix_len {
        return "*".repeat(chars.len().max(1));
    }

    let prefix = chars.iter().take(prefix_len).collect::<String>();
    let suffix = chars
        .iter()
        .skip(chars.len() - suffix_len)
        .collect::<String>();
    format!("{prefix}****{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_yunzhanghu_alipay_phone() {
        assert_eq!(
            mask_payout_method_address(
                Some(PayoutMethodType::YunzhanghuAlipay),
                Some("13812345678")
            ),
            Some("138****5678".to_string())
        );
    }

    #[test]
    fn masks_yunzhanghu_alipay_email() {
        assert_eq!(
            mask_payout_method_address(
                Some(PayoutMethodType::YunzhanghuAlipay),
                Some("alice@example.com")
            ),
            Some("a***@example.com".to_string())
        );
    }

    #[test]
    fn hides_short_addresses() {
        assert_eq!(
            mask_payout_method_address(
                Some(PayoutMethodType::YunzhanghuAlipay),
                Some("abcd")
            ),
            Some("****".to_string())
        );
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum PayoutMethodType {
    /// 云账户 - 支付宝实时支付
    YunzhanghuAlipay,
    /// 未知 / 历史遗留
    Unknown,
}

impl std::fmt::Display for PayoutMethodType {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", self.as_str())
    }
}

impl PayoutMethodType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PayoutMethodType::YunzhanghuAlipay => "yunzhanghu_alipay",
            PayoutMethodType::Unknown => "unknown",
        }
    }

    pub fn from_string(s: &str) -> PayoutMethodType {
        match s {
            "yunzhanghu_alipay" => PayoutMethodType::YunzhanghuAlipay,
            _ => PayoutMethodType::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum PayoutStatus {
    Success,
    InTransit,
    Cancelled,
    Cancelling,
    Failed,
    Unknown,
}

impl std::fmt::Display for PayoutStatus {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", self.as_str())
    }
}

impl PayoutStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PayoutStatus::Success => "success",
            PayoutStatus::InTransit => "in-transit",
            PayoutStatus::Cancelled => "cancelled",
            PayoutStatus::Cancelling => "cancelling",
            PayoutStatus::Failed => "failed",
            PayoutStatus::Unknown => "unknown",
        }
    }

    pub fn from_string(string: &str) -> PayoutStatus {
        match string {
            "success" => PayoutStatus::Success,
            "in-transit" => PayoutStatus::InTransit,
            "cancelled" => PayoutStatus::Cancelled,
            "cancelling" => PayoutStatus::Cancelling,
            "failed" => PayoutStatus::Failed,
            _ => PayoutStatus::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PayoutMethod {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: PayoutMethodType,
    pub name: String,
    pub supported_countries: Vec<String>,
    pub image_url: Option<String>,
    pub interval: PayoutInterval,
    pub fee: PayoutMethodFee,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PayoutMethodFee {
    #[serde(with = "rust_decimal::serde::float")]
    pub percentage: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub min: Decimal,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub max: Option<Decimal>,
}

#[derive(Clone)]
pub struct PayoutDecimal(pub Decimal);

impl Serialize for PayoutDecimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        rust_decimal::serde::float::serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for PayoutDecimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let decimal = rust_decimal::serde::float::deserialize(deserializer)?;
        Ok(PayoutDecimal(decimal))
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PayoutInterval {
    Standard {
        #[serde(with = "rust_decimal::serde::float")]
        min: Decimal,
        #[serde(with = "rust_decimal::serde::float")]
        max: Decimal,
    },
    Fixed {
        values: Vec<PayoutDecimal>,
    },
}
