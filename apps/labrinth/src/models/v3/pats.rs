use super::ids::Base62Id;
use crate::bitflags_serde_impl;
use crate::models::ids::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The ID of a team
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, Debug)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct PatId(pub u64);

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct Scopes: u64 {
        // 读取用户的邮箱
        const USER_READ_EMAIL = 1 << 0;
        // 读取用户数据
        const USER_READ = 1 << 1;
        // 写入用户数据
        const USER_WRITE = 1 << 2;
        // 删除用户
        const USER_DELETE = 1 << 3;
        // 修改用户认证数据
        const USER_AUTH_WRITE = 1 << 4;

        // 读取用户通知
        const NOTIFICATION_READ = 1 << 5;
        // 删除或读取通知
        const NOTIFICATION_WRITE = 1 << 6;

        // 读取用户提现数据
        const PAYOUTS_READ = 1 << 7;
        // 从用户账户中提现
        const PAYOUTS_WRITE = 1<< 8;
        // 访问用户分析（目前仅限于提现分析）
        const ANALYTICS = 1 << 9;

        // 创建项目
        const PROJECT_CREATE = 1 << 10;
        // 读取用户项目（包括私有）
        const PROJECT_READ = 1 << 11;
        // 写入项目数据（元数据、标题、团队成员等）
        const PROJECT_WRITE = 1 << 12;
        // 删除项目
        const PROJECT_DELETE = 1 << 13;

        // 创建版本
        const VERSION_CREATE = 1 << 14;
        // 读取用户版本（包括私有）
        const VERSION_READ = 1 << 15;
        // 写入版本数据（元数据、文件等）
        const VERSION_WRITE = 1 << 16;
        // 删除版本
        const VERSION_DELETE = 1 << 17;

        // 创建报告
        const REPORT_CREATE = 1 << 18;
        // 读取用户报告
        const REPORT_READ = 1 << 19;
        // 编辑报告
        const REPORT_WRITE = 1 << 20;
        // 删除报告
        const REPORT_DELETE = 1 << 21;

        // 读取帖子
        const THREAD_READ = 1 << 22;
        // 写入帖子（发送消息、删除消息）
        const THREAD_WRITE = 1 << 23;

        // 创建个人访问令牌
        const PAT_CREATE = 1 << 24;
        // 读取用户个人访问令牌
        const PAT_READ = 1 << 25;
        // 编辑个人访问令牌
        const PAT_WRITE = 1 << 26;
        // 删除个人访问令牌
        const PAT_DELETE = 1 << 27;

        // 读取用户会话
        const SESSION_READ = 1 << 28;
        // 删除会话
        const SESSION_DELETE = 1 << 29;

        // 执行分析操作
        const PERFORM_ANALYTICS = 1 << 30;

        // 创建集合
        const COLLECTION_CREATE = 1 << 31;
        // 读取用户集合
        const COLLECTION_READ = 1 << 32;
        // 写入集合
        const COLLECTION_WRITE = 1 << 33;
        // 删除集合
        const COLLECTION_DELETE = 1 << 34;

        // 创建组织
        const ORGANIZATION_CREATE = 1 << 35;
        // 读取用户组织
        const ORGANIZATION_READ = 1 << 36;
        // 写入组织
        const ORGANIZATION_WRITE = 1 << 37;
        // 删除组织
        const ORGANIZATION_DELETE = 1 << 38;

        // 仅限 BBSMC 发出的会话
        const SESSION_ACCESS = 1 << 39;

        // 写入wiki
        const WIKI_WRITE = 1 << 40;

        const NONE = 0b0;
    }
}

bitflags_serde_impl!(Scopes, u64);

impl Scopes {
    // these scopes cannot be specified in a personal access token
    pub fn restricted() -> Scopes {
        Scopes::PAT_CREATE
            | Scopes::PAT_READ
            | Scopes::PAT_WRITE
            | Scopes::PAT_DELETE
            | Scopes::SESSION_READ
            | Scopes::SESSION_DELETE
            | Scopes::SESSION_ACCESS
            | Scopes::USER_AUTH_WRITE
            | Scopes::USER_DELETE
            | Scopes::PERFORM_ANALYTICS
    }

    pub fn is_restricted(&self) -> bool {
        self.intersects(Self::restricted())
    }

    pub fn parse_from_oauth_scopes(
        scopes: &str,
    ) -> Result<Scopes, bitflags::parser::ParseError> {
        let scopes = scopes.replace(['+', ' '], "|").replace("%20", "|");
        bitflags::parser::from_str(&scopes)
    }

    pub fn to_postgres(&self) -> i64 {
        self.bits() as i64
    }

    pub fn from_postgres(value: i64) -> Self {
        Self::from_bits(value as u64).unwrap_or(Scopes::NONE)
    }
}

#[derive(Serialize, Deserialize)]
pub struct PersonalAccessToken {
    pub id: PatId,
    pub name: String,
    pub access_token: Option<String>,
    pub scopes: Scopes,
    pub user_id: UserId,
    pub created: DateTime<Utc>,
    pub expires: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}

impl PersonalAccessToken {
    pub fn from(
        data: crate::database::models::pat_item::PersonalAccessToken,
        include_token: bool,
    ) -> Self {
        Self {
            id: data.id.into(),
            name: data.name,
            access_token: if include_token {
                Some(data.access_token)
            } else {
                None
            },
            scopes: data.scopes,
            user_id: data.user_id.into(),
            created: data.created,
            expires: data.expires,
            last_used: data.last_used,
        }
    }
}
