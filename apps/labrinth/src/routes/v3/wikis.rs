use crate::auth::checks::is_visible_project;
use crate::auth::{get_user_from_headers, AuthenticationError};
use crate::database;
use crate::database::models::notification_item::NotificationBuilder;
use crate::database::models::wiki_item::{WikiDisplays, Wikis};
use crate::database::models::{generate_wiki_cache_id, generate_wiki_id, user_item, User, UserId, Wiki, WikiCache, WikiCacheId, WikiId};
use crate::database::redis::RedisPool;
use crate::models::ids::ProjectId;
use crate::models::notifications::NotificationBody;
use crate::models::pats::Scopes;
use crate::models::teams::ProjectPermissions;
use crate::queue::session::AuthQueue;
use crate::routes::v3::users::user_get_;
use crate::routes::ApiError;
use crate::util::validate::validation_errors_to_string;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashMap;
use validator::Validate;

#[derive(Deserialize, Serialize, Validate)]
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
    #[validate(range(min = 0, max = 1000))]
    pub sort_order: i32,
}

#[derive(Deserialize, Serialize, Validate)]
pub struct MsgWiki {
    #[validate(
        length(min = 1, max = 500),
    )]
    pub msg: String,
}

#[derive(Deserialize, Serialize, Validate)]
pub struct WikiSubmitAgain {
    pub id: WikiCacheId,
}



