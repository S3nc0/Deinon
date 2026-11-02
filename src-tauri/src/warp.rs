//! The warp maths on the Rust side: the same formulas as `src/engine/`, in `f64`.
//! Both are tested against `fixtures/homography.json`, so preview and export agree.

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub type Quad = [Point; 4];
pub type Matrix = [f64; 9];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

fn square_to_quad(q: &Quad) -> Matrix {
    let [p0, p1, p2, p3] = *q;
    let dx1 = p1.x - p2.x;
    let dx2 = p3.x - p2.x;
    let sx = p0.x - p1.x + p2.x - p3.x;
    let dy1 = p1.y - p2.y;
    let dy2 = p3.y - p2.y;
    let sy = p0.y - p1.y + p2.y - p3.y;
    let den = dx1 * dy2 - dx2 * dy1;
    let g = (sx * dy2 - dx2 * sy) / den;
    let h = (dx1 * sy - sx * dy1) / den;
    [
        p1.x - p0.x + g * p1.x,
        p3.x - p0.x + h * p3.x,
        p0.x,
        p1.y - p0.y + g * p1.y,
        p3.y - p0.y + h * p3.y,
        p0.y,
        g,
        h,
        1.0,
    ]
}

fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut m = [0.0; 9];
    for r in 0..3 {
        for c in 0..3 {
            m[r * 3 + c] = a[r * 3] * b[c] + a[r * 3 + 1] * b[c + 3] + a[r * 3 + 2] * b[c + 6];
        }
    }
    m
}

pub fn invert(m: &Matrix) -> Matrix {
    let [a, b, c, d, e, f, g, h, i] = *m;
    let aa = e * i - f * h;
    let bb = c * h - b * i;
    let cc = b * f - c * e;
    let dd = f * g - d * i;
    let ee = a * i - c * g;
    let ff = c * d - a * f;
    let gg = d * h - e * g;
    let hh = b * g - a * h;
    let ii = a * e - b * d;
    let det = a * aa + b * dd + c * gg;
    [
        aa / det,
        bb / det,
        cc / det,
        dd / det,
        ee / det,
        ff / det,
        gg / det,
        hh / det,
        ii / det,
    ]
}

/// H = Q(dst) × inverse(Q(src)), normalised so its last element is 1.
pub fn homography(src: &Quad, dst: &Quad) -> Matrix {
    let m = multiply(&square_to_quad(dst), &invert(&square_to_quad(src)));
    let last = m[8];
    m.map(|v| v / last)
}

/// Returns the mapped point and its `w`.
pub fn project(h: &Matrix, p: Point) -> (Point, f64) {
    let w = h[6] * p.x + h[7] * p.y + h[8];
    (
        Point {
            x: (h[0] * p.x + h[1] * p.y + h[2]) / w,
            y: (h[3] * p.x + h[4] * p.y + h[5]) / w,
        },
        w,
    )
}

pub fn corners(w: f64, h: f64) -> Quad {
    [
        Point { x: 0.0, y: 0.0 },
        Point { x: w, y: 0.0 },
        Point { x: w, y: h },
        Point { x: 0.0, y: h },
    ]
}

/// The winding direction of a convex shape: 1 or -1. Returns 0 if it is not convex.
pub fn convex_sign(q: &Quad) -> i32 {
    let mut sign = 0;
    for i in 0..4 {
        let (a, b, c) = (q[i], q[(i + 1) % 4], q[(i + 2) % 4]);
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        // Written this way round so that NaN, from a bad input, counts as not convex.
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if !(cross.abs() >= 1.0) {
            return 0;
        }
        let s = if cross > 0.0 { 1 } else { -1 };
        if sign != 0 && s != sign {
            return 0;
        }
        sign = s;
    }
    sign
}

/// Both shapes are convex, wind the same way, and nothing passes behind the camera.
pub fn is_valid(src: &Quad, dst: &Quad, w: f64, h: f64) -> bool {
    let a = convex_sign(src);
    if a == 0 || a != convex_sign(dst) {
        return false;
    }
    let m = homography(src, dst);
    corners(w, h).iter().all(|c| project(&m, *c).1 > 0.05)
}

