use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::database::models::featured_item::DBFeaturedItem;
use crate::models::ids::ProjectId;

#[derive(Serialize, Clone, Copy, Debug)]
pub struct FeaturedSection {
    pub key: &'static str,
    pub name: &'static str,
}

pub const FEATURED_SECTIONS: &[FeaturedSection] = &[
    FeaturedSection {
        key: "home",
        name: "首页轮播",
    },
    FeaturedSection {
        key: "search_mod",
        name: "模组页",
    },
    FeaturedSection {
        key: "search_plugin",
        name: "插件页",
    },
    FeaturedSection {
        key: "search_modpack",
        name: "整合包页",
    },
    FeaturedSection {
        key: "search_shader",
        name: "光影页",
    },
    FeaturedSection {
        key: "search_resourcepack",
        name: "资源包页",
    },
    FeaturedSection {
        key: "search_language",
        name: "汉化页",
    },
    FeaturedSection {
        key: "search_datapack",
        name: "数据包页",
    },
    FeaturedSection {
        key: "search_map",
        name: "地图页",
    },
    FeaturedSection {
        key: "search_software",
        name: "工具页",
    },
];

pub fn is_valid_section(key: &str) -> bool {
    FEATURED_SECTIONS.iter().any(|s| s.key == key)
}

pub fn section_keys_joined() -> String {
    FEATURED_SECTIONS
        .iter()
        .map(|s| s.key)
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FeaturedItem {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub badge: String,
    pub image_url: String,
    pub link: String,
    pub project_id: Option<ProjectId>,
}

impl From<DBFeaturedItem> for FeaturedItem {
    fn from(item: DBFeaturedItem) -> Self {
        Self {
            id: item.id,
            title: item.title,
            description: item.description,
            badge: item.badge,
            image_url: item.image_url,
            link: item.link,
            project_id: item.project_id.map(|id| ProjectId(id as u64)),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct FeaturedItemAdmin {
    pub id: i64,
    pub section: String,
    pub title: String,
    pub description: String,
    pub badge: String,
    pub image_url: String,
    pub link: String,
    pub project_id: Option<ProjectId>,
    pub sort_order: i32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<DBFeaturedItem> for FeaturedItemAdmin {
    fn from(item: DBFeaturedItem) -> Self {
        Self {
            id: item.id,
            section: item.section,
            title: item.title,
            description: item.description,
            badge: item.badge,
            image_url: item.image_url,
            link: item.link,
            project_id: item.project_id.map(|id| ProjectId(id as u64)),
            sort_order: item.sort_order,
            enabled: item.enabled,
            created_at: item.created_at,
            updated_at: item.updated_at,
        }
    }
}
