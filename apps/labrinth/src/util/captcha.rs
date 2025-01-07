use crate::routes::internal::flows::Challenge;
use crate::routes::ApiError;
use hex;
use ring::hmac;
use serde::Deserialize;
use std::collections::HashMap;

pub async fn check_hcaptcha(challenge: &Challenge) -> Result<bool, ApiError> {
    let client = reqwest::Client::new();

    #[derive(Deserialize, Debug)]
    struct Response {
        result: String,
        status: String,
    }

    let mut form = HashMap::new();
    let secret = dotenvy::var("GEETEST_SECRET")?;

    // 创建 HMAC-SHA256
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let tag = hmac::sign(&key, challenge.lot_number.as_bytes());
    let sign_token = hex::encode(tag.as_ref());

    form.insert("sign_token", &sign_token);
    form.insert("captcha_id", &challenge.captcha_id);
    form.insert("captcha_output", &challenge.captcha_output);
    form.insert("gen_time", &challenge.gen_time);
    form.insert("lot_number", &challenge.lot_number);
    form.insert("pass_token", &challenge.pass_token);

    for x in &form {
        println!("{:?}", x);
    }

    let val: Response = client
        .post("https://gcaptcha4.geetest.com/validate")
        .form(&form)
        .send()
        .await
        .map_err(|_| ApiError::Turnstile)?
        .json()
        .await
        .map_err(|_| ApiError::Turnstile)?;

    Ok(val.result == "success" && val.status == "success")
}