/// Clips a shape to the frame (Sutherland-Hodgman).
fn clip_to_frame(poly: &[Point], w: f64, h: f64) -> Vec<Point> {
    fn clip(
        pts: &[Point],
        inside: impl Fn(&Point) -> bool,
        cross: impl Fn(&Point, &Point) -> Point,
    ) -> Vec<Point> {
        let mut out = Vec::with_capacity(pts.len() + 1);
        for (i, a) in pts.iter().enumerate() {
            let b = &pts[(i + 1) % pts.len()];
            if inside(a) {
                out.push(*a);
            }
            if inside(a) != inside(b) {
                out.push(cross(a, b));
            }
        }
        out
    }
    let at_x = |x: f64| {
        move |a: &Point, b: &Point| Point {
            x,
            y: a.y + ((x - a.x) / (b.x - a.x)) * (b.y - a.y),
        }
    };
    let at_y = |y: f64| {
        move |a: &Point, b: &Point| Point {
            x: a.x + ((y - a.y) / (b.y - a.y)) * (b.x - a.x),
            y,
        }
    };
    let p = clip(poly, |a| a.x >= 0.0, at_x(0.0));
    let p = clip(&p, |a| a.x <= w, at_x(w));
    let p = clip(&p, |a| a.y >= 0.0, at_y(0.0));
    clip(&p, |a| a.y <= h, at_y(h))
}

/// Share of the frame that the warped photo covers, from 0 to 1.
pub fn coverage(m: &Matrix, w: f64, h: f64) -> f64 {
    let poly = corners(w, h).map(|c| project(m, c).0);
    let p = clip_to_frame(&poly, w, h);
    signed_area(&p).abs() / (w * h)
}

fn signed_area(p: &[Point]) -> f64 {
    let mut area = 0.0;
    for (i, a) in p.iter().enumerate() {
        let b = &p[(i + 1) % p.len()];
        area += a.x * b.y - b.x * a.y;
    }
    area / 2.0
}

/// The largest rectangle with the frame's aspect ratio that fits inside the warped photo and the
/// frame. See `cropRect` in `src/engine/crop.ts` for the method. The two must agree.
pub fn crop_rect(m: &Matrix, w: f64, h: f64) -> Option<Rect> {
    let poly = clip_to_frame(&corners(w, h).map(|c| project(m, c).0), w, h);
    let n = poly.len();
    if n < 3 {
        return None;
    }
    let area = signed_area(&poly);
    if area.abs() * 2.0 < 1e-9 {
        return None;
    }
    let sign = if area > 0.0 { 1.0 } else { -1.0 };

    // Each constraint is a·x + b·y + d·t + c ≥ 0, normalised so the tolerance is in pixels.
    let mut cons: Vec<[f64; 4]> = Vec::with_capacity(n);
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let len = (q.x - p.x).hypot(q.y - p.y);
        if len < 1e-9 {
            continue;
        }
        let a = -(q.y - p.y) * sign / len;
        let b = (q.x - p.x) * sign / len;
        let c = -(a * p.x + b * p.y);
        cons.push([a, b, (a * w).min(0.0) + (b * h).min(0.0), c]);
    }

    let mut found: Vec<(f64, f64, f64)> = Vec::new();
    let mut t_max = 0.0_f64;
    for i in 0..cons.len() {
        for j in (i + 1)..cons.len() {
            for k in (j + 1)..cons.len() {
                let Some((x, y, t)) = solve3(&cons[i], &cons[j], &cons[k]) else {
                    continue;
                };
                if t <= 0.0 {
                    continue;
                }
                if cons
                    .iter()
                    .all(|c| c[0] * x + c[1] * y + c[2] * t + c[3] >= -1e-7)
                {
                    found.push((x, y, t));
                    t_max = t_max.max(t);
                }
            }
        }
    }

    // The best size can have many positions. They form a line, so take its middle.
    let best: Vec<_> = found.into_iter().filter(|s| s.2 >= t_max - 1e-7).collect();
    if best.is_empty() {
        return None;
    }
    let t = best.iter().fold(f64::INFINITY, |acc, s| acc.min(s.2));
    let mid = |get: fn(&(f64, f64, f64)) -> f64| {
        let lo = best.iter().map(get).fold(f64::INFINITY, f64::min);
        let hi = best.iter().map(get).fold(f64::NEG_INFINITY, f64::max);
        (lo + hi) / 2.0
    };
    Some(Rect {
        x: mid(|s| s.0),
        y: mid(|s| s.1),
        w: t * w,
        h: t * h,
    })
}

