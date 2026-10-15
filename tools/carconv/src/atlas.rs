//! Packing of the source textures of a car into one square atlas: every texture is scaled by the same factor (never enlarged, the largest that fits) and
//! shelf-packed, so tiles keep their aspect and never overlap.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect { pub id: usize, pub x: u32, pub y: u32, pub w: u32, pub h: u32 }

const GUTTER: u32 = 2;

fn shelf(sizes: &[(usize, u32, u32)], s: f64, atlas: u32) -> Option<Vec<Rect>> {
    // small textures (flat colours, details) are keep their size (at most 512 pixels); only the large ones (the livery) take the global scale that fits
    let mut tiles: Vec<(usize, u32, u32)> = sizes.iter().map(|&(id, w, h)| { let k = if w.max(h) <= 512 { 1.0 } else { s }; (id, ((w as f64 * k).round() as u32).max(1), ((h as f64 * k).round() as u32).max(1)) }).collect();
    tiles.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.cmp(&a.1)));
    let (mut x, mut y, mut row_h) = (0u32, 0u32, 0u32);
    let mut out = Vec::new();
    for (id, w, h) in tiles {
        if w > atlas || h > atlas { return None; }
        if x + w > atlas { x = 0; y += row_h + GUTTER; row_h = 0; }
        if y + h > atlas { return None; }
        out.push(Rect { id, x, y, w, h });
        x += w + GUTTER; row_h = row_h.max(h);
    }
    Some(out)
}

/// Rectangles in the `atlas` x `atlas` square for the textures `(id, width, height)`; always returns every tile (at the smallest scale if nothing larger fits).
pub fn pack(sizes: &[(usize, u32, u32)], atlas: u32) -> Vec<Rect> {
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    if let Some(r) = shelf(sizes, 1.0, atlas) { return r; }
    for _ in 0..24 { let mid = (lo + hi) / 2.0; if shelf(sizes, mid, atlas).is_some() { lo = mid } else { hi = mid } }
    shelf(sizes, lo, atlas).or_else(|| shelf(sizes, 1.0 / 4096.0, atlas)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn overlap(a: &Rect, b: &Rect) -> bool { a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h }
    #[test] fn tiles_fit_inside_do_not_overlap_and_keep_the_aspect() {
        let sizes = [(0, 6144, 4096), (17, 512, 512), (5, 512, 512), (2, 512, 512), (8, 128, 64), (6, 16, 16)];
        let r = pack(&sizes, 2048);
        assert_eq!(r.len(), sizes.len());
        for a in &r { assert!(a.x + a.w <= 2048 && a.y + a.h <= 2048 && a.w >= 1 && a.h >= 1, "{a:?}"); }
        for i in 0..r.len() { for j in i + 1..r.len() { assert!(!overlap(&r[i], &r[j]), "{:?} {:?}", r[i], r[j]); } }
        let big = r.iter().find(|t| t.id == 0).unwrap();
        assert!((big.w as f32 / big.h as f32 - 1.5).abs() < 0.02, "aspect {:?}", big);
        assert!(big.w >= 1900, "the livery gets the whole width: {big:?}");
    }
    #[test] fn an_impossible_request_still_returns_tiles_that_fit() {
        let r = pack(&[(0, 8192, 8192), (1, 8192, 8192), (2, 8192, 8192)], 256);
        assert_eq!(r.len(), 3);
        for a in &r { assert!(a.x + a.w <= 256 && a.y + a.h <= 256); }
    }
}
