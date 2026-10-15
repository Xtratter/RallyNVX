//! The livery atlas of the rally car: a JPEG decoded once at start-up, with its mip chain built on the CPU.

/// The atlas made by `tools/carconv` (see docs/vehicles.md).
pub const LIVERY: &[u8] = include_bytes!("../../assets/rally_car_livery.jpg");

const MAX_SIDE: u32 = 8192;

/// RGBA8 pixels of a JPEG: `(width, height, pixels)`; `None` for anything that is not a sane JPEG.
pub fn decode(jpeg: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    use zune_jpeg::zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};
    if jpeg.len() < 4 { return None; }
    let opts = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut dec = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(jpeg), opts);
    let rgb = dec.decode().ok()?;
    let (w, h) = dec.dimensions()?;
    let (w, h) = (w as u32, h as u32);
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE || rgb.len() != (w * h * 3) as usize { return None; }
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for p in rgb.chunks_exact(3) { out.extend_from_slice(&[p[0], p[1], p[2], 255]); }
    Some((w, h, out))
}

/// The mip chain, level 0 first, down to 1 x 1: every level is the average of 2 x 2 texels of the previous one.
pub fn mips(rgba: &[u8], w: u32, h: u32) -> Vec<(u32, u32, Vec<u8>)> {
    let mut levels = vec![(w, h, rgba.to_vec())];
    while { let l = levels.last().unwrap(); l.0 > 1 || l.1 > 1 } {
        let (pw, ph, prev) = levels.last().unwrap();
        let (nw, nh) = ((*pw / 2).max(1), (*ph / 2).max(1));
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh { for x in 0..nw {
            let at = |sx: u32, sy: u32| (((sy.min(*ph - 1)) * *pw + sx.min(*pw - 1)) * 4) as usize;
            let (a, b, c, d) = (at(2 * x, 2 * y), at(2 * x + 1, 2 * y), at(2 * x, 2 * y + 1), at(2 * x + 1, 2 * y + 1));
            for k in 0..4 { next[((y * nw + x) * 4) as usize + k] = ((prev[a + k] as u32 + prev[b + k] as u32 + prev[c + k] as u32 + prev[d + k] as u32 + 2) / 4) as u8; }
        } }
        levels.push((nw, nh, next));
    }
    levels
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn the_embedded_atlas_decodes_to_2048_by_2560_rgba() {
        let (w, h, px) = decode(LIVERY).expect("livery decodes");
        assert_eq!((w, h), (2048, 2560)); assert_eq!(px.len(), 2048 * 2560 * 4); assert!(px.chunks(4).step_by(997).all(|p| p[3] == 255));
    }
    #[test] fn garbage_is_none_not_a_panic() { assert!(decode(b"").is_none() && decode(b"not a jpeg").is_none() && decode(&LIVERY[..100]).is_none()); }
    #[test] fn the_mip_chain_halves_down_to_one_pixel_and_averages() {
        let px = vec![255u8; 4 * 4 * 4];
        let m = mips(&px, 4, 4);
        assert_eq!(m.iter().map(|l| (l.0, l.1)).collect::<Vec<_>>(), vec![(4, 4), (2, 2), (1, 1)]);
        let mut chk = vec![0u8; 4 * 4 * 4]; for p in chk.chunks_mut(4) { p.copy_from_slice(&[200, 100, 0, 255]); } chk[0] = 0;
        let v = mips(&chk, 4, 4)[1].2[0];
        assert!(v < 200 && v > 100, "the average is between: {v}");
        assert_eq!(mips(&vec![255u8; 2048 * 2048 * 4], 2048, 2048).len(), 12);
        assert_eq!(mips(&vec![255u8; 8 * 2 * 4], 8, 2).iter().map(|l| (l.0, l.1)).collect::<Vec<_>>(), vec![(8, 2), (4, 1), (2, 1), (1, 1)]);
    }
}
