//! 影像管線：依縮放模式以 Lanczos3（SIMD）縮放 → 編碼。
//!
//! 全案畫質與效能命脈。原則：**後端把影像縮到顯示尺寸後才送出**，
//! 前端以 1:1 原生像素呈現，不做二次縮放。
//! 縮放交由 `fast_image_resize`（SIMD 加速）處理，畫質維持 Lanczos3。

use fast_image_resize::images::Image as FirImage;
use fast_image_resize::{FilterType as FirFilter, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder};
use image::{DynamicImage, ImageEncoder};

/// 縮放配合模式。
pub enum FitMode {
    /// 配合視窗：長短邊皆不超出視窗。
    Window,
    /// 配合寬度。
    Width,
    /// 配合高度。
    Height,
    /// 原始尺寸（1:1）。
    Original,
    /// 固定倍率。
    Fixed,
}

impl FitMode {
    pub fn parse(s: &str) -> FitMode {
        match s {
            "width" => FitMode::Width,
            "height" => FitMode::Height,
            "original" => FitMode::Original,
            "fixed" => FitMode::Fixed,
            _ => FitMode::Window,
        }
    }
}

/// 縮放請求：視窗（或單頁配額）尺寸與固定倍率。
pub struct ScaleSpec {
    pub mode: FitMode,
    pub viewport_w: u32,
    pub viewport_h: u32,
    pub fixed_scale: f32,
}

/// 縮放後單張影像的尺寸上限（像素），避免放大模式吃爆記憶體。
const MAX_DIMENSION: u32 = 12_000;

/// 對已解碼影像依模式縮放，再編碼為 PNG 位元組。
pub fn render(img: &DynamicImage, spec: &ScaleSpec) -> Result<Vec<u8>, String> {
    let (ow, oh) = (img.width(), img.height());
    if ow == 0 || oh == 0 {
        return Err("影像尺寸為零。".into());
    }

    let scale = compute_scale(&spec.mode, ow, oh, spec);
    if (scale - 1.0).abs() < 0.005 {
        return encode_png(img);
    }

    let mut tw = (ow as f32 * scale).round().max(1.0) as u32;
    let mut th = (oh as f32 * scale).round().max(1.0) as u32;
    // 夾在尺寸上限內（等比）。
    if tw > MAX_DIMENSION || th > MAX_DIMENSION {
        let cap = (MAX_DIMENSION as f32 / tw as f32).min(MAX_DIMENSION as f32 / th as f32);
        tw = (tw as f32 * cap).max(1.0) as u32;
        th = (th as f32 * cap).max(1.0) as u32;
    }

    let scaled = resize_lanczos3(img, tw, th)?;
    encode_png(&scaled)
}

/// 以 SIMD 加速的 Lanczos3 卷積縮放，依原圖色彩格式選擇通道數以兼顧速度與畫質。
fn resize_lanczos3(img: &DynamicImage, tw: u32, th: u32) -> Result<DynamicImage, String> {
    let (ow, oh) = (img.width(), img.height());

    let (pixel_type, src_buf): (PixelType, Vec<u8>) = match img {
        DynamicImage::ImageLuma8(b) => (PixelType::U8, b.as_raw().clone()),
        DynamicImage::ImageLumaA8(b) => (PixelType::U8x2, b.as_raw().clone()),
        DynamicImage::ImageRgb8(b) => (PixelType::U8x3, b.as_raw().clone()),
        DynamicImage::ImageRgba8(b) => (PixelType::U8x4, b.as_raw().clone()),
        // 其餘（16-bit 等）統一轉 RGBA8 處理。
        other => (PixelType::U8x4, other.to_rgba8().into_raw()),
    };

    let src = FirImage::from_vec_u8(ow, oh, src_buf, pixel_type)
        .map_err(|e| format!("建立來源影像失敗：{e}"))?;
    let mut dst = FirImage::new(tw, th, pixel_type);
    let mut resizer = Resizer::new();
    let opts = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FirFilter::Lanczos3));
    resizer
        .resize(&src, &mut dst, &opts)
        .map_err(|e| format!("縮放失敗：{e}"))?;
    let out = dst.into_vec();

    let rebuilt = match pixel_type {
        PixelType::U8 => DynamicImage::ImageLuma8(
            image::GrayImage::from_raw(tw, th, out).ok_or("重建影像失敗。")?,
        ),
        PixelType::U8x2 => DynamicImage::ImageLumaA8(
            image::GrayAlphaImage::from_raw(tw, th, out).ok_or("重建影像失敗。")?,
        ),
        PixelType::U8x3 => DynamicImage::ImageRgb8(
            image::RgbImage::from_raw(tw, th, out).ok_or("重建影像失敗。")?,
        ),
        _ => DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(tw, th, out).ok_or("重建影像失敗。")?,
        ),
    };
    Ok(rebuilt)
}

/// 以「快速壓縮」編碼 PNG：閱讀器在意的是翻頁延遲，而非檔案最小化。
fn encode_png(img: &DynamicImage) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let encoder = PngEncoder::new_with_quality(&mut buf, CompressionType::Fast, PngFilter::Adaptive);
    encoder
        .write_image(img.as_bytes(), img.width(), img.height(), img.color().into())
        .map_err(|e| format!("編碼影像失敗：{e}"))?;
    Ok(buf)
}

fn compute_scale(mode: &FitMode, ow: u32, oh: u32, spec: &ScaleSpec) -> f32 {
    let vw = spec.viewport_w.max(1) as f32;
    let vh = spec.viewport_h.max(1) as f32;
    let ow = ow as f32;
    let oh = oh as f32;
    match mode {
        FitMode::Window => (vw / ow).min(vh / oh),
        FitMode::Width => vw / ow,
        FitMode::Height => vh / oh,
        FitMode::Original => 1.0,
        FitMode::Fixed => spec.fixed_scale.max(0.05),
    }
}
