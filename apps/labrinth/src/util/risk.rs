use chrono::{DateTime, Utc};
use hmac::Mac;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::json;
use sha2::Digest;
use std::collections::HashMap;
use std::ops::Add;
use uuid::Uuid;

use crate::database::redis::RedisPool;
use crate::routes::ApiError;
use serde::{Deserialize, Serialize};

const HOST: &str = "open.volcengineapi.com";
const CONTENT_TYPE: &str = "application/json";

#[derive(Serialize, Deserialize)]
struct UploadLimit {
    count: u32,
    time: DateTime<Utc>,
}

impl UploadLimit {
    pub fn new(time: DateTime<Utc>) -> Self {
        Self { count: 0, time }
    }

    pub fn is_limit(&self) -> bool {
        self.count >= 5
    }

    pub fn add(&mut self) {
        self.count += 1;
    }
}

pub async fn check_text_risk(
    text: &str,
    username: &str,
    url: &str,
    pos: &str,
    redis: &RedisPool,
) -> Result<bool, ApiError> {
    // 管理员用户跳过风险检查
    let admin_usernames = dotenvy::var("ADMIN_USERNAMES").unwrap_or_default();
    if admin_usernames
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .any(|admin| admin == username.to_lowercase())
    {
        return Ok(true);
    }
    let site_url = dotenvy::var("SITE_URL")?;
    let site_url = format!("{site_url}{url}");

    let mut conn = redis.connect().await?;
    let upload_limit = conn.get("upload_limit", username).await?;

    // 60秒内出现5次风险，暂时禁止发布修改任何信息十分钟
    if upload_limit.is_some() {
        let upload_limit: UploadLimit =
            serde_json::from_str::<UploadLimit>(&upload_limit.clone().unwrap())
                .unwrap();
        if upload_limit.is_limit() {
            // 使用北京时间输出
            let time = upload_limit
                .time
                .with_timezone(
                    &chrono::FixedOffset::east_opt(8 * 3600).unwrap(),
                )
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            return Err(ApiError::RiskLimit(time));
        }
    }

    let risk = text_risk(text, username).await?;
    if risk.is_empty() {
        return Ok(true);
    }

    if let Some(limit_str) = upload_limit {
        let mut upload_limit: UploadLimit =
            serde_json::from_str::<UploadLimit>(&limit_str).unwrap();
        upload_limit.add();
        if upload_limit.is_limit() {
            // upload_limit.time 对比Utc::now()的时间增加 10分钟

            upload_limit.time = Utc::now().add(chrono::Duration::minutes(10));
            let json = serde_json::to_string(&upload_limit).unwrap();
            conn.set("upload_limit", username, &json, Some(600)).await?;

            return Err(ApiError::RiskLimit(
                upload_limit
                    .time
                    .with_timezone(
                        &chrono::FixedOffset::east_opt(8 * 3600).unwrap(),
                    )
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
            ));
        }
        let json = serde_json::to_string(&upload_limit).unwrap();
        conn.set("upload_limit", username, &json, Some(60)).await?;
    } else {
        let upload_limit = UploadLimit::new(Utc::now());
        // upload_limit 转json存到redis
        let json = serde_json::to_string(&upload_limit).unwrap();
        conn.set("upload_limit", username, &json, Some(60)).await?;
    }

    let risk_str = risk.join(",");
    send_msg(text, &risk_str, &site_url, username, pos).await?;
    Ok(false)
}

