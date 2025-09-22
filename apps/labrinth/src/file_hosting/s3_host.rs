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
        // 睡眠等待60秒
        // tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        println!("开始计算sha值 {file_name}");

        let content_sha1 = sha1::Sha1::from(&file_bytes).hexdigest();
        println!("sha1值 {content_sha1}");

        let content_sha512 = format!("{:x}", sha2::Sha512::digest(&file_bytes));
        println!("sha512值 {content_sha512}");
        println!("开始上传文件到s3 {file_name}");
        self.bucket
            .put_object_with_content_type(
                format!("/{file_name}"),
                &file_bytes,
                content_type,
            )
            .await
            .map_err(|_e| {
                FileHostingError::S3Error(
                    "Error while uploading file to S3".to_string(),
                )
            })?;
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
