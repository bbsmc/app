use crate::database;
use crate::database::models::image_item;
use crate::database::redis::RedisPool;
use crate::file_hosting::FileHost;
use crate::models::images::ImageContext;
use crate::routes::ApiError;
use sha2::Digest;

use color_thief::ColorFormat;
use image::imageops::FilterType;
use image::{
    DynamicImage, EncodableLayout, GenericImageView, ImageError, ImageFormat,
};
use std::io::Cursor;
use webp::Encoder;

pub fn get_color_from_img(data: &[u8]) -> Result<Option<u32>, ImageError> {
    let image = image::load_from_memory(data)?
        .resize(256, 256, FilterType::Nearest)
        .crop_imm(128, 128, 64, 64);
    let color = color_thief::get_palette(
        image.to_rgb8().as_bytes(),
        ColorFormat::Rgb,
        10,
        2,
    )
    .ok()
    .and_then(|x| x.first().copied())
    .map(|x| (x.r as u32) << 16 | (x.g as u32) << 8 | (x.b as u32));

    Ok(color)
}

pub struct UploadImageResult {
    pub url: String,
    pub url_path: String,

    pub raw_url: String,
    pub raw_url_path: String,

    pub color: Option<u32>,
}

pub struct UploadImagePos {
    // 应用场景
    pub pos: String,
    // 应该场景地址
    pub url: String,

    pub username: String,
}

#[allow(clippy::too_many_arguments)]
pub async fn upload_image_optimized(
    upload_folder: &str,
    bytes: bytes::Bytes,
    file_extension: &str,
    target_width: Option<u32>,
    min_aspect_ratio: Option<f32>,
    file_host: &dyn FileHost,
    pos: UploadImagePos,
    redis: &RedisPool,
    skip_risk_check: bool,
) -> Result<UploadImageResult, ApiError> {
    let content_type = crate::util::ext::get_image_content_type(file_extension)
        .ok_or_else(|| {
            ApiError::InvalidInput(format!(
                "无效的图像格式: {}",
                file_extension
            ))
        })?;

    let cdn_url = dotenvy::var("CDN_URL")?;

    let hash = format!("{:x}", sha1::Sha1::digest(&bytes));
    let (processed_image, processed_image_ext) = process_image(
        bytes.clone(),
        content_type,
        target_width,
        min_aspect_ratio,
    )?;
    let color = get_color_from_img(&bytes)?;

    // 仅当处理后的图像小于原始图像时才上传
    let processed_upload_data = if processed_image.len() < bytes.len() {
        Some(
            file_host
                .upload_file(
                    content_type,
                    &format!(
                        "{}/{}_{}.{}",
                        upload_folder,
                        hash,
                        target_width.unwrap_or(0),
                        processed_image_ext
                    ),
                    processed_image,
                )
                .await?,
        )
    } else {
        None
    };

    let upload_data = file_host
        .upload_file(
            content_type,
            &format!("{}/{}.{}", upload_folder, hash, file_extension),
            bytes,
        )
        .await?;

    let url = format!("{}/{}", cdn_url, upload_data.file_name);

    if !skip_risk_check {
        let risk = crate::util::risk::check_image_risk(
            &url,
            &pos.url,
            &pos.username,
            &pos.pos,
            redis,
        )
        .await?;
        if !risk {
            // 风控未通过，清理已上传到 S3 的图片
            let processed_url = processed_upload_data
                .as_ref()
                .map(|x| format!("{}/{}", cdn_url, x.file_name));
            let raw_url = Some(url.clone());
            if let Err(e) =
                delete_old_images(processed_url, raw_url, file_host).await
            {
                log::warn!("风控拒绝后清理 S3 图片失败: {}", e);
            }
            return Err(ApiError::InvalidInput(
                "图片包含敏感内容，已被记录该次提交，请勿在本网站使用涉及敏感或违规的图片".to_string(),
            ));
        }
    }

    Ok(UploadImageResult {
        url: processed_upload_data
            .clone()
            .map(|x| format!("{}/{}", cdn_url, x.file_name))
            .unwrap_or_else(|| url.clone()),
        url_path: processed_upload_data
            .map(|x| x.file_name)
            .unwrap_or_else(|| upload_data.file_name.clone()),

        raw_url: url,
        raw_url_path: upload_data.file_name,
        color,
    })
}

