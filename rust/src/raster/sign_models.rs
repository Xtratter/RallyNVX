//! Generated signs (no files): a post and a board facing +z whose centre is 2.15 m above the ground, made of coloured
//! layers (border, face, symbol). Shapes follow the public conventions: red warning triangles (UNECE), yellow chevron and
//! route-arrow boards, a red no-entry disc, striped distance boards, a checkered finish board, a stop octagon. No text.
use super::vehicle_models::{add_cylinder, Axis, Mesh3, MAT_PAINT, MAT_PLASTIC};
use crate::game::math::v3;
use crate::game::signs::SignKind;

const CY: f32 = 2.15;
const RED: [f32; 3] = [0.78, 0.07, 0.07];
const WHITE: [f32; 3] = [0.93, 0.93, 0.90];
const BLACK: [f32; 3] = [0.05, 0.05, 0.06];
const YELLOW: [f32; 3] = [1.0, 0.80, 0.05];
const GREEN: [f32; 3] = [0.06, 0.55, 0.25];
const POST: [f32; 3] = [0.55, 0.57, 0.60];
// z layers: the back plate, the border, the face, the symbol
const Z_BACK: (f32, f32) = (-0.03, 0.0);
const Z_BORDER: (f32, f32) = (0.0, 0.014);
const Z_FACE: (f32, f32) = (0.014, 0.028);
const Z_SYM: (f32, f32) = (0.028, 0.042);

type P = (f32, f32);

fn area(p: &[P]) -> f32 { (0..p.len()).map(|i| { let (a, b) = (p[i], p[(i + 1) % p.len()]); a.0 * b.1 - b.0 * a.1 }).sum::<f32>() * 0.5 }

/// A convex polygon extruded from `z.0` to `z.1` (the points are put in counter-clockwise order, in board coordinates around the centre).
fn prism(m: &mut Mesh3, pts: &[P], z: (f32, f32), col: [f32; 3], mat: u8) {
    let mut p: Vec<P> = pts.to_vec();
    if area(&p) < 0.0 { p.reverse(); }
    let w = |q: P, zz: f32| v3(q.0, CY + q.1, zz);
    let front: Vec<u32> = p.iter().map(|q| m.vert(w(*q, z.1), v3(0., 0., 1.), col, mat)).collect();
    for i in 1..p.len() - 1 { m.idx.extend_from_slice(&[front[0], front[i], front[i + 1]]); }
    let back: Vec<u32> = p.iter().map(|q| m.vert(w(*q, z.0), v3(0., 0., -1.), col, mat)).collect();
    for i in 1..p.len() - 1 { m.idx.extend_from_slice(&[back[0], back[i + 1], back[i]]); }
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = dx.hypot(dy).max(1e-6);
        let n = v3(dy / l, -dx / l, 0.0);
        let (a0, b0, b1, a1) = (m.vert(w(a, z.0), n, col, mat), m.vert(w(b, z.0), n, col, mat), m.vert(w(b, z.1), n, col, mat), m.vert(w(a, z.1), n, col, mat));
        m.idx.extend_from_slice(&[a0, b0, b1, a0, b1, a1]);
    }
}
fn bar(m: &mut Mesh3, a: P, b: P, width: f32, z: (f32, f32), col: [f32; 3]) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l = dx.hypot(dy).max(1e-6);
    let p = (-dy / l * width * 0.5, dx / l * width * 0.5);
    prism(m, &[(a.0 - p.0, a.1 - p.1), (b.0 - p.0, b.1 - p.1), (b.0 + p.0, b.1 + p.1), (a.0 + p.0, a.1 + p.1)], z, col, MAT_PAINT);
}
fn rect(m: &mut Mesh3, c: P, hw: f32, hh: f32, z: (f32, f32), col: [f32; 3]) { prism(m, &[(c.0 - hw, c.1 - hh), (c.0 + hw, c.1 - hh), (c.0 + hw, c.1 + hh), (c.0 - hw, c.1 + hh)], z, col, MAT_PAINT); }
fn ngon(m: &mut Mesh3, c: P, r: f32, n: usize, rot: f32, z: (f32, f32), col: [f32; 3]) {
    let pts: Vec<P> = (0..n).map(|k| { let a = rot + std::f32::consts::TAU * k as f32 / n as f32; (c.0 + r * a.cos(), c.1 + r * a.sin()) }).collect();
    prism(m, &pts, z, col, MAT_PAINT);
}
/// A polyline of thick bars ending in an arrowhead (`head` pointing along the last segment).
fn arrow_path(m: &mut Mesh3, pts: &[P], width: f32, head: f32, z: (f32, f32), col: [f32; 3]) {
    for w in pts.windows(2) { bar(m, w[0], w[1], width, z, col); }
    // round the joints a little with discs
    for p in &pts[1..pts.len() - 1] { ngon(m, *p, width * 0.5, 8, 0.0, z, col); }
    let (a, b) = (pts[pts.len() - 2], pts[pts.len() - 1]);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l = dx.hypot(dy).max(1e-6);
    let (ux, uy) = (dx / l, dy / l);
    prism(m, &[(b.0 + ux * head, b.1 + uy * head), (b.0 - uy * head * 0.7 - ux * head * 0.2, b.1 + ux * head * 0.7 - uy * head * 0.2), (b.0 + uy * head * 0.7 - ux * head * 0.2, b.1 - ux * head * 0.7 - uy * head * 0.2)], z, col, MAT_PAINT);
}
fn post(m: &mut Mesh3) { add_cylinder(m, v3(0.0, 1.0, 0.0), Axis::Y, 0.035, 2.0, 8, POST, MAT_PLASTIC); }