//  URL1 要检查的图片
//  URL2 应用场景链接
pub async fn check_image_risk(
    url: &str,
    pos_url: &str,
    username: &str,
    pos: &str,
    redis: &RedisPool,
) -> Result<bool, ApiError> {
    // 管理员用户跳过风险检查
    let admin_usernames = dotenvy::var("ADMIN_USERNAMES").unwrap_or_default();
    if admin_usernames
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .any(|admin| admin == username.to_lowercase())
    {
        return Ok(true);
    }

    let site_url = dotenvy::var("SITE_URL")?;
    let site_url = format!("{site_url}{pos_url}");

    let mut conn = redis.connect().await?;
    let upload_limit = conn.get("upload_limit", username).await?;

    // 60秒内出现5次风险，暂时禁止发布修改任何信息十分钟
    if upload_limit.is_some() {
        let upload_limit: UploadLimit =
            serde_json::from_str::<UploadLimit>(&upload_limit.clone().unwrap())
                .unwrap();
        if upload_limit.is_limit() {
            // 使用北京时间输出
            let time = upload_limit
                .time
                .with_timezone(
                    &chrono::FixedOffset::east_opt(8 * 3600).unwrap(),
                )
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            return Err(ApiError::RiskLimit(time));
        }
    }
    let risk = imasge_risk(url, username, pos).await?;
    if risk.labels.is_empty() {
        return Ok(true);
    }

    if upload_limit.is_some() {
        let mut upload_limit: UploadLimit =
            serde_json::from_str::<UploadLimit>(&upload_limit.clone().unwrap())
                .unwrap();
        upload_limit.add();
        if upload_limit.is_limit() {
            // upload_limit.time 对比Utc::now()的时间增加 10分钟

            upload_limit.time = Utc::now().add(chrono::Duration::minutes(10));
            let json = serde_json::to_string(&upload_limit).unwrap();
            conn.set("upload_limit", username, &json, Some(600)).await?;

            return Err(ApiError::RiskLimit(
                upload_limit
                    .time
                    .with_timezone(
                        &chrono::FixedOffset::east_opt(8 * 3600).unwrap(),
                    )
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
            ));
        }
        let json = serde_json::to_string(&upload_limit).unwrap();
        conn.set("upload_limit", username, &json, Some(60)).await?;
    } else {
        let upload_limit = UploadLimit::new(Utc::now());
        // upload_limit 转json存到redis
        let json = serde_json::to_string(&upload_limit).unwrap();
        conn.set("upload_limit", username, &json, Some(60)).await?;
    }

    let risk_str = risk.labels.join(",");
    send_image_warning(&risk_str, url, &site_url, username, pos).await?;
    Ok(false)
}

/// 文本风控检测，返回 (是否通过, 风控标签)
/// 通过时标签为空字符串，未通过时返回具体标签
pub async fn check_text_risk_with_labels(
    text: &str,
    username: &str,
    url: &str,
    pos: &str,
    redis: &RedisPool,
) -> Result<(bool, String), ApiError> {
    // 管理员用户跳过风险检查
    let admin_usernames = dotenvy::var("ADMIN_USERNAMES").unwrap_or_default();
    if admin_usernames
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .any(|admin| admin == username.to_lowercase())
    {
        return Ok((true, String::new()));
    }

    let site_url = dotenvy::var("SITE_URL")?;
    let site_url = format!("{site_url}{url}");

    let mut conn = redis.connect().await?;
    let upload_limit = conn.get("upload_limit", username).await?;

    if upload_limit.is_some() {
        let upload_limit: UploadLimit =
            serde_json::from_str::<UploadLimit>(&upload_limit.clone().unwrap())
                .unwrap();
        if upload_limit.is_limit() {
            let time = upload_limit
                .time
                .with_timezone(
                    &chrono::FixedOffset::east_opt(8 * 3600).unwrap(),
                )
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            return Err(ApiError::RiskLimit(time));
        }
    }

    let risk = text_risk(text, username).await?;
    if risk.is_empty() {
        return Ok((true, String::new()));
    }

    if let Some(limit_str) = upload_limit {
        let mut upload_limit: UploadLimit =
            serde_json::from_str::<UploadLimit>(&limit_str).unwrap();
        upload_limit.add();
        if upload_limit.is_limit() {
            upload_limit.time = Utc::now().add(chrono::Duration::minutes(10));
            let json = serde_json::to_string(&upload_limit).unwrap();
            conn.set("upload_limit", username, &json, Some(600)).await?;
            return Err(ApiError::RiskLimit(
                upload_limit
                    .time
                    .with_timezone(
                        &chrono::FixedOffset::east_opt(8 * 3600).unwrap(),
                    )
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
            ));
        }
        let json = serde_json::to_string(&upload_limit).unwrap();
        conn.set("upload_limit", username, &json, Some(60)).await?;
    } else {
        let upload_limit = UploadLimit::new(Utc::now());
        let json = serde_json::to_string(&upload_limit).unwrap();
        conn.set("upload_limit", username, &json, Some(60)).await?;
    }

    let risk_str = risk.join(",");
    send_msg(text, &risk_str, &site_url, username, pos).await?;
    Ok((false, risk_str))
}

