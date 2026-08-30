//! 云账户业务接口封装
//!
//! 每个函数对应一个云账户接口路径，统一通过 [`YzhClient`] 调用，
//! 自动完成加密签名 + 解密反序列化。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::client::YzhClient;
use super::error::YzhError;

// ============================================================================
// H5 签约
// ============================================================================

/// 云账户证件类型编码：身份证
pub const CERTIFICATE_TYPE_IDCARD: i32 = 0;

/// 预申请签约入参（新版 2025-06）
#[derive(Serialize)]
pub struct PresignRequest<'a> {
    pub real_name: &'a str,
    pub id_card: &'a str,
    /// 证件类型编码（0=身份证）
    pub certificate_type: i32,
    /// 0=不收集手机号（默认），1=收集
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collect_phone_no: Option<i32>,
}

#[derive(Deserialize)]
pub struct PresignResponse {
    /// H5 签约 token（2 小时有效期）
    pub token: String,
    /// 当前签约状态：0=未签约 / 1=已签约 / 2=已解约
    #[serde(default)]
    pub status: i32,
}

/// 预申请签约：根据姓名 + 身份证获取一次性 token
pub async fn h5_presign(
    client: &YzhClient,
    req: &PresignRequest<'_>,
) -> Result<PresignResponse, YzhError> {
    client.post("/api/sdk/v1/presign", to_value(req)?).await
}

/// 申请签约入参（拿 token 换 H5 URL，**新版 2026-04 改为 GET**）
#[derive(Serialize)]
pub struct SignApplyRequest<'a> {
    pub token: &'a str,
    /// 签约页面主题色（HEX），可选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<&'a str>,
    /// 签约完成回调地址：仅签约成功时回调
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<&'a str>,
    /// 跳转 URL，签约完成后浏览器跳回此页面
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<&'a str>,
    /// 签约事件回调地址：成功/失败都回调，含更完整字段
    /// 当同时传 `url` 与 `event_callback_url` 时，云账户以本字段为统一回调
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_callback_url: Option<&'a str>,
}

#[derive(Deserialize)]
pub struct SignApplyResponse {
    /// H5 签约页面 URL，前端引导用户访问
    pub url: String,
}

pub async fn h5_sign_apply(
    client: &YzhClient,
    req: &SignApplyRequest<'_>,
) -> Result<SignApplyResponse, YzhError> {
    client.get("/api/sdk/v1/sign/h5", to_value(req)?).await
}

/// 查询签约状态入参（新版 2026-04）
#[derive(Serialize)]
pub struct SignStatusRequest<'a> {
    pub real_name: &'a str,
    pub id_card: &'a str,
}

#[derive(Deserialize)]
pub struct SignStatusResponse {
    /// 0=未签约 / 1=已签约 / 2=已解约
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub signed_at: String,
    #[serde(default)]
    pub event_type: String,
    #[serde(default)]
    pub event_status: String,
}

pub async fn h5_sign_status(
    client: &YzhClient,
    req: &SignStatusRequest<'_>,
) -> Result<SignStatusResponse, YzhError> {
    client
        .get("/api/sdk/v1/sign/user/status", to_value(req)?)
        .await
}

/// 申请解约（生产可用），生成 H5 解约页面 URL，
/// 用户扫码后完成手机号/人脸验证即可解约。
///
/// 文档：<https://open.yunzhanghu.com/docs/API/用户签约/H5签约/API列表/1491465584989581314>
#[derive(Serialize)]
pub struct SignReleaseApplyRequest<'a> {
    pub real_name: &'a str,
    pub id_card: &'a str,
    /// 主题颜色（HEX），可选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<&'a str>,
    /// 解约结果异步回调 URL；为空则用云账户后台配置的"解约回调地址"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<&'a str>,
    /// 解约完成后跳转 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<&'a str>,
}

#[derive(Deserialize)]
pub struct SignReleaseApplyResponse {
    /// 当前签约状态：1=已签约 / 2=已解约
    #[serde(default)]
    pub status: i32,
    /// H5 解约页面 URL，2 小时有效期
    pub url: String,
}

pub async fn h5_release_apply(
    client: &YzhClient,
    req: &SignReleaseApplyRequest<'_>,
) -> Result<SignReleaseApplyResponse, YzhError> {
    client.get("/api/sdk/v1/release/h5", to_value(req)?).await
}

// ============================================================================
// 实时支付 - 支付宝
// ============================================================================

