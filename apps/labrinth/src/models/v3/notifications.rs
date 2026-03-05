use super::bans::{BanAppealId, UserBanId};
use super::ids::Base62Id;
use super::ids::OrganizationId;
use super::users::UserId;
use crate::database::models::WikiCacheId;
use crate::database::models::notification_item::Notification as DBNotification;
use crate::database::models::notification_item::NotificationAction as DBNotificationAction;
use crate::models::ids::{
    DiscussionId, ProjectId, ReportId, TeamId, ThreadId, ThreadMessageId,
    VersionId,
};
use crate::models::projects::ProjectStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct NotificationId(pub u64);

#[derive(Serialize, Deserialize)]
pub struct Notification {
    pub id: NotificationId,
    pub user_id: UserId,
    pub read: bool,
    pub created: DateTime<Utc>,
    pub body: NotificationBody,

    pub name: String,
    pub text: String,
    pub link: String,
    pub actions: Vec<NotificationAction>,
}

// 通知类型
#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotificationBody {
    ProjectUpdate {
        project_id: ProjectId,
        version_id: VersionId,
    },
    TeamInvite {
        project_id: ProjectId,
        team_id: TeamId,
        invited_by: UserId,
        role: String,
    },
    OrganizationInvite {
        organization_id: OrganizationId,
        invited_by: UserId,
        team_id: TeamId,
        role: String,
    },
    StatusChange {
        project_id: ProjectId,
        old_status: ProjectStatus,
        new_status: ProjectStatus,
    },
    ModeratorMessage {
        thread_id: ThreadId,
        message_id: ThreadMessageId,

        project_id: Option<ProjectId>,
        report_id: Option<ReportId>,
    },
    LegacyMarkdown {
        notification_type: Option<String>,
        name: String,
        text: String,
        link: String,
        actions: Vec<NotificationAction>,
    },
    WikiCache {
        project_id: ProjectId,
        project_title: String,
        msg: String,
        wiki_cache_id: WikiCacheId,
        type_: String,
    },
    Forum {
        forum_id: DiscussionId,
        forum_title: String,
        forum_type: String,
        number_of_posts: u32,
        project_id: Option<ProjectId>,
        sender: String,
    },
    /// 用户被封禁通知
    UserBanned {
        ban_id: UserBanId,
        ban_type: String,
        reason: String,
        expires_at: Option<DateTime<Utc>>,
    },
    /// 用户封禁解除通知
    UserUnbanned {
        ban_id: UserBanId,
        ban_type: String,
        reason: String,
    },
    /// 申诉审核结果通知
    AppealReviewed {
        appeal_id: BanAppealId,
        ban_id: UserBanId,
        status: String,
        review_notes: Option<String>,
    },
    /// 申诉线程有新消息
    BanAppealMessage {
        appeal_id: BanAppealId,
        thread_id: ThreadId,
        message_id: ThreadMessageId,
    },
    /// 创作者申请线程有新消息
    CreatorApplicationMessage {
        application_id: i64,
        thread_id: ThreadId,
        message_id: ThreadMessageId,
    },
    /// 创作者申请已批准
    CreatorApplicationApproved {
        application_id: i64,
    },
    /// 创作者申请已拒绝
    CreatorApplicationRejected {
        application_id: i64,
        reason: Option<String>,
    },
    /// 用户资料修改已提交审核
    ProfileReviewPending {
        review_id: i64,
        review_type: String,
    },
    /// 用户资料修改审核结果
    ProfileReviewResult {
        review_id: i64,
        review_type: String,
        status: String,
        review_notes: Option<String>,
    },
    /// 图片内容审核结果（仅拒绝时发送）
    ImageReviewResult {
        review_id: i64,
        source_type: String,
        status: String,
        review_notes: Option<String>,
    },
    Unknown,
}