/// 图片风控检测，返回 (是否通过, 风控标签)
/// 图片风控检查结果（含标签和 frame URL）
pub struct ImageRiskCheckResult {
    pub passed: bool,
    pub labels: String,
    /// 火山引擎缓存的图片副本 URL（用于审核记录，S3 删除后仍可查看）
    pub frame_url: Option<String>,
}

pub async fn check_image_risk_with_labels(
    url: &str,
    pos_url: &str,
    username: &str,
    pos: &str,
    redis: &RedisPool,
) -> Result<ImageRiskCheckResult, ApiError> {
    // 管理员用户跳过风险检查
    let admin_usernames = dotenvy::var("ADMIN_USERNAMES").unwrap_or_default();
    if admin_usernames
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .any(|admin| admin == username.to_lowercase())
    {
        return Ok(ImageRiskCheckResult {
            passed: true,
            labels: String::new(),
            frame_url: None,
        });
    }

    let site_url = dotenvy::var("SITE_URL")?;
    let site_url = format!("{site_url}{pos_url}");

    let mut conn = redis.connect().await?;
    let upload_limit = conn.get("upload_limit", username).await?;

    if upload_limit.is_some() {
        let upload_limit: UploadLimit =
            serde_json::from_str::<UploadLimit>(&upload_limit.clone().unwrap())
                .unwrap();
        if upload_limit.is_limit() {
            let time = upload_limit
                .time
                .with_timezone(
                    &chrono::FixedOffset::east_opt(8 * 3600).unwrap(),
                )
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            return Err(ApiError::RiskLimit(time));
        }
    }

    let risk = imasge_risk(url, username, pos).await?;
    if risk.labels.is_empty() {
        return Ok(ImageRiskCheckResult {
            passed: true,
            labels: String::new(),
            frame_url: None,
        });
    }

    if upload_limit.is_some() {
        let mut upload_limit: UploadLimit =
            serde_json::from_str::<UploadLimit>(&upload_limit.clone().unwrap())
                .unwrap();
        upload_limit.add();
        if upload_limit.is_limit() {
            upload_limit.time = Utc::now().add(chrono::Duration::minutes(10));
            let json = serde_json::to_string(&upload_limit).unwrap();
            conn.set("upload_limit", username, &json, Some(600)).await?;
            return Err(ApiError::RiskLimit(
                upload_limit
                    .time
                    .with_timezone(
                        &chrono::FixedOffset::east_opt(8 * 3600).unwrap(),
                    )
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
            ));
        }
        let json = serde_json::to_string(&upload_limit).unwrap();
        conn.set("upload_limit", username, &json, Some(60)).await?;
    } else {
        let upload_limit = UploadLimit::new(Utc::now());
        let json = serde_json::to_string(&upload_limit).unwrap();
        conn.set("upload_limit", username, &json, Some(60)).await?;
    }

    let risk_str = risk.labels.join(",");
    send_image_warning(&risk_str, url, &site_url, username, pos).await?;
    Ok(ImageRiskCheckResult {
        passed: false,
        labels: risk_str,
        frame_url: risk.frame_url,
    })
}

