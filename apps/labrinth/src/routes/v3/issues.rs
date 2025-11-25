use crate::auth::email::send_email;
use crate::auth::{AuthenticationError, get_user_from_headers};
use crate::database::models::ids::{
    IssuesCommentsId, IssuesId, generate_issues_comments_id, generate_issues_id,
};
use crate::database::models::issues::{
    ISSUE_NAMESPACE, Issue, IssueCommentBuilder, IssueCommentQuery, IssueLabel,
};
use crate::database::models::{ProjectId, UserId};
use crate::database::redis::RedisPool;
use crate::database::{self, models};
use crate::models::ids::base62_impl::parse_base62;
use crate::models::pats::Scopes;
use crate::models::teams::ProjectPermissions;
use crate::queue::session::AuthQueue;
use crate::{
    models::v3::issues::{
        CommentResponse, CommentsQueryParams, CreateCommentRequest,
        CreateIssueRequest, IssueResponse, IssuesQueryParams, LabelResponse,
        UpdateIssueRequest,
    },
    routes::ApiError,
};
use actix_web::{HttpRequest, HttpResponse, web};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("issues")
            .route("labels", web::get().to(labels_get))
            .route("{id}", web::get().to(issue_get))
            // .route("{id}", web::delete().to(issue_delete))
            .route("{id}", web::patch().to(issue_edit))
            .route("{id}/comments", web::get().to(comments_get))
            .route("{id}/comments", web::post().to(comment_create))
            // .route("comments/{comment_id}", web::patch().to(comment_edit))
            .route("comments/{comment_id}", web::delete().to(comment_delete))
            .service(
                web::scope("project/{project_id}")
                    .route("", web::get().to(project_issues_list))
                    .route("", web::post().to(project_issue_create)),
            ),
    );
}

// 获取项目的Issues
pub async fn project_issues_list(
    _req: HttpRequest,
    info: web::Path<(String,)>,
    query: web::Query<IssuesQueryParams>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    _session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let project_id_str: String = info.into_inner().0;
    let project_id = ProjectId(parse_base62(&project_id_str)? as i64);

    let params = query.into_inner();
    let page_size = params.page_size.unwrap_or(20) as i64;
    let page = params.page.unwrap_or(1) as i64;

    let mut exec = pool.acquire().await?;

    let issues =
        Issue::get_project_issues(project_id, params.state, &mut *exec, &redis)
            .await?;

    let total = issues.len();
    let offset = ((page - 1) * page_size) as usize;

    let issues = issues
        .into_iter()
        .skip(offset)
        .take(page_size as usize)
        .map(|x| x.0)
        .collect::<Vec<_>>();

    let issues = Issue::get_many(&issues, &mut *exec, &redis).await?;
    let mut issues: Vec<IssueResponse> =
        issues.into_iter().map(|x| x.into()).collect::<Vec<_>>();

    // 按创建时间排序，最新的在前面
    issues.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(HttpResponse::Ok().json(json!({
        "issues": issues,
        "pagination": {
            "total": total
        }
    })))
}

// 获取单个Issue
pub async fn issue_get(
    _req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let issue_id_str: String = info.into_inner().0;
    let issue_id = IssuesId(parse_base62(&issue_id_str)? as i64);

    let issue = Issue::get_id(issue_id.0, &**pool, &redis).await?;

    if issue.is_none() {
        return Err(ApiError::NotFound);
    }

    let issue = issue.unwrap();
    let response: IssueResponse = issue.into();

    Ok(HttpResponse::Ok().json(json!(response)))
}

