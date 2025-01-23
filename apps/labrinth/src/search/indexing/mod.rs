/// 此模块用于从任何来源进行索引。
pub mod local_import;

use crate::database::redis::RedisPool;
use crate::models::ids::base62_impl::to_base62;
use crate::search::{SearchConfig, UploadSearchProject};
use local_import::index_local;
use log::info;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::indexes::Index;
use meilisearch_sdk::settings::{PaginationSetting, Settings};
use meilisearch_sdk::SwapIndexes;
use sqlx::postgres::PgPool;
use std::error::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexingError {
    #[error("连接 MeiliSearch 数据库时出错")]
    Indexing(#[from] meilisearch_sdk::errors::Error),
    #[error("序列化或反序列化 JSON 时出错: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("数据库错误: {0}")]
    Sqlx(#[from] sqlx::error::Error),
    #[error("数据库错误: {0}")]
    Database(#[from] crate::database::models::DatabaseError),
    #[error("环境错误")]
    Env(#[from] dotenvy::Error),
    #[error("等待索引创建任务时出错")]
    Task,
}

// 添加项目到索引数据库的块大小。如果请求大小
// 太大 (>10MiB) 则请求失败。这个块大小
// 假设每个项目平均大小为 4KiB 以避免这个限制。
const MEILISEARCH_CHUNK_SIZE: usize = 10000000;
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

pub async fn remove_documents(
    ids: &[crate::models::ids::VersionId],
    config: &SearchConfig,
) -> Result<(), meilisearch_sdk::errors::Error> {
    let mut indexes = get_indexes_for_indexing(config, false).await?;
    let mut indexes_next = get_indexes_for_indexing(config, true).await?;
    indexes.append(&mut indexes_next);

    for index in indexes {
        index
            .delete_documents(
                &ids.iter().map(|x| to_base62(x.0)).collect::<Vec<_>>(),
            )
            .await?;
    }

    Ok(())
}

pub async fn index_projects(
    pool: PgPool,
    redis: RedisPool,
    config: &SearchConfig,
) -> Result<(), IndexingError> {
    info!("索引项目。");

    // 首先，确保当前索引存在（这样不会发生错误- 当前索引应该是空的最坏情况，而不是缺失）
    get_indexes_for_indexing(config, false).await?;

    // 然后，如果存在，删除下一个索引
    let indices = get_indexes_for_indexing(config, true).await?;
    for index in indices {
        index.delete().await?;
    }
    // 重新创建下一个索引进行索引
    let indices = get_indexes_for_indexing(config, true).await?;

    let all_loader_fields =
        crate::database::models::loader_fields::LoaderField::get_fields_all(
            &pool, &redis,
        )
        .await?
        .into_iter()
        .map(|x| x.field)
        .collect::<Vec<_>>();

    let uploads = index_local(&pool).await?;
    add_projects(&indices, &uploads, all_loader_fields.clone(), config).await?;

    // 交换索引
    swap_index(config, "projects").await?;
    swap_index(config, "projects_filtered").await?;

    // 删除现在已过时的索引
    for index in indices {
        index.delete().await?;
    }
    // let ups = uploads.clone();
    // 初始化一个不重复数值的Set数组

    // let mut urls = HashSet::new();

    // uploads.iter().for_each(|upload| {
    //     let type_ = upload.project_types.first();
    //     let url1 = format!(
    //         "https://bbsmc.net/{}/{}",
    //         type_.unwrap_or(&"project".to_string()).to_string(),
    //         &upload.project_id
    //     );
    //     let url2 = format!(
    //         "https://bbsmc.net/{}/{}/changelog",
    //         type_.unwrap_or(&"project".to_string()).to_string(),
    //         &upload.project_id
    //     );
    //     let url = format!(
    //         "https://bbsmc.net/{}/{}/version/{}",
    //         type_.unwrap_or(&"project".to_string()).to_string(),
    //         &upload.project_id,
    //         &upload.version_id
    //     );
    //     urls.insert(url);
    //     urls.insert(url1);
    //     urls.insert(url2);
    // });
    // let url = urls.into_iter().collect::<Vec<_>>();

    // for i in 0..url.len() / 2000 {
    //     let start = i * 2000;
    //     let end = start + 2000;
    //     // urls.push(url[start..end].to_vec());
    //     let _ = submit_urls(url[start..end].to_vec()).await;
    // }

    info!("完成添加项目。");
    Ok(())
}

async fn _submit_urls(urls: Vec<String>) -> Result<(), Box<dyn Error>> {
    // let urls = vec![
    //     "https://bbsmc.net/modpack/snk",
    //     "https://bbsmc.net/modpack/utopia-journey",
    // ];

    let api = "http://data.zz.baidu.com/urls?site=https://bbsmc.net&token=";
    let body = urls.join("\n");

    let client = reqwest::Client::new();
    let response = client
        .post(api)
        .header("Content-Type", "text/plain")
        .body(body)
        .send()
        .await?;

    let result = response.text().await?;
    println!("{}", result);

    Ok(())
}

pub async fn swap_index(
    config: &SearchConfig,
    index_name: &str,
) -> Result<(), IndexingError> {
    let client = config.make_client();
    let index_name_next = config.get_index_name(index_name, true);
    let index_name = config.get_index_name(index_name, false);
    let swap_indices = SwapIndexes {
        indexes: (index_name_next, index_name),
    };
    client
        .swap_indexes([&swap_indices])
        .await?
        .wait_for_completion(&client, None, Some(TIMEOUT))
        .await?;

    Ok(())
}

pub async fn get_indexes_for_indexing(
    config: &SearchConfig,
    next: bool, // 获取下一个索引
) -> Result<Vec<Index>, meilisearch_sdk::errors::Error> {
    let client = config.make_client();
    let project_name = config.get_index_name("projects", next);
    let project_filtered_name =
        config.get_index_name("projects_filtered", next);
    let projects_index = create_or_update_index(
        &client,
        &project_name,
        Some(&[
            "words",
            "typo",
            "proximity",
            "attribute",
            "exactness",
            "sort",
        ]),
    )
    .await?;
    let projects_filtered_index = create_or_update_index(
        &client,
        &project_filtered_name,
        Some(&[
            "sort",
            "words",
            "typo",
            "proximity",
            "attribute",
            "exactness",
        ]),
    )
    .await?;

    Ok(vec![projects_index, projects_filtered_index])
}

async fn create_or_update_index(
    client: &Client,
    name: &str,
    custom_rules: Option<&'static [&'static str]>,
) -> Result<Index, meilisearch_sdk::errors::Error> {
    info!("更新/创建索引 {}", name);

    match client.get_index(name).await {
        Ok(index) => {
            info!("更新索引设置。");

            let mut settings = default_settings();

            if let Some(custom_rules) = custom_rules {
                settings = settings.with_ranking_rules(custom_rules);
            }

            info!("执行索引设置。");
            index
                .set_settings(&settings)
                .await?
                .wait_for_completion(client, None, Some(TIMEOUT))
                .await?;
            info!("完成索引设置。");

            Ok(index)
        }
        _ => {
            info!("创建索引。");

            // 仅当索引不存在时创建索引并设置设置
            let task = client.create_index(name, Some("version_id")).await?;
            let task = task
                .wait_for_completion(client, None, Some(TIMEOUT))
                .await?;
            let index = task
                .try_make_index(client)
                .map_err(|x| x.unwrap_failure())?;

            let mut settings = default_settings();

            if let Some(custom_rules) = custom_rules {
                settings = settings.with_ranking_rules(custom_rules);
            }

            index
                .set_settings(&settings)
                .await?
                .wait_for_completion(client, None, Some(TIMEOUT))
                .await?;

            Ok(index)
        }
    }
}

async fn add_to_index(
    client: &Client,
    index: &Index,
    mods: &[UploadSearchProject],
) -> Result<(), IndexingError> {
    for chunk in mods.chunks(MEILISEARCH_CHUNK_SIZE) {
        info!("添加以版本 ID {} 开头的版本", chunk[0].version_id);
        index
            .add_or_replace(chunk, Some("version_id"))
            .await?
            .wait_for_completion(
                client,
                None,
                Some(std::time::Duration::from_secs(3600)),
            )
            .await?;
        info!("将 {} 个项目的块添加到索引中", chunk.len());
    }

    Ok(())
}

async fn update_and_add_to_index(
    client: &Client,
    index: &Index,
    projects: &[UploadSearchProject],
    _additional_fields: &[String],
) -> Result<(), IndexingError> {
    // TODO: 取消注释此代码- 硬编码加载器字段是临时修复，很快就会修复
    // let mut new_filterable_attributes: Vec<String> = index.get_filterable_attributes().await?;
    // let mut new_displayed_attributes = index.get_displayed_attributes().await?;

    // // 检查任何 'additional_fields' 是否不在索引中
    // // 仅当它们不在索引中时添加
    // let new_fields = additional_fields
    //     .iter()
    //     .filter(|x| !new_filterable_attributes.contains(x))
    //     .collect::<Vec<_>>();
    // if !new_fields.is_empty() {
    //     info!("Adding new fields to index: {:?}", new_fields);
    //     new_filterable_attributes.extend(new_fields.iter().map(|s: &&String| s.to_string()));
    //     new_displayed_attributes.extend(new_fields.iter().map(|s| s.to_string()));

    //     // 将新字段添加到索引
    //     let filterable_task = index
    //         .set_filterable_attributes(new_filterable_attributes)
    //         .await?;
    //     let displayable_task = index
    //         .set_displayed_attributes(new_displayed_attributes)
    //         .await?;

    //     // 允许长时间超时以添加新属性- 它只需要发生一次
    //     filterable_task
    //         .wait_for_completion(client, None, Some(TIMEOUT * 100))
    //         .await?;
    //     displayable_task
    //         .wait_for_completion(client, None, Some(TIMEOUT * 100))
    //         .await?;
    // }

    info!("添加到索引。");

    add_to_index(client, index, projects).await?;

    Ok(())
}

pub async fn add_projects(
    indices: &[Index],
    projects: &[UploadSearchProject],
    additional_fields: Vec<String>,
    config: &SearchConfig,
) -> Result<(), IndexingError> {
    let client = config.make_client();
    for index in indices {
        update_and_add_to_index(&client, index, projects, &additional_fields)
            .await?;
    }

    Ok(())
}

fn default_settings() -> Settings {
    Settings::new()
        .with_distinct_attribute("project_id") // 设置唯一属性为 project_id
        .with_displayed_attributes(DEFAULT_DISPLAYED_ATTRIBUTES) // 设置显示属性
        .with_searchable_attributes(DEFAULT_SEARCHABLE_ATTRIBUTES) // 设置可搜索属性
        .with_sortable_attributes(DEFAULT_SORTABLE_ATTRIBUTES) // 设置可排序属性
        .with_filterable_attributes(DEFAULT_ATTRIBUTES_FOR_FACETING) // 设置可过滤属性
        .with_pagination(PaginationSetting {
            max_total_hits: 2147483647, // 设置最大命中数为 2147483647
        })
}

const DEFAULT_DISPLAYED_ATTRIBUTES: &[&str] = &[
    "project_id",
    "version_id",
    "project_types",
    "slug",
    "author",
    "name",
    "summary",
    "categories",
    "display_categories",
    "downloads",
    "follows",
    "icon_url",
    "date_created",
    "date_modified",
    "latest_version",
    "license",
    "gallery",
    "featured_gallery",
    "color",
    // 注意：加载器字段不在这里，但会根据需要添加（因此可以根据存在的字段动态添加）。
    // TODO：删除这些- 因为它们应该被自动填充。这是一个临时解决方案。
    "server_only",
    "client_only",
    "game_versions",
    "singleplayer",
    "client_and_server",
    "mrpack_loaders",
    "software_loaders",
    // V2 逻辑一致性字段
    "client_side",
    "server_side",
    // 非搜索字段，用于填充 Project 模型。
    "license_url",
    "monetization_status",
    "team_id",
    "thread_id",
    "versions",
    "date_published",
    "date_queued",
    "status",
    "requested_status",
    "games",
    "organization_id",
    "links",
    "gallery_items",
    "loaders", // 搜索使用加载器作为类别- 这只是为了 Project 模型。
    "project_loader_fields",
    "chat_id",
];

const DEFAULT_SEARCHABLE_ATTRIBUTES: &[&str] =
    &["name", "summary", "author", "slug"];

const DEFAULT_ATTRIBUTES_FOR_FACETING: &[&str] = &[
    "categories",
    "license",
    "project_types",
    "downloads",
    "follows",
    "author",
    "name",
    "date_created",
    "created_timestamp",
    "date_modified",
    "modified_timestamp",
    "project_id",
    "open_source",
    "color",
    // 注意：加载器字段不在这里，但会根据需要添加（因此可以根据存在的字段动态添加）。
    // TODO：删除这些- 因为它们应该被自动填充。这是一个临时解决方案。
    "server_only",
    "client_only",
    "game_versions",
    "singleplayer",
    "client_and_server",
    "mrpack_loaders",
    "software_loaders",
    // V2 逻辑一致性字段
    "client_side",
    "server_side",
];

const DEFAULT_SORTABLE_ATTRIBUTES: &[&str] =
    &["downloads", "follows", "date_created", "date_modified"];
