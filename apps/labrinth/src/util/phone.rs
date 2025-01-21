use crate::routes::ApiError;
use alibaba_cloud_sdk_rust::services::dysmsapi;

pub async fn send_phone_number_code(
    phone_number: &str,
    code: &str,
) -> Result<bool, ApiError> {
    let access_key_id = dotenvy::var("ALIYUN_SMS_ACCESS_KEYID")?;
    let access_key_secret = dotenvy::var("ALIYUN_SMS_ACCESS_KEY_SECRET")?;
    let report_templeate_code =
        dotenvy::var("ALIYUN_SMS_REPORT_TEMPLETE_CODE")?;
    let sign_name = dotenvy::var("ALIYUN_SMS_SIGN_NAME")?;
    let region = dotenvy::var("ALIYUN_SMS_REGION")?;

    let mut client = dysmsapi::Client::NewClientWithAccessKey(
        &region,
        &access_key_id,
        &access_key_secret,
    )?;
    let mut request = dysmsapi::CreateSendSmsRequest();
    request.PhoneNumbers = phone_number.to_owned();
    request.SignName = sign_name.to_owned();
    request.TemplateCode = report_templeate_code.to_owned();
    request.TemplateParam = format!("{{\"code\":\"{}\"}}", code).to_owned();
    let response = client.SendSms(&mut request)?;
    println!("{:?}", &response);

    Ok(true)
}
