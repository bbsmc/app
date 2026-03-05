use log::{info, warn};
use std::sync::LazyLock;

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to create IndexNow HTTP client")
});

/// 向 Bing IndexNow API 提交 URL，通知搜索引擎内容已更新。
/// 使用 tokio::spawn 异步发送，不阻塞调用方。
/// 如果 INDEXNOW_KEY 未设置则静默跳过。
pub fn submit_urls(urls: Vec<String>) {
    let key = match dotenvy::var("INDEXNOW_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => return,
    };

    let site_url = match dotenvy::var("SITE_URL") {
        Ok(u) => u,
        _ => return,
    };

    let host = site_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or_default()
        .to_string();

    let urls_debug = format!("{:?}", urls);

    tokio::spawn(async move {
        let body = serde_json::json!({
            "host": host,
            "key": key,
            "keyLocation": format!("https://{}/{}.txt", host, key),
            "urlList": urls,
        });

        match HTTP_CLIENT
            .post("https://api.indexnow.org/indexnow")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => {
                info!(
                    "IndexNow 提交成功: status={}, urls={}",
                    resp.status(),
                    urls_debug
                );
            }
            Err(e) => {
                warn!("IndexNow 提交失败: {}, urls={}", e, urls_debug);
            }
        }
    });
}

/// 根据项目类型和 slug 构建项目 URL 并提交到 IndexNow。
pub fn notify_project(project_type: &str, slug: &str) {
    let site_url = match dotenvy::var("SITE_URL") {
        Ok(u) => u,
        _ => return,
    };

    let url = format!("{}/{}/{}", site_url, project_type, slug);
    submit_urls(vec![url]);
}

/// 提交项目主页及 changelog/versions/issues 子页面 URL 到 IndexNow。
/// 用于项目审核通过等场景，确保所有子页面被搜索引擎收录。
pub fn notify_project_with_subpages(project_type: &str, slug: &str) {
    let site_url = match dotenvy::var("SITE_URL") {
        Ok(u) => u,
        _ => return,
    };

    let base = format!("{}/{}/{}", site_url, project_type, slug);
    submit_urls(vec![
        base.clone(),
        format!("{}/changelog", base),
        format!("{}/versions", base),
        format!("{}/issues", base),
    ]);
}

/// 提交项目主页、子页面及所有版本页面 URL 到 IndexNow。
/// 用于项目审核通过等场景，确保项目及其所有版本都被搜索引擎收录。
pub fn notify_project_with_versions(
    project_type: &str,
    slug: &str,
    version_numbers: &[String],
) {
    let site_url = match dotenvy::var("SITE_URL") {
        Ok(u) => u,
        _ => return,
    };

    let base = format!("{}/{}/{}", site_url, project_type, slug);
    let mut urls = vec![
        base.clone(),
        format!("{}/changelog", base),
        format!("{}/versions", base),
        format!("{}/issues", base),
    ];
    for version_number in version_numbers {
        let encoded = urlencoding::encode(version_number);
        urls.push(format!("{}/version/{}", base, encoded));
    }
    submit_urls(urls);
}

/// 提交版本页面 URL 到 IndexNow。
pub fn notify_version(project_type: &str, slug: &str, version_number: &str) {
    let site_url = match dotenvy::var("SITE_URL") {
        Ok(u) => u,
        _ => return,
    };

    let base = format!("{}/{}/{}", site_url, project_type, slug);
    let encoded_version = urlencoding::encode(version_number);
    submit_urls(vec![
        base.clone(),
        format!("{}/version/{}", base, encoded_version),
    ]);
}

/// 提交 issue 详情页 URL 到 IndexNow。
pub fn notify_issue(project_type: &str, slug: &str, issue_id: &str) {
    let site_url = match dotenvy::var("SITE_URL") {
        Ok(u) => u,
        _ => return,
    };

    let url =
        format!("{}/{}/{}/issues/{}", site_url, project_type, slug, issue_id);
    submit_urls(vec![url]);
}
