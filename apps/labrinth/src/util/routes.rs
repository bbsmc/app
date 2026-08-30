use crate::routes::ApiError;
use crate::routes::v3::project_creation::CreateError;
use actix_multipart::Field;
use actix_web::web::Payload;
use bytes::BytesMut;
use futures::StreamExt;
use serde::de::DeserializeOwned;

pub const MAX_BULK_IDS: usize = 100;
pub const MAX_BULK_IDS_QUERY_LENGTH: usize = 8192;

pub fn parse_limited_ids_json<T>(ids: &str) -> Result<Vec<T>, ApiError>
where
    T: DeserializeOwned,
{
    if ids.len() > MAX_BULK_IDS_QUERY_LENGTH {
        return Err(ApiError::InvalidInput(format!(
            "ids 参数过长，最多允许 {MAX_BULK_IDS} 个 ID"
        )));
    }

    let parsed = serde_json::from_str::<Vec<T>>(ids)?;
    if parsed.len() > MAX_BULK_IDS {
        return Err(ApiError::InvalidInput(format!(
            "一次最多查询 {MAX_BULK_IDS} 个 ID"
        )));
    }

    Ok(parsed)
}

pub async fn read_from_payload(
    payload: &mut Payload,
    cap: usize,
    err_msg: &'static str,
) -> Result<BytesMut, ApiError> {
    let mut bytes = BytesMut::new();
    while let Some(item) = payload.next().await {
        if bytes.len() >= cap {
            return Err(ApiError::InvalidInput(String::from(err_msg)));
        } else {
            bytes.extend_from_slice(&item.map_err(|_| {
                ApiError::InvalidInput("无法解析 payload 中的字节!".to_string())
            })?);
        }
    }
    Ok(bytes)
}

pub async fn read_from_field(
    field: &mut Field,
    cap: usize,
    err_msg: &'static str,
) -> Result<BytesMut, CreateError> {
    let mut bytes = BytesMut::new();
    while let Some(chunk) = field.next().await {
        if bytes.len() >= cap {
            return Err(CreateError::InvalidInput(String::from(err_msg)));
        } else {
            bytes.extend_from_slice(&chunk?);
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_limited_ids_json_accepts_valid_arrays() {
        let ids =
            parse_limited_ids_json::<String>(r#"["alpha","beta"]"#).unwrap();

        assert_eq!(ids, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn parse_limited_ids_json_rejects_too_many_ids() {
        let ids = vec!["id"; MAX_BULK_IDS + 1];
        let encoded = serde_json::to_string(&ids).unwrap();

        assert!(parse_limited_ids_json::<String>(&encoded).is_err());
    }

    #[test]
    fn parse_limited_ids_json_rejects_too_long_query() {
        let ids = format!(r#"["{}"]"#, "a".repeat(MAX_BULK_IDS_QUERY_LENGTH));

        assert!(parse_limited_ids_json::<String>(&ids).is_err());
    }
}