#[derive(Deserialize, Serialize, Debug, Validate)]
pub struct EditWiki {
    pub id: i64,
    #[validate(length(max = 65536))]
    pub body: String,
    #[validate(range(min = 0, max = 1000))]
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
) -> Result<HttpResponse, ApiError>
{
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
    new_wiki: web::Json<EditWiki>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError>
{
    // let mut bytes = web::BytesMut::new();
    // while let Some(item) = body.next().await {
    //     bytes.extend_from_slice(&item.map_err(|_| {
    //         ApiError::InvalidInput(
    //             "Error while parsing request payload!".to_string(),
    //         )
    //     })?);
    // }
    new_wiki.validate().map_err(|err| {
        ApiError::Validation(validation_errors_to_string(err, None))
    })?;

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
        // let new_wiki: EditWiki = serde_json::from_slice(bytes.as_ref())?;

        for i in 0..cache_json.len() {
            if cache_json[i]["id"] == new_wiki.id {
                cache_json[i]["body"] = new_wiki.body.clone().into();
                cache_json[i]["sort_order"] =
                    new_wiki.sort_order.clone().into();
                break;
            } else {
                if !cache_json[i]["child"].is_null() {
                    let child_array =
                        cache_json[i]["child"].as_array_mut().unwrap();
                    for j in 0..child_array.len() {
                        if child_array[j]["id"] == new_wiki.id {
                            child_array[j]["body"] =
                                new_wiki.body.clone().into();
                            child_array[j]["sort_order"] =
                                new_wiki.sort_order.clone().into();
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
) -> Result<HttpResponse, ApiError>
{
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
                    let child_array =
                        cache_json[i]["child"].as_array_mut().unwrap();
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
    new_wiki: web::Json<CreateWiki>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    new_wiki.validate().map_err(|err| {
        ApiError::Validation(validation_errors_to_string(err, None))
    })?;

    let string = info.into_inner().0;
    let result: Option<database::models::project_item::QueryProject> =
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
        let father_id = new_wiki.father_id;

        let mut wiki = Wiki {
            id: wiki_id,
            project_id: project.inner.id,
            sort_order: new_wiki.sort_order,
            title: new_wiki.title.clone(),
            body: "".to_string(),
            parent_wiki_id: wiki_id,
            featured: false,
            created: Default::default(),
            updated: Default::default(),
            slug: new_wiki.slug.clone(),
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
            is_editor_user: true,
            editor_user: Option::from(user_option),
            is_visitors: false
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
) -> Result<HttpResponse, ApiError>
{
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
        if Utc::now() < user_option.as_ref().unwrap().wiki_ban_time {
            return Err(ApiError::WikiBan(user_option.as_ref().unwrap().wiki_ban_time.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap()).format("%Y-%m-%d %H:%M:%S").to_string()));
        }
        if !is_visible_project(&project.inner, &user_option, &pool, false)
            .await?
        {
            return Err(ApiError::NotFound);
        }
        // let wiki_cache = database::models::WikiCache::get_draft(
        //     project.inner.id,
        //     UserId::from(user_option.as_ref().unwrap().id),
        //     &**pool,
        // )
        // .await?;
        // if wiki_cache.is_some() {
        //     return Ok(HttpResponse::Ok().json(wiki_cache.unwrap()));
        //     // return return Err(ApiError::ISExists);
        // }

        let has_draft_or_review =
            database::models::WikiCache::get_has_draft_or_review(
                project.inner.id,
                &**pool,
            )
            .await?;

        if has_draft_or_review.is_some() {
            let user =
                user_item::User::get_id(has_draft_or_review.unwrap().user_id,&**pool, &redis).await?;
            return Err(ApiError::ISConflict(user.unwrap().username));
        }
        let (team_member, organization_team_member) =
            crate::database::models::TeamMember::get_for_project_permissions(
                &project.inner,
                UserId::from(user_option.as_ref().unwrap().id),
                &**pool,
            )
                .await?;

        let permissions = ProjectPermissions::get_permissions_by_role(
            &user_option.as_ref().unwrap().role,
            &team_member,
            &organization_team_member,
        );

        if permissions.is_none() && project.inner.wiki_open == false{
            return Err(ApiError::Validation("你没有权限编辑百科页面".to_string()));
        }

        if permissions.is_some()  {
            let perms = permissions.unwrap();
            if !perms.contains(ProjectPermissions::WIKI_EDIT) &&  project.inner.wiki_open == false{
                return Err(ApiError::Validation("你没有权限编辑百科页面".to_string()));
            }
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
            old: serde_json::json!(wikis_array),
            message: serde_json::json!([]),
            again_count: 0,
            again_time: Default::default(),
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
) -> Result<HttpResponse, ApiError>
{
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
            is_editor_user: false,
            editor_user: None,
            is_visitors: true,
        };

        // 缓存，正在编辑的wiki缓存
        if user_option.is_some() {
            let wiki_cache = database::models::WikiCache::get_draft_or_review(
                project.inner.id,
                &**pool,
            )
            .await?;
            // wiki_cache.sort_by_key(|x| x.sort_order);

            if wiki_cache.is_some() {
                let mut wiki_cache_ = wiki_cache.unwrap();

                let (team_member, organization_team_member) =
                    crate::database::models::TeamMember::get_for_project_permissions(
                        &project.inner,
                        UserId::from(user_option.as_ref().unwrap().id),
                        &**pool,
                    )
                        .await?;

                let permissions = ProjectPermissions::get_permissions_by_role(
                    &user_option.as_ref().unwrap().role,
                    &team_member,
                    &organization_team_member,
                );

                // 如果 有编辑权限 或者 缓存的用户id和当前用户id相同则返回所有正在编辑的wiki
                if permissions.is_some() && permissions.unwrap().contains(ProjectPermissions::WIKI_EDIT) || wiki_cache_.user_id == UserId(user_option.as_ref().unwrap().id.0.clone() as i64){
                    wiki_cache_
                        .cache
                        .as_array_mut()
                        .unwrap()
                        .sort_by_key(|x| x["sort_order"].as_i64().unwrap());
                    for x in wiki_cache_.cache.as_array_mut().unwrap() {
                        if !x["child"].is_null() {
                            x["child"]
                                .as_array_mut()
                                .unwrap()
                                .sort_by_key(|x| x["sort_order"].as_i64().unwrap());
                        }
                    }
                    let u = user_get_(wiki_cache_.user_id, pool, redis).await?;

                    wikis.is_visitors = false;
                    wikis.is_editor = true;
                    wikis.editor_user = u;
                    if  wiki_cache_.user_id == UserId(user_option.as_ref().unwrap().id.0.clone() as i64) {
                        wikis.is_editor_user = true;
                        wikis.cache = Option::from(wiki_cache_);
                    }else {
                        wikis.is_editor_user = false;
                        if wiki_cache_.status == "review".to_string() {
                            wikis.cache = Option::from(wiki_cache_);
                        }
                    }



                }else {
                    let u = user_get_(wiki_cache_.user_id, pool, redis).await?;
                    wikis.is_editor = true;
                    wikis.is_visitors = true;
                    wikis.is_editor_user = false;
                    wikis.editor_user = u;
                }



            }
        }

        Ok(HttpResponse::Ok().json(wikis))
    } else {
        Err(ApiError::NotFound)
    }
}
pub async fn wiki_accept(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError>
{
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

    if user_option.is_none() {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

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
        let wikis = Wikis {
            wikis: wikis_array,
            is_editor: false,
            cache: None,
            is_editor_user: true,
            editor_user: user_option.clone(),
            is_visitors: false
        };

        let wiki_cache = database::models::WikiCache::get_draft_or_review(
            project.inner.id,
            &**pool,
        )
            .await?;

        let (team_member, organization_team_member) =
            crate::database::models::TeamMember::get_for_project_permissions(
                &project.inner,
                UserId::from(user_option.as_ref().unwrap().id),
                &**pool,
            )
                .await?;

        let permissions = ProjectPermissions::get_permissions_by_role(
            &user_option.as_ref().unwrap().role,
            &team_member,
            &organization_team_member,
        );
        if wiki_cache.is_some() && (permissions.is_none() || !permissions.unwrap().contains(ProjectPermissions::WIKI_EDIT)) {
            return Err(ApiError::Validation("你没有权限审核百科页面".to_string()));
        }

        if wiki_cache.is_some() {


            // 只有编辑权限的用户才能跳过审核部分

            let mut wiki_cache_ = wiki_cache.unwrap();

            wiki_cache_
                .cache
                .as_array_mut()
                .unwrap()
                .sort_by_key(|x| x["sort_order"].as_i64().unwrap());
            for x in wiki_cache_.cache.as_array_mut().unwrap() {
                if !x["child"].is_null() {
                    x["child"]
                        .as_array_mut()
                        .unwrap()
                        .sort_by_key(|x| x["sort_order"].as_i64().unwrap());
                }
            }

            let mut old_wikis: HashMap<WikiId, Wiki> = HashMap::new();
            let mut new_wikis: HashMap<WikiId, Wiki> = HashMap::new();
            for wiki in wikis.wikis {
                old_wikis.insert(
                    wiki.id,
                    Wiki {
                        id: wiki.id,
                        project_id: wiki.project_id,
                        parent_wiki_id: wiki.parent_wiki_id,
                        title: wiki.title,
                        body: wiki.body,
                        sort_order: wiki.sort_order,
                        featured: wiki.featured,
                        created: wiki.created,
                        updated: wiki.updated,
                        slug: wiki.slug,
                    },
                );
                if !wiki.child.is_empty() {
                    for child in wiki.child {
                        old_wikis.insert(
                            child.id,
                            Wiki {
                                id: child.id,
                                project_id: child.project_id,
                                parent_wiki_id: child.parent_wiki_id,
                                title: child.title,
                                body: child.body,
                                sort_order: child.sort_order,
                                featured: child.featured,
                                created: child.created,
                                updated: child.updated,
                                slug: child.slug,
                            },
                        );
                    }
                }
            }
            for wiki in wiki_cache_.cache.as_array_mut().unwrap() {
                let id = WikiId(wiki["id"].as_i64().unwrap());
                new_wikis.insert(
                    id,
                    Wiki {
                        id,
                        project_id: project.inner.id,
                        parent_wiki_id: WikiId(
                            wiki["parent_wiki_id"].as_i64().unwrap(),
                        ),
                        title: wiki["title"].as_str().unwrap().to_string(),
                        body: wiki["body"].as_str().unwrap().to_string(),
                        sort_order: wiki["sort_order"].as_i64().unwrap() as i32,
                        featured: wiki["featured"].as_bool().unwrap(),
                        created: wiki["created"]
                            .as_str()
                            .unwrap()
                            .to_string()
                            .parse()
                            .unwrap(),
                        updated: wiki["updated"]
                            .as_str()
                            .unwrap()
                            .to_string()
                            .parse()
                            .unwrap(),
                        slug: wiki["slug"].as_str().unwrap().to_string(),
                    },
                );

                if !wiki["child"].is_null() {
                    for child in wiki["child"].as_array_mut().unwrap() {
                        let id = WikiId(child["id"].as_i64().unwrap());
                        new_wikis.insert(
                            id,
                            Wiki {
                                id,
                                project_id: project.inner.id,
                                parent_wiki_id: WikiId(
                                    child["parent_wiki_id"].as_i64().unwrap(),
                                ),
                                title: child["title"]
                                    .as_str()
                                    .unwrap()
                                    .to_string(),
                                body: child["body"]
                                    .as_str()
                                    .unwrap()
                                    .to_string(),
                                sort_order: child["sort_order"]
                                    .as_i64()
                                    .unwrap()
                                    as i32,
                                featured: child["featured"].as_bool().unwrap(),
                                created: child["created"]
                                    .as_str()
                                    .unwrap()
                                    .to_string()
                                    .parse()
                                    .unwrap(),
                                updated: child["updated"]
                                    .as_str()
                                    .unwrap()
                                    .to_string()
                                    .parse()
                                    .unwrap(),
                                slug: child["slug"]
                                    .as_str()
                                    .unwrap()
                                    .to_string(),
                            },
                        );
                    }
                }
            }



            let mut common_wikis: HashMap<WikiId, (Wiki, Wiki)> =
                HashMap::new();
            for (key, old_wiki) in &old_wikis {
                if let Some(new_wiki) = new_wikis.get(key) {
                    common_wikis
                        .insert(*key, (old_wiki.clone(), new_wiki.clone()));
                }
            }
            let mut added_wikis: HashMap<WikiId, Wiki> = HashMap::new();
            let mut removed_wikis: HashMap<WikiId, Wiki> = HashMap::new();

            for (key, new_wiki) in &new_wikis {
                if !old_wikis.contains_key(key) {
                    added_wikis.insert(*key, new_wiki.clone());
                }
            }

            for (key, old_wiki) in &old_wikis {
                if !new_wikis.contains_key(key) {
                    removed_wikis.insert(*key, old_wiki.clone());
                }
            }

            let mut transaction = pool.begin().await?;
            for (_key, (old_wiki, new_wiki)) in &common_wikis {
                if old_wiki.body != new_wiki.body || old_wiki.sort_order != new_wiki.sort_order || old_wiki.featured != new_wiki.featured || old_wiki.title != new_wiki.title  {
                    let mut wiki = new_wiki.clone();
                    wiki.updated = chrono::Utc::now();
                    wiki.update(&mut transaction).await?;
                    wiki.clear_cache(&redis).await?;
                }
            }
            for (_key, new_wiki) in &added_wikis {
                let wiki = new_wiki.clone();
                wiki.update(&mut transaction).await?;
                wiki.clear_cache(&redis).await?;
            }
            for (_key, old_wiki) in &removed_wikis {
                if old_wiki.id != old_wiki.parent_wiki_id {
                    old_wiki.delete(&mut transaction).await?;
                }
            }
            for (_key, old_wiki) in &removed_wikis {
                if old_wiki.id == old_wiki.parent_wiki_id {
                    old_wiki.delete(&mut transaction).await?;
                }
            }
            wiki_cache_.message_add(&user_option.as_ref().unwrap(), &"通过".to_string()).await;
            wiki_cache_.finish_cache(&mut transaction).await?;
            NotificationBuilder {
                body: NotificationBody::WikiCache {
                    project_id: ProjectId::from(project.inner.id),
                    project_title: project.inner.name.clone(),
                    wiki_cache_id: wiki_cache_.id.clone(),
                    type_: "accept".to_string(),
                    msg: "通过".to_string(),
                },
            }
                .insert(wiki_cache_.user_id, &mut transaction, &redis)
                .await?;

            transaction.commit().await?;
            crate::database::models::Project::clear_cache(
                project.inner.id,
                None,
                None,
                &redis,
            ).await?;

        } else {
            return Err(ApiError::NotFound);
        }

        Ok(HttpResponse::Ok().finish())
    } else {
        Err(ApiError::NotFound)
    }

}

pub async fn wiki_reject(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    body: web::Json<MsgWiki>,
) -> Result<HttpResponse, ApiError>
{
    body.validate().map_err(|err| {
        ApiError::Validation(validation_errors_to_string(err, None))
    })?;
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

    if user_option.is_none() {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

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

        let wiki_cache = database::models::WikiCache::get_draft_or_review(
            project.inner.id,
            &**pool,
        )
            .await?;

        let (team_member, organization_team_member) =
            crate::database::models::TeamMember::get_for_project_permissions(
                &project.inner,
                UserId::from(user_option.as_ref().unwrap().id),
                &**pool,
            )
                .await?;

        let permissions = ProjectPermissions::get_permissions_by_role(
            &user_option.as_ref().unwrap().role,
            &team_member,
            &organization_team_member,
        );
        if wiki_cache.is_some() && (permissions.is_none() || !permissions.unwrap().contains(ProjectPermissions::WIKI_EDIT)) {
            return Err(ApiError::Validation("你没有权限审核百科页面".to_string()));
        }

        if wiki_cache.is_some() {

            // 只有编辑权限的用户才能跳过审核部分
            let mut wiki_cache_ = wiki_cache.unwrap();

            let mut transaction = pool.begin().await?;

            wiki_cache_.message_add(&user_option.as_ref().unwrap(), &body.msg).await;
            wiki_cache_.reject_cache(&mut transaction).await?;
            NotificationBuilder {
                body: NotificationBody::WikiCache {
                    project_id: ProjectId::from(project.inner.id),
                    project_title: project.inner.name.clone(),
                    wiki_cache_id: wiki_cache_.id.clone(),
                    type_: "reject".to_string(),
                    msg: body.msg.clone(),
                },
            }
                .insert(wiki_cache_.user_id, &mut transaction, &redis)
                .await?;
            transaction.commit().await?;
            Ok(HttpResponse::Ok().finish())
        } else {
            Err(ApiError::NotFound)
        }
    } else {
        Err(ApiError::NotFound)
    }
}
pub async fn wiki_submit(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    body: web::Json<MsgWiki>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(|err| {
        ApiError::Validation(validation_errors_to_string(err, None))
    })?;
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

    if user_option.is_none() {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }
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
        let wikis = Wikis {
            wikis: wikis_array,
            is_editor: false,
            cache: None,
            is_editor_user: true,
            editor_user: user_option.clone(),
            is_visitors: false
        };

        let wiki_cache = database::models::WikiCache::get_draft(
            project.inner.id,
            UserId::from(user_option.as_ref().unwrap().id),
            &**pool,
        )
        .await?;

        // wiki_cache.sort_by_key(|x| x.sort_order);
        let (team_member, organization_team_member) =
            crate::database::models::TeamMember::get_for_project_permissions(
                &project.inner,
                UserId::from(user_option.as_ref().unwrap().id),
                &**pool,
            )
                .await?;

        let permissions = ProjectPermissions::get_permissions_by_role(
            &user_option.as_ref().unwrap().role,
            &team_member,
            &organization_team_member,
        );

            /*
            *
            * 如果没有管理百科的权限则直接提交为审核中
            *
            * 并且发送通知给这个百科有权限的人去审核
            *
            */

        if wiki_cache.is_some() && (permissions.is_none() || !permissions.unwrap().contains(ProjectPermissions::WIKI_EDIT)) {
            let mut transaction = pool.begin().await?;
            let mut cache = wiki_cache.unwrap();
            cache.message_add(&user_option.as_ref().unwrap(), &body.msg).await;
            cache.review_cache(&mut transaction).await?;


            let mut new_member = database::models::TeamMember::get_from_team_full(
                project.inner.team_id,
                &**pool,
                &redis,
            )
            .await?
            .into_iter()
            .filter(|x| x.permissions.contains(ProjectPermissions::WIKI_EDIT))
            .collect::<Vec<_>>();

            if project.inner.organization_id.is_some() {

                let _organization = database::models::Organization::get_id(project.inner.organization_id.unwrap(), &**pool, &redis).await?;
                if _organization.is_some() {
                    let new_member_organization = database::models::TeamMember::get_from_team_full(
                        _organization.unwrap().team_id,
                        &**pool,
                        &redis,
                    )
                        .await?
                        .into_iter()
                        .filter(|x| x.permissions.contains(ProjectPermissions::WIKI_EDIT))
                        .collect::<Vec<_>>();
                    for x in new_member_organization {
                        println!("{:?}", x.user_id);
                        let mut has = false;
                        for xx in &new_member {
                            if xx.user_id == x.user_id {
                                has = true
                            }
                        }
                        if !has {
                            new_member.push(x);
                        }
                    }
                }else {
                    println!("organization not found");
                }



            }

            // println!("new_member: {:?}", new_member);


            for member in new_member {
                println!("member: {:?}", member);
                NotificationBuilder {
                    body: NotificationBody::WikiCache {
                        project_id: ProjectId::from(project.inner.id),
                        project_title: project.inner.name.clone(),
                        wiki_cache_id: cache.id.clone(),
                        type_: "review".to_string(),
                        msg: body.msg.clone(),
                    },
                }
                    .insert(member.user_id.into(), &mut transaction, &redis)
                    .await?;
            }
            transaction.commit().await?;
            return Ok(HttpResponse::Ok().finish());
        }

        if wiki_cache.is_some() {
            

            // 只有编辑权限的用户才能跳过审核部分

            let mut wiki_cache_ = wiki_cache.unwrap();

            wiki_cache_
                .cache
                .as_array_mut()
                .unwrap()
                .sort_by_key(|x| x["sort_order"].as_i64().unwrap());
            for x in wiki_cache_.cache.as_array_mut().unwrap() {
                if !x["child"].is_null() {
                    x["child"]
                        .as_array_mut()
                        .unwrap()
                        .sort_by_key(|x| x["sort_order"].as_i64().unwrap());
                }
            }
            // wikis.cache = Option::from(wiki_cache_);
            // wikis.is_editor = true;

            let mut old_wikis: HashMap<WikiId, Wiki> = HashMap::new();
            let mut new_wikis: HashMap<WikiId, Wiki> = HashMap::new();
            for wiki in wikis.wikis {
                old_wikis.insert(
                    wiki.id,
                    Wiki {
                        id: wiki.id,
                        project_id: wiki.project_id,
                        parent_wiki_id: wiki.parent_wiki_id,
                        title: wiki.title,
                        body: wiki.body,
                        sort_order: wiki.sort_order,
                        featured: wiki.featured,
                        created: wiki.created,
                        updated: wiki.updated,
                        slug: wiki.slug,
                    },
                );
                if !wiki.child.is_empty() {
                    for child in wiki.child {
                        old_wikis.insert(
                            child.id,
                            Wiki {
                                id: child.id,
                                project_id: child.project_id,
                                parent_wiki_id: child.parent_wiki_id,
                                title: child.title,
                                body: child.body,
                                sort_order: child.sort_order,
                                featured: child.featured,
                                created: child.created,
                                updated: child.updated,
                                slug: child.slug,
                            },
                        );
                    }
                }
            }
            for wiki in wiki_cache_.cache.as_array_mut().unwrap() {
                let id = WikiId(wiki["id"].as_i64().unwrap());
                new_wikis.insert(
                    id,
                    Wiki {
                        id,
                        project_id: project.inner.id,
                        parent_wiki_id: WikiId(
                            wiki["parent_wiki_id"].as_i64().unwrap(),
                        ),
                        title: wiki["title"].as_str().unwrap().to_string(),
                        body: wiki["body"].as_str().unwrap().to_string(),
                        sort_order: wiki["sort_order"].as_i64().unwrap() as i32,
                        featured: wiki["featured"].as_bool().unwrap(),
                        created: wiki["created"]
                            .as_str()
                            .unwrap()
                            .to_string()
                            .parse()
                            .unwrap(),
                        updated: wiki["updated"]
                            .as_str()
                            .unwrap()
                            .to_string()
                            .parse()
                            .unwrap(),
                        slug: wiki["slug"].as_str().unwrap().to_string(),
                    },
                );

                if !wiki["child"].is_null() {
                    for child in wiki["child"].as_array_mut().unwrap() {
                        let id = WikiId(child["id"].as_i64().unwrap());
                        new_wikis.insert(
                            id,
                            Wiki {
                                id,
                                project_id: project.inner.id,
                                parent_wiki_id: WikiId(
                                    child["parent_wiki_id"].as_i64().unwrap(),
                                ),
                                title: child["title"]
                                    .as_str()
                                    .unwrap()
                                    .to_string(),
                                body: child["body"]
                                    .as_str()
                                    .unwrap()
                                    .to_string(),
                                sort_order: child["sort_order"]
                                    .as_i64()
                                    .unwrap()
                                    as i32,
                                featured: child["featured"].as_bool().unwrap(),
                                created: child["created"]
                                    .as_str()
                                    .unwrap()
                                    .to_string()
                                    .parse()
                                    .unwrap(),
                                updated: child["updated"]
                                    .as_str()
                                    .unwrap()
                                    .to_string()
                                    .parse()
                                    .unwrap(),
                                slug: child["slug"]
                                    .as_str()
                                    .unwrap()
                                    .to_string(),
                            },
                        );
                    }
                }
            }

            println!("old_wikis");
            for x in &old_wikis {
                println!("{:?} , {:?}", x.0, x.1);
            }

            println!("new_wikis");
            for x in &new_wikis {
                println!("{:?} , {:?}", x.0, x.1);
            }

            let mut common_wikis: HashMap<WikiId, (Wiki, Wiki)> =
                HashMap::new();
            for (key, old_wiki) in &old_wikis {
                if let Some(new_wiki) = new_wikis.get(key) {
                    common_wikis
                        .insert(*key, (old_wiki.clone(), new_wiki.clone()));
                }
            }
            println!("重复 common_wikis");
            for x in &common_wikis {
                println!("{:?} , {:?}", x.0, x.1);
            }
            let mut added_wikis: HashMap<WikiId, Wiki> = HashMap::new();
            let mut removed_wikis: HashMap<WikiId, Wiki> = HashMap::new();

            for (key, new_wiki) in &new_wikis {
                if !old_wikis.contains_key(key) {
                    added_wikis.insert(*key, new_wiki.clone());
                }
            }

            for (key, old_wiki) in &old_wikis {
                if !new_wikis.contains_key(key) {
                    removed_wikis.insert(*key, old_wiki.clone());
                }
            }

            println!("Added wikis");
            for x in &added_wikis {
                println!("{:?} , {:?}", x.0, x.1);
            }

            println!("Removed wikis");
            for x in &removed_wikis {
                println!("{:?} , {:?}", x.0, x.1);
            }

            let mut transaction = pool.begin().await?;
            for (_key, (old_wiki, new_wiki)) in &common_wikis {
                if old_wiki.body != new_wiki.body || old_wiki.sort_order != new_wiki.sort_order || old_wiki.featured != new_wiki.featured || old_wiki.title != new_wiki.title  {
                    let mut wiki = new_wiki.clone();
                    wiki.updated = chrono::Utc::now();
                    wiki.update(&mut transaction).await?;
                    wiki.clear_cache(&redis).await?;
                }
            }
            for (_key, new_wiki) in &added_wikis {
                let wiki = new_wiki.clone();
                wiki.update(&mut transaction).await?;
                wiki.clear_cache(&redis).await?;
            }
            for (_key, old_wiki) in &removed_wikis {
                if old_wiki.id != old_wiki.parent_wiki_id {
                    old_wiki.delete(&mut transaction).await?;
                }
            }
            for (_key, old_wiki) in &removed_wikis {
                if old_wiki.id == old_wiki.parent_wiki_id {
                    old_wiki.delete(&mut transaction).await?;
                }
            }
            wiki_cache_.message_add(&user_option.as_ref().unwrap(), &body.msg).await;
            wiki_cache_.finish_cache(&mut transaction).await?;
            transaction.commit().await?;
            crate::database::models::Project::clear_cache(
                project.inner.id,
                None,
                None,
                &redis,
            ).await?;




        } else {
            return Err(ApiError::NotFound);
        }

        Ok(HttpResponse::Ok().finish())
    } else {
        Err(ApiError::NotFound)
    }
}
pub async fn wiki_submit_again(
    req: HttpRequest,
    info: web::Path<(String,String)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {


    let (string, cache_id) = info.into_inner();
    println!("{:?} , {:?}", string, cache_id);
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

    if user_option.is_none() {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }
    if let Some(project) = result {
        if !is_visible_project(&project.inner, &user_option, &pool, false)
            .await?
        {
            return Err(ApiError::NotFound);
        }

        let more = database::models::WikiCache::get_draft_or_review(
            project.inner.id,
            &**pool,
        ).await?;
        if more.is_some() && more.unwrap().user_id != UserId::from(user_option.as_ref().unwrap().id) {
            return Err(ApiError::Validation("其他用户正在编辑中".to_string()));
        }


        let wiki_cache = database::models::WikiCache::get_reject_or_review(
            cache_id.parse().unwrap(),
            UserId::from(user_option.as_ref().unwrap().id),
            &**pool,
        )
        .await?;

        if wiki_cache.is_some() {
            let mut cache = wiki_cache.unwrap();

            if cache.again_count >= 5 {
                return Err(ApiError::Validation("已重复编辑过5次，本申请已无法再次编辑".to_string()));
            }

            let mut transaction = pool.begin().await?;
            cache.message_add(&user_option.as_ref().unwrap(), &"重新开始编辑".to_string()).await;
            cache.again_cache(&mut transaction).await?;

            transaction.commit().await?;
            Ok(HttpResponse::Ok().finish())
        }else {
            Err(ApiError::NotFound)
        }

    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn wiki_given_up(
    req: HttpRequest,
    info: web::Path<(String,String)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {


    let (string, cache_id) = info.into_inner();
    println!("{:?} , {:?}", string, cache_id);
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

    if user_option.is_none() {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }
    if let Some(project) = result {
        if !is_visible_project(&project.inner, &user_option, &pool, false)
            .await?
        {
            return Err(ApiError::NotFound);
        }

        let wiki_cache = database::models::WikiCache::get_reject_or_review(
            cache_id.parse().unwrap(),
            UserId::from(user_option.as_ref().unwrap().id),
            &**pool,
        )
        .await?;

        if wiki_cache.is_some() {
            let user = user_option.as_ref().unwrap();
            let mut cache = wiki_cache.unwrap();
            let mut transaction = pool.begin().await?;
            cache.message_add(&user, &"放弃修改".to_string()).await;
            cache.given_up_cache(&mut transaction).await?;
            cache.user_ban(cache.user_id,3,&mut transaction).await?;
            cache.user_overtake_count(cache.user_id,1,&mut transaction).await?;
            if user.wiki_overtake_count+1 > 3 {
                cache.user_overtake_count_set(cache.user_id,0,&mut transaction).await?;
                cache.user_ban(cache.user_id,72,&mut transaction).await?;
            }
            transaction.commit().await?;
            User::clear_caches(&[(UserId::from(user.id.clone()), Some(user.username.clone()))], &redis)
                .await?;
            Ok(HttpResponse::Ok().finish())

        }else {
            Err(ApiError::NotFound)
        }

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
