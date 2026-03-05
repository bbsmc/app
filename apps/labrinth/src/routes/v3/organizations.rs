use std::collections::HashMap;
use std::sync::Arc;

use super::ApiError;
use crate::auth::{
    checks::is_visible_organization, filter_visible_projects,
    get_user_from_headers,
};
use crate::database::models::team_item::TeamMember;
use crate::database::models::{
    Organization, generate_organization_id, team_item,
};
use crate::database::redis::RedisPool;
use crate::file_hosting::FileHost;
use crate::models::ids::UserId;
use crate::models::ids::base62_impl::parse_base62;
use crate::models::organizations::OrganizationId;
use crate::models::pats::Scopes;
use crate::models::teams::{OrganizationPermissions, ProjectPermissions};
use crate::queue::session::AuthQueue;
use crate::util::img::delete_old_images;
use crate::util::routes::read_from_payload;
use crate::util::validate::validation_errors_to_string;
use crate::{database, models};
use actix_web::{HttpRequest, HttpResponse, web};
use futures::TryStreamExt;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::Validate;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("organizations", web::get().to(organizations_get));
    cfg.service(
        web::scope("organization")
            .route("", web::post().to(organization_create))
            .route("{id}/projects", web::get().to(organization_projects_get))
            .route("{id}", web::get().to(organization_get))
            .route("{id}", web::patch().to(organizations_edit))
            .route("{id}", web::delete().to(organization_delete))
            .route("{id}/projects", web::post().to(organization_projects_add))
            .route(
                "{id}/projects/{project_id}",
                web::delete().to(organization_projects_remove),
            )
            .route("{id}/icon", web::patch().to(organization_icon_edit))
            .route("{id}/icon", web::delete().to(delete_organization_icon))
            .route(
                "{id}/members",
                web::get().to(super::teams::team_members_get_organization),
            ),
    );
}

pub async fn organization_projects_get(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let info = info.into_inner().0;
    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::ORGANIZATION_READ, Scopes::PROJECT_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();

    let possible_organization_id: Option<u64> = parse_base62(&info).ok();

    // 来源: BBSMC 上游提交 290c9fc19 - 组织可见性检查
    let organization_data = Organization::get(&info, &**pool, &redis).await?;
    if let Some(ref org) = organization_data {
        if !is_visible_organization(org, &current_user, &pool, &redis).await? {
            return Err(ApiError::NotFound);
        }
    } else {
        return Err(ApiError::NotFound);
    }

    let project_ids: Vec<database::models::ProjectId> = sqlx::query!(
        "
        SELECT m.id FROM organizations o
        INNER JOIN mods m ON m.organization_id = o.id
        WHERE (o.id = $1 AND $1 IS NOT NULL) OR (o.slug = $2 AND $2 IS NOT NULL)
        ",
        possible_organization_id.map(|x| x as i64),
        info
    )
    .fetch(&**pool)
    .map_ok(|m| database::models::ProjectId(m.id))
    .try_collect::<Vec<database::models::ProjectId>>()
    .await?;

    let projects_data = crate::database::models::Project::get_many_ids(
        &project_ids,
        &**pool,
        &redis,
    )
    .await?;

    let projects =
        filter_visible_projects(projects_data, &current_user, &pool, true)
            .await?;
    Ok(HttpResponse::Ok().json(projects))
}

#[derive(Deserialize, Validate)]
pub struct NewOrganization {
    #[validate(
        length(min = 3, max = 64),
        regex(path = *crate::util::validate::RE_URL_SAFE)
    )]
    pub slug: String,
    // 组织的显示名称
    #[validate(length(min = 3, max = 64))]
    pub name: String,
    #[validate(length(min = 3, max = 256))]
    pub description: String,
}