async fn send_msg(
    text: &str,
    label: &str,
    url: &str,
    user: &str,
    pos: &str,
) -> Result<(), ApiError> {
    let client = reqwest::Client::builder().build()?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    let json_str = json!({
        "msg_type": "interactive",
        "card": {
            "type": "template",
            "data": {
                "template_id": "AAqSh66ot1O73",
                "template_version_name": "1.0.5",
                "template_variable": {
                    "text": text,
                    "user": user,
                    "label": label,
                    "url_": url,
                    "pos": pos
                }
            }
        }
    });
    let feishu_bot_webhook = dotenvy::var("FEISHU_BOT_WEBHOOK")?;
    let request = client
        .request(reqwest::Method::POST, feishu_bot_webhook)
        .headers(headers)
        .json(&json_str);

    let response = request.send().await?;
    let body = response.text().await?;

    println!("{}", body);

    Ok(())
}

async fn send_image_warning(
    label: &str,
    url: &str,
    pos_url: &str,
    user: &str,
    pos: &str,
) -> Result<(), ApiError> {
    let client = reqwest::Client::builder().build()?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    let json_str = json!({
        "msg_type": "interactive",
        "card": {
            "type": "template",
            "data": {
                "template_id": "AAqSZ1EyTLk2U",
                "template_version_name": "1.0.2",
                "template_variable": {
                    "user": user,
                    "label": label,
                    "url_": url,
                    "pos_url": pos_url,
                    "pos": pos
                }
            }
        }
    });
    let feishu_bot_webhook = dotenvy::var("FEISHU_BOT_WEBHOOK")?;
    let request = client
        .request(reqwest::Method::POST, feishu_bot_webhook)
        .headers(headers)
        .json(&json_str);

    let response = request.send().await?;
    let body = response.text().await?;

    println!("{}", body);

    Ok(())
}

async fn text_risk(
    text: &str,
    username: &str,
) -> Result<Vec<String>, ApiError> {
    let ak = dotenvy::var("HUOSHAN_AK")?;
    let sk = dotenvy::var("HUOSHAN_SK")?;
    let now: DateTime<Utc> = Utc::now();
    let x_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let mut query = HashMap::new();
    query.insert("Action".to_string(), "TextSliceRisk".to_string());
    query.insert("Version".to_string(), "2022-11-07".to_string());
    let params = json!({
        "AppId": 676801,
        "Service": "text_risk",
        "Parameters": json!({
            "biztype": "to_text",
            "text": text,
            "account_id": username,
            "operate_time": now.timestamp()
        }).to_string()
    })
    .to_string();
    let response_body = request(
        "POST",
        &x_date,
        query,
        HashMap::new(),
        &ak,
        &sk,
        Some(&params),
    );
    let result = match response_body.await {
        Ok(r) => r,
        Err(e) => {
            log::error!("文本风控 API 请求失败: {e}");
            return Ok(vec!["风控API异常".to_string()]);
        }
    };
    if result.get("Result").is_some()
        && result.get("Result").unwrap().get("Code").unwrap() == 0
    {
        let decision = result
            .get("Result")
            .unwrap()
            .get("Data")
            .unwrap()
            .get("Decision")
            .unwrap()
            .to_string();
        if decision.contains("PASS") {
            return Ok(vec![]);
        }
        let mut vec = vec![];
        // 要输出不带斜杠的格式
        let json_str = serde_json::to_string_pretty(&result).unwrap();
        let json_str = json_str.replace("\\", "");
        println!("{}", json_str);
        let result_json: &serde_json::Value = result
            .get("Result")
            .unwrap()
            .get("Data")
            .unwrap()
            .get("Results")
            .unwrap();
        result_json.as_array().unwrap().iter().for_each(|item| {
            let text =
                item.get("RiskText").unwrap().to_string().replace("\"", "");
            let mut risks = vec![];
            item.get("Labels")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .for_each(|item| {
                    let sub_label = item
                        .get("SubLabel")
                        .unwrap()
                        .to_string()
                        .replace("\"", "");
                    let parent_label = item
                        .get("Label")
                        .map(|v| v.to_string().replace("\"", ""))
                        .unwrap_or_default();
                    let matched = get_label(&sub_label)
                        .or_else(|| get_label(&parent_label));
                    if let Some((first_level, second_level)) = matched {
                        risks
                            .push(format!("{}[{}]", first_level, second_level));
                    }
                });

            if risks.is_empty() {
                vec.push(format!("未知风控标签:{}", text));
            } else {
                vec.push(format!("{}:{}", risks.join(","), text));
            }
        });

        // 兜底：Decision 不是 PASS 但没匹配到任何标签
        if vec.is_empty() {
            let final_label = result
                .get("Result")
                .unwrap()
                .get("Data")
                .unwrap()
                .get("FinalLabel")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            vec.push(format!("未知风控标签[{}]", final_label));
        }

        Ok(vec)
    } else {
        log::error!("文本风控 API 返回错误: {:?}", result);
        Ok(vec!["风控API异常".to_string()])
    }
}

