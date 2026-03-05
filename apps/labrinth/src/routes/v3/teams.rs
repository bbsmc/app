use crate::auth::checks::{check_resource_ban, is_visible_project};
use crate::auth::get_user_from_headers;
use crate::database::Project;
use crate::database::models::notification_item::NotificationBuilder;
use crate::database::models::team_item::TeamAssociationId;
use crate::database::models::{Organization, Team, TeamMember, User};
use crate::database::redis::RedisPool;
use crate::models::notifications::NotificationBody;
use crate::models::pats::Scopes;
use crate::models::teams::{
    OrganizationPermissions, ProjectPermissions, TeamId,
};
use crate::models::users::UserId;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use actix_web::{HttpRequest, HttpResponse, web};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("teams", web::get().to(teams_get));

    cfg.service(
        web::scope("team")
            .route("{id}/members", web::get().to(team_members_get))
            .route("{id}/members/{user_id}", web::patch().to(edit_team_member))
            .route(
                "{id}/members/{user_id}",
                web::delete().to(remove_team_member),
            )
            .route("{id}/members", web::post().to(add_team_member))
            .route("{id}/join", web::post().to(join_team))
            .route("{id}/owner", web::patch().to(transfer_ownership)),
    );
}

// 返回项目的所有成员，
// 包括项目团队的成员，但
// 如果项目与组织相关联，还包括组织团队的成员
// （与 team_members_get_project 不同，它只返回项目团队的成员）
// 它们可以通过 "organization_permissions" 字段是否为空来区分
pub async fn team_members_get_project(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let string = info.into_inner().0;
    let project_data =
        crate::database::models::Project::get(&string, &**pool, &redis).await?;

    if let Some(project) = project_data {
        let current_user = get_user_from_headers(
            &req,
            &**pool,
            &redis,
            &session_queue,
            Some(&[Scopes::PROJECT_READ]),
        )
        .await
        .map(|x| x.1)
        .ok();

        if !is_visible_project(&project.inner, &current_user, &pool, false)
            .await?
        {
            return Err(ApiError::NotFound);
        }
        let members_data = TeamMember::get_from_team_full(
            project.inner.team_id,
            &**pool,
            &redis,
        )
        .await?;
        let users = User::get_many_ids(
            &members_data.iter().map(|x| x.user_id).collect::<Vec<_>>(),
            &**pool,
            &redis,
        )
        .await?;

        let user_id = current_user.as_ref().map(|x| x.id.into());
        let logged_in = if let Some(user_id) = user_id {
            let (team_member, organization_team_member) =
                TeamMember::get_for_project_permissions(
                    &project.inner,
                    user_id,
                    &**pool,
                )
                .await?;

            team_member.is_some() || organization_team_member.is_some()
        } else {
            false
        };

        let team_members: Vec<_> = members_data
            .into_iter()
            .filter(|x| {
                logged_in
                    || x.accepted
                    || user_id
                        .map(|y: crate::database::models::UserId| {
                            y == x.user_id
                        })
                        .unwrap_or(false)
            })
            .flat_map(|data| {
                users.iter().find(|x| x.id == data.user_id).map(|user| {
                    crate::models::teams::TeamMember::from(
                        data,
                        user.clone(),
                        !logged_in,
                    )
                })
            })
            .collect();

        Ok(HttpResponse::Ok().json(team_members))
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn team_members_get_organization(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let string = info.into_inner().0;
    let organization_data =
        crate::database::models::Organization::get(&string, &**pool, &redis)
            .await?;

    if let Some(organization) = organization_data {
        let current_user = get_user_from_headers(
            &req,
            &**pool,
            &redis,
            &session_queue,
            Some(&[Scopes::ORGANIZATION_READ]),
        )
        .await
        .map(|x| x.1)
        .ok();

        let members_data = TeamMember::get_from_team_full(
            organization.team_id,
            &**pool,
            &redis,
        )
        .await?;
        let users = crate::database::models::User::get_many_ids(
            &members_data.iter().map(|x| x.user_id).collect::<Vec<_>>(),
            &**pool,
            &redis,
        )
        .await?;

        let user_id = current_user.as_ref().map(|x| x.id.into());

        let logged_in = current_user
            .and_then(|user| {
                members_data
                    .iter()
                    .find(|x| x.user_id == user.id.into() && x.accepted)
            })
            .is_some();

        let team_members: Vec<_> = members_data
            .into_iter()
            .filter(|x| {
                logged_in
                    || x.accepted
                    || user_id
                        .map(|y: crate::database::models::UserId| {
                            y == x.user_id
                        })
                        .unwrap_or(false)
            })
            .flat_map(|data| {
                users.iter().find(|x| x.id == data.user_id).map(|user| {
                    crate::models::teams::TeamMember::from(
                        data,
                        user.clone(),
                        !logged_in,
                    )
                })
            })
            .collect();

        Ok(HttpResponse::Ok().json(team_members))
    } else {
        Err(ApiError::NotFound)
    }
}

// 返回团队的所有成员，但不一定包括项目团队所属组织的成员（与 team_members_get_project 不同）
pub async fn team_members_get(
    req: HttpRequest,
    info: web::Path<(TeamId,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let id = info.into_inner().0;
    let members_data =
        TeamMember::get_from_team_full(id.into(), &**pool, &redis).await?;
    let users = crate::database::models::User::get_many_ids(
        &members_data.iter().map(|x| x.user_id).collect::<Vec<_>>(),
        &**pool,
        &redis,
    )
    .await?;

    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();
    let user_id = current_user.as_ref().map(|x| x.id.into());

    let logged_in = current_user
        .and_then(|user| {
            members_data
                .iter()
                .find(|x| x.user_id == user.id.into() && x.accepted)
        })
        .is_some();

    let team_members: Vec<_> = members_data
        .into_iter()
        .filter(|x| {
            logged_in
                || x.accepted
                || user_id
                    .map(|y: crate::database::models::UserId| y == x.user_id)
                    .unwrap_or(false)
        })
        .flat_map(|data| {
            users.iter().find(|x| x.id == data.user_id).map(|user| {
                crate::models::teams::TeamMember::from(
                    data,
                    user.clone(),
                    !logged_in,
                )
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(team_members))
}

#[derive(Serialize, Deserialize)]
pub struct TeamIds {
    pub ids: String,
}

pub async fn teams_get(
    req: HttpRequest,
    web::Query(ids): web::Query<TeamIds>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    use itertools::Itertools;

    let team_ids = serde_json::from_str::<Vec<TeamId>>(&ids.ids)?
        .into_iter()
        .map(|x| x.into())
        .collect::<Vec<crate::database::models::ids::TeamId>>();

    let teams_data =
        TeamMember::get_from_team_full_many(&team_ids, &**pool, &redis).await?;
    let users = crate::database::models::User::get_many_ids(
        &teams_data.iter().map(|x| x.user_id).collect::<Vec<_>>(),
        &**pool,
        &redis,
    )
    .await?;

    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();

    let teams_groups = teams_data.into_iter().chunk_by(|data| data.team_id.0);

    let mut teams: Vec<Vec<crate::models::teams::TeamMember>> = vec![];

    for (_, member_data) in &teams_groups {
        let members = member_data.collect::<Vec<_>>();

        let logged_in = current_user
            .as_ref()
            .and_then(|user| {
                members
                    .iter()
                    .find(|x| x.user_id == user.id.into() && x.accepted)
            })
            .is_some();

        let team_members = members
            .into_iter()
            .filter(|x| logged_in || x.accepted)
            .flat_map(|data| {
                users.iter().find(|x| x.id == data.user_id).map(|user| {
                    crate::models::teams::TeamMember::from(
                        data,
                        user.clone(),
                        !logged_in,
                    )
                })
            });

        teams.push(team_members.collect());
    }

    Ok(HttpResponse::Ok().json(teams))
}

pub async fn join_team(
    req: HttpRequest,
    info: web::Path<(TeamId,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let team_id = info.into_inner().0.into();
    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await?
    .1;

    // 检查资源封禁
    check_resource_ban(&current_user, &pool).await?;

    let member = TeamMember::get_from_user_id_pending(
        team_id,
        current_user.id.into(),
        &**pool,
    )
    .await?;

    if let Some(member) = member {
        if member.accepted {
            return Err(ApiError::InvalidInput(
                "您已经是此团队的一员".to_string(),
            ));
        }
        let mut transaction = pool.begin().await?;

        // 将 Team Member 的 Accepted 设置为 True
        TeamMember::edit_team_member(
            team_id,
            current_user.id.into(),
            None,
            None,
            None,
            Some(true),
            None,
            None,
            None,
            &mut transaction,
        )
        .await?;

        transaction.commit().await?;

        User::clear_project_cache(&[current_user.id.into()], &redis).await?;
        TeamMember::clear_cache(team_id, &redis).await?;
    } else {
        return Err(ApiError::InvalidInput(
            "此团队没有待处理的请求".to_string(),
        ));
    }

    Ok(HttpResponse::NoContent().body(""))
}

fn default_role() -> String {
    "成员".to_string()
}

fn default_ordering() -> i64 {
    0
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NewTeamMember {
    pub user_id: UserId,
    #[serde(default = "default_role")]
    pub role: String,
    #[serde(default)]
    pub permissions: ProjectPermissions,
    #[serde(default)]
    pub organization_permissions: Option<OrganizationPermissions>,
    #[serde(default)]
    #[serde(with = "rust_decimal::serde::float")]
    pub payouts_split: Decimal,
    #[serde(default = "default_ordering")]
    pub ordering: i64,
}

pub async fn add_team_member(
    req: HttpRequest,
    info: web::Path<(TeamId,)>,
    pool: web::Data<PgPool>,
    new_member: web::Json<NewTeamMember>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let team_id = info.into_inner().0.into();

    let mut transaction = pool.begin().await?;

    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await?
    .1;

    // 检查资源封禁
    check_resource_ban(&current_user, &pool).await?;

    let team_association = Team::get_association(team_id, &**pool)
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("指定的团队不存在".to_string())
        })?;
    let member =
        TeamMember::get_from_user_id(team_id, current_user.id.into(), &**pool)
            .await?;
    match team_association {
        // 如果团队与项目关联，检查他们是否有权限邀请用户到该项目
        TeamAssociationId::Project(pid) => {
            let organization =
                Organization::get_associated_organization_project_id(
                    pid, &**pool,
                )
                .await?;
            let organization_team_member =
                if let Some(organization) = &organization {
                    TeamMember::get_from_user_id(
                        organization.team_id,
                        current_user.id.into(),
                        &**pool,
                    )
                    .await?
                } else {
                    None
                };
            let permissions = ProjectPermissions::get_permissions_by_role(
                &current_user.role,
                &member,
                &organization_team_member,
            )
            .unwrap_or_default();

            if !permissions.contains(ProjectPermissions::MANAGE_INVITES) {
                return Err(ApiError::CustomAuthentication(
                    "您没有权限邀请用户到此团队".to_string(),
                ));
            }
            if !permissions.contains(new_member.permissions) {
                return Err(ApiError::InvalidInput(
                    "不能赋予新成员您自身不具备的权限".to_string(),
                ));
            }

            if new_member.organization_permissions.is_some() {
                return Err(ApiError::InvalidInput(
                    "不能为项目团队成员设置组织权限".to_string(),
                ));
            }
        }
        // 如果团队与组织关联，检查他们是否有权限邀请用户到该组织
        TeamAssociationId::Organization(_) => {
            let organization_permissions =
                OrganizationPermissions::get_permissions_by_role(
                    &current_user.role,
                    &member,
                )
                .unwrap_or_default();
            if !organization_permissions
                .contains(OrganizationPermissions::MANAGE_INVITES)
            {
                return Err(ApiError::CustomAuthentication(
                    "您没有权限邀请用户到此组织".to_string(),
                ));
            }
            if !organization_permissions.contains(
                new_member.organization_permissions.unwrap_or_default(),
            ) {
                return Err(ApiError::InvalidInput(
                    "不能赋予新成员您自身不具备的组织权限".to_string(),
                ));
            }
            if !organization_permissions.contains(
                OrganizationPermissions::EDIT_MEMBER_DEFAULT_PERMISSIONS,
            ) && !new_member.permissions.is_empty()
            {
                return Err(ApiError::CustomAuthentication(
                    "您没有权限给予此用户默认项目权限，请确保 permissions 设置为空 (0)".to_string(),
                ));
            }
        }
    }

    if new_member.payouts_split < Decimal::ZERO
        || new_member.payouts_split > Decimal::from(5000)
    {
        return Err(ApiError::InvalidInput(
            "收益分成比例必须在 0 到 5000 之间".to_string(),
        ));
    }

    let request = TeamMember::get_from_user_id_pending(
        team_id,
        new_member.user_id.into(),
        &**pool,
    )
    .await?;

    if let Some(req) = request {
        if req.accepted {
            return Err(ApiError::InvalidInput(
                "该用户已经是项目成员".to_string(),
            ));
        } else {
            return Err(ApiError::InvalidInput(
                "该用户已经有待处理的请求".to_string(),
            ));
        }
    }
    let new_user = crate::database::models::User::get_id(
        new_member.user_id.into(),
        &**pool,
        &redis,
    )
    .await?
    .ok_or_else(|| ApiError::InvalidInput("无效的用户ID".to_string()))?;

    let mut force_accepted = false;
    if let TeamAssociationId::Project(pid) = team_association {
        // 我们无法将所有者添加到他们自己的组织中的项目团队
        let organization =
            Organization::get_associated_organization_project_id(pid, &**pool)
                .await?;
        let new_user_organization_team_member =
            if let Some(organization) = &organization {
                TeamMember::get_from_user_id(
                    organization.team_id,
                    new_user.id,
                    &**pool,
                )
                .await?
            } else {
                None
            };
        // println!("{:?}", new_user_organization_team_member);
        // println!("{:?}", new_member.permissions);
        if new_user_organization_team_member
            .as_ref()
            .map(|tm| tm.is_owner)
            .unwrap_or(false)
            && new_member.permissions != ProjectPermissions::all()
        {
            return Err(ApiError::InvalidInput(
                "不能在项目团队中限制组织所有者的权限".to_string(),
            ));
        }

        // 如果将一个组织中的用户添加到该组织拥有的项目中，
        // 该用户会自动被接受到该项目中。
        // 因为该用户是组织的一部分，项目团队成员资格也可以用来减少权限
        // （这不应该是用户可以拒绝的操作）
        if new_user_organization_team_member.is_some() {
            force_accepted = true;
        }
    }

    let new_id =
        crate::database::models::ids::generate_team_member_id(&mut transaction)
            .await?;
    TeamMember {
        id: new_id,
        team_id,
        user_id: new_member.user_id.into(),
        role: new_member.role.clone(),
        is_owner: false, // 不能只是创建一个所有者
        permissions: new_member.permissions,
        organization_permissions: new_member.organization_permissions,
        accepted: force_accepted,
        payouts_split: new_member.payouts_split,
        ordering: new_member.ordering,
    }
    .insert(&mut transaction)
    .await?;

    // 如果用户有机会接受邀请，发送通知
    if !force_accepted {
        match team_association {
            TeamAssociationId::Project(pid) => {
                NotificationBuilder {
                    body: NotificationBody::TeamInvite {
                        project_id: pid.into(),
                        team_id: team_id.into(),
                        invited_by: current_user.id,
                        role: new_member.role.clone(),
                    },
                }
                .insert(new_member.user_id.into(), &mut transaction, &redis)
                .await?;
            }
            TeamAssociationId::Organization(oid) => {
                NotificationBuilder {
                    body: NotificationBody::OrganizationInvite {
                        organization_id: oid.into(),
                        team_id: team_id.into(),
                        invited_by: current_user.id,
                        role: new_member.role.clone(),
                    },
                }
                .insert(new_member.user_id.into(), &mut transaction, &redis)
                .await?;
            }
        }
    }

    transaction.commit().await?;
    TeamMember::clear_cache(team_id, &redis).await?;
    User::clear_project_cache(&[new_member.user_id.into()], &redis).await?;

    Ok(HttpResponse::NoContent().body(""))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EditTeamMember {
    pub permissions: Option<ProjectPermissions>,
    pub organization_permissions: Option<OrganizationPermissions>,
    pub role: Option<String>,
    pub payouts_split: Option<Decimal>,
    pub ordering: Option<i64>,
}

pub async fn edit_team_member(
    req: HttpRequest,
    info: web::Path<(TeamId, UserId)>,
    pool: web::Data<PgPool>,
    edit_member: web::Json<EditTeamMember>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let ids = info.into_inner();
    let id = ids.0.into();
    let user_id = ids.1.into();

    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await?
    .1;

    // 检查资源封禁
    check_resource_ban(&current_user, &pool).await?;

    let team_association =
        Team::get_association(id, &**pool).await?.ok_or_else(|| {
            ApiError::InvalidInput("指定的团队不存在".to_string())
        })?;
    let member =
        TeamMember::get_from_user_id(id, current_user.id.into(), &**pool)
            .await?;
    let edit_member_db =
        TeamMember::get_from_user_id_pending(id, user_id, &**pool)
            .await?
            .ok_or_else(|| {
                ApiError::CustomAuthentication(
                    "您没有权限编辑此团队成员".to_string(),
                )
            })?;

    let mut transaction = pool.begin().await?;

    if edit_member_db.is_owner
        && (edit_member.permissions.is_some()
            || edit_member.organization_permissions.is_some())
    {
        return Err(ApiError::InvalidInput(
            "不能编辑团队所有者的权限".to_string(),
        ));
    }

    match team_association {
        TeamAssociationId::Project(project_id) => {
            let organization =
                Organization::get_associated_organization_project_id(
                    project_id, &**pool,
                )
                .await?;
            // 该用户在团队里的权限
            let organization_team_member =
                if let Some(organization) = &organization {
                    TeamMember::get_from_user_id(
                        organization.team_id,
                        current_user.id.into(),
                        &**pool,
                    )
                    .await?
                } else {
                    None
                };

            let edit_member_organization_team_member =
                if let Some(organization) = &organization {
                    TeamMember::get_from_user_id(
                        organization.team_id,
                        user_id,
                        &**pool,
                    )
                    .await?
                } else {
                    None
                };

            if edit_member_organization_team_member
                .as_ref()
                .map(|x| x.is_owner)
                .unwrap_or(false)
                && edit_member
                    .permissions
                    .map(|x| x != ProjectPermissions::all())
                    .unwrap_or(false)
            {
                return Err(ApiError::CustomAuthentication(
                    "不能限制组织所有者的项目权限".to_string(),
                ));
            }

            let permissions = ProjectPermissions::get_permissions_by_role(
                &current_user.role,
                &member.clone(),
                &organization_team_member,
            )
            .unwrap_or_default();
            if !permissions.contains(ProjectPermissions::EDIT_MEMBER) {
                return Err(ApiError::CustomAuthentication(
                    "您没有权限编辑成员的权限".to_string(),
                ));
            }

            if let Some(new_permissions) = edit_member.permissions
                && !permissions.contains(new_permissions)
            {
                return Err(ApiError::InvalidInput(
                    "不能赋予您自身不具备的权限".to_string(),
                ));
            }

            if edit_member.organization_permissions.is_some() {
                return Err(ApiError::InvalidInput(
                    "不能编辑项目团队成员的组织权限".to_string(),
                ));
            }
        }
        TeamAssociationId::Organization(_) => {
            let organization_permissions =
                OrganizationPermissions::get_permissions_by_role(
                    &current_user.role,
                    &member,
                )
                .unwrap_or_default();

            if !organization_permissions
                .contains(OrganizationPermissions::EDIT_MEMBER)
            {
                return Err(ApiError::CustomAuthentication(
                    "您没有权限编辑此团队成员".to_string(),
                ));
            }

            if let Some(new_permissions) = edit_member.organization_permissions
                && !organization_permissions.contains(new_permissions)
            {
                return Err(ApiError::InvalidInput(
                    "不能赋予您自身不具备的组织权限".to_string(),
                ));
            }

            if edit_member.permissions.is_some()
                && !organization_permissions.contains(
                    OrganizationPermissions::EDIT_MEMBER_DEFAULT_PERMISSIONS,
                )
            {
                return Err(ApiError::CustomAuthentication(
                    "您没有权限编辑此用户的默认项目权限".to_string(),
                ));
            }
        }
    }

    if let Some(payouts_split) = edit_member.payouts_split
        && (payouts_split < Decimal::ZERO
            || payouts_split > Decimal::from(5000))
    {
        return Err(ApiError::InvalidInput(
            "收益分成比例必须在 0 到 5000 之间".to_string(),
        ));
    }

    if let Some(role) = &edit_member.role {
        let risk = crate::util::risk::check_text_risk(
            role,
            &current_user.username,
            &format!("/user/{}", current_user.username),
            "团队成员角色",
            &redis,
        )
        .await?;
        if !risk {
            return Err(ApiError::InvalidInput(
                "团队成员角色包含敏感词，已被记录该次提交，请勿在本网站使用涉及敏感词的团队成员角色".to_string(),
            ));
        }
    }

    TeamMember::edit_team_member(
        id,
        user_id,
        edit_member.permissions,
        edit_member.organization_permissions,
        edit_member.role.clone(),
        None,
        edit_member.payouts_split,
        edit_member.ordering,
        None,
        &mut transaction,
    )
    .await?;

    transaction.commit().await?;
    TeamMember::clear_cache(id, &redis).await?;

    Ok(HttpResponse::NoContent().body(""))
}

#[derive(Deserialize)]
pub struct TransferOwnership {
    pub user_id: UserId,
}

pub async fn transfer_ownership(
    req: HttpRequest,
    info: web::Path<(TeamId,)>,
    pool: web::Data<PgPool>,
    new_owner: web::Json<TransferOwnership>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let id = info.into_inner().0;

    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await?
    .1;

    // 检查资源封禁
    check_resource_ban(&current_user, &pool).await?;

    // 禁止转移项目团队的所有权，这些团队由组织拥有
    // 这些团队由组织所有者拥有，必须首先从组织中删除
    // 在这些情况下不应该有所有者，但以防万一。
    let team_association_id = Team::get_association(id.into(), &**pool).await?;
    if let Some(TeamAssociationId::Project(pid)) = team_association_id {
        let result = Project::get_id(pid, &**pool, &redis).await?;
        if let Some(project_item) = result
            && project_item.inner.organization_id.is_some()
        {
            return Err(ApiError::InvalidInput(
                "不能转移属于组织的项目团队的所有权".to_string(),
            ));
        }
    }

    if !current_user.role.is_admin() {
        let member = TeamMember::get_from_user_id(
            id.into(),
            current_user.id.into(),
            &**pool,
        )
        .await?
        .ok_or_else(|| {
            ApiError::CustomAuthentication(
                "您没有权限编辑此团队成员".to_string(),
            )
        })?;

        if !member.is_owner {
            return Err(ApiError::CustomAuthentication(
                "您没有权限编辑此团队的所有权".to_string(),
            ));
        }
    }

    let new_member = TeamMember::get_from_user_id(
        id.into(),
        new_owner.user_id.into(),
        &**pool,
    )
    .await?
    .ok_or_else(|| {
        ApiError::InvalidInput("指定的新所有者不是团队成员".to_string())
    })?;

    if !new_member.accepted {
        return Err(ApiError::InvalidInput(
            "您只能将所有权转移给当前在您团队中的成员".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;

    // 以下是修改 is_owner 的唯一位置
    TeamMember::edit_team_member(
        id.into(),
        current_user.id.into(),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(false),
        &mut transaction,
    )
    .await?;

    TeamMember::edit_team_member(
        id.into(),
        new_owner.user_id.into(),
        Some(ProjectPermissions::all()),
        if matches!(
            team_association_id,
            Some(TeamAssociationId::Organization(_))
        ) {
            Some(OrganizationPermissions::all())
        } else {
            None
        },
        None,
        None,
        None,
        None,
        Some(true),
        &mut transaction,
    )
    .await?;

    let project_teams_edited =
        if let Some(TeamAssociationId::Organization(oid)) = team_association_id
        {
            // 如果适用，组织拥有的所有项目的所有者应被移除为这些项目的成员，
            // 如果他们是这些项目的成员。
            // （因为他们是这些项目的组织所有者，并且他们不应该有更多的特定权限）

            // 首先，获取此组织拥有的每个项目的团队ID
            let team_ids = sqlx::query!(
                "
            SELECT m.team_id FROM organizations o
            INNER JOIN mods m ON m.organization_id = o.id
            WHERE o.id = $1 AND $1 IS NOT NULL
            ",
                oid.0 as i64
            )
            .fetch_all(&mut *transaction)
            .await?;

            let team_ids: Vec<crate::database::models::ids::TeamId> = team_ids
                .into_iter()
                .map(|x| TeamId(x.team_id as u64).into())
                .collect();

            // 如果组织所有者是这些项目的成员，移除他们
            for team_id in team_ids.iter() {
                TeamMember::delete(
                    *team_id,
                    new_owner.user_id.into(),
                    &mut transaction,
                )
                .await?;
            }

            team_ids
        } else {
            vec![]
        };

    transaction.commit().await?;
    TeamMember::clear_cache(id.into(), &redis).await?;
    for team_id in project_teams_edited {
        TeamMember::clear_cache(team_id, &redis).await?;
    }

    Ok(HttpResponse::NoContent().body(""))
}

pub async fn remove_team_member(
    req: HttpRequest,
    info: web::Path<(TeamId, UserId)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let ids = info.into_inner();
    let id = ids.0.into();
    let user_id = ids.1.into();

    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await?
    .1;

    // 检查资源封禁
    check_resource_ban(&current_user, &pool).await?;

    let team_association =
        Team::get_association(id, &**pool).await?.ok_or_else(|| {
            ApiError::InvalidInput("指定的团队不存在".to_string())
        })?;
    let member =
        TeamMember::get_from_user_id(id, current_user.id.into(), &**pool)
            .await?;

    let delete_member =
        TeamMember::get_from_user_id_pending(id, user_id, &**pool).await?;

    if let Some(delete_member) = delete_member {
        if delete_member.is_owner {
            // 团队所有者不能被移除
            return Err(ApiError::CustomAuthentication(
                "团队所有者不能被移除".to_string(),
            ));
        }

        let mut transaction = pool.begin().await?;

        // 附加到此团队的项目团队
        match team_association {
            TeamAssociationId::Project(pid) => {
                let organization =
                    Organization::get_associated_organization_project_id(
                        pid, &**pool,
                    )
                    .await?;
                let organization_team_member =
                    if let Some(organization) = &organization {
                        TeamMember::get_from_user_id(
                            organization.team_id,
                            current_user.id.into(),
                            &**pool,
                        )
                        .await?
                    } else {
                        None
                    };
                let permissions = ProjectPermissions::get_permissions_by_role(
                    &current_user.role,
                    &member,
                    &organization_team_member,
                )
                .unwrap_or_default();

                if delete_member.accepted {
                    // 团队成员除了所有者可以离开团队，或者被具有 REMOVE_MEMBER 权限的成员移除。
                    if Some(delete_member.user_id)
                        == member.as_ref().map(|m| m.user_id)
                        || permissions
                            .contains(ProjectPermissions::REMOVE_MEMBER)
                    // 如果权限存在，但成员不存在，他们属于一个组织
                    {
                        TeamMember::delete(id, user_id, &mut transaction)
                            .await?;
                    } else {
                        return Err(ApiError::CustomAuthentication(
                            "您没有权限移除此团队成员".to_string(),
                        ));
                    }
                } else if Some(delete_member.user_id)
                    == member.as_ref().map(|m| m.user_id)
                    || permissions.contains(ProjectPermissions::MANAGE_INVITES)
                // 如果权限存在，但成员不存在，他们属于一个组织
                {
                    // 这是一个待处理的邀请，而不是成员，所以被邀请的用户或具有 MANAGE_INVITES 权限的团队成员可以移除它。
                    TeamMember::delete(id, user_id, &mut transaction).await?;
                } else {
                    return Err(ApiError::CustomAuthentication(
                        "您没有权限取消团队邀请".to_string(),
                    ));
                }
            }
            TeamAssociationId::Organization(_) => {
                let organization_permissions =
                    OrganizationPermissions::get_permissions_by_role(
                        &current_user.role,
                        &member,
                    )
                    .unwrap_or_default();
                // 组织团队需要一个 TeamMember，所以我们可以 'unwrap'
                if delete_member.accepted {
                    // 团队成员除了所有者可以离开团队，或者被具有 REMOVE_MEMBER 权限的成员移除。
                    if Some(delete_member.user_id) == member.map(|m| m.user_id)
                        || organization_permissions
                            .contains(OrganizationPermissions::REMOVE_MEMBER)
                    {
                        TeamMember::delete(id, user_id, &mut transaction)
                            .await?;
                    } else {
                        return Err(ApiError::CustomAuthentication(
                            "您没有权限移除此组织成员".to_string(),
                        ));
                    }
                } else if Some(delete_member.user_id)
                    == member.map(|m| m.user_id)
                    || organization_permissions
                        .contains(OrganizationPermissions::MANAGE_INVITES)
                {
                    // 这是一个待处理的邀请，而不是成员，所以被邀请的用户或具有 MANAGE_INVITES 权限的团队成员可以移除它。
                    TeamMember::delete(id, user_id, &mut transaction).await?;
                } else {
                    return Err(ApiError::CustomAuthentication(
                        "您没有权限取消组织邀请".to_string(),
                    ));
                }
            }
        }

        transaction.commit().await?;

        TeamMember::clear_cache(id, &redis).await?;
        User::clear_project_cache(&[delete_member.user_id], &redis).await?;

        Ok(HttpResponse::NoContent().body(""))
    } else {
        Err(ApiError::NotFound)
    }
}
