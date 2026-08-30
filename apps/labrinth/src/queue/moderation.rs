use crate::auth::checks::filter_visible_versions;
use crate::database;
use crate::database::models::notification_item::NotificationBuilder;
use crate::database::models::thread_item::ThreadMessageBuilder;
use crate::database::models::version_item::{QueryFile, QueryVersion};
use crate::database::redis::RedisPool;
use crate::models::ids::ProjectId;
use crate::models::notifications::NotificationBody;
use crate::models::pack::{PackFile, PackFileHash, PackFormat};
use crate::models::projects::ProjectStatus;
use crate::models::threads::MessageBody;
use crate::routes::ApiError;
use dashmap::DashSet;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sqlx::PgPool;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::time::Duration;
use zip::ZipArchive;

const AUTOMOD_ID: i64 = 0;

fn is_disk_only_placeholder_version(version: &QueryVersion) -> bool {
    !version.disks.is_empty()
        && version
            .files
            .iter()
            .all(|file| is_disk_placeholder_file(file, version))
}

fn is_disk_placeholder_file(file: &QueryFile, version: &QueryVersion) -> bool {
    file.id.0 < 0
        || (file.filename.is_empty()
            && file.hashes.is_empty()
            && !file.primary
            && file.size == 0
            && version.disks.iter().any(|disk| disk.url == file.url))
}

pub struct ModerationMessages {
    pub messages: Vec<ModerationMessage>,
    pub version_specific: HashMap<String, Vec<ModerationMessage>>,
}

impl ModerationMessages {
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty() && self.version_specific.is_empty()
    }

    pub fn markdown(&self, auto_mod: bool) -> String {
        let mut str = "".to_string();

        for message in &self.messages {
            str.push_str(&format!("## {}\n", message.header()));
            str.push_str(&format!("{}\n", message.body()));
            str.push('\n');
        }

        for (version_num, messages) in &self.version_specific {
            for message in messages {
                str.push_str(&format!(
                    "## 版本 {}: {}\n",
                    version_num,
                    message.header()
                ));
                str.push_str(&format!("{}\n", message.body()));
                str.push('\n');
            }
        }

        if auto_mod {
            str.push_str("<hr />\n\n");
            str.push_str("🤖 这是由自动审核系统 (AutoMod BETA) 生成的消息。如果您遇到问题，请通过邮件 support@bbsmc.net 联系我们。");
        }

        str
    }

    pub fn should_reject(&self, first_time: bool) -> bool {
        self.messages.iter().any(|x| x.rejectable(first_time))
            || self
                .version_specific
                .values()
                .any(|x| x.iter().any(|x| x.rejectable(first_time)))
    }

    pub fn approvable(&self) -> bool {
        self.messages.iter().all(|x| x.approvable())
            && self
                .version_specific
                .values()
                .all(|x| x.iter().all(|x| x.approvable()))
    }
}

pub enum ModerationMessage {
    MissingGalleryImage,
    NoPrimaryFile,
    NoSideTypes,
    PackFilesNotAllowed {
        files: HashMap<String, IdentifiedFile>,
        incomplete: bool,
    },
    MissingLicense,
    MissingCustomLicenseUrl {
        license: String,
    },
}

impl ModerationMessage {
    pub fn rejectable(&self, first_time: bool) -> bool {
        match self {
            ModerationMessage::NoPrimaryFile => true,
            ModerationMessage::PackFilesNotAllowed { files, incomplete } => {
                (!incomplete || first_time)
                    && files.values().any(|x| match x.status {
                        ApprovalType::Yes => false,
                        ApprovalType::WithAttributionAndSource => false,
                        ApprovalType::WithAttribution => false,
                        ApprovalType::No => first_time,
                        ApprovalType::PermanentNo => true,
                        ApprovalType::Unidentified => first_time,
                    })
            }
            ModerationMessage::MissingGalleryImage => true,
            ModerationMessage::MissingLicense => true,
            ModerationMessage::MissingCustomLicenseUrl { .. } => true,
            ModerationMessage::NoSideTypes => true,
        }
    }

