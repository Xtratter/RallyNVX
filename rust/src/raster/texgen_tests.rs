use super::*;
fn px(d: &[u8], size: usize, x: usize, y: usize) -> [f32; 3] { let i = (y * size + x) * 4; [d[i] as f32, d[i + 1] as f32, d[i + 2] as f32] }
fn diff(a: [f32; 3], b: [f32; 3]) -> f32 { (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs() }
fn mean(d: &[u8], size: usize) -> [f32; 3] { let mut s = [0.0; 3]; for i in 0..size * size { for c in 0..3 { s[c] += d[i * 4 + c] as f32; } } s.map(|v| v / (size * size) as f32) }

#[test] fn maps_have_the_right_size_are_opaque_and_deterministic() {
    for m in Material::ALL { let a = make(m, SIZE); assert_eq!(a.len(), SIZE * SIZE * 4); assert!(a.chunks(4).all(|p| p[3] == 255)); assert_eq!(a, make(m, SIZE), "{m:?}"); }
    assert_ne!(make(Material::Grass, 64), make(Material::Dirt, 64));
}
#[test] fn maps_tile_without_visible_seams() {
    for m in Material::ALL {
        let d = make(m, SIZE);
        let (mut inner_h, mut wrap_h, mut inner_v, mut wrap_v) = (0.0, 0.0, 0.0, 0.0);
        for y in 0..SIZE { for x in 0..SIZE - 1 { inner_h += diff(px(&d, SIZE, x, y), px(&d, SIZE, x + 1, y)); } wrap_h += diff(px(&d, SIZE, SIZE - 1, y), px(&d, SIZE, 0, y)); }
        for x in 0..SIZE { for y in 0..SIZE - 1 { inner_v += diff(px(&d, SIZE, x, y), px(&d, SIZE, x, y + 1)); } wrap_v += diff(px(&d, SIZE, x, SIZE - 1), px(&d, SIZE, x, 0)); }
        let (ih, iv) = (inner_h / (SIZE * (SIZE - 1)) as f32, inner_v / (SIZE * (SIZE - 1)) as f32);
        let (wh, wv) = (wrap_h / SIZE as f32, wrap_v / SIZE as f32);
        assert!(wh <= ih * 1.4 + 2.0 && wv <= iv * 1.4 + 2.0, "{m:?}: seam h {wh} vs {ih}, v {wv} vs {iv}");
    }
}
#[test] fn materials_have_their_expected_colours_and_some_variation() {
    let g = mean(&make(Material::Grass, SIZE), SIZE); assert!(g[1] > g[0] && g[1] > g[2], "grass {g:?}");
    let d = mean(&make(Material::Dirt, SIZE), SIZE); assert!(d[0] > d[1] && d[1] > d[2], "dirt {d:?}");
    let s = mean(&make(Material::Sand, SIZE), SIZE); assert!(s[0] > s[1] && s[1] > s[2] && s[0] > 150.0, "sand {s:?}");
    let r = mean(&make(Material::Rock, SIZE), SIZE); assert!((r[0] - r[2]).abs() < 30.0 && r[0] > 60.0 && r[0] < 170.0, "rock {r:?}");
    let a = mean(&make(Material::Asphalt, SIZE), SIZE); assert!(a[0] < 70.0 && a[1] < 70.0 && a[2] < 70.0, "asphalt {a:?}");
    for m in Material::ALL {
        let d = make(m, SIZE); let mu = mean(&d, SIZE);
        let var: f32 = (0..SIZE * SIZE).map(|i| (d[i * 4 + 1] as f32 - mu[1]).powi(2)).sum::<f32>() / (SIZE * SIZE) as f32;
        assert!(var.sqrt() > 3.0, "{m:?} is too flat: sd {}", var.sqrt());
    }
}
#[test] fn mip_chains_have_the_right_sizes_and_preserve_the_average() {
    let l0 = make(Material::Rock, SIZE); let m = mips(&l0, SIZE);
    assert_eq!(m.len(), 9);
    for (i, lvl) in m.iter().enumerate() { let s = SIZE >> i; assert_eq!(lvl.len(), s * s * 4, "level {i}"); }
    assert_eq!(m[0], l0);
    let (a0, a1, a8) = (mean(&m[0], SIZE), mean(&m[1], SIZE / 2), mean(&m[8], 1));
    for c in 0..3 { assert!((a0[c] - a1[c]).abs() < 1.5 && (a0[c] - a8[c]).abs() < 3.0, "channel {c}: {a0:?} {a1:?} {a8:?}"); }
}
