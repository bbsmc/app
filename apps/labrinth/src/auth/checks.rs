use crate::database;
use crate::database::models::Collection;
use crate::database::models::organization_item::Organization as DBOrganization;
use crate::database::models::project_item::QueryProject;
use crate::database::models::team_item::TeamMember as DBTeamMember;
use crate::database::models::version_item::QueryVersion;
use crate::database::redis::RedisPool;
use crate::database::{Project, Version, models};
use crate::models::users::User;
use crate::routes::ApiError;
use itertools::Itertools;
use sqlx::PgPool;

pub trait ValidateAuthorized {
    fn validate_authorized(
        &self,
        user_option: Option<&User>,
    ) -> Result<(), ApiError>;
}

pub trait ValidateAllAuthorized {
    fn validate_all_authorized(
        self,
        user_option: Option<&User>,
    ) -> Result<(), ApiError>;
}

impl<'a, T, A> ValidateAllAuthorized for T
where
    T: IntoIterator<Item = &'a A>,
    A: ValidateAuthorized + 'a,
{
    fn validate_all_authorized(
        self,
        user_option: Option<&User>,
    ) -> Result<(), ApiError> {
        self.into_iter()
            .try_for_each(|c| c.validate_authorized(user_option))
    }
}

pub async fn is_visible_project(
    project_data: &Project,
    user_option: &Option<User>,
    pool: &PgPool,
    hide_unlisted: bool,
) -> Result<bool, ApiError> {
    filter_visible_project_ids(
        vec![project_data],
        user_option,
        pool,
        hide_unlisted,
    )
    .await
    .map(|x| !x.is_empty())
}

pub async fn is_team_member_project(
    project_data: &Project,
    user_option: &Option<User>,
    pool: &PgPool,
) -> Result<bool, ApiError> {
    filter_enlisted_projects_ids(vec![project_data], user_option, pool)
        .await
        .map(|x| !x.is_empty())
}

pub async fn filter_visible_projects(
    mut projects: Vec<QueryProject>,
    user_option: &Option<User>,
    pool: &PgPool,
    hide_unlisted: bool,
) -> Result<Vec<crate::models::projects::Project>, ApiError> {
    let filtered_project_ids = filter_visible_project_ids(
        projects.iter().map(|x| &x.inner).collect_vec(),
        user_option,
        pool,
        hide_unlisted,
    )
    .await?;
    projects.retain(|x| filtered_project_ids.contains(&x.inner.id));
    Ok(projects.into_iter().map(|x| x.into()).collect())
}

// Filters projects for which we can see, meaning one of the following is true:
// - it's not hidden
// - the user is enlisted on the project's team (filter_enlisted_projects)
// - the user is a mod
// This is essentially whether you can know of the project's existence
pub async fn filter_visible_project_ids(
    projects: Vec<&Project>,
    user_option: &Option<User>,
    pool: &PgPool,
    hide_unlisted: bool,
) -> Result<Vec<crate::database::models::ProjectId>, ApiError> {
    let mut return_projects = Vec::new();
    let mut check_projects = Vec::new();

    // Return projects that are not hidden or we are a mod of
    for project in projects {
        if (if hide_unlisted {
            project.status.is_searchable()
        } else {
            !project.status.is_hidden()
        }) || user_option
            .as_ref()
            .map(|x| x.role.is_mod())
            .unwrap_or(false)
        {
            return_projects.push(project.id);
        } else if user_option.is_some() {
            check_projects.push(project);
        }
    }

    // For hidden projects, return a filtered list of projects for which we are enlisted on the team
    if !check_projects.is_empty() {
        return_projects.extend(
            filter_enlisted_projects_ids(check_projects, user_option, pool)
                .await?,
        );
    }

    Ok(return_projects)
}

// Filters out projects for which we are a member of the team (or a mod)
// These are projects we have internal access to and can potentially see even if they are hidden
// This is useful for getting visibility of versions, or seeing analytics or sensitive team-restricted data of a project
pub async fn filter_enlisted_projects_ids(
    projects: Vec<&Project>,
    user_option: &Option<User>,
    pool: &PgPool,
) -> Result<Vec<crate::database::models::ProjectId>, ApiError> {
    let mut return_projects = vec![];

    if let Some(user) = user_option {
        let user_id: models::ids::UserId = user.id.into();

        use futures::TryStreamExt;

        sqlx::query!(
            "
            SELECT m.id id, m.team_id team_id FROM team_members tm
            INNER JOIN mods m ON m.team_id = tm.team_id
            LEFT JOIN organizations o ON o.team_id = tm.team_id
            WHERE tm.team_id = ANY($1) AND tm.user_id = $3
            UNION
            SELECT m.id id, m.team_id team_id FROM team_members tm
            INNER JOIN organizations o ON o.team_id = tm.team_id
            INNER JOIN mods m ON m.organization_id = o.id
            WHERE o.id = ANY($2) AND tm.user_id = $3
            ",
            &projects.iter().map(|x| x.team_id.0).collect::<Vec<_>>(),
            &projects
                .iter()
                .filter_map(|x| x.organization_id.map(|x| x.0))
                .collect::<Vec<_>>(),
            user_id as database::models::ids::UserId,
        )
        .fetch(pool)
        .map_ok(|row| {
            for x in projects.iter() {
                let bool =
                    Some(x.id.0) == row.id && Some(x.team_id.0) == row.team_id;
                if bool {
                    return_projects.push(x.id);
                }
            }
        })
        .try_collect::<Vec<()>>()
        .await?;
    }
    Ok(return_projects)
}

