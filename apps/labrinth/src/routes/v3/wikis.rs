use crate::auth::checks::is_visible_project;
use crate::auth::{get_user_from_headers, AuthenticationError};
use crate::database;
use crate::database::models::wiki_item::{WikiDisplays, Wikis};
use crate::database::models::{
    generate_wiki_cache_id, generate_wiki_id, UserId, Wiki, WikiCache, WikiId,
};
use crate::database::redis::RedisPool;
use crate::models::pats::Scopes;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use actix_web::{web, HttpRequest, HttpResponse};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashMap;
use validator::Validate;

#[derive(Deserialize, Serialize,Validate)]
pub struct CreateWiki {
    #[validate(
        length(min = 1, max = 64),
        custom(function = "crate::util::validate::validate_name")
    )]
    pub title: String,
    #[validate(
        length(min = 1, max = 32),
        regex = "crate::util::validate::RE_URL_SAFE"
    )]
    pub slug: String,
    pub father_id: Option<i64>,
    pub sort_order: i32,
}

#[derive(Deserialize, Serialize,Debug)]
pub struct EditWiki {
    pub id: i64,
    #[validate(length(max = 65536))]
    pub body: String,
    pub sort_order: i32,
}

#[derive(Deserialize, Serialize)]
pub struct WikiDelete {
    pub id: i64,
}
#[derive(Deserialize, Serialize)]
pub struct WikiStar {
    pub id: i64,
}