    pub fn approvable(&self) -> bool {
        match self {
            ModerationMessage::NoPrimaryFile => false,
            ModerationMessage::PackFilesNotAllowed { files, .. } => {
                files.values().all(|x| x.status.approved())
            }
            ModerationMessage::MissingGalleryImage => false,
            ModerationMessage::MissingLicense => false,
            ModerationMessage::MissingCustomLicenseUrl { .. } => false,
            ModerationMessage::NoSideTypes => false,
        }
    }

    pub fn header(&self) -> &'static str {
        match self {
            ModerationMessage::NoPrimaryFile => "缺少主文件",
            ModerationMessage::PackFilesNotAllowed { .. } => {
                "包含受版权保护的内容"
            }
            ModerationMessage::MissingGalleryImage => "缺少展示图片",
            ModerationMessage::MissingLicense => "缺少许可证",
            ModerationMessage::MissingCustomLicenseUrl { .. } => {
                "缺少许可证链接"
            }
            ModerationMessage::NoSideTypes => "缺少运行环境信息",
        }
    }

    pub fn body(&self) -> String {
        match self {
            ModerationMessage::NoPrimaryFile => "请为此版本附加一个文件。所有项目的版本都必须关联至少一个文件。\n".to_string(),
            ModerationMessage::PackFilesNotAllowed { files, .. } => {
                let mut str = "".to_string();
                str.push_str("此整合包重新分发了受版权保护的内容。请参阅[内容规则](https://bbsmc.net/legal/rules)了解更多信息。\n\n");

                let mut attribute_mods = Vec::new();
                let mut no_mods = Vec::new();
                let mut permanent_no_mods = Vec::new();
                let mut unidentified_mods = Vec::new();
                for (_, approval) in files.iter() {
                    match approval.status {
                        ApprovalType::Yes | ApprovalType::WithAttributionAndSource  => {}
                        ApprovalType::WithAttribution => attribute_mods.push(&approval.file_name),
                        ApprovalType::No => no_mods.push(&approval.file_name),
                        ApprovalType::PermanentNo => permanent_no_mods.push(&approval.file_name),
                        ApprovalType::Unidentified => unidentified_mods.push(&approval.file_name),
                    }
                }

                fn print_mods(projects: Vec<&String>, headline: &str, val: &mut String) {
                    if projects.is_empty() { return }

                    val.push_str(&format!("{headline}\n\n"));

                    for project in &projects {
                        let additional_text = if project.contains("ftb-quests") {
                            Some("Heracles")
                        } else if project.contains("ftb-ranks") || project.contains("ftb-essentials") {
                            Some("Prometheus")
                        } else if project.contains("ftb-teams") {
                            Some("Argonauts")
                        } else if project.contains("ftb-chunks") {
                            Some("Cadmus")
                        } else {
                            None
                        };

                        val.push_str(&if let Some(additional_text) = additional_text {
                            format!("- {project}（建议使用 [{additional_text}](https://bbsmc.net/mod/{}) 替代）\n", additional_text.to_lowercase())
                        } else {
                            format!("- {project}\n")
                        })
                    }

                    if !projects.is_empty() {
                        val.push('\n');
                    }
                }

                print_mods(attribute_mods, "以下内容需要注明来源，您必须在整合包简介或版本更新日志中链接到原始内容页面（例如链接到该模组的 CurseForge 页面）：", &mut str);
                print_mods(no_mods, "由于许可证限制，以下内容不允许在整合包中使用。请直接联系作者获取授权，或从您的整合包中移除这些内容：", &mut str);
                print_mods(permanent_no_mods, "以下内容不允许在整合包中使用，无论是否获得授权。这可能是因为违反了内容规则，或者作者已明确拒绝授权。请从您的整合包中移除这些内容：", &mut str);
                print_mods(unidentified_mods, "以下内容无法识别来源。请提供其来源证明以及您获得使用许可的证明：", &mut str);

                str
            },
            ModerationMessage::MissingGalleryImage => "我们要求资源包在「图库」中上传展示图片（也可以在「简介」中展示），以便清晰有效地向用户展示您的资源包内容，详见[内容规则](https://bbsmc.net/legal/rules#general-expectations) 2.1 节。\n\n请注意以下事项：\n\n- 设置一张最能代表您资源包的精选图片。\n- 确保所有图片都有准确的标题，并可选择在图片描述中提供详细说明。\n- 建议将简介中的相关图片也上传到「图库」标签中以获得最佳展示效果。".to_string(),
            ModerationMessage::MissingLicense => "您的项目必须先选择一个许可证才能公开发布。设置许可证对于保护您的权益以及让他人按照您的意愿使用您的内容非常重要。更多信息请参阅[内容规则](https://bbsmc.net/legal/rules)。".to_string(),
            ModerationMessage::MissingCustomLicenseUrl { license } => format!("您选择了许可证 \"{license}\"，但未提供有效的许可证链接。使用自定义许可证时，您必须在许可证链接字段中提供指向该许可证的直接链接。"),
            ModerationMessage::NoSideTypes => "您的项目的运行环境目前两端均设置为「未知」。请设置准确的运行环境类型！".to_string(),
        }
    }
}

