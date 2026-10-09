//! 背景图片压缩：先检查输入尺寸与解码预算，再生成有界 JPEG。

use image::{imageops::FilterType, GenericImageView, ImageReader};
use std::io::Cursor;

pub const MAX_EDGE: u32 = 450;
pub const MAX_BYTES: usize = 100 * 1024;
pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_INPUT_EDGE: u32 = 8192;
const MAX_INPUT_PIXELS: u64 = 8 * 1024 * 1024;
const MAX_DECODE_ALLOC: u64 = 64 * 1024 * 1024;
const QUALITY_STEPS: [u8; 7] = [85, 80, 75, 70, 65, 60, 55];

fn reader(data: &[u8]) -> Result<ImageReader<Cursor<&[u8]>>, String> {
    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|error| format!("无法读取图片: {}", error))?;
    if !matches!(reader.format(), Some(image::ImageFormat::Jpeg | image::ImageFormat::Png)) {
        return Err("仅支持真实格式为 JPEG / PNG 的图片".to_string());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_INPUT_EDGE);
    limits.max_image_height = Some(MAX_INPUT_EDGE);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    reader.limits(limits);
    Ok(reader)
}

fn check_dimensions(width: u32, height: u32) -> Result<(), String> {
    if width == 0 || height == 0 || width > MAX_INPUT_EDGE || height > MAX_INPUT_EDGE
        || u64::from(width) * u64::from(height) > MAX_INPUT_PIXELS {
        return Err("图片尺寸超过预算：单边8192px、8Mi像素，且宽高不能为零".to_string());
    }
    Ok(())
}

pub fn compress_for_watch(image_data: &[u8]) -> Result<Vec<u8>, String> {
    if image_data.is_empty() || image_data.len() > MAX_INPUT_BYTES {
        return Err("图片数据为空或文件超过16MiB预算".to_string());
    }
    let (width, height) = reader(image_data)?.into_dimensions()
        .map_err(|error| format!("无法读取图片尺寸/预算: {}", error))?;
    check_dimensions(width, height)?;
    let decoded = reader(image_data)?
        .decode()
        .map_err(|error| format!("无法解码图片/预算: {}", error))?;

    let (width, height) = decoded.dimensions();
    check_dimensions(width, height)?;
    let (target_width, target_height) = scaled(width, height);
    let resized = if (target_width, target_height) == (width, height) {
        decoded
    } else {
        decoded.resize(target_width, target_height, FilterType::Lanczos3)
    };

    // JPEG不支持透明通道；只在缩小后的有界图片上合成白底。
    let rgba = resized.to_rgba8();
    drop(resized);
    let mut rgb = image::RgbImage::new(rgba.width(), rgba.height());
    for (source, target) in rgba.pixels().zip(rgb.pixels_mut()) {
        let alpha = u32::from(source[3]);
        for channel in 0..3 {
            target[channel] = ((u32::from(source[channel]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8;
        }
    }
    drop(rgba);
    let mut output = Vec::new();
    for quality in QUALITY_STEPS {
        output.clear();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, quality);
        rgb
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
        (MAX_EDGE, ((u64::from(MAX_EDGE) * u64::from(height)) / u64::from(width)).max(1) as u32)
    } else {
        (((u64::from(MAX_EDGE) * u64::from(width)) / u64::from(height)).max(1) as u32, MAX_EDGE)
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
        assert_eq!(scaled(8192, 1), (450, 1));
        assert_eq!(scaled(1, 8192), (1, 450));
    }

    #[test]
    fn 非图片数据返回可读错误() {
        let error = compress_for_watch(b"not an image").expect_err("应失败");
        assert!(error.contains("图片"), "错误文案应说明原因: {}", error);
    }

    #[test]
    fn 文件和尺寸预算在解码前拒绝() {
        assert!(compress_for_watch(&vec![0; MAX_INPUT_BYTES + 1]).unwrap_err().contains("预算"));
        assert!(check_dimensions(4096, 4096).is_err());
        assert!(check_dimensions(8193, 1).is_err());
        assert!(check_dimensions(0, 1).is_err());
        let mut data = Cursor::new(Vec::new());
        image::RgbaImage::new(1, 1).write_to(&mut data, image::ImageFormat::Png).unwrap();
        // 改IHDR并修复CRC：只需很小的文件即可声明巨大解码尺寸。
        let mut png = data.into_inner();
        png[16..20].copy_from_slice(&8192u32.to_be_bytes());
        png[20..24].copy_from_slice(&8192u32.to_be_bytes());
        let mut crc = !0u32;
        for byte in &png[12..29] {
            crc ^= u32::from(*byte);
            for _ in 0..8 { crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1)); }
        }
        png[29..33].copy_from_slice(&(!crc).to_be_bytes());
        assert!(compress_for_watch(&png).unwrap_err().contains("预算"));
    }

    #[test]
    fn 透明图合成白底并输出真实有界jpeg() {
        let rgba = image::RgbaImage::from_pixel(900, 450, image::Rgba([255, 0, 0, 0]));
        let mut data = Cursor::new(Vec::new());
        rgba.write_to(&mut data, image::ImageFormat::Png).unwrap();
        let output = compress_for_watch(data.get_ref()).unwrap();
        assert!(output.len() <= MAX_BYTES);
        assert_eq!(image::guess_format(&output).unwrap(), image::ImageFormat::Jpeg);
        let decoded = image::load_from_memory(&output).unwrap().to_rgb8();
        assert_eq!(decoded.dimensions(), (450, 225));
        assert!(decoded.get_pixel(0, 0).0.iter().all(|value| *value >= 250));
    }
}