impl From<DBNotification> for Notification {
    fn from(notif: DBNotification) -> Self {
        let (name, text, link, actions) = {
            match &notif.body {
                NotificationBody::ProjectUpdate {
                    project_id,
                    version_id,
                } => (
                    "您关注的项目已更新！".to_string(),
                    format!("项目 {} 发布了新版本：{}", project_id, version_id),
                    format!("/project/{}/version/{}", project_id, version_id),
                    vec![],
                ),

                NotificationBody::WikiCache {
                    project_id,
                    project_title,
                    type_,
                    ..
                } => (
                    "百科修改状态变更".to_string(),
                    format!(
                        "您提交的项目 {} 的百科页面修改状态变更为 {}",
                        project_title, type_
                    ),
                    format!("/project/{}/wikis", project_id),
                    vec![],
                ),
                NotificationBody::Forum {
                    forum_id,
                    forum_title,
                    forum_type,
                    number_of_posts,
                    project_id,
                    sender,
                } => (
                    "您收到了一条回复".to_string(),
                    format!(
                        "{} 在帖子 {} 中回复了您 {} {}",
                        sender,
                        forum_title,
                        forum_type,
                        project_id
                            .map_or_else(String::new, |id| format!("{}", id))
                    ),
                    format!("/d/{}?id={}", forum_id, number_of_posts),
                    vec![],
                ),
                NotificationBody::TeamInvite {
                    project_id,
                    role,
                    team_id,
                    ..
                } => (
                    "您已被邀请加入团队！".to_string(),
                    format!("已向您发送邀请，成为团队的 {}", role),
                    format!("/project/{}", project_id),
                    vec![
                        NotificationAction {
                            name: "接受".to_string(),
                            action_route: (
                                "POST".to_string(),
                                format!("team/{team_id}/join"),
                            ),
                        },
                        NotificationAction {
                            name: "拒绝".to_string(),
                            action_route: (
                                "DELETE".to_string(),
                                format!(
                                    "team/{team_id}/members/{}",
                                    UserId::from(notif.user_id)
                                ),
                            ),
                        },
                    ],
                ),
                NotificationBody::OrganizationInvite {
                    organization_id,
                    role,
                    team_id,
                    ..
                } => (
                    "您已被邀请加入团队！".to_string(),
                    format!("已向您发送邀请，成为团队的 {}", role),
                    format!("/organization/{}", organization_id),
                    vec![
                        NotificationAction {
                            name: "接受".to_string(),
                            action_route: (
                                "POST".to_string(),
                                format!("team/{team_id}/join"),
                            ),
                        },
                        NotificationAction {
                            name: "拒绝".to_string(),
                            action_route: (
                                "DELETE".to_string(),
                                format!(
                                    "organization/{organization_id}/members/{}",
                                    UserId::from(notif.user_id)
                                ),
                            ),
                        },
                    ],
                ),
                NotificationBody::StatusChange {
                    old_status,
                    new_status,
                    project_id,
                } => (
                    "项目状态已更改".to_string(),
                    format!(
                        "状态已从 {} 更改为 {}",
                        old_status.as_friendly_str(),
                        new_status.as_friendly_str()
                    ),
                    format!("/project/{}", project_id),
                    vec![],
                ),
                NotificationBody::ModeratorMessage {
                    project_id,
                    report_id,
                    ..
                } => (
                    "管理员已向您发送消息！".to_string(),
                    "点击链接查看更多信息。".to_string(),
                    if let Some(project_id) = project_id {
                        format!("/project/{}", project_id)
                    } else if let Some(report_id) = report_id {
                        format!("/project/{}", report_id)
                    } else {
                        "#".to_string()
                    },
                    vec![],
                ),
                NotificationBody::LegacyMarkdown {
                    name,
                    text,
                    link,
                    actions,
                    ..
                } => (
                    name.clone(),
                    text.clone(),
                    link.clone(),
                    actions.clone().into_iter().collect(),
                ),
                NotificationBody::UserBanned {
                    ban_type,
                    reason,
                    expires_at,
                    ..
                } => {
                    let ban_type_display = match ban_type.as_str() {
                        "global" => "全局封禁",
                        "resource" => "资源操作封禁",
                        "forum" => "论坛互动封禁",
                        _ => "封禁",
                    };
                    let expires_text = match expires_at {
                        Some(dt) => format!(
                            "，解封时间：{}",
                            dt.format("%Y-%m-%d %H:%M")
                        ),
                        None => "，此为永久封禁".to_string(),
                    };
                    (
                        "您的账户已被封禁".to_string(),
                        format!(
                            "您已被{}。原因：{}{}",
                            ban_type_display, reason, expires_text
                        ),
                        "/settings/account".to_string(),
                        vec![],
                    )
                }
                NotificationBody::UserUnbanned {
                    ban_type, reason, ..
                } => {
                    let ban_type_display = match ban_type.as_str() {
                        "global" => "全局封禁",
                        "resource" => "资源操作封禁",
                        "forum" => "论坛互动封禁",
                        _ => "封禁",
                    };
                    (
                        "您的封禁已解除".to_string(),
                        format!(
                            "您的{}已被解除。原因：{}",
                            ban_type_display, reason
                        ),
                        "/settings/account".to_string(),
                        vec![],
                    )
                }
                NotificationBody::AppealReviewed {
                    status,
                    review_notes,
                    ..
                } => {
                    let status_display = match status.as_str() {
                        "approved" => "已通过",
                        "rejected" => "已拒绝",
                        _ => "已处理",
                    };
                    let notes_text = match review_notes {
                        Some(notes) => format!("。审核备注：{}", notes),
                        None => String::new(),
                    };
                    (
                        "您的申诉已处理".to_string(),
                        format!(
                            "您的封禁申诉审核结果：{}{}",
                            status_display, notes_text
                        ),
                        "/settings/account".to_string(),
                        vec![],
                    )
                }
                NotificationBody::BanAppealMessage { .. } => (
                    "您的申诉有新回复".to_string(),
                    "管理员在您的封禁申诉中发送了新消息，请查看。".to_string(),
                    "/settings/account".to_string(),
                    vec![],
                ),
                NotificationBody::CreatorApplicationMessage { .. } => (
                    "您的创作者申请有新回复".to_string(),
                    "管理员在您的高级创作者申请中发送了新消息，请查看。"
                        .to_string(),
                    "/settings/creator".to_string(),
                    vec![],
                ),
                NotificationBody::CreatorApplicationApproved { .. } => (
                    "恭喜！您的高级创作者申请已通过".to_string(),
                    "您现在可以发布付费插件了。".to_string(),
                    "/settings/creator".to_string(),
                    vec![],
                ),
                NotificationBody::CreatorApplicationRejected {
                    reason, ..
                } => (
                    "您的高级创作者申请未通过".to_string(),
                    reason
                        .clone()
                        .unwrap_or_else(|| "请查看详情。".to_string()),
                    "/settings/creator".to_string(),
                    vec![],
                ),
                NotificationBody::ProfileReviewPending {
                    review_type, ..
                } => {
                    let type_display = match review_type.as_str() {
                        "avatar" => "头像",
                        "username" => "用户名",
                        "bio" => "简介",
                        _ => "资料",
                    };
                    (
                        "资料修改已提交审核".to_string(),
                        format!(
                            "您的{}修改已提交管理员审核，请耐心等待。",
                            type_display
                        ),
                        "/settings/profile".to_string(),
                        vec![],
                    )
                }
                NotificationBody::ProfileReviewResult {
                    review_type,
                    status,
                    review_notes,
                    ..
                } => {
                    let type_display = match review_type.as_str() {
                        "avatar" => "头像",
                        "username" => "用户名",
                        "bio" => "简介",
                        _ => "资料",
                    };
                    let status_display = match status.as_str() {
                        "approved" => "已通过",
                        "rejected" => "已拒绝",
                        _ => "已处理",
                    };
                    let notes_text = match review_notes {
                        Some(notes) => format!("。审核备注：{}", notes),
                        None => String::new(),
                    };
                    (
                        format!("{}修改审核结果", type_display),
                        format!(
                            "您的{}修改审核结果：{}{}",
                            type_display, status_display, notes_text
                        ),
                        "/settings/profile".to_string(),
                        vec![],
                    )
                }
                NotificationBody::ImageReviewResult {
                    source_type,
                    status,
                    review_notes,
                    ..
                } => {
                    let type_display = match source_type.as_str() {
                        "markdown" => "Markdown图片",
                        "gallery" => "项目渲染图",
                        _ => "图片",
                    };
                    let status_display = match status.as_str() {
                        "rejected" => "已被删除",
                        _ => "已处理",
                    };
                    let notes_text = match review_notes {
                        Some(notes) => format!("。原因：{}", notes),
                        None => String::new(),
                    };
                    (
                        format!("{}审核结果", type_display),
                        format!(
                            "您上传的{}因违规{}{}",
                            type_display, status_display, notes_text
                        ),
                        "#".to_string(),
                        vec![],
                    )
                }
                NotificationBody::Unknown => {
                    ("".to_string(), "".to_string(), "#".to_string(), vec![])
                }
            }
        };

        Self {
            id: notif.id.into(),
            user_id: notif.user_id.into(),
            body: notif.body,
            read: notif.read,
            created: notif.created,

            name,
            text,
            link,
            actions,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NotificationAction {
    pub name: String,
    /// The route to call when this notification action is called. Formatted HTTP Method, route
    pub action_route: (String, String),
}

impl From<DBNotificationAction> for NotificationAction {
    fn from(act: DBNotificationAction) -> Self {
        Self {
            name: act.name,
            action_route: (act.action_route_method, act.action_route),
        }
    }
}