pub struct AutomatedModerationQueue {
    pub projects: DashSet<ProjectId>,
}

impl Default for AutomatedModerationQueue {
    fn default() -> Self {
        Self {
            projects: DashSet::new(),
        }
    }
}

impl AutomatedModerationQueue {
    pub async fn task(&self, pool: PgPool, redis: RedisPool) {
        loop {
            let projects = self.projects.clone();
            self.projects.clear();

            for project in projects {
                async {
                    let project =
                        database::Project::get_id((project).into(), &pool, &redis).await?;

                    if let Some(project) = project {
                        let res = async {
                            let mut mod_messages = ModerationMessages {
                                messages: vec![],
                                version_specific: HashMap::new(),
                            };

                            // 跟进上游 ef04dcc37：检查 environment 单字段（取代旧 4 字段）
                            if project.project_types.iter().any(|x| ["mod", "modpack"].contains(&&**x)) && !project.aggregate_version_fields.iter().any(|x| x.field_name == "environment") {
                                mod_messages.messages.push(ModerationMessage::NoSideTypes);
                            }

                            // 如果没有设置许可证，自动设置为"保留所有权益"
                            if project.inner.license == "LicenseRef-Unknown" || project.inner.license == "LicenseRef-" {
                                sqlx::query!(
                                    "UPDATE mods SET license = 'LicenseRef-All-Rights-Reserved' WHERE id = $1",
                                    project.inner.id.0
                                )
                                .execute(&pool)
                                .await?;

                                // 清除项目缓存（包括版本缓存）
                                database::models::Project::clear_cache(
                                    project.inner.id,
                                    project.inner.slug.clone(),
                                    Some(true),
                                    &redis,
                                )
                                .await?;
                            } else if project.inner.license.starts_with("LicenseRef-") && project.inner.license != "LicenseRef-All-Rights-Reserved" && project.inner.license_url.is_none() {
                                mod_messages.messages.push(ModerationMessage::MissingCustomLicenseUrl { license: project.inner.license.clone() });
                            }

                            if (project.project_types.contains(&"resourcepack".to_string()) || project.project_types.contains(&"shader".to_string())) &&
                                project.gallery_items.is_empty() &&
                                !project.categories.contains(&"audio".to_string()) &&
                                !project.categories.contains(&"locale".to_string())
                            {
                                mod_messages.messages.push(ModerationMessage::MissingGalleryImage);
                            }

                            let versions =
                                database::Version::get_many(&project.versions, &pool, &redis)
                                    .await?
                                    .into_iter()
                                    // we only support modpacks at this time
                                    .filter(|x| x.project_types.contains(&"modpack".to_string()))
                                    .collect::<Vec<_>>();

                            for version in versions {
                                // 跳过纯网盘版本。数据库查询层会给纯网盘版本补一个
                                // url=网盘链接的占位文件，不能把它当作可解析的站内 zip。
                                if is_disk_only_placeholder_version(&version) {
                                    log::debug!("跳过云盘版本 {} 的文件检查", version.inner.id.0);
                                    continue;
                                }

                                let primary_file = version.files.iter().find_or_first(|x| x.primary);

                                if let Some(primary_file) = primary_file {
                                    // 检查文件 URL 是否有效
                                    // 如果 URL 为空或无效，报告为缺少主文件
                                    if primary_file.url.is_empty() || !primary_file.url.starts_with("http") {
                                        log::warn!(
                                            "版本 {} 的文件 URL 无效或为空: '{}'",
                                            version.inner.id.0,
                                            primary_file.url
                                        );
                                        let val = mod_messages.version_specific.entry(version.inner.version_number.clone()).or_default();
                                        val.push(ModerationMessage::NoPrimaryFile);
                                        continue;
                                    }

                                    let data = reqwest::get(&primary_file.url).await?.bytes().await?;

                                    let reader = Cursor::new(data);
                                    let mut zip = ZipArchive::new(reader)?;

                                    let pack: PackFormat = {
                                        let mut file =
                                            if let Ok(file) = zip.by_name("modrinth.index.json") {
                                                file
                                            } else {
                                                continue;
                                            };

                                        let mut contents = String::new();
                                        file.read_to_string(&mut contents)?;

                                        serde_json::from_str(&contents)?
                                    };

                                    // sha1, pack file, file path, murmur
                                    let mut hashes: Vec<(
                                        String,
                                        Option<PackFile>,
                                        String,
                                        Option<u32>,
                                    )> = pack
                                        .files
                                        .clone()
                                        .into_iter()
                                        .flat_map(|x| {
                                            let hash = x.hashes.get(&PackFileHash::Sha1);

                                            if let Some(hash) = hash {
                                                let path = x.path.to_string();
                                                Some((hash.clone(), Some(x), path, None))
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();

                                    for i in 0..zip.len() {
                                        let mut file = zip.by_index(i)?;

                                        if file.name().starts_with("overrides/mods")
                                            || file.name().starts_with("client-overrides/mods")
                                            || file.name().starts_with("server-overrides/mods")
                                            || file.name().starts_with("overrides/shaderpacks")
                                            || file.name().starts_with("client-overrides/shaderpacks")
                                            || file.name().starts_with("overrides/resourcepacks")
                                            || file.name().starts_with("client-overrides/resourcepacks")
                                        {
                                            if file.name().matches('/').count() > 2 || file.name().ends_with(".txt") {
                                                continue;
                                            }

                                            let mut contents = Vec::new();
                                            file.read_to_end(&mut contents)?;

                                            let hash = format!("{:x}", sha1::Sha1::digest(&contents));
                                            let murmur = hash_flame_murmur32(contents);

                                            hashes.push((
                                                hash,
                                                None,
                                                file.name().to_string(),
                                                Some(murmur),
                                            ));
                                        }
                                    }

                                    let files = database::models::Version::get_files_from_hash(
                                        "sha1".to_string(),
                                        &hashes.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
                                        &pool,
                                        &redis,
                                    )
                                        .await?;

                                    let version_ids =
                                        files.iter().map(|x| x.version_id).collect::<Vec<_>>();
                                    let versions_data = filter_visible_versions(
                                        database::models::Version::get_many(
                                            &version_ids,
                                            &pool,
                                            &redis,
                                        )
                                            .await?,
                                        &None,
                                        &pool,
                                        &redis,
                                    )
                                        .await?;

                                    let mut final_hashes = HashMap::new();

                                    for version in versions_data {
                                        for file in
                                        files.iter().filter(|x| x.version_id == version.id.into())
                                        {
                                            if let Some(hash) = file.hashes.get("sha1")
                                                && let Some((index, (sha1, _, file_name, _))) = hashes
                                                    .iter()
                                                    .enumerate()
                                                    .find(|(_, (value, _, _, _))| value == hash)
                                                {
                                                    final_hashes
                                                        .insert(sha1.clone(), IdentifiedFile { status: ApprovalType::Yes, file_name: file_name.clone() });

                                                    hashes.remove(index);
                                                }
                                        }
                                    }

                                    // 所有文件都在 BBSMC 上，不需要发送审核消息
                                    if hashes.is_empty() {
                                        sqlx::query!(
                                            "
                                            UPDATE files
                                            SET metadata = $1
                                            WHERE id = $2
                                            ",
                                            serde_json::to_value(&MissingMetadata {
                                                identified: final_hashes,
                                                flame_files: Default::default(),
                                                unknown_files: Default::default(),
                                            })?,
                                            primary_file.id.0
                                        )
                                            .execute(&pool)
                                            .await?;

                                        continue;
                                    }

                                    let rows = sqlx::query!(
                                        "
                                        SELECT encode(mef.sha1, 'escape') sha1, mel.status status
                                        FROM moderation_external_files mef
                                        INNER JOIN moderation_external_licenses mel ON mef.external_license_id = mel.id
                                        WHERE mef.sha1 = ANY($1)
                                        ",
                                        &hashes.iter().map(|x| x.0.as_bytes().to_vec()).collect::<Vec<_>>()
                                    )
                                        .fetch_all(&pool)
                                        .await?;

                                    for row in rows {
                                        if let Some(sha1) = row.sha1
                                            && let Some((index, (sha1, _, file_name, _))) = hashes.iter().enumerate().find(|(_, (value, _, _, _))| value == &sha1) {
                                                final_hashes.insert(sha1.clone(), IdentifiedFile { file_name: file_name.clone(), status: ApprovalType::from_string(&row.status).unwrap_or(ApprovalType::Unidentified) });
                                                hashes.remove(index);
                                            }
                                    }

                                    if hashes.is_empty() {
                                        let metadata = MissingMetadata {
                                            identified: final_hashes,
                                            flame_files: Default::default(),
                                            unknown_files: Default::default(),
                                        };

                                        sqlx::query!(
                                            "
                                            UPDATE files
                                            SET metadata = $1
                                            WHERE id = $2
                                            ",
                                            serde_json::to_value(&metadata)?,
                                            primary_file.id.0
                                        )
                                            .execute(&pool)
                                            .await?;

                                        if metadata.identified.values().any(|x| x.status != ApprovalType::Yes && x.status != ApprovalType::WithAttributionAndSource) {
                                            let val = mod_messages.version_specific.entry(version.inner.version_number).or_default();
                                            val.push(ModerationMessage::PackFilesNotAllowed {files: metadata.identified, incomplete: false });
                                        }
                                        continue;
                                    }

                                    let flame_anvil_url = dotenvy::var("FLAME_ANVIL_URL")?;

                                    // 如果 FLAME_ANVIL_URL 设置为 "none"，跳过 CurseForge 检查
                                    if flame_anvil_url == "none" || flame_anvil_url.is_empty() {
                                        continue;
                                    }

                                    let client = reqwest::Client::new();
                                    let res = client
                                        .post(format!("{}/v1/fingerprints", flame_anvil_url))
                                        .json(&serde_json::json!({
                                        "fingerprints": hashes.iter().filter_map(|x| x.3).collect::<Vec<u32>>()
                                    }))
                                        .send()
                                        .await?.text()
                                        .await?;

                                    let flame_hashes = serde_json::from_str::<FlameResponse<FingerprintResponse>>(&res)?
                                        .data
                                        .exact_matches
                                        .into_iter()
                                        .map(|x| x.file)
                                        .collect::<Vec<_>>();

                                    let mut flame_files = Vec::new();

                                    for file in flame_hashes {
                                        let hash = file
                                            .hashes
                                            .iter()
                                            .find(|x| x.algo == 1)
                                            .map(|x| x.value.clone());

                                        if let Some(hash) = hash  {
                                            flame_files.push((hash, file.mod_id))
                                        }
                                    }

                                    let rows = sqlx::query!(
                                        "
                                        SELECT mel.id, mel.flame_project_id, mel.status status
                                        FROM moderation_external_licenses mel
                                        WHERE mel.flame_project_id = ANY($1)
                                        ",
                                        &flame_files.iter().map(|x| x.1 as i32).collect::<Vec<_>>()
                                    )
                                        .fetch_all(&pool).await?;

                                    let mut insert_hashes = Vec::new();
                                    let mut insert_ids = Vec::new();

                                    for row in rows {
                                        if let Some((curse_index, (hash, _flame_id))) = flame_files.iter().enumerate().find(|(_, x)| Some(x.1 as i32) == row.flame_project_id)
                                            && let Some((index, (sha1, _, file_name, _))) = hashes.iter().enumerate().find(|(_, (value, _, _, _))| value == hash) {
                                                final_hashes.insert(sha1.clone(), IdentifiedFile {
                                                    file_name: file_name.clone(),
                                                    status: ApprovalType::from_string(&row.status).unwrap_or(ApprovalType::Unidentified),
                                                });

                                                insert_hashes.push(hash.clone().as_bytes().to_vec());
                                                insert_ids.push(row.id);

                                                hashes.remove(index);
                                                flame_files.remove(curse_index);
                                            }
                                    }

                                    if !insert_ids.is_empty() && !insert_hashes.is_empty() {
                                        sqlx::query!(
                                            "
                                            INSERT INTO moderation_external_files (sha1, external_license_id)
                                            SELECT * FROM UNNEST ($1::bytea[], $2::bigint[])
                                            ON CONFLICT (sha1) DO NOTHING
                                            ",
                                            &insert_hashes[..],
                                            &insert_ids[..]
                                        )
                                            .execute(&pool)
                                            .await?;
                                    }

                                    if hashes.is_empty() {
                                        let metadata = MissingMetadata {
                                            identified: final_hashes,
                                            flame_files: Default::default(),
                                            unknown_files: Default::default(),
                                        };

                                        sqlx::query!(
                                            "
                                            UPDATE files
                                            SET metadata = $1
                                            WHERE id = $2
                                            ",
                                            serde_json::to_value(&metadata)?,
                                            primary_file.id.0
                                        )
                                            .execute(&pool)
                                            .await?;

                                        if metadata.identified.values().any(|x| x.status != ApprovalType::Yes && x.status != ApprovalType::WithAttributionAndSource) {
                                            let val = mod_messages.version_specific.entry(version.inner.version_number).or_default();
                                            val.push(ModerationMessage::PackFilesNotAllowed {files: metadata.identified, incomplete: false });
                                        }

                                        continue;
                                    }

                                    let flame_projects  = if flame_files.is_empty() {
                                        Vec::new()
                                    } else {
                                        let flame_anvil_url = dotenvy::var("FLAME_ANVIL_URL")?;

                                        // 如果 FLAME_ANVIL_URL 设置为 "none"，跳过获取项目信息
                                        if flame_anvil_url == "none" || flame_anvil_url.is_empty() {
                                            Vec::new()
                                        } else {
                                            let res = client
                                                .post(format!("{}v1/mods", flame_anvil_url))
                                                .json(&serde_json::json!({
                                                    "modIds": flame_files.iter().map(|x| x.1).collect::<Vec<_>>()
                                                }))
                                                .send()
                                                .await?
                                                .text()
                                                .await?;

                                            serde_json::from_str::<FlameResponse<Vec<FlameProject>>>(&res)?.data
                                        }
                                    };

                                    let mut missing_metadata = MissingMetadata {
                                        identified: final_hashes,
                                        flame_files: HashMap::new(),
                                        unknown_files: HashMap::new(),
                                    };

                                    for (sha1, _pack_file, file_name, _mumur2) in hashes {
                                        let flame_file = flame_files.iter().find(|x| x.0 == sha1);

                                        if let Some((_, flame_project_id)) = flame_file
                                            && let Some(project) = flame_projects.iter().find(|x| &x.id == flame_project_id) {
                                                missing_metadata.flame_files.insert(sha1, MissingMetadataFlame {
                                                    title: project.name.clone(),
                                                    file_name,
                                                    url: project.links.website_url.clone(),
                                                    id: *flame_project_id,
                                                });

                                                continue;
                                            }

                                        missing_metadata.unknown_files.insert(sha1, file_name);
                                    }

                                    sqlx::query!(
                                        "
                                        UPDATE files
                                        SET metadata = $1
                                        WHERE id = $2
                                        ",
                                        serde_json::to_value(&missing_metadata)?,
                                        primary_file.id.0
                                    )
                                        .execute(&pool)
                                        .await?;

                                    if missing_metadata.identified.values().any(|x| x.status != ApprovalType::Yes && x.status != ApprovalType::WithAttributionAndSource) {
                                        let val = mod_messages.version_specific.entry(version.inner.version_number).or_default();
                                        val.push(ModerationMessage::PackFilesNotAllowed {files: missing_metadata.identified, incomplete: true });
                                    }
                                } else {
                                    let val = mod_messages.version_specific.entry(version.inner.version_number).or_default();
                                    val.push(ModerationMessage::NoPrimaryFile);
                                }
                            }

                            if !mod_messages.is_empty() {
                                let first_time = database::models::Thread::get(project.thread_id, &pool).await?
                                    .map(|x| x.messages.iter().all(|x| x.author_id == Some(database::models::UserId(AUTOMOD_ID)) || x.hide_identity))
                                    .unwrap_or(true);

                                let mut transaction = pool.begin().await?;
                                let id = ThreadMessageBuilder {
                                    author_id: Some(database::models::UserId(AUTOMOD_ID)),
                                    body: MessageBody::Text {
                                        body: mod_messages.markdown(true),
                                        private: false,
                                        replying_to: None,
                                        associated_images: vec![],
                                    },
                                    thread_id: project.thread_id,
                                    hide_identity: false,
                                }
                                    .insert(&mut transaction)
                                    .await?;

                                let members = database::models::TeamMember::get_from_team_full(
                                    project.inner.team_id,
                                    &pool,
                                    &redis,
                                )
                                    .await?;

                                if mod_messages.should_reject(first_time) {
                                    ThreadMessageBuilder {
                                        author_id: Some(database::models::UserId(AUTOMOD_ID)),
                                        body: MessageBody::StatusChange {
                                            new_status: ProjectStatus::Rejected,
                                            old_status: project.inner.status,
                                        },
                                        thread_id: project.thread_id,
                                        hide_identity: false,
                                    }
                                        .insert(&mut transaction)
                                        .await?;

                                    NotificationBuilder {
                                        body: NotificationBody::StatusChange {
                                            project_id: project.inner.id.into(),
                                            old_status: project.inner.status,
                                            new_status: ProjectStatus::Rejected,
                                        },
                                    }
                                        .insert_many(members.into_iter().map(|x| x.user_id).collect(), &mut transaction, &redis)
                                        .await?;

                                    if let Ok(webhook_url) = dotenvy::var("MODERATION_SLACK_WEBHOOK") {
                                        crate::util::webhook::send_slack_webhook(
                                            project.inner.id.into(),
                                            &pool,
                                            &redis,
                                            webhook_url,
                                            Some(
                                                format!(
                                                    "*<{}/user/AutoMod|AutoMod>* 将项目状态从 *{}* 更改为 *已拒绝*",
                                                    dotenvy::var("SITE_URL")?,
                                                    &project.inner.status.as_friendly_str(),
                                                )
                                                    .to_string(),
                                            ),
                                        )
                                            .await
                                            .ok();
                                    }

                                    sqlx::query!(
                                        "
                                        UPDATE mods
                                        SET status = 'rejected'
                                        WHERE id = $1
                                        ",
                                        project.inner.id.0
                                    )
                                        .execute(&pool)
                                        .await?;

                                    database::models::Project::clear_cache(
                                        project.inner.id,
                                        project.inner.slug.clone(),
                                        None,
                                        &redis,
                                    )
                                        .await?;
                                } else {
                                    NotificationBuilder {
                                        body: NotificationBody::ModeratorMessage {
                                            thread_id: project.thread_id.into(),
                                            message_id: id.into(),
                                            project_id: Some(project.inner.id.into()),
                                            report_id: None,
                                        },
                                    }
                                        .insert_many(
                                            members.into_iter().map(|x| x.user_id).collect(),
                                            &mut transaction,
                                            &redis,
                                        )
                                        .await?;
                                }

                                transaction.commit().await?;
                            }

                            Ok::<(), ApiError>(())
                        }.await;

                        if let Err(err) = res {
                            let err = err.as_api_error();

                            let mut str = String::new();
                            str.push_str("## AutoMod 内部错误\n\n");
                            str.push_str(&format!("错误代码: {}\n\n", err.error));
                            str.push_str(&format!("错误描述: {}\n\n", err.description));

                            let mut transaction = pool.begin().await?;
                            ThreadMessageBuilder {
                                author_id: Some(database::models::UserId(AUTOMOD_ID)),
                                body: MessageBody::Text {
                                    body: str,
                                    private: true,
                                    replying_to: None,
                                    associated_images: vec![],
                                },
                                thread_id: project.thread_id,
                                hide_identity: false,
                            }
                                .insert(&mut transaction)
                                .await?;
                            transaction.commit().await?;
                        }
                    }

                    Ok::<(), ApiError>(())
                }.await.ok();
            }

            tokio::time::sleep(Duration::from_secs(5)).await
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MissingMetadata {
    pub identified: HashMap<String, IdentifiedFile>,
    pub flame_files: HashMap<String, MissingMetadataFlame>,
    pub unknown_files: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct IdentifiedFile {
    pub file_name: String,
    pub status: ApprovalType,
}

#[derive(Serialize, Deserialize)]
pub struct MissingMetadataFlame {
    pub title: String,
    pub file_name: String,
    pub url: String,
    pub id: u32,
}

#[derive(Deserialize, Serialize, Copy, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalType {
    Yes,
    WithAttributionAndSource,
    WithAttribution,
    No,
    PermanentNo,
    Unidentified,
}

impl ApprovalType {
    fn approved(&self) -> bool {
        match self {
            ApprovalType::Yes => true,
            ApprovalType::WithAttributionAndSource => true,
            ApprovalType::WithAttribution => true,
            ApprovalType::No => false,
            ApprovalType::PermanentNo => false,
            ApprovalType::Unidentified => false,
        }
    }

    pub fn from_string(string: &str) -> Option<Self> {
        match string {
            "yes" => Some(ApprovalType::Yes),
            "with-attribution-and-source" => {
                Some(ApprovalType::WithAttributionAndSource)
            }
            "with-attribution" => Some(ApprovalType::WithAttribution),
            "no" => Some(ApprovalType::No),
            "permanent-no" => Some(ApprovalType::PermanentNo),
            "unidentified" => Some(ApprovalType::Unidentified),
            _ => None,
        }
    }

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ApprovalType::Yes => "yes",
            ApprovalType::WithAttributionAndSource => {
                "with-attribution-and-source"
            }
            ApprovalType::WithAttribution => "with-attribution",
            ApprovalType::No => "no",
            ApprovalType::PermanentNo => "permanent-no",
            ApprovalType::Unidentified => "unidentified",
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct FlameResponse<T> {
    pub data: T,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintResponse {
    pub exact_matches: Vec<FingerprintMatch>,
}

#[derive(Deserialize, Serialize)]
pub struct FingerprintMatch {
    pub id: u32,
    pub file: FlameFile,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlameFile {
    pub id: u32,
    pub mod_id: u32,
    pub hashes: Vec<FlameFileHash>,
    pub file_fingerprint: u32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FlameFileHash {
    pub value: String,
    pub algo: u32,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlameProject {
    pub id: u32,
    pub name: String,
    pub slug: String,
    pub links: FlameLinks,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlameLinks {
    pub website_url: String,
}

fn hash_flame_murmur32(input: Vec<u8>) -> u32 {
    murmur2::murmur2(
        &input
            .into_iter()
            .filter(|x| *x != 9 && *x != 10 && *x != 13 && *x != 32)
            .collect::<Vec<u8>>(),
        1,
    )
}
