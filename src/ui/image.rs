//! 背景图片压缩：限制最长边与体积，输出腕端可显示的 JPEG（E-20 / G-06 解码预算）。

use image::{imageops::FilterType, GenericImageView, ImageReader};
use std::io::Cursor;

pub const MAX_EDGE: u32 = 450;
pub const MAX_BYTES: usize = 100 * 1024;
const QUALITY_STEPS: [u8; 7] = [85, 80, 75, 70, 65, 60, 55];

pub fn compress_for_watch(image_data: &[u8]) -> Result<Vec<u8>, String> {
    let decoded = ImageReader::new(Cursor::new(image_data))
        .with_guessed_format()
        .map_err(|error| format!("无法读取图片: {}", error))?
        .decode()
        .map_err(|error| format!("无法解码图片: {}", error))?;

    let (width, height) = decoded.dimensions();
    let (target_width, target_height) = scaled(width, height);
    let resized = if (target_width, target_height) == (width, height) {
        decoded
    } else {
        decoded.resize(target_width, target_height, FilterType::Lanczos3)
    };

    let mut output = Vec::new();
    for quality in QUALITY_STEPS {
        output.clear();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, quality);
        resized
            .write_with_encoder(encoder)
            .map_err(|error| format!("编码 JPEG 失败: {}", error))?;
        if output.len() <= MAX_BYTES {
            return Ok(output);
        }
    }
    Err(format!("压缩后仍超过 {}KB，请换一张更小的图片", MAX_BYTES / 1024))
}

fn scaled(width: u32, height: u32) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (width, height);
    }
    let longest = width.max(height);
    if longest <= MAX_EDGE {
        return (width, height);
    }
    if width > height {
        (MAX_EDGE, (MAX_EDGE * height) / width)
    } else {
        ((MAX_EDGE * width) / height, MAX_EDGE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 等比缩放且不放大() {
        assert_eq!(scaled(1200, 600), (MAX_EDGE, 225));
        assert_eq!(scaled(600, 1200), (225, MAX_EDGE));
        assert_eq!(scaled(300, 200), (300, 200));
        assert_eq!(scaled(0, 0), (0, 0));
    }

    #[test]
    fn 非图片数据返回可读错误() {
        let error = compress_for_watch(b"not an image").expect_err("应失败");
        assert!(error.contains("图片"), "错误文案应说明原因: {}", error);
    }
}