/// 图片风控检查结果
pub struct ImageRiskDetail {
    pub labels: Vec<String>,
    pub frame_url: Option<String>,
}

async fn imasge_risk(
    url: &str,
    username: &str,
    pos: &str,
) -> Result<ImageRiskDetail, ApiError> {
    let ak = dotenvy::var("HUOSHAN_AK")?;
    let sk = dotenvy::var("HUOSHAN_SK")?;
    let now: DateTime<Utc> = Utc::now();
    let x_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let mut query = HashMap::new();
    query.insert("Action".to_string(), "ImageContentRiskV2".to_string());
    query.insert("Version".to_string(), "2021-11-29".to_string());
    let params = json!({
        "AppId": 676801,
        "Service": "image_content_risk",
        "Parameters": json!({
            "biztype": if pos.contains("头像") { "avatar" } else { "imasges" },
            "url": url,
            "account_id": username,
            "data_id": Uuid::new_v4().to_string().replace("-", ""),
            "operate_time": now.timestamp()
        }).to_string()
    })
    .to_string();
    let response_body = request(
        "POST",
        &x_date,
        query,
        HashMap::new(),
        &ak,
        &sk,
        Some(&params),
    );
    let result = match response_body.await {
        Ok(r) => r,
        Err(e) => {
            log::error!("图片风控 API 请求失败: {e}");
            return Ok(ImageRiskDetail {
                labels: vec!["风控API异常".to_string()],
                frame_url: None,
            });
        }
    };
    if result.get("Result").is_some()
        && result.get("Result").unwrap().get("Code").unwrap() == 0
    {
        let decision = result
            .get("Result")
            .unwrap()
            .get("Data")
            .unwrap()
            .get("Decision")
            .unwrap()
            .to_string();
        if decision.contains("PASS") {
            return Ok(ImageRiskDetail {
                labels: vec![],
                frame_url: None,
            });
        }
        let json_str = serde_json::to_string_pretty(&result).unwrap();
        let json_str = json_str.replace("\\", "");
        println!("{}", json_str);
        let mut vec = vec![];
        let mut frame_url: Option<String> = None;
        let result_json = result
            .get("Result")
            .unwrap()
            .get("Data")
            .unwrap()
            .get("Results")
            .unwrap();
        result_json.as_array().unwrap().iter().for_each(|item| {
            let item_json = item.as_object().unwrap();
            let sub_label = item_json
                .get("SubLabel")
                .unwrap()
                .to_string()
                .replace("\"", "");
            let label = item_json
                .get("Label")
                .unwrap()
                .to_string()
                .replace("\"", "");

            // 提取第一个 frame URL（火山引擎缓存的图片副本）
            if frame_url.is_none()
                && let Some(frames) = item_json.get("Frames")
                && let Some(frames_arr) = frames.as_array()
                && let Some(first_frame) = frames_arr.first()
                && let Some(url) = first_frame.get("url")
                && let Some(url_str) = url.as_str()
            {
                frame_url = Some(url_str.to_string());
            }

            // 优先匹配 SubLabel，匹配不到则尝试 Label（父标签）
            let matched =
                get_label_image(&sub_label).or_else(|| get_label_image(&label));

            if let Some((first_level, second_level)) = matched {
                vec.push(format!("{}[{}]", first_level, second_level));
            }
        });

        // 如果 Decision 不是 PASS 但没匹配到任何已知标签，使用 FinalLabel 作为兜底
        if vec.is_empty() {
            let final_label = result
                .get("Result")
                .unwrap()
                .get("Data")
                .unwrap()
                .get("FinalLabel")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            vec.push(format!("未知风控标签[{}]", final_label));
        }

        Ok(ImageRiskDetail {
            labels: vec,
            frame_url,
        })
    } else {
        log::error!("图片风控 API 返回错误: {:?}", result);
        Ok(ImageRiskDetail {
            labels: vec!["风控API异常".to_string()],
            frame_url: None,
        })
    }
}

