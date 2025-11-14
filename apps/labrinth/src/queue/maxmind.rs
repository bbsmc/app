use flate2::read::GzDecoder;
use log::{info, warn};
use maxminddb::geoip2::Country;
use reqwest::Client;
use std::io::{Cursor, Read};
use std::net::Ipv6Addr;
use std::time::Duration;
use tar::Archive;
use tokio::sync::RwLock;

pub struct MaxMindIndexer {
    pub reader: RwLock<Option<maxminddb::Reader<Vec<u8>>>>,
}

impl MaxMindIndexer {
    pub async fn new() -> Result<Self, reqwest::Error> {
        let reader = MaxMindIndexer::inner_index(false).await.ok().flatten();

        Ok(MaxMindIndexer {
            reader: RwLock::new(reader),
        })
    }

    pub async fn index(&self) -> Result<(), reqwest::Error> {
        let reader = MaxMindIndexer::inner_index(false).await?;

        if let Some(reader) = reader {
            let mut reader_new = self.reader.write().await;
            *reader_new = Some(reader);
        }

        Ok(())
    }

    async fn inner_index(
        should_panic: bool,
    ) -> Result<Option<maxminddb::Reader<Vec<u8>>>, reqwest::Error> {
        // 创建一个 reqwest Client 并设置超时时间
        let client = Client::builder()
            .timeout(Duration::from_secs(30)) // 增加超时时间到 30 秒
            .build()?;

        // 定义多个镜像源，按优先级尝试
        let mirror_urls = vec![
            ("国内 GitMirror 镜像", "https://raw.gitmirror.com/adysec/IP_database/main/geolite/GeoLite2-Country.mmdb"),
            ("GitHub adysec 仓库", "https://raw.githubusercontent.com/adysec/IP_database/main/geolite/GeoLite2-Country.mmdb"),
            ("GitHub P3TERX 镜像", "https://github.com/P3TERX/GeoLite.mmdb/raw/download/GeoLite2-Country.mmdb"),
        ];

        let mut response = None;
        let mut last_error = String::new();

        // 依次尝试每个镜像源
        for (name, url) in mirror_urls.iter() {
            info!("尝试从 {} 下载 MaxMind 数据库...", name);
            match client.get(*url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("成功从 {} 下载", name);
                    response = Some(resp);
                    break;
                }
                Ok(resp) => {
                    last_error = format!("{} 返回状态码: {}", name, resp.status());
                    info!("{}", last_error);
                }
                Err(e) => {
                    last_error = format!("{} 下载失败: {}", name, e);
                    info!("{}", last_error);
                }
            }
        }

        // 如果所有镜像都失败，尝试官方源（需要 license key）
        let response = if let Some(resp) = response {
            resp
        } else {
            info!("所有镜像源下载失败，尝试 MaxMind 官方源...");
            let license_key = dotenvy::var("MAXMIND_LICENSE_KEY").unwrap_or_default();

            if !license_key.is_empty() {
                let official_url = format!(
                    "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-Country&license_key={}&suffix=tar.gz",
                    license_key
                );

                match client.get(&official_url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        info!("已从 MaxMind 官方下载成功");
                        resp
                    }
                    Ok(resp) => {
                        info!("MaxMind 官方下载失败: {}", resp.status());
                        return Ok(None);
                    }
                    Err(e) => {
                        info!("MaxMind 官方下载失败: {}", e);
                        return Ok(None);
                    }
                }
            } else {
                info!("未配置 MAXMIND_LICENSE_KEY，无法使用官方源");
                info!("最后错误: {}", last_error);
                return Ok(None);
            }
        };

        info!("Downloaded maxmind database.");
        let bytes = response.bytes().await?.as_ref().to_vec();

        // 检查是否是 tar.gz 格式（官方源）还是直接的 mmdb 文件（GitHub 镜像）
        let is_tar_gz = bytes.len() > 2 && bytes[0] == 0x1f && bytes[1] == 0x8b;

        if is_tar_gz {
            // 处理 tar.gz 格式（官方源）
            let tarfile = GzDecoder::new(Cursor::new(bytes));
            let mut archive = Archive::new(tarfile);

            if let Ok(entries) = archive.entries() {
                for mut file in entries.flatten() {
                    if let Ok(path) = file.header().path() {
                        if path.extension().and_then(|x| x.to_str()) == Some("mmdb")
                        {
                            let mut buf = Vec::new();
                            file.read_to_end(&mut buf).unwrap();

                            let reader =
                                maxminddb::Reader::from_source(buf).unwrap();

                            return Ok(Some(reader));
                        }
                    }
                }
            }
        } else {
            // 直接处理 mmdb 文件（GitHub 镜像）
            match maxminddb::Reader::from_source(bytes) {
                Ok(reader) => return Ok(Some(reader)),
                Err(e) => {
                    warn!("Failed to parse maxmind database: {}", e);
                }
            }
        }

        if should_panic {
            panic!("Unable to download maxmind database- did you get a license key?")
        } else {
            warn!("Unable to download maxmind database.");

            Ok(None)
        }
    }

    pub async fn query(&self, ip: Ipv6Addr) -> Option<String> {
        let maxmind = self.reader.read().await;

        if let Some(ref maxmind) = *maxmind {
            maxmind.lookup::<Country>(ip.into()).ok().and_then(|x| {
                x.country.and_then(|x| x.iso_code.map(|x| x.to_string()))
            })
        } else {
            None
        }
    }
}
