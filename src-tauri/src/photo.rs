//! Opening photos: decoding, EXIF orientation, keeping the full-resolution original in memory,
//! and making the preview that the webview shows.

use std::io::{BufRead, Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine as _;
use image::codecs::jpeg::JpegEncoder;
use image::metadata::Orientation;
use image::{
    DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageFormat, ImageReader, Limits,
    RgbImage, RgbaImage,
};
use serde::Serialize;

use crate::error::AppError;

/// The longest edge of the preview shown in the webview.
pub const PREVIEW_LONG_EDGE: u32 = 2560;
const PREVIEW_QUALITY: u8 = 88;
/// 400 megapixels takes 1.6 GB of memory as RGBA.
pub const MAX_MEGAPIXELS: u64 = 400;

/// A fully decoded photo, always upright.
pub struct Photo {
    pub pixels: RgbaImage,
    pub has_alpha: bool,
    pub icc: Option<Vec<u8>>,
    /// EXIF with its orientation already cleared, because the pixels are upright.
    pub exif: Option<Vec<u8>>,
    /// Where it came from. The sample photo has none.
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedPhoto {
    pub width: u32,
    pub height: u32,
    pub file_name: String,
    /// A base64 JPEG, at most `PREVIEW_LONG_EDGE` on its long edge.
    pub preview: String,
}

/// The formats the design lists. Anything else is refused, even if a decoder exists.
fn is_supported(format: ImageFormat) -> bool {
    matches!(
        format,
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::Tiff | ImageFormat::WebP
    )
}

pub fn decode_file(path: &Path) -> Result<Photo, AppError> {
    let reader = ImageReader::open(path).map_err(|e| AppError::Read(e.to_string()))?;
    let mut photo = decode(reader)?;
    photo.path = Some(path.to_path_buf());
    Ok(photo)
}

pub fn decode_bytes(bytes: &[u8]) -> Result<Photo, AppError> {
    decode(ImageReader::new(Cursor::new(bytes)))
}

fn decode<R: BufRead + Seek>(reader: ImageReader<R>) -> Result<Photo, AppError> {
    let reader = reader
        .with_guessed_format()
        .map_err(|e| AppError::Read(e.to_string()))?;
    let format = match reader.format() {
        Some(f) if is_supported(f) => f,
        _ => return Err(AppError::Unsupported),
    };

    // The decoder can't read a TIFF's colour profile, so find it first and rewind.
    let mut source = reader.into_inner();
    let tiff_icc = if format == ImageFormat::Tiff {
        tiff_icc_profile(&mut source)
    } else {
        None
    };
    source
        .seek(SeekFrom::Start(0))
        .map_err(|e| AppError::Read(e.to_string()))?;
    let mut reader = ImageReader::with_format(source, format);

    // Decoders need 8 bytes a pixel at worst, for 16-bit RGBA. The size is checked exactly below.
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_MEGAPIXELS * 1_000_000 * 8 + 256 * 1024 * 1024);
    reader.limits(limits);

    let mut decoder = reader.into_decoder().map_err(map_decode_error)?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > MAX_MEGAPIXELS * 1_000_000 {
        return Err(AppError::TooLarge {
            width,
            height,
            limit: MAX_MEGAPIXELS,
        });
    }
    if width == 0 || height == 0 {
        return Err(AppError::Corrupt("the image is empty".into()));
    }

    // Damaged metadata should not stop a photo opening whose pixels are fine.
    let icc = tiff_icc.or_else(|| decoder.icc_profile().ok().flatten());
    let mut exif = decoder.exif_metadata().ok().flatten();
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    if let Some(chunk) = exif.as_mut() {
        let _ = Orientation::remove_from_exif_chunk(chunk);
    }

    let mut image = DynamicImage::from_decoder(decoder).map_err(map_decode_error)?;
    image.apply_orientation(orientation);

    let has_alpha = image.color().has_alpha();
    Ok(Photo {
        pixels: image.into_rgba8(),
        has_alpha,
        icc,
        exif,
        path: None,
    })
}

fn map_decode_error(e: image::ImageError) -> AppError {
    use image::error::ImageError;
    match e {
        ImageError::Unsupported(_) => AppError::Unsupported,
        ImageError::Limits(_) => AppError::TooLargeToDecode {
            limit: MAX_MEGAPIXELS,
        },
        other => AppError::Corrupt(other.to_string()),
    }
}

/// Reads the ICC profile tag from the first directory of a classic TIFF. The file is untrusted, so
/// anything unexpected just means no profile.
pub fn tiff_icc_profile<R: Read + Seek>(r: &mut R) -> Option<Vec<u8>> {
    const ICC_TAG: u16 = 34675;
    const MAX_PROFILE: u32 = 64 * 1024 * 1024;

    r.seek(SeekFrom::Start(0)).ok()?;
    let mut head = [0u8; 8];
    r.read_exact(&mut head).ok()?;
    let little = match (&head[..2], head[2], head[3]) {
        (b"II", 42, 0) => true,
        (b"MM", 0, 42) => false,
        _ => return None, // BigTIFF and anything else
    };
    let u16_at = |b: &[u8]| {
        let v = [b[0], b[1]];
        if little { u16::from_le_bytes(v) } else { u16::from_be_bytes(v) }
    };
    let u32_at = |b: &[u8]| {
        let v = [b[0], b[1], b[2], b[3]];
        if little { u32::from_le_bytes(v) } else { u32::from_be_bytes(v) }
    };

    r.seek(SeekFrom::Start(u64::from(u32_at(&head[4..])))).ok()?;
    let mut count = [0u8; 2];
    r.read_exact(&mut count).ok()?;
    for _ in 0..u16_at(&count) {
        let mut entry = [0u8; 12];
        r.read_exact(&mut entry).ok()?;
        if u16_at(&entry) != ICC_TAG {
            continue;
        }
        let size = u32_at(&entry[4..]);
        if size == 0 || size > MAX_PROFILE {
            return None;
        }
        if size <= 4 {
            return Some(entry[8..8 + size as usize].to_vec());
        }
        r.seek(SeekFrom::Start(u64::from(u32_at(&entry[8..])))).ok()?;
        let mut profile = Vec::new();
        r.take(u64::from(size)).read_to_end(&mut profile).ok()?;
        return (profile.len() == size as usize).then_some(profile);
    }
    None
}

/// Flattens RGBA onto white. Used for formats and previews that have no transparency.
pub fn flatten(rgba: &RgbaImage) -> RgbImage {
    let mut out = RgbImage::new(rgba.width(), rgba.height());
    for (d, s) in out.chunks_exact_mut(3).zip(rgba.chunks_exact(4)) {
        let a = u32::from(s[3]);
        for i in 0..3 {
            d[i] = ((u32::from(s[i]) * a + 255 * (255 - a) + 127) / 255) as u8;
        }
    }
    out
}

/// A JPEG of the photo scaled to at most `PREVIEW_LONG_EDGE`, as base64.
pub fn make_preview(photo: &Photo) -> Result<String, AppError> {
    let (w, h) = photo.pixels.dimensions();
    let long = w.max(h);
    let rgb = if long > PREVIEW_LONG_EDGE {
        let k = f64::from(PREVIEW_LONG_EDGE) / f64::from(long);
        let pw = ((f64::from(w) * k).round() as u32).max(1);
        let ph = ((f64::from(h) * k).round() as u32).max(1);
        flatten(&image::imageops::thumbnail(&photo.pixels, pw, ph))
    } else {
        flatten(&photo.pixels)
    };

    let mut jpeg = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg, PREVIEW_QUALITY);
    if let Some(icc) = &photo.icc {
        // So the preview shows the same colours as the original. Not fatal if it can't be kept.
        let _ = encoder.set_icc_profile(icc.clone());
    }
    encoder
        .write_image(rgb.as_raw(), rgb.width(), rgb.height(), ExtendedColorType::Rgb8)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(jpeg))
}