pub async fn is_visible_version(
    version_data: &Version,
    user_option: &Option<User>,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<bool, ApiError> {
    filter_visible_version_ids(vec![version_data], user_option, pool, redis)
        .await
        .map(|x| !x.is_empty())
}

pub async fn is_team_member_version(
    version_data: &Version,
    user_option: &Option<User>,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<bool, ApiError> {
    filter_enlisted_version_ids(vec![version_data], user_option, pool, redis)
        .await
        .map(|x| !x.is_empty())
}

pub async fn filter_visible_versions(
    mut versions: Vec<QueryVersion>,
    user_option: &Option<User>,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<Vec<crate::models::projects::Version>, ApiError> {
    let filtered_version_ids = filter_visible_version_ids(
        versions.iter().map(|x| &x.inner).collect_vec(),
        user_option,
        pool,
        redis,
    )
    .await?;
    versions.retain(|x| filtered_version_ids.contains(&x.inner.id));
    Ok(versions.into_iter().map(|x| x.into()).collect())
}

impl ValidateAuthorized for models::OAuthClient {
    fn validate_authorized(
        &self,
        user_option: Option<&User>,
    ) -> Result<(), ApiError> {
        if let Some(user) = user_option {
            return if user.role.is_mod() || user.id == self.created_by.into() {
                Ok(())
            } else {
                Err(ApiError::CustomAuthentication(
                    "您没有足够的权限操作此 OAuth 应用".to_string(),
                ))
            };
        }

        Ok(())
    }
}

pub async fn filter_visible_version_ids(
    versions: Vec<&Version>,
    user_option: &Option<User>,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<Vec<crate::database::models::VersionId>, ApiError> {
    let mut return_versions = Vec::new();
    let mut check_versions = Vec::new();

    // First, filter out versions belonging to projects we can't see
    // (ie: a hidden project, but public version, should still be hidden)
    // Gets project ids of versions
    let project_ids = versions.iter().map(|x| x.project_id).collect::<Vec<_>>();

    // Get visible projects- ones we are allowed to see public versions for.
    let visible_project_ids = filter_visible_project_ids(
        Project::get_many_ids(&project_ids, pool, redis)
            .await?
            .iter()
            .map(|x| &x.inner)
            .collect(),
        user_option,
        pool,
        false,
    )
    .await?;

    // Then, get enlisted versions (Versions that are a part of a project we are a member of)
    let enlisted_version_ids =
        filter_enlisted_version_ids(versions.clone(), user_option, pool, redis)
            .await?;

    // Return versions that are not hidden, we are a mod of, or we are enlisted on the team of
    for version in versions {
        // We can see the version if:
        // - it's not hidden and we can see the project
        // - we are a mod
        // - we are enlisted on the team of the mod
        if (!version.status.is_hidden()
            && visible_project_ids.contains(&version.project_id))
            || user_option
                .as_ref()
                .map(|x| x.role.is_mod())
                .unwrap_or(false)
            || enlisted_version_ids.contains(&version.id)
        {
            return_versions.push(version.id);
        } else if user_option.is_some() {
            check_versions.push(version);
        }
    }

    Ok(return_versions)
}

pub async fn filter_enlisted_version_ids(
    versions: Vec<&Version>,
    user_option: &Option<User>,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<Vec<crate::database::models::VersionId>, ApiError> {
    let mut return_versions = Vec::new();

    // Get project ids of versions
    let project_ids = versions.iter().map(|x| x.project_id).collect::<Vec<_>>();

    // Get enlisted projects- ones we are allowed to see hidden versions for.
    let authorized_project_ids = filter_enlisted_projects_ids(
        Project::get_many_ids(&project_ids, pool, redis)
            .await?
            .iter()
            .map(|x| &x.inner)
            .collect(),
        user_option,
        pool,
    )
    .await?;

    for version in versions {
        if user_option
            .as_ref()
            .map(|x| x.role.is_mod())
            .unwrap_or(false)
            || (user_option.is_some()
                && authorized_project_ids.contains(&version.project_id))
        {
            return_versions.push(version.id);
        }
    }

    Ok(return_versions)
}

pub async fn is_visible_collection(
    collection_data: &Collection,
    user_option: &Option<User>,
) -> Result<bool, ApiError> {
    let mut authorized = !collection_data.status.is_hidden();
    if let Some(user) = &user_option
        && !authorized
        && (user.role.is_mod() || user.id == collection_data.user_id.into())
    {
        authorized = true;
    }
    Ok(authorized)
}

pub async fn filter_visible_collections(
    collections: Vec<Collection>,
    user_option: &Option<User>,
) -> Result<Vec<crate::models::collections::Collection>, ApiError> {
    let mut return_collections = Vec::new();
    let mut check_collections = Vec::new();

    for collection in collections {
        if !collection.status.is_hidden()
            || user_option
                .as_ref()
                .map(|x| x.role.is_mod())
                .unwrap_or(false)
        {
            return_collections.push(collection.into());
        } else if user_option.is_some() {
            check_collections.push(collection);
        }
    }

    for collection in check_collections {
        // Collections are simple- if we are the owner or a mod, we can see it
        if let Some(user) = user_option
            && (user.role.is_mod() || user.id == collection.user_id.into())
        {
            return_collections.push(collection.into());
        }
    }

    Ok(return_collections)
}

// ==================== 用户封禁检查 ====================

/// 检查用户是否被封禁（直接查询数据库，不用缓存）
///
/// # 参数
/// - `user_id`: 用户ID
/// - `ban_type`: 需要检查的封禁类型
/// - `pool`: 数据库连接池
///
/// # 返回
/// - `Ok(())`: 用户未被封禁
/// - `Err(ApiError::Banned(...))`: 用户被封禁
async fn check_user_ban_direct(
    user_id: models::ids::UserId,
    ban_type: &str,
    pool: &PgPool,
) -> Result<(), ApiError> {
    // 直接查询数据库，不使用缓存（避免锁问题）
    let ban_exists = sqlx::query!(
        "SELECT EXISTS(
            SELECT 1 FROM user_bans
            WHERE user_id = $1
            AND ban_type = $2
            AND is_active = true
            AND (expires_at IS NULL OR expires_at > NOW())
        ) as exists",
        user_id.0 as i64,
        ban_type
    )
    .fetch_one(pool)
    .await?;

    if ban_exists.exists.unwrap_or(false) {
        let message = match ban_type {
            "global" => {
                "您的账号已被全局封禁，无法执行任何操作。如有疑问，请提交申诉。"
            }
            "resource" => {
                "您已被禁止资源操作（创建/编辑/删除项目、上传版本等）。如有疑问，请提交申诉。"
            }
            "forum" => {
                "您已被禁止社交互动（评论、发帖、编辑百科、发送消息等）。如有疑问，请提交申诉。"
            }
            _ => "您的账号已被封禁。如有疑问，请提交申诉。",
        };
        return Err(ApiError::Banned(message.to_string()));
    }
    Ok(())
}

/// 检查用户是否被资源类封禁
///
/// 适用于：创建/编辑/删除项目、上传版本、团队管理等操作
/// 同时检查全局封禁，因为全局封禁应阻止所有操作
pub async fn check_resource_ban(
    user: &User,
    pool: &PgPool,
) -> Result<(), ApiError> {
    // 先检查全局封禁
    check_user_ban_direct(user.id.into(), "global", pool).await?;
    // 再检查资源封禁
    check_user_ban_direct(user.id.into(), "resource", pool).await
}

/// 检查用户是否被论坛类封禁
///
/// 适用于：评论、发帖、百科编辑、发送消息、举报等操作
/// 同时检查全局封禁，因为全局封禁应阻止所有操作
pub async fn check_forum_ban(
    user: &User,
    pool: &PgPool,
) -> Result<(), ApiError> {
    // 先检查全局封禁
    check_user_ban_direct(user.id.into(), "global", pool).await?;
    // 再检查论坛封禁
    check_user_ban_direct(user.id.into(), "forum", pool).await
}

/// 检查用户是否被全局封禁
///
/// 适用于：登录验证、敏感操作等
pub async fn check_global_ban(
    user: &User,
    pool: &PgPool,
) -> Result<(), ApiError> {
    check_user_ban_direct(user.id.into(), "global", pool).await
}

/// 检查组织是否对当前用户可见
/// 来源: BBSMC 上游提交 290c9fc19 - hide orgs without a purpose (#4426)
///
/// 组织可见条件（满足任一即可）：
/// 1. 组织有可搜索的项目（approved 或 archived 状态）
/// 2. 组织有超过1个已接受的成员
/// 3. 当前用户是版主或组织成员
pub async fn is_visible_organization(
    organization: &DBOrganization,
    viewing_user: &Option<User>,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<bool, ApiError> {
    let members =
        DBTeamMember::get_from_team_full(organization.team_id, pool, redis)
            .await?;

    // 检查组织是否有可搜索的项目
    let has_searchable_projects = sqlx::query_scalar!(
        "SELECT TRUE FROM mods WHERE organization_id = $1 AND status IN ('approved', 'archived') LIMIT 1",
        organization.id as database::models::ids::OrganizationId
    )
    .fetch_optional(pool)
    .await?
    .flatten()
    .unwrap_or(false);

    let visible = has_searchable_projects
        || members.iter().filter(|member| member.accepted).count() > 1
        || viewing_user.as_ref().is_some_and(|viewing_user| {
            viewing_user.role.is_mod()
                || members
                    .iter()
                    .any(|member| member.user_id == viewing_user.id.into())
        });

    Ok(visible)
}