pub async fn organization_create(
    req: HttpRequest,
    new_organization: web::Json<NewOrganization>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::ORGANIZATION_CREATE]),
    )
    .await?
    .1;

    new_organization.validate().map_err(|err| {
        ApiError::Validation(validation_errors_to_string(err, None))
    })?;

    let mut transaction = pool.begin().await?;

    // 尝试解析 slug
    let name_organization_id_option: Option<OrganizationId> =
        serde_json::from_str(&format!("\"{}\"", new_organization.slug)).ok();
    let mut organization_strings = vec![];
    if let Some(name_organization_id) = name_organization_id_option {
        organization_strings.push(name_organization_id.to_string());
    }
    organization_strings.push(new_organization.slug.clone());
    let results = Organization::get_many(
        &organization_strings,
        &mut *transaction,
        &redis,
    )
    .await?;
    if !results.is_empty() {
        return Err(ApiError::InvalidInput("该 Slug 已被占用！".to_string()));
    }

    let organization_id = generate_organization_id(&mut transaction).await?;

    // 创建组织管理团队
    let team = team_item::TeamBuilder {
        members: vec![team_item::TeamMemberBuilder {
            user_id: current_user.id.into(),
            role: crate::models::teams::DEFAULT_ROLE.to_owned(),
            is_owner: true,
            permissions: ProjectPermissions::all(),
            organization_permissions: Some(OrganizationPermissions::all()),
            accepted: true,
            payouts_split: Decimal::ONE_HUNDRED,
            ordering: 0,
        }],
    };
    let team_id = team.insert(&mut transaction).await?;

    let risk = crate::util::risk::check_text_risk(
        &new_organization.name,
        &current_user.username,
        &format!("/organization/{}", new_organization.slug.clone()),
        "创建组织-名称",
        &redis,
    )
    .await?;
    if !risk {
        return Err(ApiError::InvalidInput(
            "组织名称包含敏感词，已被记录该次提交，请勿在本网站使用涉及敏感词的组织名称".to_string(),
        ));
    }

    let risk = crate::util::risk::check_text_risk(
        &new_organization.description,
        &current_user.username,
        &format!("/organization/{}", new_organization.slug.clone()),
        "创建组织-描述",
        &redis,
    )
    .await?;
    if !risk {
        return Err(ApiError::InvalidInput(
            "组织描述包含敏感词，已被记录该次提交，请勿在本网站使用涉及敏感词的组织描述".to_string(),
        ));
    }
    let risk = crate::util::risk::check_text_risk(
        &new_organization.slug,
        &current_user.username,
        &format!("/organization/{}", new_organization.slug.clone()),
        "创建组织-slug",
        &redis,
    )
    .await?;
    if !risk {
        return Err(ApiError::InvalidInput(
            "组织URL包含敏感词，已被记录该次提交，请勿在本网站使用涉及敏感词的组织URL".to_string(),
        ));
    }

    // 创建组织
    let organization = Organization {
        id: organization_id,
        slug: new_organization.slug.clone(),
        name: new_organization.name.clone(),
        description: new_organization.description.clone(),
        team_id,
        icon_url: None,
        raw_icon_url: None,
        color: None,
    };
    organization.clone().insert(&mut transaction).await?;
    transaction.commit().await?;

    // 只有成员是所有者，即当前登录用户
    let member_data = TeamMember::get_from_team_full(team_id, &**pool, &redis)
        .await?
        .into_iter()
        .next();
    let members_data = if let Some(member_data) = member_data {
        vec![crate::models::teams::TeamMember::from_model(
            member_data,
            current_user.clone(),
            false,
        )]
    } else {
        return Err(ApiError::InvalidInput(
            "获取新创建的团队失败".to_owned(), // 应该永远不会发生
        ));
    };

    let organization =
        models::organizations::Organization::from(organization, members_data);

    Ok(HttpResponse::Ok().json(organization))
}

pub async fn organization_get(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let id = info.into_inner().0;
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
    let user_id = current_user.as_ref().map(|x| x.id.into());

    let organization_data = Organization::get(&id, &**pool, &redis).await?;
    // 来源: BBSMC 上游提交 290c9fc19 - 组织可见性检查
    if let Some(data) = organization_data {
        if !is_visible_organization(&data, &current_user, &pool, &redis).await?
        {
            return Err(ApiError::NotFound);
        }

        let members_data =
            TeamMember::get_from_team_full(data.team_id, &**pool, &redis)
                .await?;

        let users = crate::database::models::User::get_many_ids(
            &members_data.iter().map(|x| x.user_id).collect::<Vec<_>>(),
            &**pool,
            &redis,
        )
        .await?;
        let logged_in = current_user
            .as_ref()
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

        let organization =
            models::organizations::Organization::from(data, team_members);
        return Ok(HttpResponse::Ok().json(organization));
    }
    Err(ApiError::NotFound)
}

#[derive(Deserialize)]
pub struct OrganizationIds {
    pub ids: String,
}

