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

        Ok(S3Host { bucket })
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
        println!("开始计算sha值 {file_name}");

        let content_sha1 = sha1::Sha1::from(&file_bytes).hexdigest();
        println!("sha1值 {content_sha1}");

        let content_sha512 = format!("{:x}", sha2::Sha512::digest(&file_bytes));
        println!("sha512值 {content_sha512}");

        let file_size = file_bytes.len();
        println!("开始上传文件到s3 {file_name}, 大小: {} bytes", file_size);

        // 根据文件大小设置超时时间
        // 假设上传速度至少 1MB/s，再加上额外的缓冲时间
        let timeout_seconds = std::cmp::max(30, (file_size / (1024 * 1024)) + 60);
        println!("设置上传超时时间: {} 秒", timeout_seconds);

        // 使用 tokio::time::timeout 来限制上传时间
        let upload_future = self.bucket.put_object_with_content_type(
            format!("/{file_name}"),
            &file_bytes,
            content_type,
        );

        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_seconds as u64),
            upload_future
        ).await {
            Ok(Ok(_)) => {
                println!("文件上传成功: {file_name}");
            }
            Ok(Err(e)) => {
                println!("文件上传失败: {file_name}, S3错误: {:?}", e);
                return Err(FileHostingError::S3Error(
                    format!("S3 upload error: {:?}", e),
                ));
            }
            Err(_) => {
                println!("文件上传超时: {file_name}, 超时时间: {}秒", timeout_seconds);
                // 如果是大文件超时，可以考虑重试或使用分片上传
                if file_size > 50 * 1024 * 1024 { // 大于 50MB
                    println!("大文件上传超时，建议使用分片上传或直传S3");
                }
                return Err(FileHostingError::S3Error(
                    format!("Upload timeout after {} seconds for file size {} bytes", timeout_seconds, file_size),
                ));
            }
        }
        println!("上传完成");

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
        self.bucket
            .delete_object(format!("/{file_name}"))
            .await
            .map_err(|_| {
                FileHostingError::S3Error("从 S3 删除文件时出错 ".to_string())
            })?;

        Ok(DeleteFileData {
            file_id: file_id.to_string(),
            file_name: file_name.to_string(),
        })
    }
}