fn get_label(code: &str) -> Option<(String, String)> {
    let labels = vec![
        ("101001", "涉黄", "高危色情"),
        ("101002", "涉黄", "性骚扰"),
        ("101003", "涉黄", "性暗示"),
        ("101004", "涉黄", "性癖好"),
        ("101005", "涉黄", "性侮辱"),
        ("102", "涉敏1", "政治敏感1"),
        ("103", "涉敏2", "政治敏感2"),
        ("104001", "广告", "色情广告"),
        ("104002", "广告", "招嫖广告"),
        ("104003", "广告", "医疗广告"),
        ("104004", "广告", "美容美体广告"),
        ("104005", "广告", "赌博广告"),
        ("104006", "广告", "售假广告"),
        ("104007", "广告", "普通商业广告"),
        ("104008", "广告", "游戏交易广告"),
        ("104009", "广告", "游戏拉人广告"),
        ("104010", "广告", "放贷催收广告"),
        ("104011", "广告", "互粉互赞广告"),
        ("104012", "广告", "招聘广告"),
        ("104013", "广告", "账号买卖广告"),
        ("104014", "广告", "黑灰产广告"),
        ("104015", "广告", "站内导流"),
        ("104016", "广告", "站外导流"),
        ("104017", "广告", "竞品拉人广告"),
        ("105001", "谩骂", "恶意攻击"),
        ("105002", "谩骂", "轻度攻击"),
        ("105003", "谩骂", "不文明用语"),
        ("106001", "违禁", "毒品"),
        ("106002", "违禁", "赌博"),
        ("106003", "违禁", "违禁品"),
        ("106004", "违禁", "违禁行为"),
        ("106005", "违禁", "劣迹艺人"),
        ("107001", "联系方式", "联系方式"),
        ("107002", "联系方式", "URL"),
        ("107005", "联系方式", "email"),
        ("108001", "诈骗", "杀猪盘诈骗"),
        ("108002", "诈骗", "刷单致富诈骗"),
        ("108003", "诈骗", "招工致富诈骗"),
        ("108004", "诈骗", "投资理财诈骗"),
        ("108005", "诈骗", "金融征信诈骗"),
        ("108006", "诈骗", "卖惨人设诈骗"),
        ("108007", "诈骗", "解除账号限制类诈骗"),
        ("108008", "诈骗", "盗号诈骗"),
        ("110001", "违反公序良俗", "违背道德伦理"),
        ("112001", "低质", "灌水、无意义"),
        ("199001", "自定义", "用户自定义黑名单"),
        ("199002", "自定义", "用户自定义白名单"),
        ("199003", "自定义", "用户自定义疑似名单"),
        ("199004", "自定义", "自定义"),
    ];

    for (c, first_level, second_level) in labels {
        if c == code {
            return Some((first_level.to_string(), second_level.to_string()));
        }
    }
    None
}