pub async fn organizations_get(
    req: HttpRequest,
    web::Query(ids): web::Query<OrganizationIds>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let ids = serde_json::from_str::<Vec<&str>>(&ids.ids)?;
    let organizations_data =
        Organization::get_many(&ids, &**pool, &redis).await?;
    let team_ids = organizations_data
        .iter()
        .map(|x| x.team_id)
        .collect::<Vec<_>>();

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
        Some(&[Scopes::ORGANIZATION_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();
    let user_id = current_user.as_ref().map(|x| x.id.into());

    let mut organizations = vec![];

    let mut team_groups = HashMap::new();
    for item in teams_data {
        team_groups.entry(item.team_id).or_insert(vec![]).push(item);
    }

    // 来源: BBSMC 上游提交 290c9fc19 - 组织可见性过滤
    for data in organizations_data {
        // 过滤不可见的组织
        if !is_visible_organization(&data, &current_user, &pool, &redis).await?
        {
            continue;
        }

        let members_data = team_groups.remove(&data.team_id).unwrap_or(vec![]);
        let logged_in = current_user
            .as_ref()
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

        let organization =
            models::organizations::Organization::from(data, team_members);
        organizations.push(organization);
    }

    Ok(HttpResponse::Ok().json(organizations))
}

#[derive(Serialize, Deserialize, Validate)]
pub struct OrganizationEdit {
    #[validate(length(min = 3, max = 256))]
    pub description: Option<String>,
    #[validate(
        length(min = 3, max = 64),
        regex(path = *crate::util::validate::RE_URL_SAFE)
    )]
    pub slug: Option<String>,
    #[validate(length(min = 3, max = 64))]
    pub name: Option<String>,
}

