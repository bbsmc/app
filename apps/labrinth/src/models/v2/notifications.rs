use crate::database::models::WikiCacheId;
use crate::models::{
    ids::{
        DiscussionId, NotificationId, OrganizationId, ProjectId, ReportId,
        TeamId, ThreadId, ThreadMessageId, UserId, VersionId,
    },
    notifications::{Notification, NotificationAction, NotificationBody},
    projects::ProjectStatus,
    v3::bans::{BanAppealId, UserBanId},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LegacyNotification {
    pub id: NotificationId,
    pub user_id: UserId,
    pub read: bool,
    pub created: DateTime<Utc>,
    pub body: LegacyNotificationBody,

    // DEPRECATED: use body field instead
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub title: String,
    pub text: String,
    pub link: String,
    pub actions: Vec<LegacyNotificationAction>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LegacyNotificationAction {
    pub title: String,
    /// The route to call when this notification action is called. Formatted HTTP Method, route
    pub action_route: (String, String),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LegacyNotificationBody {
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
        title: String,
        text: String,
        link: String,
        actions: Vec<NotificationAction>,
    },
    // 通过，拒绝 提交
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
    /// 图片内容审核结果
    ImageReviewResult {
        review_id: i64,
        source_type: String,
        status: String,
        review_notes: Option<String>,
    },
    Unknown,
}

impl LegacyNotification {
    pub fn from(notification: Notification) -> Self {
        let type_ = match &notification.body {
            NotificationBody::ProjectUpdate { .. } => {
                Some("project_update".to_string())
            }
            NotificationBody::TeamInvite { .. } => {
                Some("team_invite".to_string())
            }
            NotificationBody::OrganizationInvite { .. } => {
                Some("organization_invite".to_string())
            }
            NotificationBody::StatusChange { .. } => {
                Some("status_change".to_string())
            }
            NotificationBody::ModeratorMessage { .. } => {
                Some("moderator_message".to_string())
            }
            NotificationBody::WikiCache { .. } => {
                Some("wiki_cache".to_string())
            }
            NotificationBody::Forum { .. } => Some("forum".to_string()),
            NotificationBody::UserBanned { .. } => {
                Some("user_banned".to_string())
            }
            NotificationBody::UserUnbanned { .. } => {
                Some("user_unbanned".to_string())
            }
            NotificationBody::AppealReviewed { .. } => {
                Some("appeal_reviewed".to_string())
            }
            NotificationBody::BanAppealMessage { .. } => {
                Some("ban_appeal_message".to_string())
            }
            NotificationBody::CreatorApplicationMessage { .. } => {
                Some("creator_application_message".to_string())
            }
            NotificationBody::CreatorApplicationApproved { .. } => {
                Some("creator_application_approved".to_string())
            }
            NotificationBody::CreatorApplicationRejected { .. } => {
                Some("creator_application_rejected".to_string())
            }
            NotificationBody::ProfileReviewPending { .. } => {
                Some("profile_review_pending".to_string())
            }
            NotificationBody::ProfileReviewResult { .. } => {
                Some("profile_review_result".to_string())
            }
            NotificationBody::ImageReviewResult { .. } => {
                Some("image_review_result".to_string())
            }
            NotificationBody::LegacyMarkdown {
                notification_type, ..
            } => notification_type.clone(),
            NotificationBody::Unknown => None,
        };

        let legacy_body = match notification.body {
            NotificationBody::ProjectUpdate {
                project_id,
                version_id,
            } => LegacyNotificationBody::ProjectUpdate {
                project_id,
                version_id,
            },
            NotificationBody::TeamInvite {
                project_id,
                team_id,
                invited_by,
                role,
            } => LegacyNotificationBody::TeamInvite {
                project_id,
                team_id,
                invited_by,
                role,
            },
            NotificationBody::OrganizationInvite {
                organization_id,
                invited_by,
                team_id,
                role,
            } => LegacyNotificationBody::OrganizationInvite {
                organization_id,
                invited_by,
                team_id,
                role,
            },
            NotificationBody::StatusChange {
                project_id,
                old_status,
                new_status,
            } => LegacyNotificationBody::StatusChange {
                project_id,
                old_status,
                new_status,
            },
            NotificationBody::ModeratorMessage {
                thread_id,
                message_id,
                project_id,
                report_id,
            } => LegacyNotificationBody::ModeratorMessage {
                thread_id,
                message_id,
                project_id,
                report_id,
            },
            NotificationBody::LegacyMarkdown {
                notification_type,
                name,
                text,
                link,
                actions,
            } => LegacyNotificationBody::LegacyMarkdown {
                notification_type,
                title: name,
                text,
                link,
                actions,
            },
            NotificationBody::WikiCache {
                project_id,
                project_title,
                msg,
                wiki_cache_id,
                type_,
            } => LegacyNotificationBody::WikiCache {
                project_id,
                project_title,
                msg,
                wiki_cache_id,
                type_,
            },
            NotificationBody::Forum {
                forum_id,
                forum_title,
                forum_type,
                number_of_posts,
                project_id,
                sender,
            } => LegacyNotificationBody::Forum {
                forum_id,
                forum_title,
                forum_type,
                number_of_posts,
                project_id,
                sender,
            },
            NotificationBody::UserBanned {
                ban_id,
                ban_type,
                reason,
                expires_at,
            } => LegacyNotificationBody::UserBanned {
                ban_id,
                ban_type,
                reason,
                expires_at,
            },
            NotificationBody::UserUnbanned {
                ban_id,
                ban_type,
                reason,
            } => LegacyNotificationBody::UserUnbanned {
                ban_id,
                ban_type,
                reason,
            },
            NotificationBody::AppealReviewed {
                appeal_id,
                ban_id,
                status,
                review_notes,
            } => LegacyNotificationBody::AppealReviewed {
                appeal_id,
                ban_id,
                status,
                review_notes,
            },
            NotificationBody::BanAppealMessage {
                appeal_id,
                thread_id,
                message_id,
            } => LegacyNotificationBody::BanAppealMessage {
                appeal_id,
                thread_id,
                message_id,
            },
            NotificationBody::CreatorApplicationMessage {
                application_id,
                thread_id,
                message_id,
            } => LegacyNotificationBody::CreatorApplicationMessage {
                application_id,
                thread_id,
                message_id,
            },
            NotificationBody::CreatorApplicationApproved { application_id } => {
                LegacyNotificationBody::CreatorApplicationApproved {
                    application_id,
                }
            }
            NotificationBody::CreatorApplicationRejected {
                application_id,
                reason,
            } => LegacyNotificationBody::CreatorApplicationRejected {
                application_id,
                reason,
            },
            NotificationBody::ProfileReviewPending {
                review_id,
                review_type,
            } => LegacyNotificationBody::ProfileReviewPending {
                review_id,
                review_type,
            },
            NotificationBody::ProfileReviewResult {
                review_id,
                review_type,
                status,
                review_notes,
            } => LegacyNotificationBody::ProfileReviewResult {
                review_id,
                review_type,
                status,
                review_notes,
            },
            NotificationBody::ImageReviewResult {
                review_id,
                source_type,
                status,
                review_notes,
            } => LegacyNotificationBody::ImageReviewResult {
                review_id,
                source_type,
                status,
                review_notes,
            },
            NotificationBody::Unknown => LegacyNotificationBody::Unknown,
        };

        Self {
            id: notification.id,
            user_id: notification.user_id,
            read: notification.read,
            created: notification.created,
            body: legacy_body,
            type_,
            title: notification.name,
            text: notification.text,
            link: notification.link,
            actions: notification
                .actions
                .into_iter()
                .map(LegacyNotificationAction::from)
                .collect(),
        }
    }
}

impl LegacyNotificationAction {
    pub fn from(notification_action: NotificationAction) -> Self {
        Self {
            title: notification_action.name,
            action_route: notification_action.action_route,
        }
    }
}
