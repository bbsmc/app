use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use validator::{ValidationErrors, ValidationErrorsKind};

use crate::models::pats::Scopes;

lazy_static! {
    pub static ref RE_URL_SAFE: Regex =
        Regex::new(r#"^[\p{L}\p{N}!@$()`.+,_"-]*$"#).unwrap();
}

//TODO: 为了确保可读性，只打印第一个错误，这可能需要在将来扩展！
pub fn validation_errors_to_string(
    errors: ValidationErrors,
    adder: Option<String>,
) -> String {
    let mut output = String::new();

    let map = errors.into_errors();

    if let Some((field, error)) = map.iter().next() {
        return match error {
            ValidationErrorsKind::Struct(errors) => {
                validation_errors_to_string(
                    *errors.clone(),
                    Some(format!("项目 {field}")),
                )
            }
            ValidationErrorsKind::List(list) => {
                if let Some((index, errors)) = list.iter().next() {
                    output.push_str(&validation_errors_to_string(
                        *errors.clone(),
                        Some(format!("列表 {field} 中第 {index} 项")),
                    ));
                }

                output
            }
            ValidationErrorsKind::Field(errors) => {
                if let Some(error) = errors.first() {
                    // 优先使用自定义消息
                    if let Some(msg) = &error.message {
                        return msg.to_string();
                    }

                    // 字段名翻译
                    let field_str = field.as_ref();
                    let field_name = match field_str {
                        "username" => "用户名",
                        "name" | "real_name" => "姓名",
                        "slug" => "标识ID",
                        "summary" => "简介",
                        "contact_info" => "联系方式",
                        "id_card_number" => "身份证号",
                        "portfolio_links" => "作品链接",
                        "application_reason" => "申请理由",
                        _ => field_str,
                    };

                    // 错误码翻译
                    let error_reason = match error.code.as_ref() {
                        "length" => {
                            // 尝试获取具体的长度限制
                            let min = error
                                .params
                                .get("min")
                                .and_then(|v| v.as_u64());
                            let max = error
                                .params
                                .get("max")
                                .and_then(|v| v.as_u64());
                            match (min, max) {
                                (Some(min), Some(max)) => format!(
                                    "长度必须在 {} 到 {} 之间",
                                    min, max
                                ),
                                (Some(min), None) => {
                                    format!("长度不能少于 {} 个字符", min)
                                }
                                (None, Some(max)) => {
                                    format!("长度不能超过 {} 个字符", max)
                                }
                                _ => "长度不符合要求".to_string(),
                            }
                        }
                        "range" => "取值范围不正确".to_string(),
                        "email" => "邮箱格式不正确".to_string(),
                        "url" => "链接格式不正确".to_string(),
                        "regex" => "格式不正确".to_string(),
                        code => code.to_string(),
                    };

                    if let Some(adder) = adder {
                        output.push_str(&format!(
                            "{} {} {}",
                            field_name, adder, error_reason
                        ));
                    } else {
                        if field_str == "username" {
                            output.push_str("建议使用您的Minecraft正版ID,支持使用各类语言文字，允许：字母、数字、下划线 _、连字符 -");
                        }
                        output.push_str(&format!(
                            "{}{}",
                            field_name, error_reason
                        ));
                    }
                }

                output
            }
        };
    }

    String::new()
}

pub fn validate_deps(
    values: &[crate::models::projects::Dependency],
) -> Result<(), validator::ValidationError> {
    if values
        .iter()
        .duplicates_by(|x| {
            format!(
                "{}-{}-{}",
                x.version_id
                    .unwrap_or(crate::models::projects::VersionId(0)),
                x.project_id
                    .unwrap_or(crate::models::projects::ProjectId(0)),
                x.file_name.as_deref().unwrap_or_default()
            )
        })
        .next()
        .is_some()
    {
        return Err(validator::ValidationError::new("重复依赖"));
    }

    Ok(())
}

pub fn validate_url(value: &str) -> Result<(), validator::ValidationError> {
    let url = url::Url::parse(value)
        .ok()
        .ok_or_else(|| validator::ValidationError::new("无效的 URL"))?;

    if url.scheme() != "https" {
        return Err(validator::ValidationError::new("URL 必须是 https"));
    }

    Ok(())
}

pub fn validate_url_hashmap_optional_values(
    values: &std::collections::HashMap<String, Option<String>>,
) -> Result<(), validator::ValidationError> {
    for value in values.values().flatten() {
        validate_url(value)?;
    }

    Ok(())
}

pub fn validate_url_hashmap_values(
    values: &std::collections::HashMap<String, String>,
) -> Result<(), validator::ValidationError> {
    for value in values.values() {
        validate_url(value)?;
    }

    Ok(())
}

pub fn validate_no_restricted_scopes(
    value: &Scopes,
) -> Result<(), validator::ValidationError> {
    if value.is_restricted() {
        return Err(validator::ValidationError::new("不允许受限范围"));
    }

    Ok(())
}

pub fn validate_name(value: &str) -> Result<(), validator::ValidationError> {
    if value.trim().is_empty() {
        return Err(validator::ValidationError::new("名称不能仅包含空格"));
    }

    Ok(())
}
