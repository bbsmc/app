use crate::file_hosting::{
    DeleteFileData, FileHost, FileHostingError, UploadFileData,
};
use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use sha2::Digest;

pub struct S3Host {
    bucket: Bucket,
}

impl S3Host {
    pub fn new(
        bucket_name: &str,
        url: &str,
        access_token: &str,
        secret: &str,
    ) -> Result<S3Host, FileHostingError> {
        let mut bucket = Bucket::new(
            bucket_name,
            Region::Custom {
                region: "".to_owned(),
                endpoint: url.to_string(),
            },
            Credentials::new(
                Some(access_token),
                Some(secret),
                None,
                None,
                None,
            )
            .map_err(|_| {
                FileHostingError::S3Error(
                    "Error while creating credentials".to_string(),
                )
            })?,
        )
        .map_err(|_| {
            FileHostingError::S3Error(
                "Error while creating Bucket instance".to_string(),
            )
        })?;
        bucket.set_path_style();
        bucket.set_request_timeout(None);

        Ok(S3Host { bucket: *bucket })
    }
}

#[async_trait]
impl FileHost for S3Host {
    async fn upload_file(
        &self,
        content_type: &str,
        file_name: &str,
        file_bytes: Bytes,
    ) -> Result<UploadFileData, FileHostingError> {
        let content_sha1 = format!("{:x}", sha1::Sha1::digest(&file_bytes));

        let content_sha512 = format!("{:x}", sha2::Sha512::digest(&file_bytes));

        let file_size = file_bytes.len();

        // 根据文件大小设置超时时间
        // 假设上传速度至少 1MB/s，再加上额外的缓冲时间
        let timeout_seconds =
            std::cmp::max(30, (file_size / (1024 * 1024)) + 60);

        // 使用 tokio::time::timeout 来限制上传时间
        let upload_future = self.bucket.put_object_with_content_type(
            format!("/{file_name}"),
            &file_bytes,
            content_type,
        );

        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_seconds as u64),
            upload_future,
        )
        .await
        {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                return Err(FileHostingError::S3Error(format!(
                    "S3 upload error: {:?}",
                    e
                )));
            }
            Err(_) => {
                return Err(FileHostingError::S3Error(format!(
                    "Upload timeout after {} seconds for file size {} bytes",
                    timeout_seconds, file_size
                )));
            }
        }
        Ok(UploadFileData {
            file_id: file_name.to_string(),
            file_name: file_name.to_string(),
            content_length: file_bytes.len() as u32,
            content_sha512,
            content_sha1,
            content_md5: None,
            content_type: content_type.to_string(),
            upload_timestamp: Utc::now().timestamp() as u64,
        })
    }

    async fn delete_file_version(
        &self,
        file_id: &str,
        file_name: &str,
    ) -> Result<DeleteFileData, FileHostingError> {
        let response = self
            .bucket
            .delete_object(format!("/{file_name}"))
            .await
            .map_err(|e| {
                log::error!(
                    "S3 删除文件失败: file_name={}, error={:?}",
                    file_name,
                    e
                );
                FileHostingError::S3Error(format!(
                    "从 S3 删除文件时出错: {:?}",
                    e
                ))
            })?;

        log::info!(
            "S3 删除文件响应: file_name={}, status_code={}",
            file_name,
            response.status_code()
        );

        // S3 删除成功后，尝试 HEAD 验证对象是否确实被删除
        match self.bucket.head_object(format!("/{file_name}")).await {
            Ok((_, code)) => {
                if code == 200 {
                    log::warn!(
                        "S3 删除后对象仍然存在! file_name={}, head_status={}",
                        file_name,
                        code
                    );
                } else {
                    log::info!(
                        "S3 删除验证: 对象已确认移除, file_name={}, head_status={}",
                        file_name,
                        code
                    );
                }
            }
            Err(_) => {
                // HEAD 请求失败通常意味着对象不存在（404），这是预期的删除后行为
                log::info!(
                    "S3 删除验证: 对象已确认移除 (HEAD 返回错误), file_name={}",
                    file_name
                );
            }
        }

        Ok(DeleteFileData {
            file_id: file_id.to_string(),
            file_name: file_name.to_string(),
        })
    }
}
