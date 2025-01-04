use crate::database::models::forum::PostBuilder;
use crate::database::models::forum::{Discussion, PostIndex};
use crate::database::models::ids::{DiscussionId, PostId};
use crate::database::models::UserId;
use crate::database::redis::RedisPool;
use crate::models::ids::base62_impl::parse_base62;
use crate::util::validate::validation_errors_to_string;
use crate::{
    models::v3::forum::{PostResponse, PostsQueryParams},
    routes::ApiError,
};
use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use validator::Validate;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("forum")
            .route("{id}/posts", web::get().to(posts_get))
            .route("{id}/post", web::post().to(posts_post)),
    );
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct PostRequest {
    #[validate(length(max = 65536))]
    pub content: String,
    pub replied_to: Option<String>,
}

pub async fn posts_get(
    _req: HttpRequest,
    info: web::Path<(String,)>,
    query: web::Query<PostsQueryParams>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let discussion_id = info.into_inner().0;
    let params = query.into_inner().clone();
    let page_size = params.page_size.unwrap_or(20) as i64;
    let page = params.page.unwrap_or(1) as i64;
    let mut exec = pool.acquire().await?;
    let exec_ref = &mut *exec;
    let discussion_id =
        DiscussionId(parse_base62(&discussion_id.to_string()).unwrap() as i64);
    let discussion = crate::database::models::forum::Discussion::get_id(
        discussion_id.0,
        exec_ref,
        &redis,
    )
    .await?;
    if discussion.is_none() {
        return Err(ApiError::NotFound);
    }
    let mut forum_floor_numbers: Vec<PostIndex> = discussion.unwrap().posts;

    // 对forum_floor_numbers进行排序
    forum_floor_numbers.sort_by(|a, b| a.floor_number.cmp(&b.floor_number));

    // 在使用前克隆一份
    let forum_floor_numbers_clone: Vec<PostIndex> = forum_floor_numbers.clone();

    // 获取需要查询的楼层号
    let offset = ((page - 1) * page_size) as usize;
    let floor_numbers: Vec<PostIndex> = forum_floor_numbers
        .clone()
        .into_iter()
        .skip(offset)
        .take(page_size as usize)
        .collect();
    // 使用获取到的 floor_numbers 从 redis 查询完整的帖子信息
    let ids = &floor_numbers
        .iter()
        .map(|x| x.post_id.0)
        .collect::<Vec<i64>>();
    // 修改输出所有查询到的 floor_number
    let mut posts: Vec<PostResponse> =
        crate::database::models::forum::PostQuery::get_many(
            ids,
            &discussion_id,
            &**pool,
            &redis,
        )
        .await?
        .into_iter()
        .map(|x| x.into())
        .collect::<Vec<PostResponse>>();

    // 对 posts 进行排序
    posts.sort_by(|a, b| a.floor_number.cmp(&b.floor_number));

    Ok(HttpResponse::Ok().json(json!({
        "posts": posts,
        "pagination": {
            "total": forum_floor_numbers_clone.len()
        }
    })))
}

pub async fn posts_post(
    _req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<PostRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(|err| {
        ApiError::Validation(validation_errors_to_string(err, None))
    })?;
    let string = info.into_inner().0;
    let discussion_id = DiscussionId(parse_base62(&string)? as i64);
    let discussion =
        Discussion::get_id(discussion_id.0, &**pool, &redis).await?;

    if discussion.is_none() {
        return Err(ApiError::NotFound);
    }

    let mut transaction = pool.begin().await?;

    let post_id: PostId =
        crate::database::models::ids::generate_post_id(&mut transaction)
            .await?;

    let post = PostBuilder {
        id: post_id,
        discussion_id,
        content: body.content.clone(),
        created_at: chrono::Utc::now(),
        user_id: UserId(187799526438262),
        replied_to: body
            .replied_to
            .clone()
            .map(|x| parse_base62(&x).unwrap() as i64),
    };

    post.insert(&mut transaction).await?;
    transaction.commit().await?;

    crate::database::models::forum::Discussion::clear_cache(
        &[discussion_id],
        &redis,
    )
    .await?;

    let posts: Vec<PostResponse> =
        crate::database::models::forum::PostQuery::get_many(
            &[post_id.0],
            &discussion_id,
            &**pool,
            &redis,
        )
        .await?
        .into_iter()
        .map(|x| x.into())
        .collect::<Vec<PostResponse>>();

    // info!("d: {:?}", discussion);
    Ok(HttpResponse::Ok().json(json!({
        "post": posts.first().unwrap()
    })))
}
