use crate::auth::checks::is_visible_project;
use crate::auth::{get_user_from_headers, AuthenticationError};
use crate::database;
use crate::database::models::wiki_item::{WikiDisplays, Wikis};
use crate::database::models::{
    generate_wiki_cache_id, UserId, Wiki, WikiCache,
};
use crate::database::redis::RedisPool;
use crate::models::pats::Scopes;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use std::collections::HashMap;

pub async fn wiki_edit(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let string = info.into_inner().0;
    let result =
        database::models::Project::get(&string, &**pool, &redis).await?;
    let user_option = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ, Scopes::VERSION_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();
    if let Some(project) = result {
        if !&user_option.is_some() {
            return Err(ApiError::Authentication(
                AuthenticationError::InvalidCredentials,
            ));
        }
        if !is_visible_project(&project.inner, &user_option, &pool, false)
            .await?
        {
            return Err(ApiError::NotFound);
        }
        let wiki_cache = database::models::WikiCache::get_draft(
            project.inner.id,
            UserId::from(user_option.as_ref().unwrap().id),
            &**pool,
            &redis,
        )
        .await?;
        if wiki_cache.is_some() {
            return Ok(HttpResponse::Ok().json(wiki_cache.unwrap()));
            // return return Err(ApiError::ISExists);
        }

        let mut transaction = pool.begin().await?;

        let id = user_option.unwrap().id;

        let wiki_cache_id =
            generate_wiki_cache_id(&mut transaction).await?.into();

        let mut wikis =
            database::models::Wiki::get_many(&*project.wikis, &**pool, &redis)
                .await?
                .into_iter()
                .collect::<Vec<_>>();
        wikis.sort_by_key(|x| x.sort_order);
        let wikis_array = wiki_format(wikis);


        let wiki_cache = WikiCache {
            id: wiki_cache_id,
            project_id: project.inner.id,
            user_id: UserId(id.0 as i64),
            created: Default::default(),
            status: "".to_string(),
            cache: serde_json::json!(wikis_array),
        }
        .insert(&redis, &mut transaction)
        .await?;
        transaction.commit().await?;
        return Ok(HttpResponse::Ok().json(wiki_cache));
    }

    Err(ApiError::NotFound)
}

pub async fn wiki_list(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let string = info.into_inner().0;
    let result =
        database::models::Project::get(&string, &**pool, &redis).await?;

    let user_option = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ, Scopes::VERSION_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();

    if let Some(project) = result {
        if !is_visible_project(&project.inner, &user_option, &pool, false)
            .await?
        {
            return Err(ApiError::NotFound);
        }

        let mut wikis =
            database::models::Wiki::get_many(&*project.wikis, &**pool, &redis)
                .await?
                .into_iter()
                .collect::<Vec<_>>();
        wikis.sort_by_key(|x| x.sort_order);
        let wikis_array = wiki_format(wikis);
        let mut wikis = Wikis {
            wikis: wikis_array,
            is_editor: false,
            cache: None,
        };

        // 缓存，正在编辑的wiki缓存
        if user_option.is_some() {
            let wiki_cache = database::models::WikiCache::get_draft(
                project.inner.id,
                UserId::from(user_option.as_ref().unwrap().id),
                &**pool,
                &redis,
            )
                .await?;

            if wiki_cache.is_some() {
                wikis.cache = wiki_cache;
                wikis.is_editor = true;
            }
        }


        Ok(HttpResponse::Ok().json(wikis))
    } else {
        Err(ApiError::NotFound)
    }
}

pub fn wiki_format(wikis: Vec<Wiki>) -> Vec<WikiDisplays> {
    let mut wikis_: HashMap<i64, WikiDisplays> = HashMap::new();
    for wiki in &wikis {
        if wiki.id == wiki.parent_wiki_id {
            wikis_.insert(
                wiki.id.0,
                WikiDisplays {
                    id: wiki.id.clone(),
                    project_id: wiki.project_id.clone(),
                    parent_wiki_id: wiki.parent_wiki_id.clone(),
                    title: wiki.title.clone(),
                    body: wiki.body.clone(),
                    sort_order: wiki.sort_order.clone(),
                    featured: wiki.featured.clone(),
                    created: wiki.created.clone(),
                    updated: wiki.updated.clone(),
                    slug: wiki.slug.clone(),
                    child: [].to_vec(),
                },
            );
        }
    }

    for wiki in wikis {
        if wiki.id != wiki.parent_wiki_id {
            if let Some(mut wk) = wikis_.get_mut(&wiki.parent_wiki_id.0) {
                wk.child.push(wiki);
                wk.child.sort_by_key(|x| x.sort_order);
            }
        }
    }
    wikis_.into_values().collect()
}
