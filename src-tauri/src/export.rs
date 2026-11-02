//! The real export: warps the full-resolution original into the output frame and writes a file.

use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::tiff::TiffEncoder;
use image::{ExtendedColorType, ImageEncoder};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::photo::Photo;
use crate::warp::{crop_rect, homography, invert, is_valid, Matrix, Point, Quad, Rect};

/// Rendering is reported as this share of the work. Encoding and writing take the rest.
const RENDER_SHARE: f64 = 0.9;
const WEBP_MAX_SIDE: u32 = 16383;
const JPEG_MAX_SIDE: u32 = 65535;
const MAX_LONG_EDGE: u32 = 16384;
const MAX_SUPERSAMPLE: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Jpeg,
    Png,
    Tiff,
    Webp,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Jpeg => "jpg",
            Format::Png => "png",
            Format::Tiff => "tif",
            Format::Webp => "webp",
        }
    }

    fn accepts_extension(self, ext: &str) -> bool {
        let ext = ext.to_ascii_lowercase();
        match self {
            Format::Jpeg => ext == "jpg" || ext == "jpeg",
            Format::Png => ext == "png",
            Format::Tiff => ext == "tif" || ext == "tiff",
            Format::Webp => ext == "webp",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Format::Jpeg => "JPEG",
            Format::Png => "PNG",
            Format::Tiff => "TIFF",
            Format::Webp => "WebP",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmptyAreas {
    Crop,
    Stretch,
    Clear,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOptions {
    pub format: Format,
    /// The long edge of the output, or none for the original size.
    pub long_edge: Option<u32>,
    pub quality: u8,
    pub empty: EmptyAreas,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub file_name: String,
    pub width: u32,
    pub height: u32,
}

/// Rendered pixels: RGB, or RGBA when `channels` is 4.
pub struct Rendered {
    pub width: u32,
    pub height: u32,
    pub channels: usize,
    pub data: Vec<u8>,
}

fn check_input(photo: &Photo, src: &Quad, dst: &Quad, options: &ExportOptions) -> Result<(), AppError> {
    let (w, h) = photo.pixels.dimensions();
    if w == 0 || h == 0 {
        return Err(AppError::NoPhoto);
    }
    let finite = |q: &Quad| q.iter().all(|p: &Point| p.x.is_finite() && p.y.is_finite());
    if !finite(src) || !finite(dst) || !is_valid(src, dst, f64::from(w), f64::from(h)) {
        return Err(AppError::InvalidShape);
    }
    if !(40..=100).contains(&options.quality) {
        return Err(AppError::InvalidOptions(
            "Quality must be between 40 and 100%.".into(),
        ));
    }
    if let Some(edge) = options.long_edge {
        if edge == 0 || edge > MAX_LONG_EDGE {
            return Err(AppError::InvalidOptions(format!(
                "The long edge must be between 1 and {MAX_LONG_EDGE} px."
            )));
        }
    }
    if options.empty == EmptyAreas::Clear && options.format == Format::Jpeg {
        return Err(AppError::InvalidOptions(
            "JPEG can't keep areas transparent. Choose PNG, TIFF or WebP, or another option for empty areas.".into(),
        ));
    }
    Ok(())
}

/// The part of the frame to output, and the output's size in pixels.
fn plan(photo: &Photo, m: &Matrix, options: &ExportOptions) -> Result<(Rect, u32, u32), AppError> {
    let (w, h) = (
        f64::from(photo.pixels.width()),
        f64::from(photo.pixels.height()),
    );
    let view = match options.empty {
        EmptyAreas::Crop => crop_rect(m, w, h).ok_or(AppError::NothingToCrop)?,
        _ => Rect { x: 0.0, y: 0.0, w, h },
    };
    let scale = match options.long_edge {
        Some(edge) => f64::from(edge) / view.w.max(view.h),
        None => 1.0,
    };
    let out_w = (view.w * scale).round();
    let out_h = (view.h * scale).round();
    if out_w < 1.0 || out_h < 1.0 {
        return Err(AppError::NothingToCrop);
    }
    Ok((view, out_w as u32, out_h as u32))
}

/// How many samples per side each output pixel needs, so shrinking doesn't alias.
/// Measures how many photo pixels one output pixel spans, at a grid of places in the view.
fn supersample(inv: &Matrix, view: &Rect, out_w: u32, out_h: u32, w: f64, h: f64) -> u32 {
    let (sx, sy) = (view.w / f64::from(out_w), view.h / f64::from(out_h));
    let map = |x: f64, y: f64| {
        let d = inv[6] * x + inv[7] * y + inv[8];
        (d > 1e-12).then(|| {
            (
                (inv[0] * x + inv[1] * y + inv[2]) / d,
                (inv[3] * x + inv[4] * y + inv[5]) / d,
            )
        })
    };
    let mut span = 0.0_f64;
    for i in 0..=8 {
        for j in 0..=8 {
            let x = view.x + view.w * f64::from(i) / 8.0;
            let y = view.y + view.h * f64::from(j) / 8.0;
            let Some((u, v)) = map(x, y) else { continue };
            if u < 0.0 || v < 0.0 || u > w || v > h {
                continue;
            }
            for (x2, y2) in [(x + sx, y), (x, y + sy)] {
                if let Some((u2, v2)) = map(x2, y2) {
                    span = span.max((u2 - u).abs()).max((v2 - v).abs());
                }
            }
        }
    }
    // The margin keeps a plain 1:1 export sampling each pixel exactly once.
    ((span - 0.25).ceil() as u32).clamp(1, MAX_SUPERSAMPLE)
}

/// Bilinear sample at a continuous position, where pixel i covers [i, i + 1). Premultiplied.
#[inline]
fn sample(pixels: &[u8], w: usize, h: usize, u: f64, v: f64) -> [f32; 4] {
    let tx = (u - 0.5).clamp(0.0, (w - 1) as f64);
    let ty = (v - 0.5).clamp(0.0, (h - 1) as f64);
    let x0 = tx as usize;
    let y0 = ty as usize;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let fx = (tx - x0 as f64) as f32;
    let fy = (ty - y0 as f64) as f32;
    let at = |x: usize, y: usize, weight: f32, out: &mut [f32; 4]| {
        if weight == 0.0 {
            return;
        }
        let i = (y * w + x) * 4;
        let a = f32::from(pixels[i + 3]);
        out[0] += f32::from(pixels[i]) * a * weight;
        out[1] += f32::from(pixels[i + 1]) * a * weight;
        out[2] += f32::from(pixels[i + 2]) * a * weight;
        out[3] += a * weight;
    };
    let mut out = [0.0; 4];
    at(x0, y0, (1.0 - fx) * (1.0 - fy), &mut out);
    at(x1, y0, fx * (1.0 - fy), &mut out);
    at(x0, y1, (1.0 - fx) * fy, &mut out);
    at(x1, y1, fx * fy, &mut out);
    out
}

/// Maps each output pixel backwards through the inverse homography, so there are no holes.
pub fn render(
    photo: &Photo,
    src: &Quad,
    dst: &Quad,
    options: &ExportOptions,
    cancel: &AtomicBool,
    progress: &(dyn Fn(f64) + Sync),
) -> Result<Rendered, AppError> {
    check_input(photo, src, dst, options)?;
    let m = homography(src, dst);
    let inv = invert(&m);
    if inv.iter().any(|v| !v.is_finite()) {
        return Err(AppError::InvalidShape);
    }
    let (view, out_w, out_h) = plan(photo, &m, options)?;

    let (w, h) = (photo.pixels.width() as usize, photo.pixels.height() as usize);
    let (wf, hf) = (w as f64, h as f64);
    let ss = supersample(&inv, &view, out_w, out_h, wf, hf);
    let per_sample = 1.0 / (ss * ss) as f32;

    let keep_outside = options.empty != EmptyAreas::Clear;
    let alpha_out = options.format != Format::Jpeg && (!keep_outside || photo.has_alpha);
    let channels = if alpha_out { 4 } else { 3 };

    let (sx, sy) = (view.w / f64::from(out_w), view.h / f64::from(out_h));
    let pixels: &[u8] = photo.pixels.as_raw();
    let row_len = out_w as usize * channels;
    let total = u64::from(out_w) * u64::from(out_h) * channels as u64;
    let total = usize::try_from(total)
        .map_err(|_| AppError::InvalidOptions("That output size is too large. Choose a smaller size.".into()))?;
    let mut data = Vec::new();
    data.try_reserve_exact(total).map_err(|_| {
        AppError::InvalidOptions("There isn't enough memory for that output size. Choose a smaller size.".into())
    })?;
    data.resize(total, 0u8);

    let rows_done = AtomicUsize::new(0);
    let last_percent = AtomicU32::new(0);

    data.par_chunks_mut(row_len)
        .enumerate()
        .try_for_each(|(oy, row)| {
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::Cancelled);
            }
            for ox in 0..out_w as usize {
                let mut sum = [0.0_f32; 4];
                for j in 0..ss {
                    let fy = view.y + (oy as f64 + (f64::from(j) + 0.5) / f64::from(ss)) * sy;
                    for i in 0..ss {
                        let fx = view.x + (ox as f64 + (f64::from(i) + 0.5) / f64::from(ss)) * sx;
                        let d = inv[6] * fx + inv[7] * fy + inv[8];
                        let d_safe = d.max(1e-12);
                        let u = (inv[0] * fx + inv[1] * fy + inv[2]) / d_safe;
                        let v = (inv[3] * fx + inv[4] * fy + inv[5]) / d_safe;
                        let inside = d > 1e-12 && u >= 0.0 && v >= 0.0 && u <= wf && v <= hf;
                        if inside || keep_outside {
                            let s = sample(pixels, w, h, u, v);
                            for c in 0..4 {
                                sum[c] += s[c];
                            }
                        }
                    }
                }
                let a = sum[3] * per_sample;
                let out = &mut row[ox * channels..(ox + 1) * channels];
                if alpha_out {
                    if sum[3] > 0.0 {
                        out[0] = (sum[0] / sum[3] + 0.5) as u8;
                        out[1] = (sum[1] / sum[3] + 0.5) as u8;
                        out[2] = (sum[2] / sum[3] + 0.5) as u8;
                    }
                    out[3] = (a + 0.5) as u8;
                } else {
                    // Nowhere to keep alpha, so any transparency in the photo goes onto white.
                    let back = 255.0 * (255.0 - a);
                    out[0] = ((sum[0] * per_sample + back) / 255.0 + 0.5) as u8;
                    out[1] = ((sum[1] * per_sample + back) / 255.0 + 0.5) as u8;
                    out[2] = ((sum[2] * per_sample + back) / 255.0 + 0.5) as u8;
                }
            }

            let done = rows_done.fetch_add(1, Ordering::Relaxed) + 1;
            let percent = (done * 100 / out_h as usize) as u32;
            if last_percent.fetch_max(percent, Ordering::Relaxed) < percent {
                progress(f64::from(percent) / 100.0 * RENDER_SHARE);
            }
            Ok(())
        })?;

    Ok(Rendered {
        width: out_w,
        height: out_h,
        channels,
        data,
    })
}

fn map_write(e: impl std::fmt::Display) -> AppError {
    AppError::Write(e.to_string())
}

/// Encodes in the chosen format, keeping the colour profile and metadata where the format allows.
pub fn encode(
    out: &mut impl Write,
    image: &Rendered,
    photo: &Photo,
    options: &ExportOptions,
) -> Result<(), AppError> {
    let color = if image.channels == 4 {
        ExtendedColorType::Rgba8
    } else {
        ExtendedColorType::Rgb8
    };
    let side = image.width.max(image.height);
    let too_big = |limit: u32| {
        AppError::InvalidOptions(format!(
            "{} files can be at most {limit} px on each side. Choose a smaller size or another format.",
            options.format.label()
        ))
    };

    // Encoders report a profile they can't store as an error. That only loses metadata, so it is
    // not fatal. docs/DECISIONS.md lists what each format keeps.
    fn write_with(
        mut encoder: impl ImageEncoder,
        image: &Rendered,
        color: ExtendedColorType,
        photo: &Photo,
    ) -> Result<(), AppError> {
        if let Some(icc) = &photo.icc {
            let _ = encoder.set_icc_profile(icc.clone());
        }
        if let Some(exif) = &photo.exif {
            let _ = encoder.set_exif_metadata(exif.clone());
        }
        encoder
            .write_image(&image.data, image.width, image.height, color)
            .map_err(map_write)
    }

    match options.format {
        Format::Jpeg => {
            if side > JPEG_MAX_SIDE {
                return Err(too_big(JPEG_MAX_SIDE));
            }
            write_with(
                JpegEncoder::new_with_quality(&mut *out, options.quality),
                image,
                color,
                photo,
            )
        }
        Format::Png => write_with(PngEncoder::new(&mut *out), image, color, photo),
        Format::Tiff => {
            // The TIFF encoder needs to seek, so it writes to memory first.
            let mut buffer = std::io::Cursor::new(Vec::new());
            write_with(TiffEncoder::new(&mut buffer), image, color, photo)?;
            out.write_all(buffer.get_ref()).map_err(map_write)
        }
        Format::Webp => {
            if side > WEBP_MAX_SIDE {
                return Err(too_big(WEBP_MAX_SIDE));
            }
            let layout = if image.channels == 4 {
                webp::PixelLayout::Rgba
            } else {
                webp::PixelLayout::Rgb
            };
            let encoded = webp::Encoder::new(&image.data, layout, image.width, image.height)
                .encode_simple(false, f32::from(options.quality))
                .map_err(|e| AppError::Write(format!("{e:?}")))?;
            let bytes = webp_with_metadata(
                &encoded,
                image.width,
                image.height,
                photo.icc.as_deref(),
                photo.exif.as_deref(),
            );
            out.write_all(&bytes).map_err(map_write)
        }
    }
}

/// libwebp's simple encoder drops metadata, so this rebuilds the RIFF file with an extended
/// header and ICCP and EXIF chunks. Without metadata the file is returned as it is.
fn webp_with_metadata(
    file: &[u8],
    width: u32,
    height: u32,
    icc: Option<&[u8]>,
    exif: Option<&[u8]>,
) -> Vec<u8> {
    if (icc.is_none() && exif.is_none()) || file.len() < 12 {
        return file.to_vec();
    }
    fn push_chunk(out: &mut Vec<u8>, id: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(id);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        if data.len() % 2 == 1 {
            out.push(0);
        }
    }

    // Keep the image chunks that libwebp wrote, in order.
    let mut image_chunks = Vec::new();
    let mut has_alpha = false;
    let mut at = 12;
    while at + 8 <= file.len() {
        let id = &file[at..at + 4];
        let len = u32::from_le_bytes([file[at + 4], file[at + 5], file[at + 6], file[at + 7]]) as usize;
        let end = (at + 8 + len).min(file.len());
        if id == b"ALPH" {
            has_alpha = true;
        }
        if id != b"VP8X" {
            image_chunks.extend_from_slice(&file[at..end]);
            if len % 2 == 1 && end < file.len() {
                image_chunks.push(file[end]);
            }
        }
        at = end + (len % 2);
    }

    let mut flags = 0u8;
    if icc.is_some() {
        flags |= 0x20;
    }
    if has_alpha {
        flags |= 0x10;
    }
    if exif.is_some() {
        flags |= 0x08;
    }
    let mut vp8x = vec![flags, 0, 0, 0];
    vp8x.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
    vp8x.extend_from_slice(&(height - 1).to_le_bytes()[..3]);

    let mut body = Vec::with_capacity(file.len() + icc.map_or(0, <[u8]>::len) + exif.map_or(0, <[u8]>::len) + 64);
    body.extend_from_slice(b"WEBP");
    push_chunk(&mut body, b"VP8X", &vp8x);
    if let Some(icc) = icc {
        push_chunk(&mut body, b"ICCP", icc);
    }
    body.extend_from_slice(&image_chunks);
    if let Some(exif) = exif {
        push_chunk(&mut body, b"EXIF", exif);
    }

    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// Makes sure the file name ends in an extension of the chosen format.
pub fn with_format_extension(path: &Path, format: Format) -> PathBuf {
    let matches = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| format.accepts_extension(e));
    if matches {
        return path.to_path_buf();
    }
    let mut name = path.as_os_str().to_os_string();
    name.push(".");
    name.push(format.extension());
    PathBuf::from(name)
}

/// Removes characters that file systems refuse, and falls back if nothing is left.
pub fn clean_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();
    let trimmed = cleaned.trim().trim_matches('.');
    if trimmed.is_empty() {
        "photo_shifted".to_string()
    } else {
        trimmed.to_string()
    }
}

