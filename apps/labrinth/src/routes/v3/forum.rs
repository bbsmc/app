use crate::database::redis::RedisPool;
use crate::{
    models::v3::forum::{PostResponse, PostsQueryParams},
    routes::ApiError,
};
use actix_web::{web, HttpRequest, HttpResponse};
use dashmap::DashMap;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::models::forum::{PostIndex, Replay, ReplayContent};
use crate::util::validate::validation_errors_to_string;
use sqlx::PgPool;
use validator::Validate;


pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("forum")
        .route("{id}/posts", web::get().to(posts_get))
        .route("{id}/post", web::post().to(posts_post))
        ,
    );
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct PostRequest {
    #[validate(length(max = 65536))]
    pub content: String,
    pub discussion_id: i64,
    pub replied_to: Option<i64>,
}



pub async fn posts_get(
    _req: HttpRequest,
    info: web::Path<(String,)>,
    query: web::Query<PostsQueryParams>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let discussion_id = info.into_inner().0;
    let discussion_id_clone = discussion_id.clone();
    let params = query.into_inner().clone();
    let page_size = params.page_size.unwrap_or(20) as i64;
    let page = params.page.unwrap_or(1) as i64;
    let mut exec = pool.acquire().await?;
    let exec_ref = &mut *exec;

    let mut _forum_floor_numbers = redis
        .get_cached_key(
            &"forum_posts".to_string(),
            discussion_id.clone(),
            || async move {
                // 从数据库查询指定讨论下的所有楼层号
                let floor_numbers = sqlx::query!(
                    "SELECT id,floor_number FROM posts
                WHERE discussion_id = $1 ORDER BY floor_number ASC",
                    discussion_id.parse::<i64>().unwrap()
                )
                .fetch(exec_ref)
                .map_ok(|row| PostIndex{
                    id: row.id,
                    floor_number: row.floor_number ,
                })
                .try_collect::<Vec<PostIndex>>()
                .await?;

                Ok(floor_numbers)
            },
        )
        .await;
    if _forum_floor_numbers.is_err() {
        return Err(ApiError::NotFound);
    }
    let mut forum_floor_numbers: Vec<PostIndex> = _forum_floor_numbers.unwrap();
    
    // 对forum_floor_numbers进行排序 
    forum_floor_numbers.sort_by(|a, b| a.floor_number.cmp(&b.floor_number));

    // 在使用前克隆一份
    let forum_floor_numbers_clone: Vec<PostIndex> = forum_floor_numbers.clone();

    // 获取需要查询的楼层号
    let offset = ((page - 1) * page_size as i64) as usize;
    let floor_numbers: Vec<PostIndex>  = forum_floor_numbers.clone()
        .into_iter()
        .skip(offset)
        .take(page_size as usize)
        .collect();
    // 使用获取到的 floor_numbers 从 redis 查询完整的帖子信息
    let ids = &floor_numbers
        .iter()
        .map(|x| x.id)
        .collect::<Vec<i64>>();
    // 修改输出所有查询到的 floor_number
   let posts = crate::database::models::forum::PostQuery::get_many(ids, &i64::from(discussion_id), &pool, &redis).await;
    // 筛查 posts，删除不存在于 floor_numbers 的元素
    // let posts = posts.into_iter().filter(|x| floor_numbers.contains(&x.floor_number)).collect::<Vec<PostResponse>>();


    if posts.is_err() {

    }

    let mut floor_numbers1 = posts.iter().map(|x| x.floor_number).collect::<Vec<i64>>();
    floor_numbers1.sort(); 

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

    


    Ok(HttpResponse::Ok().json(json!({
        "message": "Post created successfully"
    })))
}