pub async fn organizations_edit(
    req: HttpRequest,
    info: web::Path<(String,)>,
    new_organization: web::Json<OrganizationEdit>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::ORGANIZATION_WRITE]),
    )
    .await?
    .1;

    new_organization.validate().map_err(|err| {
        ApiError::Validation(validation_errors_to_string(err, None))
    })?;

    let string = info.into_inner().0;
    let result =
        database::models::Organization::get(&string, &**pool, &redis).await?;
    if let Some(organization_item) = result {
        let id = organization_item.id;

        let team_member = database::models::TeamMember::get_from_user_id(
            organization_item.team_id,
            user.id.into(),
            &**pool,
        )
        .await?;

        let permissions = OrganizationPermissions::get_permissions_by_role(
            &user.role,
            &team_member,
        );

        if let Some(perms) = permissions {
            let mut transaction = pool.begin().await?;
            if let Some(description) = &new_organization.description {
                if !perms.contains(OrganizationPermissions::EDIT_DETAILS) {
                    return Err(ApiError::CustomAuthentication(
                        "您没有权限编辑此组织的描述！".to_string(),
                    ));
                }
                let risk = crate::util::risk::check_text_risk(
                    description,
                    &user.username,
                    &format!(
                        "/organization/{}",
                        organization_item.slug.clone()
                    ),
                    "修改组织描述",
                    &redis,
                )
                .await?;
                if !risk {
                    return Err(ApiError::InvalidInput(
                        "组织描述包含敏感词，已被记录该次提交，请勿在本网站使用涉及敏感词的组织描述".to_string(),
                    ));
                }
                sqlx::query!(
                    "
                    UPDATE organizations
                    SET description = $1
                    WHERE (id = $2)
                    ",
                    description,
                    id as database::models::ids::OrganizationId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            if let Some(name) = &new_organization.name {
                if !perms.contains(OrganizationPermissions::EDIT_DETAILS) {
                    return Err(ApiError::CustomAuthentication(
                        "您没有权限编辑此组织的名称！".to_string(),
                    ));
                }

                let risk = crate::util::risk::check_text_risk(
                    name,
                    &user.username,
                    &format!(
                        "/organization/{}",
                        organization_item.slug.clone()
                    ),
                    "修改组织名称",
                    &redis,
                )
                .await?;
                if !risk {
                    return Err(ApiError::InvalidInput(
                        "组织名称包含敏感词，已被记录该次提交，请勿在本网站使用涉及敏感词的组织名称".to_string(),
                    ));
                }

                sqlx::query!(
                    "
                    UPDATE organizations
                    SET name = $1
                    WHERE (id = $2)
                    ",
                    name,
                    id as database::models::ids::OrganizationId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            if let Some(slug) = &new_organization.slug {
                if !perms.contains(OrganizationPermissions::EDIT_DETAILS) {
                    return Err(ApiError::CustomAuthentication(
                        "您没有权限编辑此组织的 slug！".to_string(),
                    ));
                }

                // BBSMC 上游提交 79c263301: 使用 Organization::get 检查 slug 是否与现有组织 ID 冲突
                let existing = Organization::get(
                    &slug.to_lowercase(),
                    &mut *transaction,
                    &redis,
                )
                .await?;
                if existing.is_some() {
                    return Err(ApiError::InvalidInput(
                        "Slug 与另一个组织的 ID 冲突！".to_string(),
                    ));
                }

                // 确保新的 slug 与旧的不同
                // 这里可以安全地 unwrap，因为 slug 始终存在
                if !slug.eq(&organization_item.slug.clone()) {
                    // BBSMC 上游提交 79c263301: 添加 text_id_lower 检查
                    let results = sqlx::query!(
                        "
                        SELECT EXISTS(
                            SELECT 1 FROM organizations
                            WHERE
                                LOWER(slug) = LOWER($1)
                                OR text_id_lower = LOWER($1)
                        )
                        ",
                        slug
                    )
                    .fetch_one(&mut *transaction)
                    .await?;

                    if results.exists.unwrap_or(true) {
                        return Err(ApiError::InvalidInput(
                            "Slug 与另一个组织的 slug 或 ID 冲突！".to_string(),
                        ));
                    }
                }

                let risk = crate::util::risk::check_text_risk(
                    slug,
                    &user.username,
                    &format!(
                        "/organization/{}",
                        organization_item.slug.clone()
                    ),
                    "修改组织URL",
                    &redis,
                )
                .await?;
                if !risk {
                    return Err(ApiError::InvalidInput(
                        "组织URL包含敏感词，已被记录该次提交，请勿在本网站使用涉及敏感词的组织URL".to_string(),
                    ));
                }

                // BBSMC 上游提交 79c263301: slug 存储为小写
                sqlx::query!(
                    "
                    UPDATE organizations
                    SET slug = LOWER($1)
                    WHERE (id = $2)
                    ",
                    Some(slug),
                    id as database::models::ids::OrganizationId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            transaction.commit().await?;
            database::models::Organization::clear_cache(
                organization_item.id,
                Some(organization_item.slug),
                &redis,
            )
            .await?;

            Ok(HttpResponse::NoContent().body(""))
        } else {
            Err(ApiError::CustomAuthentication(
                "您没有权限编辑此组织！".to_string(),
            ))
        }
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn organization_delete(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::ORGANIZATION_DELETE]),
    )
    .await?
    .1;
    let string = info.into_inner().0;

    let organization =
        database::models::Organization::get(&string, &**pool, &redis)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidInput("指定的组织不存在！".to_string())
            })?;

    if !user.role.is_admin() {
        let team_member =
            database::models::TeamMember::get_from_user_id_organization(
                organization.id,
                user.id.into(),
                false,
                &**pool,
            )
            .await
            .map_err(ApiError::Database)?
            .ok_or_else(|| {
                ApiError::InvalidInput("指定的组织不存在！".to_string())
            })?;

        let permissions = OrganizationPermissions::get_permissions_by_role(
            &user.role,
            &Some(team_member),
        )
        .unwrap_or_default();

        if !permissions.contains(OrganizationPermissions::DELETE_ORGANIZATION) {
            return Err(ApiError::CustomAuthentication(
                "您没有权限删除此组织！".to_string(),
            ));
        }
    }

    let owner_id = sqlx::query!(
        "
        SELECT user_id FROM team_members
        WHERE team_id = $1 AND is_owner = TRUE
        ",
        organization.team_id as database::models::ids::TeamId
    )
    .fetch_one(&**pool)
    .await?
    .user_id;
    let owner_id = database::models::ids::UserId(owner_id);

    let mut transaction = pool.begin().await?;

    // 处理项目- 每个在组织中的项目需要将其所有者更改为组织所有者
    // 现在，没有项目应该有所有者，如果它在组织中，并且
    // 组织的所有者不应该是一个团队成员在任何项目中
    let organization_project_teams = sqlx::query!(
        "
        SELECT t.id FROM organizations o
        INNER JOIN mods m ON m.organization_id = o.id
        INNER JOIN teams t ON t.id = m.team_id
        WHERE o.id = $1 AND $1 IS NOT NULL
        ",
        organization.id as database::models::ids::OrganizationId
    )
    .fetch(&mut *transaction)
    .map_ok(|c| database::models::TeamId(c.id))
    .try_collect::<Vec<_>>()
    .await?;

    for organization_project_team in organization_project_teams.iter() {
        let new_id = crate::database::models::ids::generate_team_member_id(
            &mut transaction,
        )
        .await?;
        let member = TeamMember {
            id: new_id,
            team_id: *organization_project_team,
            user_id: owner_id,
            role: "Inherited Owner".to_string(),
            is_owner: true,
            permissions: ProjectPermissions::all(),
            organization_permissions: None,
            accepted: true,
            payouts_split: Decimal::ZERO,
            ordering: 0,
        };
        member.insert(&mut transaction).await?;
    }
    // 安全地删除组织
    let result = database::models::Organization::remove(
        organization.id,
        &mut transaction,
        &redis,
    )
    .await?;

    transaction.commit().await?;

    database::models::Organization::clear_cache(
        organization.id,
        Some(organization.slug),
        &redis,
    )
    .await?;

    for team_id in organization_project_teams {
        database::models::TeamMember::clear_cache(team_id, &redis).await?;
    }

    if result.is_some() {
        Ok(HttpResponse::NoContent().body(""))
    } else {
        Err(ApiError::NotFound)
    }
}

#[derive(Deserialize)]
pub struct OrganizationProjectAdd {
    pub project_id: String, // 也允许名称/slug
}
pub async fn organization_projects_add(
    req: HttpRequest,
    info: web::Path<(String,)>,
    project_info: web::Json<OrganizationProjectAdd>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let info = info.into_inner().0;
    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE, Scopes::ORGANIZATION_WRITE]),
    )
    .await?
    .1;

    let organization =
        database::models::Organization::get(&info, &**pool, &redis)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidInput("指定的组织不存在！".to_string())
            })?;

    let project_item = database::models::Project::get(
        &project_info.project_id,
        &**pool,
        &redis,
    )
    .await?
    .ok_or_else(|| ApiError::InvalidInput("指定的项目不存在！".to_string()))?;
    if project_item.inner.organization_id.is_some() {
        return Err(ApiError::InvalidInput(
            "指定的项目已由组织拥有！".to_string(),
        ));
    }

    let project_team_member =
        database::models::TeamMember::get_from_user_id_project(
            project_item.inner.id,
            current_user.id.into(),
            false,
            &**pool,
        )
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("您不是此项目的成员！".to_string())
        })?;
    let organization_team_member =
        database::models::TeamMember::get_from_user_id_organization(
            organization.id,
            current_user.id.into(),
            false,
            &**pool,
        )
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("您不是此组织的成员！".to_string())
        })?;

    // 需要项目所有者身份才能将其添加到组织中
    if !current_user.role.is_admin() && !project_team_member.is_owner {
        return Err(ApiError::CustomAuthentication(
            "您需要是项目的所有者才能将其添加到组织中！".to_string(),
        ));
    }

    let permissions = OrganizationPermissions::get_permissions_by_role(
        &current_user.role,
        &Some(organization_team_member),
    )
    .unwrap_or_default();
    if permissions.contains(OrganizationPermissions::ADD_PROJECT) {
        let mut transaction = pool.begin().await?;
        sqlx::query!(
            "
            UPDATE mods
            SET organization_id = $1
            WHERE (id = $2)
            ",
            organization.id as database::models::OrganizationId,
            project_item.inner.id as database::models::ids::ProjectId
        )
        .execute(&mut *transaction)
        .await?;

        // 原来的所有者不再是所有者（因为它现在是组织的，'给予'给他们）
        // 原来的所有者仍然是项目的成员，但不再是所有者
        // 当后来从组织中移除时，项目将由指定为新所有者的人拥有

        let organization_owner_user_id = sqlx::query!(
            "
            SELECT u.id 
            FROM team_members
            INNER JOIN users u ON u.id = team_members.user_id
            WHERE team_id = $1 AND is_owner = TRUE
            ",
            organization.team_id as database::models::ids::TeamId
        )
        .fetch_one(&mut *transaction)
        .await?;
        let organization_owner_user_id =
            database::models::ids::UserId(organization_owner_user_id.id);

        sqlx::query!(
            "
            DELETE FROM team_members
            WHERE team_id = $1 AND (is_owner = TRUE OR user_id = $2)
            ",
            project_item.inner.team_id as database::models::ids::TeamId,
            organization_owner_user_id as database::models::ids::UserId,
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        database::models::User::clear_project_cache(
            &[current_user.id.into()],
            &redis,
        )
        .await?;
        database::models::TeamMember::clear_cache(
            project_item.inner.team_id,
            &redis,
        )
        .await?;
        database::models::Project::clear_cache(
            project_item.inner.id,
            project_item.inner.slug,
            None,
            &redis,
        )
        .await?;
    } else {
        return Err(ApiError::CustomAuthentication(
            "您没有权限将项目添加到此组织中！".to_string(),
        ));
    }
    Ok(HttpResponse::Ok().finish())
}