/// 支付宝实时支付入参（见
/// <https://open.yunzhanghu.com/docs/API/实时支付/API列表/单笔支付/202201251642000401>）
#[derive(Serialize)]
pub struct AlipayOrderRequest<'a> {
    /// 平台企业订单号（≤ 64 个英文字符）
    pub order_id: &'a str,
    pub real_name: &'a str,
    /// 支付宝账号（手机号或邮箱）
    pub card_no: &'a str,
    /// 身份证号（含 X 需大写）
    pub id_card: &'a str,
    pub phone_no: &'a str,
    /// 订单金额，单位元，2 位小数字符串，例 `"100.00"`
    pub pay: &'a str,

    /// 订单备注（显示在用户支付宝账单的"理由"中）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pay_remark: Option<&'a str>,
    /// 支付宝转账备注，默认显示"云账户"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_title: Option<&'a str>,
    /// 校验支付宝姓名是否与 real_name 一致，固定值 `"Check"`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_name: Option<&'a str>,
    /// 订单异步通知地址
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_url: Option<&'a str>,

    /// 收入来源的互联网平台名称，最大 48 字符（云账户合规必填）
    pub dealer_platform_name: &'a str,
    /// 用户在平台展示的名称/昵称，最大 48 字符
    pub dealer_user_nickname: &'a str,
    /// 用户在平台内的唯一标识，最大 80 字符
    pub dealer_user_id: &'a str,
}

#[derive(Deserialize)]
pub struct AlipayOrderResponse {
    /// 原值返回
    pub order_id: String,
    /// 综合服务平台流水号（唯一）
    #[serde(rename = "ref")]
    pub ref_id: String,
    pub pay: String,
}

pub async fn alipay_order(
    client: &YzhClient,
    req: &AlipayOrderRequest<'_>,
) -> Result<AlipayOrderResponse, YzhError> {
    client
        .post("/api/payment/v1/order-alipay", to_value(req)?)
        .await
}

// ============================================================================
// 连续劳务税费试算 - 订单税费试算
// ============================================================================

#[derive(Serialize)]
pub struct CalcTaxRequest<'a> {
    pub real_name: &'a str,
    pub id_card: &'a str,
    /// 测算金额，单位元，2 位小数字符串。
    pub pay: &'a str,
    /// before_tax（默认）/ after_tax。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_type: Option<&'a str>,
    /// after_tax 测算时的税前金额返回值类型。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_tax_amount_type: Option<&'a str>,
    /// 1：纳入追缴税费，2：不纳入。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_recovery_amount: Option<i32>,
    /// 1：纳入劳动者服务费，2：不纳入。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_user_service_fee: Option<i32>,
}

#[derive(Deserialize, Clone, Default)]
pub struct CalcTaxDetail {
    #[serde(default)]
    pub personal_tax: String,
    #[serde(default)]
    pub value_added_tax: String,
    #[serde(default)]
    pub additional_tax: String,
    #[serde(default)]
    pub additional_urban_tax: String,
    #[serde(default)]
    pub additional_tuition_tax: String,
    #[serde(default)]
    pub additional_local_tuition_tax: String,
    #[serde(default)]
    pub user_personal_tax: String,
    #[serde(default)]
    pub user_value_added_tax: String,
    #[serde(default)]
    pub user_additional_tax: String,
    #[serde(default)]
    pub dealer_personal_tax: String,
    #[serde(default)]
    pub dealer_value_added_tax: String,
    #[serde(default)]
    pub dealer_additional_tax: String,
    #[serde(default)]
    pub broker_personal_tax: String,
    #[serde(default)]
    pub broker_value_added_tax: String,
    #[serde(default)]
    pub broker_additional_tax: String,
    #[serde(default)]
    pub personal_tax_rate: String,
    #[serde(default)]
    pub deduct_tax: String,
}

#[derive(Deserialize, Clone, Default)]
pub struct CalcTaxResponse {
    #[serde(default)]
    pub pay: String,
    #[serde(default)]
    pub before_tax_amount: String,
    #[serde(default)]
    pub after_tax_amount: String,
    #[serde(default)]
    pub user_real_excluding_vat_amount: String,
    #[serde(default)]
    pub tax: String,
    #[serde(default)]
    pub user_tax: String,
    #[serde(default)]
    pub dealer_tax: String,
    #[serde(default)]
    pub broker_tax: String,
    #[serde(default)]
    pub user_fee: String,
    #[serde(default)]
    pub user_recover_tax_amount: String,
    #[serde(default)]
    pub user_recover_personal_tax_amount: String,
    #[serde(default)]
    pub user_refund_tax_amount: String,
    #[serde(default)]
    pub user_refund_personal_tax_amount: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub status_detail: String,
    #[serde(default)]
    pub status_message: String,
    #[serde(default)]
    pub status_detail_message: String,
    #[serde(default)]
    pub tax_detail: CalcTaxDetail,
}

