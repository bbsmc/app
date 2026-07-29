use crate::auth::get_user_from_headers;
use crate::database::models::UserId as DBUserId;
use crate::database::models::ids::ProjectId as DBProjectId;
use crate::database::models::thread_item::{
    ThreadBuilder, ThreadMessageBuilder,
};
use crate::database::models::{self};
use crate::database::redis::RedisPool;
use crate::models::ids::base62_impl::{parse_base62, to_base62};
use crate::models::pats::Scopes;
use crate::models::projects::ProjectStatus;
use crate::models::teams::ProjectPermissions;
use crate::models::threads::{MessageBody, ThreadType};
use crate::queue::incentive::{
    current_project_payout_members, payable_members,
};
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use actix_web::{HttpRequest, HttpResponse, web};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub fn config(cfg: &mut web::ServiceConfig) {
    // 注意：`project/{id}/incentive/*` 三条路由不在这里注册，
    // 已合并进 projects::config 的 `web::scope("project")`，避免被外层 scope 拦截 404。
    cfg.service(
        web::scope("dashboard/incentive")
            .route("", web::get().to(my_incentive_overview))
            .route("{id}", web::get().to(project_incentive_detail)),
    );
}

#[derive(Serialize)]
pub struct IncentiveSummary {
    pub pending_amount: Decimal,
    pub settled_amount: Decimal,
    pub project_count: i64,
}

#[derive(Serialize)]
pub struct IncentiveProjectRow {
    pub project_id: String,
    pub title: String,
    pub slug: Option<String>,
    pub incentive_enabled: bool,
    pub lifetime_eff_downloads: i64,
    pub current_unit_payout: Decimal,
    pub next_tier_remaining: Option<i64>,
    pub your_split_pct: Decimal,
    pub your_pending: Decimal,
    pub your_settled: Decimal,
}

#[derive(Serialize)]
pub struct IncentiveOverview {
    pub summary: IncentiveSummary,
    pub projects: Vec<IncentiveProjectRow>,
}

/// GET /v3/dashboard/incentive — 当前用户旗下所有项目的激励总览
pub async fn my_incentive_overview(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?
    .1;

    let rows = sqlx::query!(
        r#"
        SELECT DISTINCT m.id AS project_id, m.name AS title, m.slug AS slug,
               EXISTS(SELECT 1 FROM incentive_enabled_projects e WHERE e.project_id = m.id) AS "enabled!",
               c.lifetime_eff_downloads AS "lifetime_eff_downloads?"
        FROM mods m
        LEFT JOIN organizations o ON m.organization_id = o.id
        JOIN team_members tm ON tm.accepted = TRUE
          AND tm.user_id = $1
          AND (tm.team_id = m.team_id OR tm.team_id = o.team_id)
        LEFT JOIN incentive_project_counters c ON c.project_id = m.id
        ORDER BY m.name
        "#,
        user.id.0 as i64,
    )
    .fetch_all(pool.as_ref())
    .await?;

    let mut projects: Vec<IncentiveProjectRow> = Vec::with_capacity(rows.len());
    let mut total_pending = Decimal::ZERO;
    let mut total_settled = Decimal::ZERO;

    for r in rows {
        let lifetime = r.lifetime_eff_downloads.unwrap_or(0);

        let Some(project_payout) =
            current_project_payout_members(pool.as_ref(), r.project_id).await?
        else {
            continue;
        };
        let members = payable_members(project_payout.members);
        let total_split: f64 = members.iter().map(|m| m.split).sum();
        let Some(member) =
            members.iter().find(|m| m.user_id == user.id.0 as i64)
        else {
            continue;
        };
        if total_split <= 0.0 {
            continue;
        }

        let frac_dec =
            Decimal::from_f64(member.split / total_split).unwrap_or_default();
        let effective_split =
            Decimal::from_f64(member.split).unwrap_or_default();

        // 项目 pending / settled 总额
        let amounts = sqlx::query!(
            "
            SELECT pending_amount, settled_amount
            FROM incentive_project_counters WHERE project_id = $1
            ",
            r.project_id,
        )
        .fetch_optional(pool.as_ref())
        .await?;
        let (proj_pending, proj_settled) = amounts
            .map(|a| (a.pending_amount, a.settled_amount))
            .unwrap_or((Decimal::ZERO, Decimal::ZERO));

        let your_pending = proj_pending * frac_dec;
        let your_settled = proj_settled * frac_dec;
        total_pending += your_pending;
        total_settled += your_settled;

        let (current_unit, next_remaining) = current_tier(lifetime);

        projects.push(IncentiveProjectRow {
            project_id: to_base62(r.project_id as u64),
            title: r.title,
            slug: r.slug,
            incentive_enabled: r.enabled,
            lifetime_eff_downloads: lifetime,
            current_unit_payout: current_unit,
            next_tier_remaining: next_remaining,
            your_split_pct: effective_split,
            your_pending,
            your_settled,
        });
    }

    let project_count = projects.len() as i64;
    Ok(HttpResponse::Ok().json(IncentiveOverview {
        summary: IncentiveSummary {
            pending_amount: total_pending,
            settled_amount: total_settled,
            project_count,
        },
        projects,
    }))
}

