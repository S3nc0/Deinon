//! The whole path through the Rust code: open a generated image, shift it, export it, read it back.

use std::sync::atomic::AtomicBool;

use fpv_shifter_lib::export::{export_to_path, EmptyAreas, ExportOptions, Format};
use fpv_shifter_lib::photo;
use fpv_shifter_lib::warp::{homography, project, Point, Quad};
use image::{ImageReader, Rgb, RgbImage};

fn quad(points: [(f64, f64); 4]) -> Quad {
    points.map(|(x, y)| Point { x, y })
}

#[test]
fn a_generated_image_opens_and_exports_as_predicted() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.png");

    // A white ground with one red block, stored at full size on disk.
    let (w, h) = (800u32, 600u32);
    let mut img = RgbImage::from_pixel(w, h, Rgb([255, 255, 255]));
    let (bx, by) = (500u32, 200u32);
    for y in by - 8..by + 8 {
        for x in bx - 8..bx + 8 {
            img.put_pixel(x, y, Rgb([220, 0, 0]));
        }
    }
    img.save(&input).unwrap();

    let (opened_photo, opened) = photo::open(&input).unwrap();
    assert_eq!((opened.width, opened.height), (w, h));
    assert_eq!(opened.file_name, "input.png");
    assert!(!opened.preview.is_empty());

    let src = quad([(160.0, 120.0), (640.0, 120.0), (640.0, 480.0), (160.0, 480.0)]);
    let dst = quad([(120.0, 150.0), (700.0, 90.0), (660.0, 500.0), (150.0, 460.0)]);
    let cancel = AtomicBool::new(false);
    let options = ExportOptions {
        format: Format::Png,
        long_edge: None,
        quality: 90,
        empty: EmptyAreas::Clear,
    };

    let output = dir.path().join("output.png");
    let result = export_to_path(&opened_photo, &src, &dst, &options, &output, &cancel, &|_| {}).unwrap();
    assert_eq!(result.file_name, "output.png");
    assert_eq!((result.width, result.height), (w, h));

    let decoded = ImageReader::open(&output).unwrap().decode().unwrap().to_rgba8();
    assert_eq!(decoded.dimensions(), (w, h));

    // The red block's centre lands where the homography says it should.
    let m = homography(&src, &dst);
    let (q, _) = project(&m, Point { x: f64::from(bx), y: f64::from(by) });
    let px = decoded.get_pixel(q.x as u32, q.y as u32).0;
    assert!(px[0] > 180 && px[1] < 60 && px[3] == 255, "{px:?} at {q:?}");

    // The input file is untouched.
    let again = ImageReader::open(&input).unwrap().decode().unwrap().to_rgb8();
    assert_eq!(again, img);
}