fn process_image(
    image_bytes: bytes::Bytes,
    content_type: &str,
    target_width: Option<u32>,
    min_aspect_ratio: Option<f32>,
) -> Result<(bytes::Bytes, String), ImageError> {
    if content_type.to_lowercase() == "image/gif" {
        return Ok((image_bytes.clone(), "gif".to_string()));
    }

    let mut img = image::load_from_memory(&image_bytes)?;

    let webp_bytes = convert_to_webp(&img)?;
    img = image::load_from_memory(&webp_bytes)?;

    // 调整图像大小
    let (orig_width, orig_height) = img.dimensions();
    let aspect_ratio = orig_width as f32 / orig_height as f32;

    if let Some(target_width) = target_width
        && img.width() > target_width
    {
        let new_height = (target_width as f32 / aspect_ratio).round() as u32;
        img = img.resize(target_width, new_height, FilterType::Lanczos3);
    }

    if let Some(min_aspect_ratio) = min_aspect_ratio {
        // 如果需要裁剪
        if aspect_ratio < min_aspect_ratio {
            let crop_height =
                (img.width() as f32 / min_aspect_ratio).round() as u32;
            let y_offset = (img.height() - crop_height) / 2;
            img = img.crop_imm(0, y_offset, img.width(), crop_height);
        }
    }

    // 优化和压缩
    let mut output = Vec::new();
    img.write_to(&mut Cursor::new(&mut output), ImageFormat::WebP)?;

    Ok((bytes::Bytes::from(output), "webp".to_string()))
}

fn convert_to_webp(img: &DynamicImage) -> Result<Vec<u8>, ImageError> {
    let rgba = img.to_rgba8();
    let encoder = Encoder::from_rgba(&rgba, img.width(), img.height());
    let webp = encoder.encode(75.0); // 质量因子: 0-100, 75 是平衡
    Ok(webp.to_vec())
}

pub async fn delete_old_images(
    image_url: Option<String>,
    raw_image_url: Option<String>,
    file_host: &dyn FileHost,
) -> Result<(), ApiError> {
    let cdn_url = dotenvy::var("CDN_URL")?;
    let cdn_url_start = format!("{cdn_url}/");
    if let Some(ref image_url) = image_url {
        let name = image_url.split(&cdn_url_start).nth(1);

        if let Some(icon_path) = name {
            log::info!(
                "删除图片: cdn_url={}, s3_path={}",
                image_url,
                icon_path
            );
            file_host.delete_file_version("", icon_path).await?;
        } else {
            log::warn!(
                "无法从 CDN URL 提取 S3 路径: url={}, cdn_prefix={}",
                image_url,
                cdn_url_start
            );
        }
    }

    if let Some(ref raw_image_url) = raw_image_url {
        let name = raw_image_url.split(&cdn_url_start).nth(1);

        if let Some(icon_path) = name {
            log::info!(
                "删除原始图片: cdn_url={}, s3_path={}",
                raw_image_url,
                icon_path
            );
            file_host.delete_file_version("", icon_path).await?;
        } else {
            log::warn!(
                "无法从 CDN URL 提取 S3 路径: url={}, cdn_prefix={}",
                raw_image_url,
                cdn_url_start
            );
        }
    }

    Ok(())
}

// 检查与图像相关的更改
// 如果它们不再存在于字符串列表中，则删除它们
// 例如：如果描述被修改并且不再包含图像的链接
pub async fn delete_unused_images(
    context: ImageContext,
    reference_strings: Vec<&str>,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    redis: &RedisPool,
) -> Result<(), ApiError> {
    let uploaded_images =
        database::models::Image::get_many_contexted(context, transaction)
            .await?;

    for image in uploaded_images {
        let mut should_delete = true;
        for reference in &reference_strings {
            if image.url.contains(reference) {
                should_delete = false;
                break;
            }
        }

        if should_delete {
            image_item::Image::remove(image.id, transaction, redis).await?;
            image_item::Image::clear_cache(image.id, redis).await?;
        }
    }

    Ok(())
}