// 为项目创建Issue
pub async fn project_issue_create(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<CreateIssueRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let project_id_str: String = info.into_inner().0;
    let project_id = ProjectId(parse_base62(&project_id_str)? as i64);

    let user_option = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await
    .map(|x| x.1)
    .ok();

    if user_option.is_none() {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

    let user = user_option.unwrap();

    let result =
        database::models::Project::get(&project_id_str, &**pool, &redis)
            .await?;

    if result.is_none() {
        return Err(ApiError::NotFound);
    }

    let project = result.unwrap();

    // 检查用户是否绑定手机号
    if user.has_phonenumber.is_none() || !user.has_phonenumber.unwrap() {
        return Err(ApiError::InvalidInput(
            "请先绑定手机号，再创建问题".to_string(),
        ));
    }

    if body.title.is_empty() {
        return Err(ApiError::InvalidInput("请输入标题".to_string()));
    }

    if body.body.is_empty() {
        return Err(ApiError::InvalidInput("请输入内容".to_string()));
    }

    // 检查内容安全性
    let risk = true; // 暂时禁用内容安全检查
    if !risk {
        return Err(ApiError::InvalidInput(
            "内容包含敏感词，已被记录该次提交".to_string(),
        ));
    }

    let risk = true; // 暂时禁用内容安全检查
    if !risk {
        return Err(ApiError::InvalidInput(
            "标题包含敏感词，已被记录该次提交".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;
    let issue_id = generate_issues_id(&mut transaction).await?;

    let issue = Issue {
        id: issue_id,
        mod_id: project_id,
        title: body.title.clone(),
        body: body.body.clone(),
        state: "open".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        closed_at: None,
        author_id: UserId::from(user.id),
        author_name: user.username.clone(),
        author_avatar: user.avatar_url.clone(),
        locked: false,
        deleted: false,
        deleted_at: None,
        labels: Vec::new(),
        assignees: Vec::new(),
    };

    issue.insert(&mut transaction).await?;
    transaction.commit().await?;

    // 清除单个Issue的缓存
    Issue::clear_cache(&[issue_id], &redis).await?;

    // 清除项目Issues列表的缓存（因为新创建的issue会影响项目的issues列表）
    let mut redis_conn = redis.connect().await?;
    let cache_keys = vec![
        (
            ISSUE_NAMESPACE,
            Some(format!("project_{}_all", project_id.0)),
        ),
        (
            ISSUE_NAMESPACE,
            Some(format!("project_{}_open", project_id.0)),
        ),
        (
            ISSUE_NAMESPACE,
            Some(format!("project_{}_closed", project_id.0)),
        ),
    ];

    redis_conn.delete_many(cache_keys).await?;

    let id: crate::models::v3::issues::IssuesId = issue_id.into();

    // 发送邮件通知
    let users = project.inner.get_all_users(&**pool, &redis).await?;
    let users = models::User::get_many_ids(&users, &**pool, &redis).await?;
    // print username
    for user in users {
        send_email(
            user.email.unwrap(),
            "新问题通知",
            &format!(
                "{} 在 {} 创建了新问题：{}",
                user.username, project.inner.name, body.title
            ),
            &body.body,
            Some((
                "查看问题",
                &format!(
                    "{}/project/{}/issues/{}",
                    dotenvy::var("SITE_URL")?,
                    project.inner.slug.clone().unwrap(),
                    id
                ),
            )),
        )?;
    }

    Ok(HttpResponse::Ok().json(json!({
        "issue": id
    })))
}

// 编辑Issue
pub async fn issue_edit(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<UpdateIssueRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let issue_id_str: String = info.into_inner().0;
    let issue_id = IssuesId(parse_base62(&issue_id_str)? as i64);

    let user_option = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await
    .map(|x| x.1)
    .ok();

    if user_option.is_none() {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

    let user = user_option.unwrap();

    let issue = Issue::get_id(issue_id.0, &**pool, &redis).await?;

    if issue.is_none() {
        return Err(ApiError::NotFound);
    }

    let issue: database::models::issues::QueryIssue = issue.unwrap();

    let result =
        database::models::Project::get_id(issue.inner.mod_id, &**pool, &redis)
            .await?;

    if result.is_none() {
        return Err(ApiError::NotFound);
    }

    let project = result.unwrap();

    let (team_member, organization_team_member) =
        crate::database::models::TeamMember::get_for_project_permissions(
            &project.inner,
            UserId::from(user.id),
            &**pool,
        )
        .await?;

    let permissions = ProjectPermissions::get_permissions_by_role(
        &user.role,
        &team_member,
        &organization_team_member,
    );

    // // 检查权限
    // if issue.inner.author_id.0 != UserId::from(user.id).0
    //     && !user.role.is_admin()
    // {
    //     return Err(ApiError::InvalidInput("您没有权限修改此问题".to_string()));
    // }

    if issue.inner.locked && !user.role.is_admin() {
        return Err(ApiError::InvalidInput(
            "此问题已被锁定，无法修改".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;
    let mut has_changes = false;

    // 更新状态
    if let Some(state) = &body.state {
        // 如果新的状态为closed，则只有问题创建者和管理员可以修改
        if state == "closed"
            && permissions.is_none()
            && issue.inner.author_id.0 != UserId::from(user.id).0
        {
            return Err(ApiError::InvalidInput(
                "您没有权限关闭此问题".to_string(),
            ));
        }

        // 若新的状态为open， 则 只有 管理员可以打开
        if state == "open" && permissions.is_none() {
            return Err(ApiError::InvalidInput(
                "您没有权限重新打开此问题".to_string(),
            ));
        }

        if issue.inner.state != *state {
            issue
                .inner
                .update_state(state.clone(), &mut transaction)
                .await?;
            has_changes = true;

            // 发送邮件通知
            let mut users =
                project.inner.get_all_users(&**pool, &redis).await?;
            users.push(UserId(issue.inner.author_id.0));
            let users =
                models::User::get_many_ids(&users, &**pool, &redis).await?;
            for user in users {
                send_email(
                    user.email.unwrap(),
                    "问题状态更新通知",
                    &format!(
                        "{} 更新了 {} 的问题状态：{}",
                        user.username,
                        issue.inner.title.clone(),
                        if state == "open" {
                            "重新打开"
                        } else if state == "closed" {
                            "关闭"
                        } else {
                            "未知"
                        }
                    ),
                    "",
                    Some((
                        "查看问题",
                        &format!(
                            "{}/project/{}/issues/{}",
                            dotenvy::var("SITE_URL")?,
                            project.inner.slug.clone().unwrap(),
                            &issue_id_str
                        ),
                    )),
                )?;
            }
        }
    }

    // 更新标签
    if let Some(label_ids) = &body.labels {
        if permissions.is_none() {
            return Err(ApiError::InvalidInput(
                "您没有权限修改此问题的标签".to_string(),
            ));
        }

        // 获取当前标签ID列表进行比较
        let current_label_ids: Vec<i32> =
            issue.inner.labels.iter().map(|l| l.id).collect();
        let mut new_label_ids = label_ids.clone();
        new_label_ids.sort();
        let mut current_sorted = current_label_ids.clone();
        current_sorted.sort();

        if new_label_ids != current_sorted {
            issue
                .inner
                .update_labels(label_ids.clone(), &mut transaction)
                .await?;
            has_changes = true;
        }
    }

    if !has_changes {
        return Err(ApiError::InvalidInput("未做任何修改".to_string()));
    }

    transaction.commit().await?;

    // 清除单个Issue的缓存
    Issue::clear_cache(&[issue_id], &redis).await?;

    // 清除项目Issues列表的缓存（因为列表中也包含标签信息）
    let mut redis_conn = redis.connect().await?;
    let project_id = issue.inner.mod_id.0;
    let cache_keys = vec![
        (ISSUE_NAMESPACE, Some(format!("project_{}_all", project_id))),
        (
            ISSUE_NAMESPACE,
            Some(format!("project_{}_open", project_id)),
        ),
        (
            ISSUE_NAMESPACE,
            Some(format!("project_{}_closed", project_id)),
        ),
    ];

    redis_conn.delete_many(cache_keys).await?;

    Ok(HttpResponse::NoContent().finish())
}

// 删除Issue
// pub async fn issue_delete(
//     req: HttpRequest,
//     info: web::Path<(String,)>,
//     pool: web::Data<PgPool>,
//     redis: web::Data<RedisPool>,
//     session_queue: web::Data<AuthQueue>,
// ) -> Result<HttpResponse, ApiError> {
//     let issue_id_str: String = info.into_inner().0;
//     let issue_id = IssuesId(parse_base62(&issue_id_str)? as i64);

//     let user_option = get_user_from_headers(
//         &req,
//         &**pool,
//         &redis,
//         &session_queue,
//         Some(&[Scopes::PROJECT_WRITE]),
//     )
//     .await
//     .map(|x| x.1)
//     .ok();

//     if user_option.is_none() {
//         return Err(ApiError::Authentication(
//             AuthenticationError::InvalidCredentials,
//         ));
//     }

//     let user = user_option.unwrap();

//     let issue = Issue::get_id(issue_id.0, &**pool, &redis).await?;

//     if issue.is_none() {
//         return Err(ApiError::NotFound);
//     }

//     let issue = issue.unwrap();

//     // 检查权限
//     if issue.inner.author_id.0 != UserId::from(user.id).0
//         && !user.role.is_admin()
//     {
//         return Err(ApiError::InvalidInput("您没有权限删除此问题".to_string()));
//     }

//     let project_id = issue.inner.mod_id.0;

//     let mut transaction = pool.begin().await?;
//     issue.inner.delete_issue(&mut transaction).await?;
//     transaction.commit().await?;

//     // 清除单个Issue的缓存
//     Issue::clear_cache(&[issue_id], &redis).await?;

//     // 清除项目Issues列表的缓存（因为删除issue会影响项目的issues列表）
//     let mut redis_conn = redis.connect().await?;
//     let cache_keys = vec![
//         (ISSUE_NAMESPACE, Some(format!("project_{}_all", project_id))),
//         (
//             ISSUE_NAMESPACE,
//             Some(format!("project_{}_open", project_id)),
//         ),
//         (
//             ISSUE_NAMESPACE,
//             Some(format!("project_{}_closed", project_id)),
//         ),
//     ];

//     redis_conn.delete_many(cache_keys).await?;

//     Ok(HttpResponse::NoContent().finish())
// }

// 获取Issue评论
pub async fn comments_get(
    _req: HttpRequest,
    info: web::Path<(String,)>,
    query: web::Query<CommentsQueryParams>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let issue_id_str: String = info.into_inner().0;
    let issue_id = IssuesId(parse_base62(&issue_id_str)? as i64);

    let params = query.into_inner();
    let page_size = params.page_size.unwrap_or(20) as i64;
    let page = params.page.unwrap_or(1) as i64;

    let issue = Issue::get_id(issue_id.0, &**pool, &redis).await?;

    if issue.is_none() {
        return Err(ApiError::NotFound);
    }

    let issue = issue.unwrap();
    let comments = issue.comments.clone();

    // 过滤掉已删除的评论，但保持楼层号不变
    let mut valid_comment_ids = Vec::new();
    for comment_index in &comments {
        // 检查评论是否已删除
        let is_deleted = sqlx::query!(
            "SELECT deleted FROM issue_comments WHERE id = $1",
            comment_index.comment_id.0
        )
        .fetch_optional(&**pool)
        .await?;

        if let Some(row) = is_deleted
            && !row.deleted
        {
            valid_comment_ids.push(comment_index.clone());
        }
    }

    valid_comment_ids.sort_by(|a, b| a.floor_number.cmp(&b.floor_number));

    let total = valid_comment_ids.len();
    let offset = ((page - 1) * page_size) as usize;

    let comment_ids: Vec<i64> = valid_comment_ids
        .into_iter()
        .skip(offset)
        .take(page_size as usize)
        .map(|x| x.comment_id.0)
        .collect();

    let mut comments =
        IssueCommentQuery::get_many(&comment_ids, &issue_id, &**pool, &redis)
            .await?;

    // 按创建时间排序评论，确保返回顺序正确
    comments.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let comments: Vec<CommentResponse> =
        comments.into_iter().map(|x| x.into()).collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(json!({
        "comments": comments,
        "pagination": {
            "total": total
        }
    })))
}

// 创建评论
pub async fn comment_create(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<CreateCommentRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let issue_id_str: String = info.into_inner().0;
    let issue_id = IssuesId(parse_base62(&issue_id_str)? as i64);

    let user_option = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await
    .map(|x| x.1)
    .ok();

    if user_option.is_none() {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

    let user = user_option.unwrap();

    // 检查用户是否绑定手机号
    if user.has_phonenumber.is_none() || !user.has_phonenumber.unwrap() {
        return Err(ApiError::InvalidInput(
            "请先绑定手机号，再进行评论".to_string(),
        ));
    }

    let issue = Issue::get_id(issue_id.0, &**pool, &redis).await?;

    if issue.is_none() {
        return Err(ApiError::NotFound);
    }

    let issue = issue.unwrap();

    let result =
        database::models::Project::get_id(issue.inner.mod_id, &**pool, &redis)
            .await?;

    if result.is_none() {
        return Err(ApiError::NotFound);
    }
    let project = result.unwrap();

    // 检查Issue状态
    if issue.inner.locked && !user.role.is_admin() {
        return Err(ApiError::InvalidInput(
            "此问题已被锁定，无法评论".to_string(),
        ));
    }

    // 检查内容安全性
    let risk = true; // 暂时禁用内容安全检查
    if !risk {
        return Err(ApiError::InvalidInput(
            "回复内容包含敏感词，已被记录该次提交".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;
    let comment_id = generate_issues_comments_id(&mut transaction).await?;

    let comment = IssueCommentBuilder {
        id: comment_id,
        issue_id,
        author_id: UserId::from(user.id),
        body: body.body.clone(),
        comment_type: body.comment_type.clone().unwrap_or("reply".to_string()),
        reply_to_id: body.reply_to_id.map(|id| IssuesCommentsId(id.0 as i64)),
        created_at: Utc::now(),
    };

    comment.insert(&mut transaction).await?;
    transaction.commit().await?;

    Issue::clear_cache(&[issue_id], &redis).await?;

    let comments = IssueCommentQuery::get_many(
        &[comment_id.0],
        &issue_id,
        &**pool,
        &redis,
    )
    .await?;
    let comment_response: Vec<CommentResponse> =
        comments.into_iter().map(|x| x.into()).collect::<Vec<_>>();

    // 发送邮件通知
    // let mut users: Vec<UserId> = project.inner.get_all_users(&**pool, &redis).await?;
    let mut users: Vec<UserId> = vec![];
    if body.reply_to_id.is_some() {
        let reply_to_id = IssuesCommentsId(body.reply_to_id.unwrap().0 as i64);
        println!("reply_to_id: {:?}", reply_to_id);
        let reply_to_user =
            IssueCommentQuery::get_id(reply_to_id, &issue_id, &**pool, &redis)
                .await?;
        if let Some(reply_user) = reply_to_user {
            users.push(reply_user.author_id);
        }
    }
    users.push(user.id.into());
    let users = models::User::get_many_ids(&users, &**pool, &redis).await?;
    // print username
    for user in users {
        send_email(
            user.email.unwrap(),
            "问题收到新的回复通知",
            &format!(
                "{} 在 {} 回复了你的消息：{}",
                user.username, project.inner.name, body.body
            ),
            &body.body,
            Some((
                "查看问题",
                &format!(
                    "{}/project/{}/issues/{}",
                    dotenvy::var("SITE_URL")?,
                    project.inner.slug.clone().unwrap(),
                    &issue_id_str
                ),
            )),
        )?;
    }

    Ok(HttpResponse::Ok().json(json!({
        "comment": comment_response.first().unwrap()
    })))
}

// 编辑评论
// pub async fn comment_edit(
//     req: HttpRequest,
//     info: web::Path<(String,)>,
//     body: web::Json<UpdateCommentRequest>,
//     pool: web::Data<PgPool>,
//     redis: web::Data<RedisPool>,
//     session_queue: web::Data<AuthQueue>,
// ) -> Result<HttpResponse, ApiError> {
//     let comment_id_str: String = info.into_inner().0;
//     let comment_id = IssuesCommentsId(parse_base62(&comment_id_str)? as i64);

//     let user_option = get_user_from_headers(
//         &req,
//         &**pool,
//         &redis,
//         &session_queue,
//         Some(&[Scopes::PROJECT_WRITE]),
//     )
//     .await
//     .map(|x| x.1)
//     .ok();

//     if user_option.is_none() {
//         return Err(ApiError::Authentication(
//             AuthenticationError::InvalidCredentials,
//         ));
//     }

//     let _user = user_option.unwrap();

//     // 检查内容安全性
//     let risk = true; // 暂时禁用内容安全检查
//     if !risk {
//         return Err(ApiError::InvalidInput(
//             "评论内容包含敏感词，已被记录该次提交".to_string(),
//         ));
//     }

//     // TODO: 添加权限检查，确保只有评论作者或管理员可以编辑

//     let mut transaction = pool.begin().await?;
//     IssueCommentQuery::update_body(
//         comment_id,
//         body.body.clone(),
//         &mut transaction,
//     )
//     .await?;
//     transaction.commit().await?;

//     Ok(HttpResponse::NoContent().finish())
// }

// 删除评论
pub async fn comment_delete(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let comment_id_str: String = info.into_inner().0;
    let comment_id = IssuesCommentsId(parse_base62(&comment_id_str)? as i64);

    let user_option = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await
    .map(|x| x.1)
    .ok();

    if user_option.is_none() {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

    let user = user_option.unwrap();

    // 首先获取评论信息以进行权限检查和获取issue_id
    let comment_info = sqlx::query!(
            "SELECT ic.author_id, ic.issue_id FROM issue_comments ic WHERE ic.id = $1 AND ic.deleted = false",
            comment_id.0
        )
        .fetch_optional(&**pool)
        .await?;

    if comment_info.is_none() {
        return Err(ApiError::NotFound);
    }

    let comment_info = comment_info.unwrap();
    let issue_id = IssuesId(comment_info.issue_id);

    let issue = Issue::get_id(issue_id.0, &**pool, &redis).await?;

    if issue.is_none() {
        return Err(ApiError::NotFound);
    }

    let issue: database::models::issues::QueryIssue = issue.unwrap();

    let result =
        database::models::Project::get_id(issue.inner.mod_id, &**pool, &redis)
            .await?;

    if result.is_none() {
        return Err(ApiError::NotFound);
    }

    let project = result.unwrap();

    let (team_member, organization_team_member) =
        crate::database::models::TeamMember::get_for_project_permissions(
            &project.inner,
            UserId::from(user.id),
            &**pool,
        )
        .await?;

    let permissions = ProjectPermissions::get_permissions_by_role(
        &user.role,
        &team_member,
        &organization_team_member,
    );

    // 权限检查：只有评论作者或管理员可以删除
    if comment_info.author_id != UserId::from(user.id).0
        && permissions.is_none()
    {
        return Err(ApiError::InvalidInput("您没有权限删除此评论".to_string()));
    }

    let mut transaction = pool.begin().await?;
    IssueCommentQuery::delete_comment(comment_id, &mut transaction).await?;
    transaction.commit().await?;

    // 清除相关缓存
    Issue::clear_cache(&[issue_id], &redis).await?;

    // 清除评论相关的缓存 - 需要清除所有该Issue的评论缓存
    let mut redis_conn = redis.connect().await?;

    // 获取该Issue的所有评论ID，清除它们的缓存
    let all_comment_ids = sqlx::query!(
        "SELECT id FROM issue_comments WHERE issue_id = $1 AND deleted = false",
        issue_id.0
    )
    .fetch_all(&**pool)
    .await?;

    let mut cache_keys = vec![(ISSUE_NAMESPACE, Some(issue_id.0.to_string()))];

    // 添加所有评论的缓存键
    for comment in all_comment_ids {
        cache_keys.push(("issue_comment", Some(comment.id.to_string())));
    }

    redis_conn.delete_many(cache_keys).await?;

    Ok(HttpResponse::NoContent().finish())
}

// 获取所有标签
pub async fn labels_get(
    _req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let mut exec = pool.acquire().await?;
    let labels = IssueLabel::get_all(&mut *exec, &redis).await?;
    let labels: Vec<LabelResponse> =
        labels.into_iter().map(|x| x.into()).collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(json!({
        "labels": labels
    })))
}