#[derive(Serialize)]
pub struct DailyPoint {
    pub date: chrono::NaiveDate,
    pub effective_downloads: i64,
    pub daily_amount: Decimal,
}

#[derive(Serialize)]
pub struct ViewerCapability {
    /// 当前查看者是站点 admin（admin 身份仅提供全局读取能力）
    pub is_admin: bool,
    /// 是项目直属团队或所属组织的 accepted 成员
    pub is_team_member: bool,
    /// 有 EDIT_DETAILS 权限（可申请/撤回）
    pub can_manage: bool,
}

#[derive(Serialize)]
pub struct ProjectIncentiveDetail {
    pub project_id: String,
    pub title: String,
    pub incentive_enabled: bool,
    pub lifetime_eff_downloads: i64,
    pub pending_amount: Decimal,
    pub settled_amount: Decimal,
    pub voided_amount: Decimal,
    pub current_unit_payout: Decimal,
    pub next_tier_remaining: Option<i64>,
    pub last_30_days: Vec<DailyPoint>,
    pub viewer: ViewerCapability,
}

/// GET /v3/dashboard/incentive/{id} — 单项目激励详情（含财务数据，要求 EDIT_DETAILS）
pub async fn project_incentive_detail(
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
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?
    .1;

    let project_id = parse_base62(&info.0)
        .map_err(|_| ApiError::InvalidInput("无效的项目 ID".to_string()))?
        as i64;

    // admin 始终可读；若 admin 同时也是项目/组织成员，仍返回真实成员能力。
    let member_permissions = find_member_permissions(
        project_id,
        user.id.0 as i64,
        pool.as_ref(),
        &redis,
    )
    .await?;
    let viewer = viewer_capability(user.role.is_admin(), member_permissions);
    let viewer_can_view_payouts =
        member_permissions.is_some_and(|permissions| {
            permissions.contains(ProjectPermissions::VIEW_PAYOUTS)
        });

    // 激励详情包含财务数据，非 admin 需要项目内可管理或可查看收益权限。
    if !(viewer.is_admin || viewer.can_manage || viewer_can_view_payouts) {
        return Err(ApiError::CustomAuthentication(
            "你没有查看项目激励的权限".to_string(),
        ));
    }

    let proj = sqlx::query!("SELECT name FROM mods WHERE id = $1", project_id)
        .fetch_optional(pool.as_ref())
        .await?
        .ok_or(ApiError::NotFound)?;

    let enabled = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM incentive_enabled_projects WHERE project_id = $1) AS "exists!""#,
        project_id,
    )
    .fetch_one(pool.as_ref())
    .await?;

    let counter = sqlx::query!(
        "
        SELECT lifetime_eff_downloads, pending_amount, settled_amount, voided_amount
        FROM incentive_project_counters WHERE project_id = $1
        ",
        project_id,
    )
    .fetch_optional(pool.as_ref())
    .await?;

    let (lifetime_eff_downloads, pending_amount, settled_amount, voided_amount) =
        match counter {
            Some(c) => (
                c.lifetime_eff_downloads,
                c.pending_amount,
                c.settled_amount,
                c.voided_amount,
            ),
            None => (0, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO),
        };

    // 按应用时区分日聚合，PG session 已设为 Asia/Shanghai
    let daily_rows = sqlx::query!(
        r#"
        SELECT DATE(recorded_at) AS "date!",
               COUNT(*)::bigint AS "effective_downloads!",
               COALESCE(SUM(payout_amount), 0)::numeric AS "daily_amount!"
        FROM incentive_download_events
        WHERE project_id = $1 AND recorded_at >= NOW() - INTERVAL '30 days'
        GROUP BY DATE(recorded_at)
        ORDER BY DATE(recorded_at)
        "#,
        project_id,
    )
    .fetch_all(pool.as_ref())
    .await?;

    let daily: Vec<DailyPoint> = daily_rows
        .into_iter()
        .map(|r| DailyPoint {
            date: r.date,
            effective_downloads: r.effective_downloads,
            daily_amount: r.daily_amount,
        })
        .collect();

    let (current_unit, next_remaining) = current_tier(lifetime_eff_downloads);

    Ok(HttpResponse::Ok().json(ProjectIncentiveDetail {
        project_id: to_base62(project_id as u64),
        title: proj.name,
        incentive_enabled: enabled,
        lifetime_eff_downloads,
        pending_amount,
        settled_amount,
        voided_amount,
        current_unit_payout: current_unit,
        next_tier_remaining: next_remaining,
        last_30_days: daily,
        viewer,
    }))
}