/// A red warning triangle with a white face; `symbol` draws in the face (centre at (0, 0)).
fn triangle(symbol: impl Fn(&mut Mesh3)) -> Mesh3 {
    let mut m = Mesh3::default();
    post(&mut m);
    let tri = |k: f32| [(0.0, 0.52 * k), (-0.45 * k, -0.26 * k), (0.45 * k, -0.26 * k)];
    prism(&mut m, &tri(1.08), Z_BACK, POST, MAT_PAINT);
    prism(&mut m, &tri(1.0), Z_BORDER, RED, MAT_PAINT);
    prism(&mut m, &tri(0.66), Z_FACE, WHITE, MAT_PAINT);
    symbol(&mut m);
    m
}
fn curve(left: bool) -> impl Fn(&mut Mesh3) {
    let s = if left { -1.0 } else { 1.0 };
    move |m: &mut Mesh3| arrow_path(m, &[(0.0, -0.20), (0.0, -0.03), (s * 0.11, 0.11)], 0.05, 0.11, Z_SYM, BLACK)
}
fn scurve(left_first: bool) -> impl Fn(&mut Mesh3) {
    let s = if left_first { -1.0 } else { 1.0 };
    move |m: &mut Mesh3| arrow_path(m, &[(0.0, -0.20), (0.0, -0.12), (s * 0.08, -0.02), (-s * 0.0, 0.06), (-s * 0.06, 0.16)], 0.045, 0.1, Z_SYM, BLACK)
}
fn hairpin(left: bool) -> impl Fn(&mut Mesh3) {
    let s = if left { -1.0 } else { 1.0 };
    move |m: &mut Mesh3| arrow_path(m, &[(-s * 0.08, -0.2), (-s * 0.08, 0.02), (-s * 0.04, 0.12), (s * 0.04, 0.12), (s * 0.08, 0.02), (s * 0.08, -0.07)], 0.045, 0.1, Z_SYM, BLACK)
}
fn bump(m: &mut Mesh3) {
    let arc: Vec<P> = (0..=8).map(|k| { let a = std::f32::consts::PI * (1.0 - k as f32 / 8.0); (0.16 * a.cos(), -0.07 + 0.16 * a.sin()) }).collect();
    for w in arc.windows(2) { bar(m, w[0], w[1], 0.05, Z_SYM, BLACK); }
    bar(m, (-0.24, -0.11), (0.24, -0.11), 0.045, Z_SYM, BLACK);
}
fn chevrons(left: bool) -> Mesh3 {
    let mut m = Mesh3::default();
    post(&mut m);
    rect(&mut m, (0.0, 0.0), 0.62, 0.40, Z_BACK, POST);
    rect(&mut m, (0.0, 0.0), 0.60, 0.38, Z_BORDER, BLACK);
    rect(&mut m, (0.0, 0.0), 0.56, 0.34, Z_FACE, YELLOW);
    let s = if left { -1.0 } else { 1.0 };
    for dx in [-0.22f32, 0.16] {
        // a chevron pointing the way the road turns
        bar(&mut m, (s * (dx + 0.14), 0.24), (s * (dx - 0.06), 0.0), 0.10, Z_SYM, BLACK);
        bar(&mut m, (s * (dx - 0.06), 0.0), (s * (dx + 0.14), -0.24), 0.10, Z_SYM, BLACK);
    }
    m
}
fn route_arrow() -> Mesh3 {
    let mut m = Mesh3::default();
    post(&mut m);
    rect(&mut m, (0.0, 0.0), 0.47, 0.47, Z_BACK, POST);
    rect(&mut m, (0.0, 0.0), 0.45, 0.45, Z_BORDER, BLACK);
    rect(&mut m, (0.0, 0.0), 0.41, 0.41, Z_FACE, YELLOW);
    arrow_path(&mut m, &[(0.0, -0.28), (0.0, 0.08)], 0.12, 0.2, Z_SYM, BLACK);
    m
}
fn no_entry() -> Mesh3 {
    let mut m = Mesh3::default();
    post(&mut m);
    ngon(&mut m, (0.0, 0.0), 0.39, 24, 0.0, Z_BACK, POST);
    ngon(&mut m, (0.0, 0.0), 0.37, 24, 0.0, Z_BORDER, RED);
    bar(&mut m, (-0.24, 0.0), (0.24, 0.0), 0.13, Z_FACE, WHITE);
    m
}
fn distance(n: usize) -> Mesh3 {
    let mut m = Mesh3::default();
    post(&mut m);
    rect(&mut m, (0.0, 0.0), 0.30, 0.55, Z_BACK, POST);
    rect(&mut m, (0.0, 0.0), 0.28, 0.53, Z_BORDER, BLACK);
    rect(&mut m, (0.0, 0.0), 0.25, 0.50, Z_FACE, YELLOW);
    for k in 0..n { let y = 0.30 - 0.30 * k as f32; bar(&mut m, (-0.17, y - 0.11), (0.17, y + 0.11), 0.09, Z_SYM, RED); }
    m
}
fn finish_board() -> Mesh3 {
    let mut m = Mesh3::default();
    post(&mut m);
    rect(&mut m, (0.0, 0.0), 0.50, 0.40, Z_BACK, POST);
    rect(&mut m, (0.0, 0.0), 0.48, 0.38, Z_BORDER, RED);
    for r in 0..3 { for c in 0..5 {
        let col = if (r + c) % 2 == 0 { BLACK } else { WHITE };
        rect(&mut m, (-0.32 + 0.16 * c as f32, 0.16 - 0.16 * r as f32), 0.08, 0.08, Z_FACE, col);
    } }
    m
}
fn stop() -> Mesh3 {
    let mut m = Mesh3::default();
    post(&mut m);
    let rot = std::f32::consts::PI / 8.0;
    ngon(&mut m, (0.0, 0.0), 0.43, 8, rot, Z_BACK, POST);
    ngon(&mut m, (0.0, 0.0), 0.41, 8, rot, Z_BORDER, WHITE);
    ngon(&mut m, (0.0, 0.0), 0.37, 8, rot, Z_FACE, RED);
    bar(&mut m, (-0.22, 0.0), (0.22, 0.0), 0.10, Z_SYM, WHITE);
    m
}
fn start() -> Mesh3 {
    let mut m = Mesh3::default();
    post(&mut m);
    rect(&mut m, (0.0, 0.0), 0.50, 0.32, Z_BACK, POST);
    rect(&mut m, (0.0, 0.0), 0.48, 0.30, Z_BORDER, WHITE);
    rect(&mut m, (0.0, 0.0), 0.44, 0.26, Z_FACE, GREEN);
    arrow_path(&mut m, &[(0.0, -0.17), (0.0, 0.03)], 0.10, 0.17, Z_SYM, WHITE);
    m
}

/// One mesh per `SignKind`, in `SignKind::ALL` order.
pub fn build_all() -> Vec<Mesh3> {
    SignKind::ALL.iter().map(|k| match k {
        SignKind::CurveL => triangle(curve(true)), SignKind::CurveR => triangle(curve(false)),
        SignKind::SCurveL => triangle(scurve(true)), SignKind::SCurveR => triangle(scurve(false)),
        SignKind::HairpinL => triangle(hairpin(true)), SignKind::HairpinR => triangle(hairpin(false)),
        SignKind::Bump => triangle(bump),
        SignKind::ChevL => chevrons(true), SignKind::ChevR => chevrons(false),
        SignKind::RouteArrow => route_arrow(), SignKind::NoEntry => no_entry(),
        SignKind::Dist1 => distance(1), SignKind::Dist2 => distance(2), SignKind::Dist3 => distance(3),
        SignKind::FinishBoard => finish_board(), SignKind::Stop => stop(), SignKind::Start => start(),
    }).collect()
}

#[cfg(test)]
#[path = "sign_models_tests.rs"]
mod tests;