fn get_label_image(code: &str) -> Option<(String, String)> {
    let labels = vec![
        ("302", "政治敏感1"),
        ("303", "政治敏感2"),
        ("301001", "通用色情"),
        ("301002", "通用低俗"),
        ("301003", "色情动作"),
        ("301004", "色情物体"),
        ("301005", "色情动漫"),
        ("301006", "色情裸露"),
        ("301007", "性行为"),
        ("301008", "性暗示"),
        ("301009", "动物性器官"),
        ("301010", "性分泌物"),
        ("304001", "色情广告"),
        ("304002", "不当交友广告"),
        ("304003", "招嫖广告"),
        ("304004", "医疗广告"),
        ("304005", "赌博广告"),
        ("305001", "恶心引人不适"),
        ("305002", "虐待动物"),
        ("305003", "恐怖引人不适"),
        ("305004", "尸体引人不适"),
        ("305005", "血腥引人不适"),
        ("306001", "图片武器"),
        ("306002", "违禁药品"),
        ("306003", "赌博"),
        ("306004", "涉爆物品"),
        ("306005", "打架"),
        ("306006", "图片爆炸"),
        ("306007", "野生动物相关"),
        ("306008", "劣迹艺人"),
        ("308001", "诈骗"),
        ("308002", "刷单致富诈骗"),
        ("308003", "招工致富诈骗"),
        ("308004", "投资理财诈骗"),
        ("308005", "金融征信诈骗"),
        ("308006", "解除账号限制类诈骗"),
        ("308007", "仿冒客服"),
        ("308008", "仿冒明星"),
        ("308009", "宠粉诈骗"),
        ("309001", "自我伤害"),
        ("309002", "夸张吃播"),
        ("309003", "丧葬恶搞"),
        ("309004", "不良社会风气"),
        ("309005", "不当车播"),
        ("309006", "吸烟"),
        ("310001", "儿童色情"),
        ("310002", "儿童低俗"),
        ("310003", "未成年吸烟"),
        ("310004", "未成年喝酒"),
        ("310005", "儿童邪典"),
        ("311001", "女肩部裸露"),
        ("311002", "女背部裸露"),
        ("311003", "女胸部露沟"),
        ("311004", "女胸部侧漏"),
        ("311005", "女背部全裸"),
        ("311006", "女腰腹部裸露"),
        ("311007", "女腿部裸露"),
        ("311008", "臀部裸露"),
        ("311009", "女胸部特写"),
        ("311010", "女腹部特写"),
        ("311011", "女腿部特写"),
        ("311012", "女脚部特写"),
        ("311013", "女开腿"),
        ("311014", "女诱惑动作"),
        ("311015", "疑似未穿衣"),
        ("311016", "女正常内衣裤"),
        ("311017", "黑丝"),
        ("311018", "紧身衣"),
        ("311019", "男内裤"),
        ("311020", "男性上身裸露"),
        ("311021", "亲吻"),
        ("311022", "舌吻"),
        ("311023", "摸胸下体"),
        ("311024", "穿衣性行为"),
        ("311025", "动物性行为"),
        ("311026", "未拆封避孕套"),
        ("311027", "搂抱"),
        ("320001", "涉黄"),
        ("320002", "涉敏1"),
        ("320003", "涉敏2"),
        ("320004", "广告"),
        ("320005", "谩骂"),
        ("320006", "违禁"),
        ("320007", "联系方式"),
        ("320008", "诈骗"),
        ("320009", "违反公序良俗"),
        ("320010", "低质"),
        ("320096", "黑名单"),
        ("320097", "白名单"),
        ("320098", "疑似名单"),
        ("320099", "自定义"),
        ("351001", "模糊"),
        ("351002", "纯色边框"),
        ("351003", "滤镜"),
        ("351004", "马赛克"),
        ("351005", "水印"),
        ("351006", "纯色屏"),
        ("351007", "镜像"),
        ("352001", "婴儿"),
        ("352002", "幼儿"),
        ("352003", "学龄前儿童"),
        ("352004", "未成年儿童"),
        ("352005", "青年"),
        ("352006", "中年"),
        ("352007", "老年"),
        ("399001", "黑名单"),
        ("399002", "白名单"),
        ("399003", "疑似名单"),
        ("399004", "自定义"),
    ];

    for (c, description) in labels {
        if c == code {
            return Some((c.to_string(), description.to_string()));
        }
    }
    None
}

