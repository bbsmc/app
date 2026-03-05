use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures::TryStreamExt;
use itertools::Itertools;
use log::info;
use std::collections::HashMap;

use super::IndexingError;
use crate::database::models::loader_fields::{
    QueryLoaderField, QueryLoaderFieldEnumValue, QueryVersionField,
    VersionField,
};
use crate::database::models::{
    LoaderFieldEnumId, LoaderFieldEnumValueId, LoaderFieldId, ProjectId,
    VersionId,
};
use crate::models::projects::from_duplicate_version_fields;
use crate::models::v2::projects::LegacyProject;
use crate::routes::v2_reroute;
use crate::search::UploadSearchProject;
use sqlx::postgres::PgPool;

/// 每批处理的项目数量，避免大查询一次性占满数据库连接导致 API 请求超时
const INDEX_BATCH_SIZE: usize = 500;

pub async fn index_local(
    pool: &PgPool,
) -> Result<Vec<UploadSearchProject>, IndexingError> {
    info!("索引本地项目");

    struct PartialProject {
        id: ProjectId,
        name: String,
        summary: String,
        downloads: i32,
        follows: i32,
        icon_url: Option<String>,
        updated: DateTime<Utc>,
        approved: DateTime<Utc>,
        slug: Option<String>,
        color: Option<i32>,
        license: String,
    }

    // 步骤 1: 获取所有可搜索的项目列表（轻量查询）
    let db_projects = sqlx::query!(
        "
        SELECT m.id id, m.name name, m.summary summary, m.downloads downloads, m.follows follows,
        m.icon_url icon_url, m.updated updated, m.approved approved, m.published, m.license license, m.slug slug, m.color
        FROM mods m
        WHERE m.status = ANY($1)
        GROUP BY m.id;
        ",
        &*crate::models::projects::ProjectStatus::iterator()
        .filter(|x| x.is_searchable())
        .map(|x| x.to_string())
        .collect::<Vec<String>>(),
    )
        .fetch(pool)
        .map_ok(|m| {
            PartialProject {
                id: ProjectId(m.id),
                name: m.name,
                summary: m.summary,
                downloads: m.downloads,
                follows: m.follows,
                icon_url: m.icon_url,
                updated: m.updated,
                approved: m.approved.unwrap_or(m.published),
                slug: m.slug,
                color: m.color,
                license: m.license,
            }
        })
        .try_collect::<Vec<PartialProject>>()
        .await?;

    // 步骤 2: 获取全局数据（小表，不随项目变化）
    info!("获取所有加载器字段");
    let loader_fields: Vec<QueryLoaderField> = sqlx::query!(
        "
        SELECT DISTINCT id, field, field_type, enum_type, min_val, max_val, optional
        FROM loader_fields lf
        ",
    )
    .fetch(pool)
    .map_ok(|m| QueryLoaderField {
        id: LoaderFieldId(m.id),
        field: m.field,
        field_type: m.field_type,
        enum_type: m.enum_type.map(LoaderFieldEnumId),
        min_val: m.min_val,
        max_val: m.max_val,
        optional: m.optional,
    })
    .try_collect()
    .await?;
    let loader_fields: Vec<&QueryLoaderField> = loader_fields.iter().collect();

    info!("索引所有加载器字段枚举值");

    let loader_field_enum_values: Vec<QueryLoaderFieldEnumValue> =
        sqlx::query!(
            "
        SELECT DISTINCT id, enum_id, value, ordering, created, metadata
        FROM loader_field_enum_values lfev
        ORDER BY enum_id, ordering, created DESC
        "
        )
        .fetch(pool)
        .map_ok(|m| QueryLoaderFieldEnumValue {
            id: LoaderFieldEnumValueId(m.id),
            enum_id: LoaderFieldEnumId(m.enum_id),
            value: m.value,
            ordering: m.ordering,
            created: m.created,
            metadata: m.metadata,
        })
        .try_collect()
        .await?;

    // 步骤 3: 分批处理项目，每批 INDEX_BATCH_SIZE 个
    let mut uploads = Vec::new();
    let total_len = db_projects.len();
    let num_batches = total_len.div_ceil(INDEX_BATCH_SIZE);

    struct PartialGallery {
        url: String,
        featured: bool,
        ordering: i64,
    }

    for batch_idx in 0..num_batches {
        let start = batch_idx * INDEX_BATCH_SIZE;
        let end = (start + INDEX_BATCH_SIZE).min(total_len);
        let batch = &db_projects[start..end];
        let batch_project_ids: Vec<i64> =
            batch.iter().map(|x| x.id.0).collect();

        info!(
            "索引批次 {}/{}: 项目 {}-{} (共 {} 个项目)",
            batch_idx + 1,
            num_batches,
            start + 1,
            end,
            total_len
        );

        // 获取本批次的渲染图
        let mods_gallery: DashMap<ProjectId, Vec<PartialGallery>> =
            sqlx::query!(
                "
                SELECT mod_id, image_url, featured, ordering
                FROM mods_gallery
                WHERE mod_id = ANY($1)
                ",
                &batch_project_ids,
            )
            .fetch(pool)
            .try_fold(
                DashMap::new(),
                |acc: DashMap<ProjectId, Vec<PartialGallery>>, m| {
                    acc.entry(ProjectId(m.mod_id)).or_default().push(
                        PartialGallery {
                            url: m.image_url,
                            featured: m.featured.unwrap_or(false),
                            ordering: m.ordering,
                        },
                    );
                    async move { Ok(acc) }
                },
            )
            .await?;

        // 获取本批次的分类
        let categories: DashMap<ProjectId, Vec<(String, bool)>> =
            sqlx::query!(
                "
                SELECT mc.joining_mod_id mod_id, c.category name, mc.is_additional is_additional
                FROM mods_categories mc
                INNER JOIN categories c ON mc.joining_category_id = c.id
                WHERE joining_mod_id = ANY($1)
                ",
                &batch_project_ids,
            )
            .fetch(pool)
            .try_fold(
                DashMap::new(),
                |acc: DashMap<ProjectId, Vec<(String, bool)>>, m| {
                    acc.entry(ProjectId(m.mod_id))
                        .or_default()
                        .push((m.name, m.is_additional));
                    async move { Ok(acc) }
                },
            )
            .await?;

        // 获取本批次的版本
        let mut versions =
            index_versions(pool, batch_project_ids.clone()).await?;

        // 获取本批次的组织所有者
        let mods_org_owners: DashMap<ProjectId, String> = sqlx::query!(
            "
            SELECT m.id mod_id, u.username
            FROM mods m
            INNER JOIN organizations o ON o.id = m.organization_id
            INNER JOIN team_members tm ON tm.is_owner = TRUE and tm.team_id = o.team_id
            INNER JOIN users u ON u.id = tm.user_id
            WHERE m.id = ANY($1)
            ",
            &batch_project_ids,
        )
        .fetch(pool)
        .try_fold(
            DashMap::new(),
            |acc: DashMap<ProjectId, String>, m| {
                acc.insert(ProjectId(m.mod_id), m.username);
                async move { Ok(acc) }
            },
        )
        .await?;

        // 获取本批次的Team权限所有者
        let mods_team_owners: DashMap<ProjectId, String> = sqlx::query!(
            "
            SELECT m.id mod_id, u.username
            FROM mods m
            INNER JOIN team_members tm ON tm.is_owner = TRUE and tm.team_id = m.team_id
            INNER JOIN users u ON u.id = tm.user_id
            WHERE m.id = ANY($1)
            ",
            &batch_project_ids,
        )
        .fetch(pool)
        .try_fold(
            DashMap::new(),
            |acc: DashMap<ProjectId, String>, m| {
                acc.insert(ProjectId(m.mod_id), m.username);
                async move { Ok(acc) }
            },
        )
        .await?;

        // 处理本批次的项目
        for project in batch {
            let owner = if let Some((_, org_owner)) =
                mods_org_owners.remove(&project.id)
            {
                org_owner
            } else if let Some((_, team_owner)) =
                mods_team_owners.remove(&project.id)
            {
                team_owner
            } else {
                println!(
                    "未找到项目的组织所有者 {} id: {}!",
                    project.name, project.id.0
                );
                continue;
            };

            let license = match project.license.split(' ').next() {
                Some(license) => license.to_string(),
                None => project.license.clone(),
            };

            let open_source = match spdx::license_id(&license) {
                Some(id) => id.is_osi_approved(),
                _ => false,
            };

            let (featured_gallery, gallery) =
                if let Some((_, gallery)) = mods_gallery.remove(&project.id) {
                    let mut vals = Vec::new();
                    let mut featured = None;

                    for x in gallery
                        .into_iter()
                        .sorted_by(|a, b| a.ordering.cmp(&b.ordering))
                    {
                        if x.featured && featured.is_none() {
                            featured = Some(x.url);
                        } else {
                            vals.push(x.url);
                        }
                    }

                    (featured, vals)
                } else {
                    (None, vec![])
                };

            let (categories, display_categories) =
                if let Some((_, categories)) = categories.remove(&project.id) {
                    let mut vals = Vec::new();
                    let mut featured_vals = Vec::new();

                    for (val, is_additional) in categories {
                        if !is_additional {
                            featured_vals.push(val.clone());
                        }
                        vals.push(val);
                    }
                    (vals, featured_vals)
                } else {
                    (vec![], vec![])
                };

            if let Some(versions) = versions.remove(&project.id) {
                // Aggregated project loader fields
                let project_version_fields = versions
                    .iter()
                    .flat_map(|x| x.version_fields.clone())
                    .collect::<Vec<_>>();
                let aggregated_version_fields = VersionField::from_query_json(
                    project_version_fields,
                    &loader_fields,
                    &loader_field_enum_values,
                    true,
                );
                let project_loader_fields =
                    from_duplicate_version_fields(aggregated_version_fields);

                // aggregated project loaders
                let project_loaders = versions
                    .iter()
                    .flat_map(|x| x.loaders.clone())
                    .collect::<Vec<_>>();

                for version in versions {
                    let version_fields = VersionField::from_query_json(
                        version.version_fields,
                        &loader_fields,
                        &loader_field_enum_values,
                        false,
                    );
                    let unvectorized_loader_fields = version_fields
                        .iter()
                        .map(|vf| {
                            (
                                vf.field_name.clone(),
                                vf.value.serialize_internal(),
                            )
                        })
                        .collect();
                    let mut loader_fields =
                        from_duplicate_version_fields(version_fields);
                    let project_types = version.project_types;
                    let mut version_loaders = version.loaders;

                    // Uses version loaders, not project loaders.
                    let mut categories = categories.clone();
                    categories.append(&mut version_loaders.clone());

                    let display_categories = display_categories.clone();
                    categories.append(&mut version_loaders);

                    // SPECIAL BEHAVIOUR
                    // Todo: revisit.
                    // For consistency with v2 searching, we consider the loader field 'mrpack_loaders' to be a category.
                    // These were previously considered the loader, and in v2, the loader is a category for searching.
                    // So to avoid breakage or awkward conversions, we just consider those loader_fields to be categories.
                    // The loaders are kept in loader_fields as well, so that no information is lost on retrieval.
                    let mrpack_loaders = loader_fields
                        .get("mrpack_loaders")
                        .cloned()
                        .map(|x| {
                            x.into_iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    categories.extend(mrpack_loaders);

                    let software_loaders = loader_fields
                        .get("software_loaders")
                        .cloned()
                        .map(|x| {
                            x.into_iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    categories.extend(software_loaders);

                    if loader_fields.contains_key("mrpack_loaders") {
                        categories.retain(|x| *x != "mrpack");
                    }
                    if loader_fields.contains_key("software_loaders") {
                        categories.retain(|x| *x != "software");
                    }

                    // 特殊行为：
                    // 为了与 v2 搜索保持一致，我们手动将 loader 字段中的 client\_side 和 server\_side 字段
                    // 输入到单独的 loader 字段中。
                    // 尽管它们不再是 v3 字段，但 meilisearch 仍然支持 'client\_side' 和 'server\_side'。
                    let (_, v2_og_project_type) =
                        LegacyProject::get_project_type(&project_types);
                    let (client_side, server_side) =
                        v2_reroute::convert_side_types_v2(
                            &unvectorized_loader_fields,
                            Some(&v2_og_project_type),
                        );

                    if let Ok(client_side) = serde_json::to_value(client_side) {
                        loader_fields.insert(
                            "client_side".to_string(),
                            vec![client_side],
                        );
                    }
                    if let Ok(server_side) = serde_json::to_value(server_side) {
                        loader_fields.insert(
                            "server_side".to_string(),
                            vec![server_side],
                        );
                    }

                    let usp = UploadSearchProject {
                        version_id: crate::models::ids::VersionId::from(
                            version.id,
                        )
                        .to_string(),
                        project_id: crate::models::ids::ProjectId::from(
                            project.id,
                        )
                        .to_string(),
                        name: project.name.clone(),
                        summary: project.summary.clone(),
                        categories: categories.clone(),
                        display_categories: display_categories.clone(),
                        follows: project.follows,
                        downloads: project.downloads,
                        icon_url: project.icon_url.clone(),
                        author: owner.clone(),
                        date_created: project.approved,
                        created_timestamp: project.approved.timestamp(),
                        date_modified: project.updated,
                        modified_timestamp: project.updated.timestamp(),
                        license: license.clone(),
                        slug: project.slug.clone(),
                        // TODO
                        project_types,
                        gallery: gallery.clone(),
                        featured_gallery: featured_gallery.clone(),
                        open_source,
                        color: project.color.map(|x| x as u32),
                        loader_fields,
                        project_loader_fields: project_loader_fields.clone(),
                        // 'loaders' is aggregate of all versions' loaders
                        loaders: project_loaders.clone(),
                    };

                    uploads.push(usp);
                }
            }
        }

        // 批次间休息，让 API 请求有机会获取数据库连接
        if batch_idx < num_batches - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }

    Ok(uploads)
}

struct PartialVersion {
    id: VersionId,
    loaders: Vec<String>,
    project_types: Vec<String>,
    version_fields: Vec<QueryVersionField>,
}

async fn index_versions(
    pool: &PgPool,
    project_ids: Vec<i64>,
) -> Result<HashMap<ProjectId, Vec<PartialVersion>>, IndexingError> {
    let versions: HashMap<ProjectId, Vec<VersionId>> = sqlx::query!(
        "
        SELECT v.id, v.mod_id
        FROM versions v
        WHERE mod_id = ANY($1)
        ",
        &project_ids,
    )
    .fetch(pool)
    .try_fold(
        HashMap::new(),
        |mut acc: HashMap<ProjectId, Vec<VersionId>>, m| {
            acc.entry(ProjectId(m.mod_id))
                .or_default()
                .push(VersionId(m.id));
            async move { Ok(acc) }
        },
    )
    .await?;

    // Get project types, loaders
    #[derive(Default)]
    struct VersionLoaderData {
        loaders: Vec<String>,
        project_types: Vec<String>,
    }

    let all_version_ids = versions
        .iter()
        .flat_map(|(_, version_ids)| version_ids.iter())
        .map(|x| x.0)
        .collect::<Vec<i64>>();

    let loaders_ptypes: DashMap<VersionId, VersionLoaderData> = sqlx::query!(
        "
        SELECT DISTINCT version_id,
            ARRAY_AGG(DISTINCT l.loader) filter (where l.loader is not null) loaders,
            ARRAY_AGG(DISTINCT pt.name) filter (where pt.name is not null) project_types
        FROM versions v
        INNER JOIN loaders_versions lv ON v.id = lv.version_id
        INNER JOIN loaders l ON lv.loader_id = l.id
        INNER JOIN loaders_project_types lpt ON lpt.joining_loader_id = l.id
        INNER JOIN project_types pt ON pt.id = lpt.joining_project_type_id
        WHERE v.id = ANY($1)
        GROUP BY version_id
        ",
        &all_version_ids
    )
    .fetch(pool)
    .map_ok(|m| {
        let version_id = VersionId(m.version_id);

        let version_loader_data = VersionLoaderData {
            loaders: m.loaders.unwrap_or_default(),
            project_types: m.project_types.unwrap_or_default(),
        };
        (version_id, version_loader_data)
    })
    .try_collect()
    .await?;

    // Get version fields
    let version_fields: DashMap<VersionId, Vec<QueryVersionField>> =
        sqlx::query!(
            "
        SELECT version_id, field_id, int_value, enum_value, string_value
        FROM version_fields
        WHERE version_id = ANY($1)
        ",
            &all_version_ids,
        )
        .fetch(pool)
        .try_fold(
            DashMap::new(),
            |acc: DashMap<VersionId, Vec<QueryVersionField>>, m| {
                let qvf = QueryVersionField {
                    version_id: VersionId(m.version_id),
                    field_id: LoaderFieldId(m.field_id),
                    int_value: m.int_value,
                    enum_value: m.enum_value.map(LoaderFieldEnumValueId),
                    string_value: m.string_value,
                };

                acc.entry(VersionId(m.version_id)).or_default().push(qvf);
                async move { Ok(acc) }
            },
        )
        .await?;

    // Convert to partial versions
    let mut res_versions: HashMap<ProjectId, Vec<PartialVersion>> =
        HashMap::new();
    for (project_id, version_ids) in versions.iter() {
        for version_id in version_ids {
            // Extract version-specific data fetched
            // We use 'remove' as every version is only in the map once
            let version_loader_data = loaders_ptypes
                .remove(version_id)
                .map(|(_, version_loader_data)| version_loader_data)
                .unwrap_or_default();

            let version_fields = version_fields
                .remove(version_id)
                .map(|(_, version_fields)| version_fields)
                .unwrap_or_default();

            res_versions
                .entry(*project_id)
                .or_default()
                .push(PartialVersion {
                    id: *version_id,
                    loaders: version_loader_data.loaders,
                    project_types: version_loader_data.project_types,
                    version_fields,
                });
        }
    }

    Ok(res_versions)
}