/// 当前档位单价 + 距离下一档位还差多少（最高档返回 None）
fn current_tier(lifetime: i64) -> (Decimal, Option<i64>) {
    use std::str::FromStr;
    if lifetime < 1000 {
        (Decimal::from_str("0.02").unwrap(), Some(1000 - lifetime))
    } else if lifetime < 10000 {
        (Decimal::from_str("0.01").unwrap(), Some(10000 - lifetime))
    } else {
        (Decimal::from_str("0.008").unwrap(), None)
    }
}

// ==================== 申请开通激励（作者侧）====================

#[derive(Deserialize)]
pub struct ApplyBody {
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct ApplicationView {
    pub id: i64,
    pub project_id: String,
    pub applicant_user_id: String,
    pub reason: Option<String>,
    pub status: String,
    pub review_notes: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub thread_id: Option<String>,
}

/// 选择用户在项目激励流程中的有效成员权限。
/// 项目直属成员的权限优先于所属组织成员的默认项目权限。
fn effective_member_permissions(
    project_member: Option<(bool, ProjectPermissions)>,
    organization_member: Option<(bool, ProjectPermissions)>,
) -> Option<ProjectPermissions> {
    project_member
        .filter(|(accepted, _)| *accepted)
        .map(|(_, permissions)| permissions)
        .or_else(|| {
            organization_member
                .filter(|(accepted, _)| *accepted)
                .map(|(_, permissions)| permissions)
        })
}

async fn get_member_permissions(
    project_id: i64,
    user_id: i64,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<ProjectPermissions, ApiError> {
    find_member_permissions(project_id, user_id, pool, redis)
        .await?
        .ok_or_else(|| {
            ApiError::CustomAuthentication("你不是该项目的成员".to_string())
        })
}

async fn find_member_permissions(
    project_id: i64,
    user_id: i64,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<Option<ProjectPermissions>, ApiError> {
    let project = models::Project::get_id(DBProjectId(project_id), pool, redis)
        .await?
        .ok_or(ApiError::NotFound)?;

    let (project_member, organization_member) =
        models::TeamMember::get_for_project_permissions(
            &project.inner,
            DBUserId(user_id),
            pool,
        )
        .await?;

    Ok(effective_member_permissions(
        project_member.map(|member| (member.accepted, member.permissions)),
        organization_member.map(|member| (member.accepted, member.permissions)),
    ))
}

fn viewer_capability(
    is_admin: bool,
    member_permissions: Option<ProjectPermissions>,
) -> ViewerCapability {
    ViewerCapability {
        is_admin,
        is_team_member: member_permissions.is_some(),
        can_manage: member_permissions.is_some_and(|permissions| {
            permissions.contains(ProjectPermissions::EDIT_DETAILS)
        }),
    }
}

async fn ensure_member_with_permission(
    project_id: i64,
    user_id: i64,
    require: Option<ProjectPermissions>,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<(), ApiError> {
    let permissions =
        get_member_permissions(project_id, user_id, pool, redis).await?;

    if let Some(required) = require
        && !permissions.contains(required)
    {
        return Err(ApiError::CustomAuthentication(
            "你没有执行此操作的权限".to_string(),
        ));
    }

    Ok(())
}

async fn ensure_member_can_view_incentive(
    project_id: i64,
    user_id: i64,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<(), ApiError> {
    let permissions =
        get_member_permissions(project_id, user_id, pool, redis).await?;

    if !permissions.contains(ProjectPermissions::EDIT_DETAILS)
        && !permissions.contains(ProjectPermissions::VIEW_PAYOUTS)
    {
        return Err(ApiError::CustomAuthentication(
            "你没有查看项目激励的权限".to_string(),
        ));
    }

    Ok(())
}

/// POST /v3/project/{id}/incentive/apply
pub async fn apply_incentive(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    body: web::Json<ApplyBody>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await?
    .1;

    let project_id = parse_base62(&info.0)
        .map_err(|_| ApiError::InvalidInput("无效的项目 ID".to_string()))?
        as i64;

    // 申请属于"修改项目"，要求 EDIT_DETAILS
    ensure_member_with_permission(
        project_id,
        user.id.0 as i64,
        Some(ProjectPermissions::EDIT_DETAILS),
        pool.as_ref(),
        &redis,
    )
    .await?;

    let project_status = sqlx::query_scalar!(
        "
        SELECT status
        FROM mods
        WHERE id = $1
        ",
        project_id,
    )
    .fetch_optional(pool.as_ref())
    .await?
    .ok_or(ApiError::NotFound)?;

    if !ProjectStatus::from_string(&project_status).is_approved() {
        return Err(ApiError::InvalidInput(
            "资源通过审核后才能申请创作者激励".to_string(),
        ));
    }

    // 已开通则不允许重复申请
    let already = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM incentive_enabled_projects WHERE project_id = $1) AS "exists!""#,
        project_id,
    )
    .fetch_one(pool.as_ref())
    .await?;
    if already {
        return Err(ApiError::InvalidInput("该项目已开通激励".to_string()));
    }

    // 检查是否已有 pending 申请
    let pending_exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM incentive_applications WHERE project_id = $1 AND status = 'pending') AS "exists!""#,
        project_id,
    )
    .fetch_one(pool.as_ref())
    .await?;
    if pending_exists {
        return Err(ApiError::InvalidInput("已有待审核申请".to_string()));
    }

    let mut tx = pool.begin().await?;

    let db_user_id = DBUserId(user.id.0 as i64);

    // 1. 创建 thread
    let thread_id = ThreadBuilder {
        type_: ThreadType::IncentiveApplication,
        members: vec![db_user_id],
        project_id: None,
        report_id: None,
        ban_appeal_id: None,
        creator_application_id: None,
    }
    .insert(&mut tx)
    .await?;

    // 2. 申请理由作为首条消息
    let initial_text = body
        .reason
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "（未填写申请理由）".to_string());

