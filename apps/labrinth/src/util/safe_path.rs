// 安全路径验证模块
// 来源于 Modrinth 上游提交 ab6e9dd5d - stricter mrpack file path validation (#4482)
// 防止路径遍历攻击和 Windows 保留设备名称利用

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// 安全的相对路径，经过验证确保：
/// 1. 不包含反斜杠
/// 2. 不包含特殊路径组件 (/, ., ..)
/// 3. 不包含 Windows 保留设备名称
#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct SafeRelativePath(String);

impl SafeRelativePath {
    /// 验证并创建安全路径
    pub fn new(path: String) -> Result<Self, String> {
        // 检查空路径
        if path.is_empty() {
            return Err("File path cannot be empty".to_string());
        }

        // 拒绝反斜杠以确保跨平台一致行为
        if path.contains('\\') {
            return Err("File path must not contain backslashes".to_string());
        }

        // 检查路径组件
        for component in path.split('/') {
            if component.is_empty() {
                // 跳过空组件（可能由连续斜杠产生）
                continue;
            }

            // 拒绝特殊组件
            if component == "." || component == ".." {
                return Err(
                    "File path cannot contain any special component or prefix"
                        .to_string(),
                );
            }

            // 检查 Windows 保留设备名称
            if is_windows_reserved_name(component) {
                return Err(
                    "File path contains a reserved Windows device name"
                        .to_string(),
                );
            }
        }

        // 检查路径是否以斜杠开头（绝对路径）
        if path.starts_with('/') {
            return Err("File path must be a relative path".to_string());
        }

        Ok(Self(path))
    }

    /// 获取内部路径字符串
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 检查是否为 Windows 保留设备名称
fn is_windows_reserved_name(name: &str) -> bool {
    let upper_name = name.to_ascii_uppercase();

    // Windows 保留的特殊 DOS 设备名称
    const RESERVED_WINDOWS_DEVICE_NAMES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5",
        "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
        "LPT6", "LPT7", "LPT8", "LPT9", "CONIN$", "CONOUT$",
    ];

    RESERVED_WINDOWS_DEVICE_NAMES.iter().any(|reserved| {
        upper_name == *reserved
            || upper_name.starts_with(&format!("{}.", reserved))
            || upper_name.starts_with(&format!("{}:", reserved))
    })
}

impl fmt::Display for SafeRelativePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for SafeRelativePath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SafeRelativePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SafeRelativePath::new(s).map_err(serde::de::Error::custom)
    }
}

impl Serialize for SafeRelativePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 规范化路径输出
        let normalized: String = self
            .0
            .split('/')
            .filter(|c| !c.is_empty() && *c != ".")
            .collect::<Vec<_>>()
            .join("/");

        if normalized.is_empty() {
            return Err(serde::ser::Error::custom("File path cannot be empty"));
        }

        normalized.serialize(serializer)
    }
}