pub async fn wiki_delete(
    req: HttpRequest,
    info: web::Path<(String,)>,
    mut body: web::Payload,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {

    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item.map_err(|_| {
            ApiError::InvalidInput(
                "Error while parsing request payload!".to_string(),
            )
        })?);
    }
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
        )
        .await?;
        if !wiki_cache.is_some() {
            return Err(ApiError::NotFound);
        }
        let mut cache = wiki_cache.unwrap();
        let cache_json = cache.cache.as_array_mut().unwrap();
        let wiki_delete: WikiDelete = serde_json::from_slice(bytes.as_ref())?;

        for i in 0..cache_json.len() {
            if cache_json[i]["id"] == wiki_delete.id {
                cache_json.remove(i);
                break;
            } else {
                if !cache_json[i]["child"].is_null() {
                    let child_array =
                        cache_json[i]["child"].as_array_mut().unwrap();
                    for j in 0..child_array.len() {
                        if child_array[j]["id"] == wiki_delete.id {
                            child_array.remove(j);
                            break;
                        }
                    }
                }
            }
        }
        let mut transaction = pool.begin().await?;
        cache.update_cache(&mut transaction).await?;
        transaction.commit().await?;
        Ok(HttpResponse::Ok().finish())
    } else {
        Err(ApiError::NotFound)
    }
}
pub async fn wiki_edit(
    req: HttpRequest,
    info: web::Path<(String,)>,
    mut body: web::Payload,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item.map_err(|_| {
            ApiError::InvalidInput(
                "Error while parsing request payload!".to_string(),
            )
        })?);
    }
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
        )
        .await?;
        if !wiki_cache.is_some() {
            return Err(ApiError::NotFound);
        }
        let mut cache = wiki_cache.unwrap();
        let cache_json = cache.cache.as_array_mut().unwrap();
        let new_wiki: EditWiki = serde_json::from_slice(bytes.as_ref())?;

        for i in 0..cache_json.len() {
            if cache_json[i]["id"] == new_wiki.id {
                cache_json[i]["body"] = new_wiki.body.clone().into();
                break;
            } else {
                if !cache_json[i]["child"].is_null() {
                    let child_array =
                        cache_json[i]["child"].as_array_mut().unwrap();
                    for j in 0..child_array.len() {
                        if child_array[j]["id"] == new_wiki.id {
                            child_array[j]["body"] =
                                new_wiki.body.clone().into();
                            break;
                        }
                    }
                }
            }
        }
        let mut transaction = pool.begin().await?;
        cache.update_cache(&mut transaction).await?;
        transaction.commit().await?;

        Ok(HttpResponse::Ok().finish())
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn wiki_star(
    req: HttpRequest,
    info: web::Path<(String,)>,
    mut body: web::Payload,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item.map_err(|_| {
            ApiError::InvalidInput(
                "Error while parsing request payload!".to_string(),
            )
        })?);
    }
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
        )
        .await?;
        if !wiki_cache.is_some() {
            return Err(ApiError::NotFound);
        }
        let mut cache = wiki_cache.unwrap();
        let cache_json = cache.cache.as_array_mut().unwrap();
        let new_wiki: WikiStar = serde_json::from_slice(bytes.as_ref())?;

        for i in 0..cache_json.len() {
            if cache_json[i]["id"] == new_wiki.id {
                cache_json[i]["featured"] = Value::from(true);

                if !cache_json[i]["child"].is_null() {
                    let child_array =
                        cache_json[i]["child"].as_array_mut().unwrap();
                    for j in 0..child_array.len() {
                        child_array[j]["featured"] = Value::from(false);
                    }
                }
            } else {
                cache_json[i]["featured"] = Value::from(false);
                if !cache_json[i]["child"].is_null() {
                    let child_array = cache_json[i]["child"].as_array_mut().unwrap();
                    for j in 0..child_array.len() {
                        if child_array[j]["id"] == new_wiki.id {
                            child_array[j]["featured"] = Value::from(true);
                        } else {
                            child_array[j]["featured"] = Value::from(false);
                        }
                    }
                }
            }
        }
        let mut transaction = pool.begin().await?;
        cache.update_cache(&mut transaction).await?;
        transaction.commit().await?;

        Ok(HttpResponse::Ok().finish())
    } else {
        Err(ApiError::NotFound)
    }
}
pub async fn wiki_create(
    req: HttpRequest,
    info: web::Path<(String,)>,
    mut body: web::Payload,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item.map_err(|_| {
            ApiError::InvalidInput(
                "Error while parsing request payload!".to_string(),
            )
        })?);
    }

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
        )
        .await?;
        if !wiki_cache.is_some() {
            return Err(ApiError::NotFound);
        }

        let mut transaction = pool.begin().await?;

        let wiki_id = generate_wiki_id(&mut transaction).await?.into();
        let new_wiki: CreateWiki = serde_json::from_slice(bytes.as_ref())?;
        let father_id = new_wiki.father_id;

        let mut wiki = Wiki {
            id: wiki_id,
            project_id: project.inner.id,
            sort_order: new_wiki.sort_order,
            title: new_wiki.title,
            body: "".to_string(),
            parent_wiki_id: wiki_id,
            featured: false,
            created: Default::default(),
            updated: Default::default(),
            slug: new_wiki.slug,
        };
        if father_id.is_some() {
            wiki.parent_wiki_id = WikiId(father_id.unwrap());
        }
        let wiki_ = wiki.insert(&mut transaction).await?;

        let mut cache = wiki_cache.unwrap();
        let cache_json = cache.cache.as_array_mut().unwrap();
        if wiki_.parent_wiki_id == wiki.id {
            cache_json.push(serde_json::json!(wiki_));
        } else {
            for i in 0..cache_json.len() {
                if cache_json[i]["id"] == wiki_.parent_wiki_id.0 {
                    if cache_json[i]["child"].is_null() {
                        cache_json[i]["child"] = serde_json::json!([]);
                    }
                    cache_json[i]["child"]
                        .as_array_mut()
                        .unwrap()
                        .push(serde_json::json!(wiki_));
                    break;
                }
            }
        }
        cache.cache = cache_json.clone().into();
        // println!("{:?}",cache);
        cache = cache.update_cache(&mut transaction).await?;

        transaction.commit().await?;

        let mut wikis = database::models::Wiki::get_many(
            &*project.wikis,
            false,
            &**pool,
            &redis,
        )
        .await?
        .into_iter()
        .collect::<Vec<_>>();
        wikis.sort_by_key(|x| x.sort_order);
        let wikis_array = wiki_format(wikis);
        let wikis = Wikis {
            wikis: wikis_array,
            is_editor: true,
            cache: Option::from(cache),
        };

        Ok(HttpResponse::Ok().json(wikis))
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn wiki_edit_start(
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

        let mut wikis = database::models::Wiki::get_many(
            &*project.wikis,
            false,
            &**pool,
            &redis,
        )
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
        .insert(&mut transaction)
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

        let mut wikis = database::models::Wiki::get_many(
            &*project.wikis,
            false,
            &**pool,
            &redis,
        )
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
            if let Some(wk) = wikis_.get_mut(&wiki.parent_wiki_id.0) {
                wk.child.push(wiki);
                wk.child.sort_by_key(|x| x.sort_order);
            }
        }
    }
    wikis_.into_values().collect()
}