/// Where 3 constraints meet, by Cramer's rule.
fn solve3(p: &[f64; 4], q: &[f64; 4], r: &[f64; 4]) -> Option<(f64, f64, f64)> {
    let det3 = |m: [f64; 9]| {
        m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
            + m[2] * (m[3] * m[7] - m[4] * m[6])
    };
    let det = det3([p[0], p[1], p[2], q[0], q[1], q[2], r[0], r[1], r[2]]);
    if det.abs() < 1e-12 {
        return None;
    }
    Some((
        det3([-p[3], p[1], p[2], -q[3], q[1], q[2], -r[3], r[1], r[2]]) / det,
        det3([p[0], -p[3], p[2], q[0], -q[3], q[2], r[0], -r[3], r[2]]) / det,
        det3([p[0], p[1], -p[3], q[0], q[1], -q[3], r[0], r[1], -r[3]]) / det,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Fixtures {
        frame: Frame,
        cases: Vec<Case>,
    }
    #[derive(Deserialize)]
    struct Frame {
        width: f64,
        height: f64,
    }
    #[derive(Deserialize)]
    struct Case {
        name: String,
        src: Quad,
        dst: Quad,
        matrix: Matrix,
        samples: Vec<Sample>,
        coverage: f64,
        crop: CropBox,
    }
    #[derive(Deserialize)]
    struct Sample {
        from: Point,
        to: Point,
    }
    #[derive(Deserialize)]
    struct CropBox {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    }

    fn fixtures() -> Fixtures {
        serde_json::from_str(include_str!("../../fixtures/homography.json")).unwrap()
    }

    fn close(a: f64, b: f64, what: &str, name: &str) {
        assert!((a - b).abs() < 1e-6, "{name}: {what}: {a} vs {b}");
    }

    #[test]
    fn every_fixture_matrix_and_sample_matches() {
        let f = fixtures();
        assert!(f.cases.len() >= 6);
        for c in &f.cases {
            let m = homography(&c.src, &c.dst);
            for i in 0..9 {
                close(m[i], c.matrix[i], "matrix", &c.name);
            }
            for s in &c.samples {
                let (q, _) = project(&m, s.from);
                close(q.x, s.to.x, "sample x", &c.name);
                close(q.y, s.to.y, "sample y", &c.name);
            }
        }
    }

    #[test]
    fn every_fixture_is_valid_with_the_same_coverage_and_crop() {
        let f = fixtures();
        let (w, h) = (f.frame.width, f.frame.height);
        for c in &f.cases {
            assert!(is_valid(&c.src, &c.dst, w, h), "{}", c.name);
            let m = homography(&c.src, &c.dst);
            close(coverage(&m, w, h), c.coverage, "coverage", &c.name);
            let r = crop_rect(&m, w, h).unwrap();
            close(r.x, c.crop.x, "crop x", &c.name);
            close(r.y, c.crop.y, "crop y", &c.name);
            close(r.w, c.crop.w, "crop w", &c.name);
            close(r.h, c.crop.h, "crop h", &c.name);
        }
    }

    #[test]
    fn crossed_flipped_and_nan_shapes_are_invalid() {
        let q = corners(100.0, 100.0);
        assert!(is_valid(&q, &q, 100.0, 100.0));
        assert!(!is_valid(&q, &[q[0], q[2], q[1], q[3]], 100.0, 100.0));
        assert!(!is_valid(&q, &[q[1], q[0], q[3], q[2]], 100.0, 100.0));
        let nan = Point {
            x: f64::NAN,
            y: 0.0,
        };
        assert!(!is_valid(&q, &[nan, q[1], q[2], q[3]], 100.0, 100.0));
    }

    #[test]
    fn crop_is_none_when_the_photo_has_left_the_frame() {
        let q = corners(100.0, 100.0);
        let away = q.map(|p| Point {
            x: p.x + 500.0,
            y: p.y,
        });
        assert_eq!(crop_rect(&homography(&q, &away), 100.0, 100.0), None);
    }
}