pub fn describe(photo: &Photo, file_name: String) -> Result<OpenedPhoto, AppError> {
    Ok(OpenedPhoto {
        width: photo.pixels.width(),
        height: photo.pixels.height(),
        file_name,
        preview: make_preview(photo)?,
    })
}

pub fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "photo".to_string())
}

/// Decodes a file and makes its preview, ready to be stored in app state.
pub fn open(path: &Path) -> Result<(Arc<Photo>, OpenedPhoto), AppError> {
    let photo = decode_file(path)?;
    let opened = describe(&photo, file_name_of(path))?;
    Ok((Arc::new(photo), opened))
}

pub const SAMPLE_BYTES: &[u8] = include_bytes!("../../design/assets/sample.jpg");
pub const SAMPLE_NAME: &str = "gallery_sample.jpg";

pub fn open_sample() -> Result<(Arc<Photo>, OpenedPhoto), AppError> {
    let photo = decode_bytes(SAMPLE_BYTES)?;
    let opened = describe(&photo, SAMPLE_NAME.to_string())?;
    Ok((Arc::new(photo), opened))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::png::PngEncoder;
    use image::codecs::tiff::TiffEncoder;
    use image::Rgb;

    fn pattern(w: u32, h: u32) -> RgbImage {
        RgbImage::from_fn(w, h, |x, y| {
            Rgb([(x * 255 / w.max(1)) as u8, (y * 255 / h.max(1)) as u8, 90])
        })
    }

    fn encode(format: ImageFormat, img: &RgbImage) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, format).unwrap();
        out.into_inner()
    }

    #[test]
    fn each_format_decodes_with_the_right_dimensions() {
        let img = pattern(64, 48);
        for format in [
            ImageFormat::Jpeg,
            ImageFormat::Png,
            ImageFormat::Tiff,
            ImageFormat::WebP,
        ] {
            let photo = decode_bytes(&encode(format, &img)).unwrap();
            assert_eq!(photo.pixels.dimensions(), (64, 48), "{format:?}");
            assert!(!photo.has_alpha, "{format:?}");
        }
    }

    #[test]
    fn a_format_outside_the_list_is_refused() {
        // The start of a BMP, recognised as an image format but not one the app supports.
        let bytes = b"BM\x46\0\0\0\0\0\0\0\x36\0\0\0\x28\0\0\0";
        assert!(matches!(decode_bytes(bytes), Err(AppError::Unsupported)));
    }

    #[test]
    fn text_is_refused_as_unsupported() {
        assert!(matches!(
            decode_bytes(b"this is not an image, just some text"),
            Err(AppError::Unsupported)
        ));
    }

    #[test]
    fn a_corrupt_file_returns_an_error_and_does_not_panic() {
        for format in [
            ImageFormat::Jpeg,
            ImageFormat::Png,
            ImageFormat::Tiff,
            ImageFormat::WebP,
        ] {
            let mut bytes = encode(format, &pattern(64, 48));
            bytes.truncate(bytes.len() / 3);
            assert!(decode_bytes(&bytes).is_err(), "{format:?}");

            let mut scrambled = encode(format, &pattern(64, 48));
            let head = 12.min(scrambled.len());
            for b in scrambled[head..].iter_mut() {
                *b = b.wrapping_mul(31).wrapping_add(7);
            }
            // Either outcome is fine, so long as it does not panic.
            let _ = decode_bytes(&scrambled);
        }
    }

    /// A minimal EXIF chunk holding only an orientation tag.
    fn exif_with_orientation(value: u16) -> Vec<u8> {
        let mut c = vec![0x49, 0x49, 42, 0, 8, 0, 0, 0, 1, 0];
        c.extend_from_slice(&0x0112u16.to_le_bytes());
        c.extend_from_slice(&3u16.to_le_bytes());
        c.extend_from_slice(&1u32.to_le_bytes());
        c.extend_from_slice(&value.to_le_bytes());
        c.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
        c
    }

    #[test]
    fn exif_orientation_is_applied_and_then_cleared() {
        // A 4 × 2 image whose top-left pixel is marked. Orientation 6 means rotate 90° clockwise.
        let mut img = RgbImage::from_pixel(4, 2, Rgb([0, 0, 255]));
        img.put_pixel(0, 0, Rgb([255, 0, 0]));

        let mut bytes = Vec::new();
        let mut encoder = PngEncoder::new(&mut bytes);
        encoder.set_exif_metadata(exif_with_orientation(6)).unwrap();
        encoder
            .write_image(img.as_raw(), 4, 2, ExtendedColorType::Rgb8)
            .unwrap();

        let photo = decode_bytes(&bytes).unwrap();
        assert_eq!(photo.pixels.dimensions(), (2, 4));
        // The marked corner moves from the top left to the top right.
        assert_eq!(photo.pixels.get_pixel(1, 0).0, [255, 0, 0, 255]);
        assert_eq!(photo.pixels.get_pixel(0, 0).0, [0, 0, 255, 255]);

        let exif = photo.exif.as_deref().unwrap();
        assert_eq!(
            Orientation::from_exif_chunk(exif),
            Some(Orientation::NoTransforms)
        );
    }

    #[test]
    fn the_icc_profile_is_read() {
        let icc = vec![7u8; 200];
        let mut bytes = Vec::new();
        let mut encoder = PngEncoder::new(&mut bytes);
        encoder.set_icc_profile(icc.clone()).unwrap();
        encoder
            .write_image(pattern(8, 8).as_raw(), 8, 8, ExtendedColorType::Rgb8)
            .unwrap();
        assert_eq!(decode_bytes(&bytes).unwrap().icc, Some(icc));
    }

    #[test]
    fn the_icc_profile_is_read_from_a_tiff() {
        let icc = vec![9u8; 301];
        let mut bytes = Cursor::new(Vec::new());
        let mut encoder = TiffEncoder::new(&mut bytes);
        encoder.set_icc_profile(icc.clone()).unwrap();
        encoder
            .write_image(pattern(8, 8).as_raw(), 8, 8, ExtendedColorType::Rgb8)
            .unwrap();
        assert_eq!(decode_bytes(bytes.get_ref()).unwrap().icc, Some(icc));
    }

    #[test]
    fn a_damaged_tiff_directory_means_no_profile_and_no_panic() {
        for bytes in [
            &b"II*\0"[..],
            &b"II*\0\xff\xff\xff\xff"[..],
            &b"MM\0*\0\0\0\x08\xff\xff"[..],
            &b"II*\0\x08\0\0\0\x01\0\x73\x87\x01\0\xff\xff\xff\x7f\x1a\0\0\0\0\0\0\0"[..],
        ] {
            assert_eq!(tiff_icc_profile(&mut Cursor::new(bytes)), None);
        }
    }

    #[test]
    fn an_oversized_image_fails_with_a_clear_error_before_decoding() {
        // A PNG header claiming 30000 × 30000, far over the limit, with no pixel data.
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(b"IHDR");
        ihdr.extend_from_slice(&30000u32.to_be_bytes());
        ihdr.extend_from_slice(&30000u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(&ihdr);
        bytes.extend_from_slice(&crc32(&ihdr).to_be_bytes());
        // The decoder reads up to the first image data before it reports a size.
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(b"IDAT");
        bytes.extend_from_slice(&crc32(b"IDAT").to_be_bytes());
        match decode_bytes(&bytes) {
            Err(AppError::TooLarge { width, height, .. }) => {
                assert_eq!((width, height), (30000, 30000));
            }
            Err(other) => panic!("wrong error: {other}"),
            Ok(_) => panic!("decoded an oversized image"),
        }
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn the_preview_is_limited_and_keeps_the_aspect_ratio() {
        let photo = Photo {
            pixels: RgbaImage::from_pixel(5120, 2560, image::Rgba([10, 20, 30, 255])),
            has_alpha: false,
            icc: None,
            exif: None,
            path: None,
        };
        let opened = describe(&photo, "a.png".into()).unwrap();
        assert_eq!((opened.width, opened.height), (5120, 2560));
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&opened.preview)
            .unwrap();
        let preview = image::load_from_memory_with_format(&bytes, ImageFormat::Jpeg).unwrap();
        assert_eq!((preview.width(), preview.height()), (2560, 1280));
    }

    #[test]
    fn tiff_goes_through_the_preview_as_jpeg() {
        let mut bytes = Cursor::new(Vec::new());
        TiffEncoder::new(&mut bytes)
            .write_image(pattern(32, 16).as_raw(), 32, 16, ExtendedColorType::Rgb8)
            .unwrap();
        let photo = decode_bytes(bytes.get_ref()).unwrap();
        let opened = describe(&photo, "a.tif".into()).unwrap();
        let jpeg = base64::engine::general_purpose::STANDARD
            .decode(&opened.preview)
            .unwrap();
        assert_eq!(&jpeg[..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn the_sample_photo_opens() {
        let (photo, opened) = open_sample().unwrap();
        assert_eq!(opened.file_name, "gallery_sample.jpg");
        assert_eq!(photo.pixels.width(), opened.width);
        assert!(photo.path.is_none());
    }

    #[test]
    fn flatten_composites_on_white() {
        let img = RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 0, 0]));
        assert_eq!(flatten(&img).get_pixel(0, 0).0, [255, 255, 255]);
        let img = RgbaImage::from_pixel(1, 1, image::Rgba([10, 20, 30, 255]));
        assert_eq!(flatten(&img).get_pixel(0, 0).0, [10, 20, 30]);
    }
}