#[derive(Deserialize)]
pub struct OrganizationProjectRemoval {
    // 必须提供一个新所有者。
    // 该用户必须是组织的成员，但不一定必须是项目的成员。
    pub new_owner: UserId,
}

pub async fn organization_projects_remove(
    req: HttpRequest,
    info: web::Path<(String, String)>,
    pool: web::Data<PgPool>,
    data: web::Json<OrganizationProjectRemoval>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (organization_id, project_id) = info.into_inner();
    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE, Scopes::ORGANIZATION_WRITE]),
    )
    .await?
    .1;

    let organization =
        database::models::Organization::get(&organization_id, &**pool, &redis)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidInput("指定的组织不存在！".to_string())
            })?;

    let project_item =
        database::models::Project::get(&project_id, &**pool, &redis)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidInput("指定的项目不存在！".to_string())
            })?;

    if !project_item
        .inner
        .organization_id
        .eq(&Some(organization.id))
    {
        return Err(ApiError::InvalidInput(
            "指定的项目不属于此组织！".to_string(),
        ));
    }

    let organization_team_member =
        database::models::TeamMember::get_from_user_id_organization(
            organization.id,
            current_user.id.into(),
            false,
            &**pool,
        )
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("您不是此组织的成员！".to_string())
        })?;

    let permissions = OrganizationPermissions::get_permissions_by_role(
        &current_user.role,
        &Some(organization_team_member),
    )
    .unwrap_or_default();
    if permissions.contains(OrganizationPermissions::REMOVE_PROJECT) {
        // 权限已确认，接下来验证新用户是否为组织成员
        database::models::TeamMember::get_from_user_id_organization(
            organization.id,
            data.new_owner.into(),
            false,
            &**pool,
        )
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("指定的用户不是此组织的成员！".to_string())
        })?;

        // 然后，我们获取项目的团队成员和该用户（如果存在）
        // 我们直接使用团队成员获取
        let new_owner = database::models::TeamMember::get_from_user_id_project(
            project_item.inner.id,
            data.new_owner.into(),
            true,
            &**pool,
        )
        .await?;

        let mut transaction = pool.begin().await?;

        // 如果用户不是项目的成员，我们添加他们
        let new_owner = match new_owner {
            Some(new_owner) => new_owner,
            None => {
                let new_id =
                    crate::database::models::ids::generate_team_member_id(
                        &mut transaction,
                    )
                    .await?;
                let member = TeamMember {
                    id: new_id,
                    team_id: project_item.inner.team_id,
                    user_id: data.new_owner.into(),
                    role: "Inherited Owner".to_string(),
                    is_owner: false,
                    permissions: ProjectPermissions::all(),
                    organization_permissions: None,
                    accepted: true,
                    payouts_split: Decimal::ZERO,
                    ordering: 0,
                };
                member.insert(&mut transaction).await?;
                member
            }
        };

        // 将新所有者设置为所有者
        sqlx::query!(
            "
            UPDATE team_members
            SET 
                is_owner = TRUE,
                accepted = TRUE,
                permissions = $2,
                organization_permissions = NULL,
                role = '继承所有者'
            WHERE (id = $1)
            ",
            new_owner.id as database::models::ids::TeamMemberId,
            ProjectPermissions::all().bits() as i64
        )
        .execute(&mut *transaction)
        .await?;

        sqlx::query!(
            "
            UPDATE mods
            SET organization_id = NULL
            WHERE (id = $1)
            ",
            project_item.inner.id as database::models::ids::ProjectId
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        database::models::User::clear_project_cache(
            &[current_user.id.into()],
            &redis,
        )
        .await?;
        database::models::TeamMember::clear_cache(
            project_item.inner.team_id,
            &redis,
        )
        .await?;
        database::models::Project::clear_cache(
            project_item.inner.id,
            project_item.inner.slug,
            None,
            &redis,
        )
        .await?;
    } else {
        return Err(ApiError::CustomAuthentication(
            "您没有权限从此组织中移除项目！".to_string(),
        ));
    }
    Ok(HttpResponse::Ok().finish())
}