    ThreadMessageBuilder {
        author_id: Some(db_user_id),
        body: MessageBody::Text {
            body: initial_text,
            private: false,
            replying_to: None,
            associated_images: vec![],
        },
        thread_id,
        hide_identity: false,
    }
    .insert(&mut tx)
    .await?;

    // 3. 写入申请
    sqlx::query!(
        "
        INSERT INTO incentive_applications
            (project_id, applicant_user_id, reason, status, thread_id)
        VALUES ($1, $2, $3, 'pending', $4)
        ",
        project_id,
        user.id.0 as i64,
        body.reason.as_deref(),
        thread_id.0,
    )
    .execute(&mut *tx)
    .await?;

    // 不群发版主通知，待审申请会显示在 moderation 后台

    tx.commit().await?;

    let _ = crate::queue::incentive::audit_log(
        pool.as_ref(),
        Some(user.id.0 as i64),
        "apply",
        "project",
        project_id,
        body.reason
            .as_ref()
            .map(|r| serde_json::json!({"reason": r})),
    )
    .await;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    Ok(HttpResponse::NoContent().finish())
}

/// GET /v3/project/{id}/incentive/application
/// 返回最新一条申请（不论状态）
pub async fn get_application(
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
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?
    .1;

    let project_id = parse_base62(&info.0)
        .map_err(|_| ApiError::InvalidInput("无效的项目 ID".to_string()))?
        as i64;

    // admin 全局可读；其他用户要求项目内可管理或可查看收益权限
    if !user.role.is_admin() {
        ensure_member_can_view_incentive(
            project_id,
            user.id.0 as i64,
            pool.as_ref(),
            &redis,
        )
        .await?;
    }

    let row = sqlx::query!(
        "
        SELECT id, project_id, applicant_user_id, reason, status, review_notes,
               reviewed_by, reviewed_at, created_at, thread_id
        FROM incentive_applications
        WHERE project_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        ",
        project_id,
    )
    .fetch_optional(pool.as_ref())
    .await?;

    match row {
        Some(r) => Ok(HttpResponse::Ok().json(ApplicationView {
            id: r.id,
            project_id: to_base62(r.project_id as u64),
            applicant_user_id: to_base62(r.applicant_user_id as u64),
            reason: r.reason,
            status: r.status,
            review_notes: r.review_notes,
            reviewed_by: r.reviewed_by.map(|v| to_base62(v as u64)),
            reviewed_at: r.reviewed_at,
            created_at: r.created_at,
            thread_id: r.thread_id.map(|v| to_base62(v as u64)),
        })),
        None => Ok(HttpResponse::NoContent().finish()),
    }
}

