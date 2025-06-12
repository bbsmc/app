use crate::routes::ApiError;
use serde::Deserialize;

pub async fn check_hcaptcha(challenge: &String) -> Result<bool, ApiError> {
    let client = reqwest::Client::new();

    #[derive(Deserialize, Debug)]
    struct Response {
        code: i32,
        msg: String,
        data: Option<serde_json::Value>,
        success: bool,
    }

    let url = dotenvy::var("TAC_URL")?;

    // 拼接 url 和 challenge
    let url_challenge = format!("{}{}", url, challenge);
    println!("Checking {}", url_challenge);
    let val: Response = client
        .post(url_challenge)
        .send()
        .await
        .map_err(|_| ApiError::Turnstile)?
        .json()
        .await
        .map_err(|_| ApiError::Turnstile)?;

    if val.code == 200 && val.success {
        Ok(true)
    } else {
        Err(ApiError::InvalidInput(val.msg))
    }
}