fn same_as_original(photo: &Photo, target: &Path) -> bool {
    photo.path.as_deref().is_some_and(|original| {
        original == target || same_file::is_same_file(original, target).unwrap_or(false)
    })
}

/// Renders and writes. The file only appears at its final path once it is complete, so a failed
/// or cancelled export leaves nothing behind.
pub fn export_to_path(
    photo: &Photo,
    src: &Quad,
    dst: &Quad,
    options: &ExportOptions,
    path: &Path,
    cancel: &AtomicBool,
    progress: &(dyn Fn(f64) + Sync),
) -> Result<ExportResult, AppError> {
    let target = with_format_extension(path, options.format);
    if same_as_original(photo, &target) {
        return Err(AppError::WouldOverwriteOriginal);
    }
    // The save dialog only confirmed the name it was given, not one with an added extension.
    if target != path && target.exists() {
        return Err(AppError::AlreadyExists);
    }
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| AppError::Write("the file has no folder".into()))?;

    let rendered = render(photo, src, dst, options, cancel, progress)?;

    let mut builder = tempfile::Builder::new();
    builder.prefix(".fpv-shifter-").suffix(".tmp");
    // A temporary file is private by default. The finished photo should have normal permissions.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(
            std::fs::metadata(&target)
                .map(|m| m.permissions())
                .unwrap_or_else(|_| std::fs::Permissions::from_mode(0o644)),
        );
    }
    let temp = builder.tempfile_in(parent).map_err(map_write)?;

    {
        let mut writer = BufWriter::new(temp.as_file());
        encode(&mut writer, &rendered, photo, options)?;
        writer.flush().map_err(map_write)?;
    }
    temp.as_file().sync_all().map_err(map_write)?;

    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::Cancelled);
    }
    temp.persist(&target).map_err(|e| map_write(e.error))?;
    progress(1.0);

    Ok(ExportResult {
        file_name: target
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        width: rendered.width,
        height: rendered.height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::warp::{corners, project};
    use image::{ImageDecoder, ImageFormat, ImageReader, Rgba, RgbaImage};
    use std::fs;
    use std::io::Cursor;

    static NEVER: AtomicBool = AtomicBool::new(false);

    fn noop(_: f64) {}

    fn pattern(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            Rgba([
                (x % 251) as u8,
                (y % 241) as u8,
                ((x * 7 + y * 13) % 256) as u8,
                255,
            ])
        })
    }

    fn photo(pixels: RgbaImage) -> Photo {
        Photo {
            pixels,
            has_alpha: false,
            icc: None,
            exif: None,
            path: None,
        }
    }

    fn opts(format: Format, empty: EmptyAreas) -> ExportOptions {
        ExportOptions {
            format,
            long_edge: None,
            quality: 90,
            empty,
        }
    }

    fn inset(w: f64, h: f64) -> Quad {
        [
            Point { x: w * 0.2, y: h * 0.2 },
            Point { x: w * 0.8, y: h * 0.2 },
            Point { x: w * 0.8, y: h * 0.8 },
            Point { x: w * 0.2, y: h * 0.8 },
        ]
    }

    /// Slid right and down, leaving a gap on the left and top.
    fn slid(q: &Quad, dx: f64, dy: f64) -> Quad {
        q.map(|p| Point { x: p.x + dx, y: p.y + dy })
    }

    /// A perspective shift with gaps along every edge.
    fn tilted(w: f64, h: f64) -> Quad {
        [
            Point { x: w * 0.28, y: h * 0.3 },
            Point { x: w * 0.74, y: h * 0.24 },
            Point { x: w * 0.7, y: h * 0.72 },
            Point { x: w * 0.3, y: h * 0.68 },
        ]
    }

    #[test]
    fn no_shift_reproduces_the_input_pixel_for_pixel() {
        let p = photo(pattern(257, 131));
        let q = inset(257.0, 131.0);
        for empty in [EmptyAreas::Stretch, EmptyAreas::Crop] {
            let out = render(&p, &q, &q, &opts(Format::Png, empty), &NEVER, &noop).unwrap();
            assert_eq!((out.width, out.height, out.channels), (257, 131, 3));
            for (a, b) in out.data.chunks_exact(3).zip(p.pixels.chunks_exact(4)) {
                assert_eq!(a, &b[..3]);
            }
        }
        // Keep transparent: every pixel is in the photo, so alpha is full too.
        let out = render(&p, &q, &q, &opts(Format::Png, EmptyAreas::Clear), &NEVER, &noop).unwrap();
        assert_eq!(out.channels, 4);
        assert_eq!(out.data, *p.pixels.as_raw());
    }

    #[test]
    fn a_test_pattern_lands_where_the_homography_predicts() {
        // Small blocks on a grey ground, so each one can be found in the output.
        let (w, h) = (640u32, 480u32);
        let mut img = RgbaImage::from_pixel(w, h, Rgba([128, 128, 128, 255]));
        let marks = [(120u32, 90u32), (500, 110), (320, 240), (150, 380), (480, 360)];
        for &(cx, cy) in &marks {
            for y in cy - 3..=cy + 3 {
                for x in cx - 3..=cx + 3 {
                    img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                }
            }
        }
        let p = photo(img);
        let (wf, hf) = (f64::from(w), f64::from(h));
        let src = inset(wf, hf);
        let dst = [
            Point { x: wf * 0.16, y: hf * 0.24 },
            Point { x: wf * 0.84, y: hf * 0.16 },
            Point { x: wf * 0.78, y: hf * 0.8 },
            Point { x: wf * 0.24, y: hf * 0.76 },
        ];
        let m = homography(&src, &dst);
        let out = render(&p, &src, &dst, &opts(Format::Png, EmptyAreas::Clear), &NEVER, &noop).unwrap();

        for &(cx, cy) in &marks {
            // Pixel (cx, cy) has its centre at +0.5 in continuous coordinates.
            let (q, _) = project(&m, Point { x: f64::from(cx) + 0.5, y: f64::from(cy) + 0.5 });
            // Find the middle of the red blob near the prediction.
            let (mut sx, mut sy, mut n) = (0.0, 0.0, 0.0);
            for y in (q.y as i64 - 12).max(0)..(q.y as i64 + 12).min(i64::from(h)) {
                for x in (q.x as i64 - 12).max(0)..(q.x as i64 + 12).min(i64::from(w)) {
                    let i = (y as usize * w as usize + x as usize) * 4;
                    if out.data[i] > 200 && out.data[i + 1] < 60 {
                        sx += x as f64 + 0.5;
                        sy += y as f64 + 0.5;
                        n += 1.0;
                    }
                }
            }
            assert!(n > 10.0, "no mark found near {q:?}");
            assert!((sx / n - q.x).abs() < 0.75, "x {} vs {}", sx / n, q.x);
            assert!((sy / n - q.y).abs() < 0.75, "y {} vs {}", sy / n, q.y);
        }
    }

    #[test]
    fn keep_transparent_is_clear_outside_the_photo_and_opaque_inside() {
        let (w, h) = (320u32, 200u32);
        let p = photo(pattern(w, h));
        let (wf, hf) = (f64::from(w), f64::from(h));
        let src = inset(wf, hf);
        let dst = tilted(wf, hf);
        let m = homography(&src, &dst);
        let out = render(&p, &src, &dst, &opts(Format::Png, EmptyAreas::Clear), &NEVER, &noop).unwrap();
        assert_eq!(out.channels, 4);
        let poly = corners(wf, hf).map(|c| project(&m, c).0);
        let (mut clear, mut solid) = (0, 0);
        for y in 0..h as usize {
            for x in 0..w as usize {
                let a = out.data[(y * w as usize + x) * 4 + 3];
                let c = Point { x: x as f64 + 0.5, y: y as f64 + 0.5 };
                // Skip pixels right on the edge, which can go either way.
                match signed_distance_inside(&poly, c) {
                    d if d > 1.0 => {
                        assert_eq!(a, 255, "({x},{y}) should be opaque");
                        solid += 1;
                    }
                    d if d < -1.0 => {
                        assert_eq!(a, 0, "({x},{y}) should be clear");
                        clear += 1;
                    }
                    _ => {}
                }
            }
        }
        assert!(clear > 100 && solid > 100, "{clear} clear, {solid} solid");
    }

    /// Positive inside a convex shape: the distance to its nearest edge.
    fn signed_distance_inside(poly: &[Point; 4], p: Point) -> f64 {
        let mut area = 0.0;
        for i in 0..4 {
            let (a, b) = (poly[i], poly[(i + 1) % 4]);
            area += a.x * b.y - b.x * a.y;
        }
        let sign = area.signum();
        let mut best = f64::INFINITY;
        for i in 0..4 {
            let (a, b) = (poly[i], poly[(i + 1) % 4]);
            let len = (b.x - a.x).hypot(b.y - a.y);
            best = best.min(sign * ((b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)) / len);
        }
        best
    }

    #[test]
    fn stretch_clamps_to_the_edge_of_the_photo() {
        let (w, h) = (120u32, 80u32);
        // Left half red, right half blue.
        let img = RgbaImage::from_fn(w, h, |x, _| {
            if x < w / 2 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 255])
            }
        });
        let p = photo(img);
        let (wf, hf) = (f64::from(w), f64::from(h));
        let src = inset(wf, hf);
        // Slide right by 30 px, opening a gap down the left side.
        let dst = slid(&src, 30.0, 0.0);
        let out = render(&p, &src, &dst, &opts(Format::Png, EmptyAreas::Stretch), &NEVER, &noop).unwrap();
        assert_eq!(out.channels, 3);
        for y in 0..h as usize {
            // The gap repeats the photo's left edge, which is red.
            for x in 0..30 {
                let i = (y * w as usize + x) * 3;
                assert_eq!(&out.data[i..i + 3], &[255, 0, 0], "({x},{y})");
            }
            // The blue half now starts at 60 + 30.
            let i = (y * w as usize + 95) * 3;
            assert_eq!(&out.data[i..i + 3], &[0, 0, 255]);
        }
    }

    #[test]
    fn stretch_handles_pixels_past_the_horizon_without_failing() {
        let (w, h) = (200u32, 200u32);
        let p = photo(pattern(w, h));
        let src = inset(200.0, 200.0);
        // Strong convergence, so part of the frame is beyond the photo's horizon line.
        let dst = [
            Point { x: 40.0, y: 30.0 },
            Point { x: 90.0, y: 80.0 },
            Point { x: 90.0, y: 120.0 },
            Point { x: 40.0, y: 170.0 },
        ];
        if is_valid(&src, &dst, 200.0, 200.0) {
            let out = render(&p, &src, &dst, &opts(Format::Png, EmptyAreas::Stretch), &NEVER, &noop).unwrap();
            assert_eq!(out.data.len(), 200 * 200 * 3);
        }
    }

    #[test]
    fn crop_contains_no_empty_pixels_and_keeps_the_aspect_ratio() {
        let (w, h) = (640u32, 400u32);
        let pixels = RgbaImage::from_pixel(w, h, Rgba([0, 255, 0, 255]));
        let p = photo(pixels);
        let (wf, hf) = (f64::from(w), f64::from(h));
        let src = inset(wf, hf);
        let dst = tilted(wf, hf);
        let m = homography(&src, &dst);

        let out = render(&p, &src, &dst, &opts(Format::Png, EmptyAreas::Crop), &NEVER, &noop).unwrap();
        assert!(out.width < w && out.height < h, "should have cropped");
        let ratio = f64::from(out.width) / f64::from(out.height);
        assert!((ratio - wf / hf).abs() < 0.01, "ratio {ratio}");

        // Every output pixel centre, mapped back, lands inside the photo.
        let r = crop_rect(&m, wf, hf).unwrap();
        let inv = invert(&m);
        for oy in 0..out.height {
            for ox in 0..out.width {
                let c = Point {
                    x: r.x + (f64::from(ox) + 0.5) * r.w / f64::from(out.width),
                    y: r.y + (f64::from(oy) + 0.5) * r.h / f64::from(out.height),
                };
                let (s, d) = project(&inv, c);
                assert!(d > 0.0);
                assert!(s.x >= 0.0 && s.x <= wf && s.y >= 0.0 && s.y <= hf, "({ox},{oy})");
            }
        }

        // And compare with a render that keeps gaps transparent: inside the crop's outer row of
        // pixels, which can straddle the photo's edge in that render, everything is opaque.
        let clear = render(&p, &src, &dst, &opts(Format::Png, EmptyAreas::Clear), &NEVER, &noop).unwrap();
        for oy in 1..out.height - 1 {
            for ox in 1..out.width - 1 {
                let fx = (r.x + (f64::from(ox) + 0.5) * r.w / f64::from(out.width)) as usize;
                let fy = (r.y + (f64::from(oy) + 0.5) * r.h / f64::from(out.height)) as usize;
                assert_eq!(clear.data[(fy * w as usize + fx) * 4 + 3], 255, "({ox},{oy})");
            }
        }
    }

    #[test]
    fn crop_fails_clearly_when_nothing_is_left() {
        let p = photo(pattern(100, 100));
        let src = inset(100.0, 100.0);
        let dst = slid(&src, 500.0, 0.0);
        assert!(matches!(
            render(&p, &src, &dst, &opts(Format::Png, EmptyAreas::Crop), &NEVER, &noop),
            Err(AppError::NothingToCrop)
        ));
    }

    #[test]
    fn size_options_scale_and_keep_the_aspect_ratio() {
        let p = photo(pattern(640, 400));
        let q = inset(640.0, 400.0);
        let mut o = opts(Format::Png, EmptyAreas::Stretch);
        o.long_edge = Some(320);
        let out = render(&p, &q, &q, &o, &NEVER, &noop).unwrap();
        assert_eq!((out.width, out.height), (320, 200));
        o.long_edge = Some(1280);
        let out = render(&p, &q, &q, &o, &NEVER, &noop).unwrap();
        assert_eq!((out.width, out.height), (1280, 800));

        // A size applies to what is output, so a crop still reaches the chosen long edge.
        let dst = tilted(640.0, 400.0);
        o.long_edge = Some(320);
        o.empty = EmptyAreas::Crop;
        let out = render(&p, &q, &dst, &o, &NEVER, &noop).unwrap();
        assert_eq!(out.width, 320);
    }

    #[test]
    fn shrinking_averages_instead_of_aliasing() {
        // A 1 px checker averages to mid grey. Plain point sampling would keep it black and white.
        let img = RgbaImage::from_fn(400, 400, |x, y| {
            let v = if (x + y) % 2 == 0 { 0 } else { 255 };
            Rgba([v, v, v, 255])
        });
        let p = photo(img);
        let q = inset(400.0, 400.0);
        let mut o = opts(Format::Png, EmptyAreas::Stretch);
        o.long_edge = Some(100);
        let out = render(&p, &q, &q, &o, &NEVER, &noop).unwrap();
        for px in out.data.chunks_exact(3) {
            assert!((96..=159).contains(&px[0]), "{}", px[0]);
        }
    }

    #[test]
    fn bad_input_is_rejected_with_errors() {
        let p = photo(pattern(100, 100));
        let q = inset(100.0, 100.0);
        let ok = opts(Format::Png, EmptyAreas::Stretch);
        let run = |s: &Quad, d: &Quad, o: &ExportOptions| render(&p, s, d, o, &NEVER, &noop);

        let crossed = [q[0], q[2], q[1], q[3]];
        assert!(matches!(run(&q, &crossed, &ok), Err(AppError::InvalidShape)));
        let flipped = [q[1], q[0], q[3], q[2]];
        assert!(matches!(run(&q, &flipped, &ok), Err(AppError::InvalidShape)));
        let nan = [Point { x: f64::NAN, y: 0.0 }, q[1], q[2], q[3]];
        assert!(matches!(run(&q, &nan, &ok), Err(AppError::InvalidShape)));
        let huge = [Point { x: f64::INFINITY, y: 0.0 }, q[1], q[2], q[3]];
        assert!(matches!(run(&huge, &q, &ok), Err(AppError::InvalidShape)));

        let mut o = ok.clone();
        o.quality = 10;
        assert!(matches!(run(&q, &q, &o), Err(AppError::InvalidOptions(_))));
        let mut o = ok.clone();
        o.long_edge = Some(0);
        assert!(matches!(run(&q, &q, &o), Err(AppError::InvalidOptions(_))));
        let mut o = ok.clone();
        o.long_edge = Some(1_000_000);
        assert!(matches!(run(&q, &q, &o), Err(AppError::InvalidOptions(_))));
        let o = opts(Format::Jpeg, EmptyAreas::Clear);
        assert!(matches!(run(&q, &q, &o), Err(AppError::InvalidOptions(_))));
    }

    #[test]
    fn each_format_writes_a_file_that_decodes_back_at_the_right_size() {
        let dir = tempfile::tempdir().unwrap();
        let p = photo(pattern(96, 64));
        let q = inset(96.0, 64.0);
        // Gaps down one side, so the transparent exports really have transparency to keep.
        let shifted = slid(&q, 12.0, 0.0);
        for (format, empty, expected) in [
            (Format::Jpeg, EmptyAreas::Stretch, ImageFormat::Jpeg),
            (Format::Png, EmptyAreas::Clear, ImageFormat::Png),
            (Format::Tiff, EmptyAreas::Clear, ImageFormat::Tiff),
            (Format::Webp, EmptyAreas::Clear, ImageFormat::WebP),
            (Format::Webp, EmptyAreas::Stretch, ImageFormat::WebP),
        ] {
            let path = dir.path().join(format!("out_{:?}_{:?}.{}", format, empty, format.extension()));
            let result =
                export_to_path(&p, &q, &shifted, &opts(format, empty), &path, &NEVER, &noop).unwrap();
            assert_eq!((result.width, result.height), (96, 64));
            let reader = ImageReader::open(&path).unwrap().with_guessed_format().unwrap();
            assert_eq!(reader.format(), Some(expected));
            let decoded = reader.decode().unwrap();
            assert_eq!((decoded.width(), decoded.height()), (96, 64), "{format:?}");
            if empty == EmptyAreas::Clear {
                assert!(decoded.color().has_alpha(), "{format:?} lost its alpha");
            }
        }
    }

    #[test]
    fn webp_quality_changes_the_file_size() {
        let p = photo(pattern(256, 256));
        let q = inset(256.0, 256.0);
        let rendered = render(&p, &q, &q, &opts(Format::Webp, EmptyAreas::Stretch), &NEVER, &noop).unwrap();
        let size = |quality: u8| {
            let mut o = opts(Format::Webp, EmptyAreas::Stretch);
            o.quality = quality;
            let mut out = Vec::new();
            encode(&mut out, &rendered, &p, &o).unwrap();
            out.len()
        };
        assert!(size(40) < size(100));
    }

    fn sample_exif() -> Vec<u8> {
        let mut c = vec![0x49, 0x49, 42, 0, 8, 0, 0, 0, 1, 0];
        c.extend_from_slice(&0x0112u16.to_le_bytes());
        c.extend_from_slice(&3u16.to_le_bytes());
        c.extend_from_slice(&1u32.to_le_bytes());
        c.extend_from_slice(&1u16.to_le_bytes());
        c.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
        c
    }

    #[test]
    fn the_colour_profile_and_exif_are_kept_where_the_format_allows() {
        let icc: Vec<u8> = (0..301u32).map(|i| (i % 256) as u8).collect();
        let exif = sample_exif();
        let mut p = photo(pattern(48, 32));
        p.icc = Some(icc.clone());
        p.exif = Some(exif.clone());
        let q = inset(48.0, 32.0);
        let rendered_opaque = render(&p, &q, &q, &opts(Format::Png, EmptyAreas::Stretch), &NEVER, &noop).unwrap();
        let shifted = slid(&q, 6.0, 0.0);
        let rendered_alpha = render(&p, &q, &shifted, &opts(Format::Png, EmptyAreas::Clear), &NEVER, &noop).unwrap();
        assert_eq!(rendered_alpha.channels, 4);

        // format, image, should EXIF survive?
        for (format, image, keeps_exif) in [
            (Format::Jpeg, &rendered_opaque, true),
            (Format::Png, &rendered_opaque, true),
            (Format::Tiff, &rendered_opaque, false),
            (Format::Webp, &rendered_opaque, true),
            (Format::Webp, &rendered_alpha, true),
        ] {
            let mut out = Vec::new();
            encode(&mut out, image, &p, &opts(format, EmptyAreas::Stretch)).unwrap();
            let mut decoder = ImageReader::new(Cursor::new(&out))
                .with_guessed_format()
                .unwrap()
                .into_decoder()
                .unwrap();
            assert_eq!(decoder.dimensions(), (48, 32), "{format:?}");
            let got_icc = if format == Format::Tiff {
                crate::photo::tiff_icc_profile(&mut Cursor::new(&out))
            } else {
                decoder.icc_profile().unwrap()
            };
            assert_eq!(got_icc.as_deref(), Some(&icc[..]), "{format:?} ICC");
            let got = decoder.exif_metadata().unwrap();
            if keeps_exif {
                assert_eq!(got.as_deref(), Some(&exif[..]), "{format:?} EXIF");
            }
            // The pixels still decode.
            let mut buf = vec![0; decoder.total_bytes() as usize];
            decoder.read_image(&mut buf).unwrap();
        }
    }

    #[test]
    fn cancelling_stops_the_export_and_leaves_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = photo(pattern(64, 64));
        let q = inset(64.0, 64.0);
        let path = dir.path().join("cancelled.png");
        let cancel = AtomicBool::new(true);
        let result = export_to_path(&p, &q, &q, &opts(Format::Png, EmptyAreas::Stretch), &path, &cancel, &noop);
        assert!(matches!(result, Err(AppError::Cancelled)));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0, "left a file behind");
    }

    #[test]
    fn a_failed_export_leaves_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = photo(pattern(64, 64));
        let q = inset(64.0, 64.0);
        let path = dir.path().join("bad.png");
        let crossed = [q[0], q[2], q[1], q[3]];
        assert!(export_to_path(&p, &q, &crossed, &opts(Format::Png, EmptyAreas::Stretch), &path, &NEVER, &noop).is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);

        // An encoder limit is only found after rendering starts, and leaves nothing behind either.
        let wide = photo(pattern(16384, 2));
        let q = inset(16384.0, 2.0);
        let dst = [
            Point { x: q[0].x, y: 0.0 },
            Point { x: q[1].x, y: 0.0 },
            Point { x: q[2].x, y: 2.0 },
            Point { x: q[3].x, y: 2.0 },
        ];
        let src = dst;
        let path = dir.path().join("wide.webp");
        let result = export_to_path(&wide, &src, &dst, &opts(Format::Webp, EmptyAreas::Stretch), &path, &NEVER, &noop);
        assert!(matches!(result, Err(AppError::InvalidOptions(_))));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn the_original_is_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("photo.png");
        fs::write(&original, b"the original").unwrap();
        let mut p = photo(pattern(32, 32));
        p.path = Some(original.clone());
        let q = inset(32.0, 32.0);
        let o = opts(Format::Png, EmptyAreas::Stretch);

        let result = export_to_path(&p, &q, &q, &o, &original, &NEVER, &noop);
        assert!(matches!(result, Err(AppError::WouldOverwriteOriginal)));

        // The same file reached by another path.
        let indirect = dir.path().join(".").join("photo.png");
        let result = export_to_path(&p, &q, &q, &o, &indirect, &NEVER, &noop);
        assert!(matches!(result, Err(AppError::WouldOverwriteOriginal)));

        // Adding the extension must not get round the check.
        let no_ext = dir.path().join("photo");
        let result = export_to_path(&p, &q, &q, &o, &no_ext, &NEVER, &noop);
        assert!(matches!(result, Err(AppError::WouldOverwriteOriginal)));

        assert_eq!(fs::read(&original).unwrap(), b"the original");
    }

    #[test]
    fn an_existing_file_is_only_replaced_at_the_name_the_dialog_confirmed() {
        let dir = tempfile::tempdir().unwrap();
        let p = photo(pattern(32, 32));
        let q = inset(32.0, 32.0);
        let o = opts(Format::Png, EmptyAreas::Stretch);

        // Confirmed in the dialog: replaced in full.
        let confirmed = dir.path().join("a.png");
        fs::write(&confirmed, b"old").unwrap();
        export_to_path(&p, &q, &q, &o, &confirmed, &NEVER, &noop).unwrap();
        assert!(fs::read(&confirmed).unwrap().starts_with(&[0x89, b'P', b'N', b'G']));

        // Dialog returned "b", the extension is added, and "b.png" was never confirmed.
        let existing = dir.path().join("b.png");
        fs::write(&existing, b"old").unwrap();
        let result = export_to_path(&p, &q, &q, &o, &dir.path().join("b"), &NEVER, &noop);
        assert!(matches!(result, Err(AppError::AlreadyExists)));
        assert_eq!(fs::read(&existing).unwrap(), b"old");
    }

    #[cfg(unix)]
    #[test]
    fn the_finished_file_has_normal_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let p = photo(pattern(16, 16));
        let q = inset(16.0, 16.0);
        let path = dir.path().join("a.png");
        export_to_path(&p, &q, &q, &opts(Format::Png, EmptyAreas::Stretch), &path, &NEVER, &noop).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o644);
    }

    #[test]
    fn progress_rises_to_the_end() {
        let dir = tempfile::tempdir().unwrap();
        let p = photo(pattern(64, 256));
        let q = inset(64.0, 256.0);
        let seen = std::sync::Mutex::new(Vec::new());
        let report = |f: f64| seen.lock().unwrap().push(f);
        let path = dir.path().join("a.png");
        export_to_path(&p, &q, &q, &opts(Format::Png, EmptyAreas::Stretch), &path, &NEVER, &report).unwrap();
        let seen = seen.into_inner().unwrap();
        assert!(seen.len() > 2);
        assert_eq!(*seen.last().unwrap(), 1.0);
    }

    #[test]
    fn names_and_extensions_are_cleaned() {
        assert_eq!(clean_file_name("my/photo:1"), "myphoto1");
        assert_eq!(clean_file_name("  ..  "), "photo_shifted");
        assert_eq!(clean_file_name(""), "photo_shifted");
        assert_eq!(clean_file_name("../../etc/passwd"), "etcpasswd");
        assert_eq!(
            with_format_extension(Path::new("/a/b.JPEG"), Format::Jpeg),
            PathBuf::from("/a/b.JPEG")
        );
        assert_eq!(
            with_format_extension(Path::new("/a/b.jpg"), Format::Png),
            PathBuf::from("/a/b.jpg.png")
        );
        assert_eq!(
            with_format_extension(Path::new("/a/b"), Format::Tiff),
            PathBuf::from("/a/b.tif")
        );
    }
}