#[derive(Serialize, Deserialize)]
pub struct Extension {
    pub ext: String,
}

#[allow(clippy::too_many_arguments)]
pub async fn organization_icon_edit(
    web::Query(ext): web::Query<Extension>,
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
    mut payload: web::Payload,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::ORGANIZATION_WRITE]),
    )
    .await?
    .1;
    let string = info.into_inner().0;

    let organization_item =
        database::models::Organization::get(&string, &**pool, &redis)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidInput("指定的组织不存在！".to_string())
            })?;

    if !user.role.is_mod() {
        let team_member = database::models::TeamMember::get_from_user_id(
            organization_item.team_id,
            user.id.into(),
            &**pool,
        )
        .await
        .map_err(ApiError::Database)?;

        let permissions = OrganizationPermissions::get_permissions_by_role(
            &user.role,
            &team_member,
        )
        .unwrap_or_default();

        if !permissions.contains(OrganizationPermissions::EDIT_DETAILS) {
            return Err(ApiError::CustomAuthentication(
                "您没有权限编辑此组织的图标！".to_string(),
            ));
        }
    }

    delete_old_images(
        organization_item.icon_url,
        organization_item.raw_icon_url,
        &***file_host,
    )
    .await?;

    let bytes =
        read_from_payload(&mut payload, 262144, "图标必须小于256KiB").await?;

    let organization_id: OrganizationId = organization_item.id.into();
    let upload_result = crate::util::img::upload_image_optimized(
        &format!("data/{}", organization_id),
        bytes.freeze(),
        &ext.ext,
        Some(96),
        Some(1.0),
        &***file_host,
        crate::util::img::UploadImagePos {
            pos: "团队头像".to_string(),
            url: format!("/organization/{}", organization_id),
            username: user.username.clone(),
        },
        &redis,
        false,
    )
    .await?;

    let mut transaction = pool.begin().await?;

    sqlx::query!(
        "
        UPDATE organizations
        SET icon_url = $1, raw_icon_url = $2, color = $3
        WHERE (id = $4)
        ",
        upload_result.url,
        upload_result.raw_url,
        upload_result.color.map(|x| x as i32),
        organization_item.id as database::models::ids::OrganizationId,
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;
    database::models::Organization::clear_cache(
        organization_item.id,
        Some(organization_item.slug),
        &redis,
    )
    .await?;

    Ok(HttpResponse::NoContent().body(""))
}

