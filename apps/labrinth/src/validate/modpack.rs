use crate::models::pack::{PackFileHash, PackFormat};
use crate::util::validate::validation_errors_to_string;
use crate::validate::{
    SupportedGameVersions, ValidationError, ValidationResult,
};
use std::io::{Cursor, Read};
// 注意：路径验证已迁移到 SafeRelativePath 类型中，在反序列化时自动执行
// 来源于上游提交 ab6e9dd5d - stricter mrpack file path validation (#4482)
use validator::Validate;
use zip::ZipArchive;

pub struct ModpackValidator;

impl super::Validator for ModpackValidator {
    fn get_file_extensions(&self) -> &[&str] {
        &["mrpack", "zip"]
    }

    fn get_supported_loaders(&self) -> &[&str] {
        &["mrpack"]
    }

    fn get_supported_game_versions(&self) -> SupportedGameVersions {
        SupportedGameVersions::All
    }

    fn validate(
        &self,
        archive: &mut ZipArchive<Cursor<bytes::Bytes>>,
    ) -> Result<ValidationResult, ValidationError> {
        if dotenvy::var("DEV")
            .ok()
            .and_then(|x| x.parse::<bool>().ok())
            .unwrap_or(false)
        {
            return Ok(ValidationResult::Pass);
        }
        if archive.by_name("manifest.json").is_ok() {
            return Ok(ValidationResult::Pass);
            // return Ok(ValidationResult::PassWithPackDataAndFiles {
            //     format: {
            //         let mut file = if let Ok(file) =
            //             archive.by_name("manifest.json")
            //         {
            //             file
            //         } else {
            //             return Ok(ValidationResult::Warning("文件中缺少 manifest.json 文件！"));
            //         };
            //
            //         let mut contents = String::new();
            //         file.read_to_string(&mut contents)?;
            //
            //         serde_json::from_str(&contents)?
            //     },
            //     files: archive
            //         .file_names()
            //         .filter(|x| {
            //             (x.ends_with("jar") || x.ends_with("zip"))
            //                 && (x.starts_with("overrides/mods")
            //                 || x.starts_with("client-overrides/mods")
            //                 || x.starts_with("server-overrides/mods")
            //                 || x.starts_with("overrides/resourcepacks")
            //                 || x.starts_with("server-overrides/resourcepacks")
            //                 || x.starts_with("overrides/shaderpacks")
            //                 || x.starts_with("client-overrides/shaderpacks"))
            //         })
            //         .flat_map(|x| x.rsplit('/').next().map(|x| x.to_string()))
            //         .collect::<Vec<String>>(),
            // })
        }

        let pack: PackFormat = {
            let mut file =
                if let Ok(file) = archive.by_name("modrinth.index.json") {
                    file
                } else {
                    return Ok(ValidationResult::Warning(
                        "文件中缺少 modrinth.index.json 文件！",
                    ));
                };

            let mut contents = String::new();
            file.read_to_string(&mut contents)?;

            serde_json::from_str(&contents)?
        };

        pack.validate().map_err(|err| {
            ValidationError::InvalidInput(
                validation_errors_to_string(err, None).into(),
            )
        })?;

        if pack.game != "minecraft" {
            return Err(ValidationError::InvalidInput(
                format!("游戏 {0} 不存在！", pack.game).into(),
            ));
        }

        if pack.files.is_empty()
            && !archive.file_names().any(|x| x.starts_with("overrides/"))
        {
            return Err(ValidationError::InvalidInput("包中没有文件！".into()));
        }

        for file in &pack.files {
            if !file.hashes.contains_key(&PackFileHash::Sha1) {
                return Err(ValidationError::InvalidInput(
                    "所有包文件必须提供 SHA1 哈希！".into(),
                ));
            }

            if !file.hashes.contains_key(&PackFileHash::Sha512) {
                return Err(ValidationError::InvalidInput(
                    "所有包文件必须提供 SHA512 哈希！".into(),
                ));
            }

            // 路径验证已在 SafeRelativePath 反序列化时自动完成
            // 包括：空路径检查、反斜杠检查、特殊组件检查、Windows 保留名称检查
        }

        Ok(ValidationResult::PassWithPackDataAndFiles {
            format: pack,
            files: archive
                .file_names()
                .filter(|x| {
                    (x.ends_with("jar") || x.ends_with("zip"))
                        && (x.starts_with("overrides/mods")
                            || x.starts_with("client-overrides/mods")
                            || x.starts_with("server-overrides/mods")
                            || x.starts_with("overrides/resourcepacks")
                            || x.starts_with("server-overrides/resourcepacks")
                            || x.starts_with("overrides/shaderpacks")
                            || x.starts_with("client-overrides/shaderpacks"))
                })
                .flat_map(|x| x.rsplit('/').next().map(|x| x.to_string()))
                .collect::<Vec<String>>(),
        })
    }
}