/// 检查风控标签字符串是否包含涉政内容
/// 基于火山引擎图片风控的标签码 302（政治敏感1）和 303（政治敏感2）
pub fn contains_political_labels(labels: &str) -> bool {
    labels.contains("302[") || labels.contains("303[")
}

fn norm_query(params: &HashMap<String, String>) -> String {
    let mut pairs: Vec<_> =
        params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    pairs.sort();
    pairs.join("&")
}

fn hmac_sha256(key: &[u8], content: &str) -> Vec<u8> {
    let mut hasher = hmac::Hmac::<sha2::Sha256>::new_from_slice(key)
        .expect("HMAC can take key of any size");
    hasher.update(content.as_bytes());
    hasher.finalize().into_bytes().to_vec()
}

fn hash_sha256(content: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

async fn request(
    method: &str,
    date: &str,
    query: HashMap<String, String>,
    _header: HashMap<String, String>,
    ak: &str,
    sk: &str,
    body: Option<&str>,
) -> reqwest::Result<serde_json::Value> {
    let body = body.unwrap_or("");
    let x_content_sha256 = hash_sha256(body);
    let signed_headers_str = "content-type;host;x-content-sha256;x-date";
    let canonical_request_str = format!(
        "{}\n/\n{}\ncontent-type:{}\nhost:{}\nx-content-sha256:{}\nx-date:{}\n\n{}\n{}",
        method,
        norm_query(&query),
        CONTENT_TYPE,
        HOST,
        x_content_sha256,
        date,
        signed_headers_str,
        x_content_sha256
    );

    let hashed_canonical_request = hash_sha256(&canonical_request_str);
    let short_date = &date[..8];
    let credential_scope = format!(
        "{}/{}/{}/{}",
        short_date, "cn-north-1", "BusinessSecurity", "request"
    );
    let string_to_sign = format!(
        "HMAC-SHA256\n{}\n{}\n{}",
        date, credential_scope, hashed_canonical_request
    );

    let k_date = hmac_sha256(sk.as_bytes(), short_date);
    let k_region = hmac_sha256(&k_date, "cn-north-1");
    let k_service = hmac_sha256(&k_region, "BusinessSecurity");
    let k_signing = hmac_sha256(&k_service, "request");
    let signature = hex::encode(hmac_sha256(&k_signing, &string_to_sign));

    let authorization = format!(
        "HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        ak, credential_scope, signed_headers_str, signature
    );

    let mut headers = HeaderMap::new();
    headers.insert("Host", HeaderValue::from_static(HOST));
    headers.insert("Content-Type", HeaderValue::from_static(CONTENT_TYPE));
    headers.insert(
        "X-Content-Sha256",
        HeaderValue::from_str(&x_content_sha256).unwrap(),
    );
    headers.insert("X-Date", HeaderValue::from_str(date).unwrap());
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&authorization).unwrap(),
    );

    let client = reqwest::Client::new();
    let response = client
        .request(method.parse().unwrap(), format!("https://{}/", HOST))
        .headers(headers)
        .query(&query)
        .body(body.to_string())
        .send()
        .await?;

    response.json().await
}