pub async fn delete_organization_icon(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::ORGANIZATION_WRITE]),
    )
    .await?
    .1;
    let string = info.into_inner().0;

    let organization_item =
        database::models::Organization::get(&string, &**pool, &redis)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidInput("指定的组织不存在！".to_string())
            })?;

    if !user.role.is_mod() {
        let team_member = database::models::TeamMember::get_from_user_id(
            organization_item.team_id,
            user.id.into(),
            &**pool,
        )
        .await
        .map_err(ApiError::Database)?;

        let permissions = OrganizationPermissions::get_permissions_by_role(
            &user.role,
            &team_member,
        )
        .unwrap_or_default();

        if !permissions.contains(OrganizationPermissions::EDIT_DETAILS) {
            return Err(ApiError::CustomAuthentication(
                "您没有权限编辑此组织的图标！".to_string(),
            ));
        }
    }

    delete_old_images(
        organization_item.icon_url,
        organization_item.raw_icon_url,
        &***file_host,
    )
    .await?;

    let mut transaction = pool.begin().await?;

    sqlx::query!(
        "
        UPDATE organizations
        SET icon_url = NULL, raw_icon_url = NULL, color = NULL
        WHERE (id = $1)
        ",
        organization_item.id as database::models::ids::OrganizationId,
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    database::models::Organization::clear_cache(
        organization_item.id,
        Some(organization_item.slug),
        &redis,
    )
    .await?;

    Ok(HttpResponse::NoContent().body(""))
}