/// DELETE /v3/project/{id}/incentive/application — 撤回 pending 申请
pub async fn withdraw_application(
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
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await?
    .1;

    let project_id = parse_base62(&info.0)
        .map_err(|_| ApiError::InvalidInput("无效的项目 ID".to_string()))?
        as i64;

    // 撤回属于"修改项目"，要求 EDIT_DETAILS
    ensure_member_with_permission(
        project_id,
        user.id.0 as i64,
        Some(ProjectPermissions::EDIT_DETAILS),
        pool.as_ref(),
        &redis,
    )
    .await?;

    let mut tx = pool.begin().await?;

    let pending = sqlx::query!(
        "
        SELECT id, thread_id FROM incentive_applications
        WHERE project_id = $1 AND status = 'pending' FOR UPDATE
        ",
        project_id,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::InvalidInput("没有待审核的申请可撤回".to_string())
    })?;

    sqlx::query!(
        "
        UPDATE incentive_applications
        SET status = 'withdrawn', reviewed_at = NOW()
        WHERE id = $1
        ",
        pending.id,
    )
    .execute(&mut *tx)
    .await?;

    // thread 写一条系统消息（author_id = NULL）
    if let Some(tid) = pending.thread_id {
        ThreadMessageBuilder {
            author_id: None,
            body: MessageBody::Text {
                body: "申请人撤回了该申请".to_string(),
                private: false,
                replying_to: None,
                associated_images: vec![],
            },
            thread_id: crate::database::models::ids::ThreadId(tid),
            hide_identity: false,
        }
        .insert(&mut tx)
        .await?;
    }

    tx.commit().await?;

    let _ = crate::queue::incentive::audit_log(
        pool.as_ref(),
        Some(user.id.0 as i64),
        "withdraw",
        "application",
        pending.id,
        Some(serde_json::json!({"project_id": project_id})),
    )
    .await;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use super::{effective_member_permissions, viewer_capability};
    use crate::models::teams::ProjectPermissions;

    #[test]
    fn organization_member_permissions_are_used_without_project_override() {
        let permissions = effective_member_permissions(
            None,
            Some((true, ProjectPermissions::EDIT_DETAILS)),
        );

        assert_eq!(permissions, Some(ProjectPermissions::EDIT_DETAILS));
    }

    #[test]
    fn accepted_project_member_permissions_override_organization_defaults() {
        let permissions = effective_member_permissions(
            Some((true, ProjectPermissions::empty())),
            Some((true, ProjectPermissions::EDIT_DETAILS)),
        );

        assert_eq!(permissions, Some(ProjectPermissions::empty()));
    }

    #[test]
    fn pending_project_invite_does_not_override_accepted_organization_member() {
        let permissions = effective_member_permissions(
            Some((false, ProjectPermissions::empty())),
            Some((true, ProjectPermissions::VIEW_PAYOUTS)),
        );

        assert_eq!(permissions, Some(ProjectPermissions::VIEW_PAYOUTS));
    }

    #[test]
    fn admin_member_keeps_real_project_capabilities() {
        let viewer =
            viewer_capability(true, Some(ProjectPermissions::EDIT_DETAILS));

        assert!(viewer.is_admin);
        assert!(viewer.is_team_member);
        assert!(viewer.can_manage);
    }

    #[test]
    fn admin_organization_member_keeps_default_project_capabilities() {
        let permissions = effective_member_permissions(
            None,
            Some((true, ProjectPermissions::EDIT_DETAILS)),
        );
        let viewer = viewer_capability(true, permissions);

        assert!(viewer.is_admin);
        assert!(viewer.is_team_member);
        assert!(viewer.can_manage);
    }

    #[test]
    fn admin_without_membership_stays_global_read_only() {
        let viewer = viewer_capability(true, None);

        assert!(viewer.is_admin);
        assert!(!viewer.is_team_member);
        assert!(!viewer.can_manage);
    }
}