pub async fn calc_tax(
    client: &YzhClient,
    req: &CalcTaxRequest<'_>,
) -> Result<CalcTaxResponse, YzhError> {
    client
        .post("/api/payment/v1/calc-tax", to_value(req)?)
        .await
}

// ============================================================================
// 订单状态查询（主动查单兜底）
// ============================================================================

#[derive(Serialize)]
pub struct QueryOrderRequest<'a> {
    pub order_id: &'a str,
    /// 支付路径：`银行卡` / `支付宝` / `微信`
    pub channel: &'a str,
}

/// 主要字段（参考
/// <https://open.yunzhanghu.com/docs/API/实时支付/API列表/762734317649530881>）
#[derive(Deserialize)]
pub struct QueryOrderResponse {
    pub order_id: String,
    /// 综合服务平台流水号
    #[serde(default, rename = "ref")]
    pub ref_id: String,
    pub pay: String,
    #[serde(default)]
    pub user_real_amount: String,
    #[serde(default)]
    pub user_real_excluding_vat_amount: String,
    #[serde(default)]
    pub user_fee: String,
    #[serde(default)]
    pub received_user_fee: String,
    #[serde(default)]
    pub tax: String,
    #[serde(default)]
    pub received_tax_amount: String,
    #[serde(default)]
    pub tax_detail: QueryOrderTaxDetail,

    /// 订单主状态：
    /// `0` 处理中 / `1` 成功 / `2` 失败 / `3` 挂起 / `4` 退汇 / `5` 撤销
    pub status: String,
    /// 详细状态码（见
    /// <https://open.yunzhanghu.com/docs/API/实时支付/1324759047668641792>）
    #[serde(default)]
    pub status_detail: String,
    #[serde(default)]
    pub status_message: String,
    #[serde(default)]
    pub status_detail_message: String,

    /// 1=劳动者退款，2=支付渠道退汇
    #[serde(default)]
    pub refund_origin: String,
}

#[derive(Deserialize, Default)]
pub struct QueryOrderTaxDetail {
    #[serde(default)]
    pub personal_tax: String,
    #[serde(default)]
    pub value_added_tax: String,
    #[serde(default)]
    pub additional_tax: String,
    #[serde(default)]
    pub user_personal_tax: String,
    #[serde(default)]
    pub user_value_added_tax: String,
    #[serde(default)]
    pub user_additional_tax: String,
    #[serde(default)]
    pub received_personal_tax: String,
    #[serde(default)]
    pub user_received_personal_tax: String,
    #[serde(default)]
    pub received_value_added_tax: String,
    #[serde(default)]
    pub user_received_value_added_tax: String,
    #[serde(default)]
    pub received_additional_tax: String,
    #[serde(default)]
    pub user_received_additional_tax: String,
}

pub async fn query_order(
    client: &YzhClient,
    req: &QueryOrderRequest<'_>,
) -> Result<QueryOrderResponse, YzhError> {
    // 注意：query-order 只接受 order_id + channel，不要 dealer_id/broker_id
    // 否则会返回 1004 加密错误
    client
        .get_raw("/api/payment/v1/query-order", to_value(req)?)
        .await
}

// ============================================================================
// 订单状态码 → BBSMC PayoutStatus 的映射
// ============================================================================

/// 把云账户订单状态字符串映射到 BBSMC 内部状态。
/// 兼容数字状态码（`0-5`）与文本码（`dealing/success/failed/freeze/refund/cancelled`）。
pub fn map_order_status(
    code: &str,
) -> Option<crate::models::payouts::PayoutStatus> {
    use crate::models::payouts::PayoutStatus;
    match code {
        // 处理中 / 默认
        "0" | "dealing" | "processing" => Some(PayoutStatus::InTransit),
        // 成功
        "1" | "success" => Some(PayoutStatus::Success),
        // 失败
        "2" | "failed" | "fail" => Some(PayoutStatus::Failed),
        // 挂起：通常是服务费不足、用户未签约等，需人工干预；先维持 InTransit
        "3" | "freeze" | "freezing" => Some(PayoutStatus::InTransit),
        // 退汇 / 撤销 → 视同失败（用户余额回退）
        "4" | "refund" | "5" | "cancelled" | "canceled" => {
            Some(PayoutStatus::Cancelled)
        }
        _ => None,
    }
}

// ============================================================================
// 工具
// ============================================================================

fn to_value<T: Serialize>(req: &T) -> Result<Value, YzhError> {
    serde_json::to_value(req).map_err(YzhError::Serde)
}
