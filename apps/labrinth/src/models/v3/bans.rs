use super::ids::Base62Id;
use crate::models::ids::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ==================== ID 类型 ====================

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Hash)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct UserBanId(pub u64);

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Hash)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct BanHistoryId(pub u64);

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Hash)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct BanAppealId(pub u64);

// ==================== 封禁类型枚举 ====================

/// 封禁类型（3种大类）
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BanType {
    /// 全局封禁：无法登录系统
    Global,
    /// 资源类封禁：禁止上传/创建/编辑/删除资源、团队管理、提现等
    Resource,
    /// 论坛类封禁：禁止评论/发帖/百科编辑/发送消息/举报等
    Forum,
}

impl BanType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BanType::Global => "global",
            BanType::Resource => "resource",
            BanType::Forum => "forum",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "global" => Some(BanType::Global),
            "resource" => Some(BanType::Resource),
            "forum" => Some(BanType::Forum),
            _ => None,
        }
    }

    /// 获取封禁类型的显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            BanType::Global => "全局封禁",
            BanType::Resource => "资源类封禁",
            BanType::Forum => "论坛类封禁",
        }
    }

    /// 获取封禁类型的描述
    pub fn description(&self) -> &'static str {
        match self {
            BanType::Global => "无法登录系统，所有功能被禁用",
            BanType::Resource => {
                "禁止上传版本、创建/编辑/删除项目、团队管理等所有资源操作"
            }
            BanType::Forum => {
                "禁止评论、发帖/回帖、编辑百科、发送消息等所有社交互动功能"
            }
        }
    }
}

impl std::fmt::Display for BanType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ==================== 申诉状态枚举 ====================

/// 申诉状态
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AppealStatus {
    /// 待处理
    Pending,
    /// 已批准（解除封禁）
    Approved,
    /// 已拒绝
    Rejected,
}

impl AppealStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppealStatus::Pending => "pending",
            AppealStatus::Approved => "approved",
            AppealStatus::Rejected => "rejected",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(AppealStatus::Pending),
            "approved" => Some(AppealStatus::Approved),
            "rejected" => Some(AppealStatus::Rejected),
            _ => None,
        }
    }
}

impl std::fmt::Display for AppealStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ==================== API 响应结构 ====================

/// 用户封禁信息（完整版，包含内部信息）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserBan {
    pub id: UserBanId,
    pub user_id: UserId,
    pub ban_type: BanType,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_reason: Option<String>,
    pub banned_by: UserId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banned_by_username: Option<String>,
    pub banned_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_appeal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appeal: Option<BanAppeal>,
}

/// 用户封禁信息（用户视角，不包含内部信息）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserBanPublic {
    pub id: UserBanId,
    pub ban_type: BanType,
    pub reason: String,
    pub banned_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub can_appeal: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appeal_id: Option<BanAppealId>,
}

/// 封禁申诉信息
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BanAppeal {
    pub id: BanAppealId,
    pub ban_id: UserBanId,
    pub user_id: UserId,
    pub reason: String,
    pub status: AppealStatus,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<UserId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_by_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_notes: Option<String>,
    /// 申诉交流线程ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<crate::models::threads::ThreadId>,
}

/// 封禁历史记录
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BanHistoryEntry {
    pub id: BanHistoryId,
    pub ban_id: UserBanId,
    pub action: String,
    pub operator_id: UserId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_username: Option<String>,
    pub operated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_data: Option<serde_json::Value>,
    pub new_data: serde_json::Value,
    pub reason: String,
}

// ==================== API 请求结构 ====================

/// 创建封禁请求
#[derive(Deserialize, Debug)]
pub struct CreateBanRequest {
    pub ban_type: BanType,
    pub reason: String,
    #[serde(default)]
    pub internal_reason: Option<String>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default = "default_true")]
    pub notify_user: bool,
}

/// 修改封禁请求
#[derive(Deserialize, Debug)]
pub struct UpdateBanRequest {
    #[serde(default)]
    pub expires_at: Option<Option<DateTime<Utc>>>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub internal_reason: Option<String>,
    #[serde(default)]
    pub modification_reason: Option<String>,
}

/// 解除封禁请求
#[derive(Deserialize, Debug)]
pub struct RevokeBanRequest {
    pub reason: String,
    #[serde(default = "default_true")]
    pub notify_user: bool,
}

/// 创建申诉请求
#[derive(Deserialize, Debug)]
pub struct CreateAppealRequest {
    pub reason: String,
}

/// 审核申诉请求
#[derive(Deserialize, Debug)]
pub struct ReviewAppealRequest {
    pub status: AppealStatus,
    #[serde(default)]
    pub review_notes: Option<String>,
    #[serde(default = "default_true")]
    pub notify_user: bool,
}

/// 封禁列表查询参数
#[derive(Deserialize, Debug, Default)]
pub struct BansQueryParams {
    #[serde(default)]
    pub ban_type: Option<BanType>,
    #[serde(default)]
    pub is_active: Option<bool>,
    #[serde(default)]
    pub user_id: Option<UserId>,
    #[serde(default)]
    pub banned_by: Option<UserId>,
    #[serde(default)]
    pub include_inactive: Option<bool>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub limit: Option<i64>,
}

/// 申诉列表查询参数
#[derive(Deserialize, Debug, Default)]
pub struct AppealsQueryParams {
    #[serde(default)]
    pub status: Option<AppealStatus>,
    #[serde(default)]
    pub user_id: Option<UserId>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub limit: Option<i64>,
}

/// 批量获取封禁查询参数
#[derive(Deserialize, Debug)]
pub struct BatchBansQuery {
    /// JSON 数组格式的封禁 ID 列表
    pub ids: String,
}

// ==================== 分页响应 ====================

/// 分页封禁列表响应
#[derive(Serialize, Debug)]
pub struct PaginatedBans {
    pub bans: Vec<UserBan>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

/// 分页申诉列表响应
#[derive(Serialize, Debug)]
pub struct PaginatedAppeals {
    pub appeals: Vec<BanAppeal>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

/// 用户活跃封禁列表响应
#[derive(Serialize, Debug)]
pub struct UserActiveBans {
    pub active_bans: Vec<UserBanPublic>,
}

// ==================== 辅助函数 ====================

fn default_true() -> bool {
    true
}
