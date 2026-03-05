use super::bans::BanAppealId;
use super::ids::{Base62Id, ImageId};
use crate::models::ids::{ProjectId, ReportId};
use crate::models::projects::ProjectStatus;
use crate::models::users::{User, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct ThreadId(pub u64);

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct ThreadMessageId(pub u64);

/// 高级创作者申请 ID
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct CreatorApplicationId(pub u64);

#[derive(Serialize, Deserialize)]
pub struct Thread {
    pub id: ThreadId,
    #[serde(rename = "type")]
    pub type_: ThreadType,
    pub project_id: Option<ProjectId>,
    pub report_id: Option<ReportId>,
    pub ban_appeal_id: Option<BanAppealId>,
    pub creator_application_id: Option<CreatorApplicationId>,
    pub messages: Vec<ThreadMessage>,
    pub members: Vec<User>,
}

#[derive(Serialize, Deserialize)]
pub struct ThreadMessage {
    pub id: ThreadMessageId,
    pub author_id: Option<UserId>,
    pub body: MessageBody,
    pub created: DateTime<Utc>,
    pub hide_identity: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageBody {
    Text {
        body: String,
        #[serde(default)]
        private: bool,
        replying_to: Option<ThreadMessageId>,
        #[serde(default)]
        associated_images: Vec<ImageId>,
    },
    StatusChange {
        new_status: ProjectStatus,
        old_status: ProjectStatus,
    },
    ThreadClosure,
    ThreadReopen,
    Deleted {
        #[serde(default)]
        private: bool,
    },
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ThreadType {
    Report,
    Project,
    DirectMessage,
    VersionLink,
    BanAppeal,
    CreatorApplication,
}

impl std::fmt::Display for ThreadType {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", self.as_str())
    }
}

impl ThreadType {
    // These are constant, so this can remove unneccessary allocations (`to_string`)
    pub fn as_str(&self) -> &'static str {
        match self {
            ThreadType::Report => "report",
            ThreadType::Project => "project",
            ThreadType::DirectMessage => "direct_message",
            ThreadType::VersionLink => "version_link",
            ThreadType::BanAppeal => "ban_appeal",
            ThreadType::CreatorApplication => "creator_application",
        }
    }

    pub fn from_string(string: &str) -> ThreadType {
        match string {
            "report" => ThreadType::Report,
            "project" => ThreadType::Project,
            "direct_message" => ThreadType::DirectMessage,
            "version_link" => ThreadType::VersionLink,
            "ban_appeal" => ThreadType::BanAppeal,
            "creator_application" => ThreadType::CreatorApplication,
            _ => ThreadType::DirectMessage,
        }
    }
}

impl Thread {
    pub fn from(
        data: crate::database::models::Thread,
        users: Vec<User>,
        user: &User,
    ) -> Self {
        let thread_type = data.type_;

        Thread {
            id: data.id.into(),
            type_: thread_type,
            project_id: data.project_id.map(|x| x.into()),
            report_id: data.report_id.map(|x| x.into()),
            ban_appeal_id: data.ban_appeal_id.map(|x| BanAppealId(x.0 as u64)),
            creator_application_id: data
                .creator_application_id
                .map(|x| CreatorApplicationId(x.0 as u64)),
            messages: data
                .messages
                .into_iter()
                .filter(|x| {
                    if let MessageBody::Text { private, .. } = x.body {
                        !private || user.role.is_mod()
                    } else if let MessageBody::Deleted { private, .. } = x.body
                    {
                        !private || user.role.is_mod()
                    } else {
                        true
                    }
                })
                .map(|x| ThreadMessage::from(x, user))
                .collect(),
            members: users,
        }
    }
}

impl ThreadMessage {
    pub fn from(
        data: crate::database::models::ThreadMessage,
        user: &User,
    ) -> Self {
        Self {
            id: data.id.into(),
            author_id: if data.hide_identity && !user.role.is_mod() {
                None
            } else {
                data.author_id.map(|x| x.into())
            },
            body: data.body,
            created: data.created,
            hide_identity: data.hide_identity,
        }
    }
}

// CreatorApplicationId 转换实现
impl From<Base62Id> for CreatorApplicationId {
    fn from(id: Base62Id) -> Self {
        CreatorApplicationId(id.0)
    }
}

impl From<CreatorApplicationId> for Base62Id {
    fn from(id: CreatorApplicationId) -> Self {
        Base62Id(id.0)
    }
}

impl From<crate::database::models::ids::CreatorApplicationId>
    for CreatorApplicationId
{
    fn from(id: crate::database::models::ids::CreatorApplicationId) -> Self {
        CreatorApplicationId(id.0 as u64)
    }
}
