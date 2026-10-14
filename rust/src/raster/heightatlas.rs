//! Height atlas for the hybrid renderer's terrain ray shadows: 8x8 tiles of 64x64 texels (2 m per texel) in a ring
//! addressed by `tile mod 8`, stored as R16Float on the GPU. Pure CPU bookkeeping, tested on the host.
use std::collections::HashSet;

use crate::game::terrain::TILE_CELLS;

pub const ATLAS_TILES: i32 = 8;
pub const TEXELS: usize = TILE_CELLS;
/// Height written where no tile is loaded: far below any terrain, so nothing shadows from there.
pub const EMPTY: f32 = -10_000.0;
const SIDE: usize = ATLAS_TILES as usize * TEXELS;

/// f32 to IEEE half, round to nearest even; overflow gives infinity, tiny values flush to subnormals or zero.
pub fn f32_to_f16(v: f32) -> u16 {
    let b = v.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xFF) as i32;
    let man = b & 0x007F_FFFF;
    if exp == 0xFF { return sign | 0x7C00 | if man != 0 { 0x0200 } else { 0 }; }
    let e = exp - 127 + 15;
    if e >= 0x1F { return sign | 0x7C00; }
    if e <= 0 {
        if e < -10 { return sign; }
        let m = (man | 0x0080_0000) >> (1 - e);
        let m = m + 0x0FFF + ((m >> 13) & 1);
        return sign | (m >> 13) as u16;
    }
    let mut h = ((e as u32) << 10) | (man >> 13);
    let rest = man & 0x1FFF;
    if rest > 0x1000 || (rest == 0x1000 && (h & 1) == 1) { h += 1; }
    sign | h as u16
}

/// The atlas slot of tile (tx, tz): `tile mod 8` for negative tiles too.
pub fn slot(tx: i32, tz: i32) -> (u32, u32) { (tx.rem_euclid(ATLAS_TILES) as u32, tz.rem_euclid(ATLAS_TILES) as u32) }

/// 64x64 heights (texel (i, j) at world offset (2i, 2j) inside the tile) from a tile mesh of `lod`: the first
/// `(n + 1)^2` vertices are the grid `j * (n + 1) + i` with `n = 64 >> lod`; coarser grids are upsampled bilinearly.
pub fn heights_from_mesh(verts: &[[f32; 6]], lod: u32) -> Vec<f32> {
    let n = TEXELS >> lod;
    let w = n + 1;
    let at = |i: usize, j: usize| verts[j.min(n) * w + i.min(n)][1];
    let k = n as f32 / TEXELS as f32;
    let mut out = Vec::with_capacity(TEXELS * TEXELS);
    for j in 0..TEXELS { for i in 0..TEXELS {
        let (gx, gz) = (i as f32 * k, j as f32 * k);
        let (x0, z0) = (gx.floor() as usize, gz.floor() as usize);
        let (fx, fz) = (gx - x0 as f32, gz - z0 as f32);
        let top = at(x0, z0) * (1.0 - fx) + at(x0 + 1, z0) * fx;
        let bot = at(x0, z0 + 1) * (1.0 - fx) + at(x0 + 1, z0 + 1) * fx;
        out.push(top * (1.0 - fz) + bot * fz);
    } }
    out
}

/// A 64x64 R16Float block to upload into `slot` (texel offset `slot * 64`).
pub struct SlotWrite { pub slot: (u32, u32), pub data: Vec<u16> }

pub struct HeightAtlas { h: Vec<f32>, owner: [[Option<(i32, i32)>; ATLAS_TILES as usize]; ATLAS_TILES as usize] }

impl Default for HeightAtlas { fn default() -> Self { HeightAtlas::new() } }

impl HeightAtlas {
    pub fn new() -> HeightAtlas { HeightAtlas { h: vec![EMPTY; SIDE * SIDE], owner: [[None; ATLAS_TILES as usize]; ATLAS_TILES as usize] } }

    fn write_slot(&mut self, s: (u32, u32), heights: Option<&[f32]>) -> SlotWrite {
        let (ox, oz) = (s.0 as usize * TEXELS, s.1 as usize * TEXELS);
        let mut data = Vec::with_capacity(TEXELS * TEXELS);
        for j in 0..TEXELS { for i in 0..TEXELS {
            let v = heights.map_or(EMPTY, |h| h[j * TEXELS + i]);
            self.h[(oz + j) * SIDE + ox + i] = v;
            data.push(f32_to_f16(v));
        } }
        SlotWrite { slot: s, data }
    }

    /// Stores tile (tx, tz) in its ring slot (replacing whatever was there).
    pub fn put(&mut self, tx: i32, tz: i32, heights: &[f32]) -> SlotWrite {
        assert_eq!(heights.len(), TEXELS * TEXELS);
        let s = slot(tx, tz);
        self.owner[s.1 as usize][s.0 as usize] = Some((tx, tz));
        self.write_slot(s, Some(heights))
    }

    /// Clears the slots whose tile is not in `present` (the tile meshes were dropped).
    pub fn evict_missing(&mut self, present: &HashSet<(i32, i32)>) -> Vec<SlotWrite> {
        let mut out = Vec::new();
        for sz in 0..ATLAS_TILES as usize { for sx in 0..ATLAS_TILES as usize {
            if let Some(t) = self.owner[sz][sx] {
                if !present.contains(&t) { self.owner[sz][sx] = None; out.push(self.write_slot((sx as u32, sz as u32), None)); }
            }
        } }
        out
    }

    /// Empties every owned slot (a new level).
    pub fn clear(&mut self) -> Vec<SlotWrite> {
        let mut out = Vec::new();
        for sz in 0..ATLAS_TILES as usize { for sx in 0..ATLAS_TILES as usize {
            if self.owner[sz][sx].take().is_some() { out.push(self.write_slot((sx as u32, sz as u32), None)); }
        } }
        out
    }

    /// The shader's lookup on the CPU: ring addressed bilinear sample (texel `i` sits at world `2 i`, so the texture
    /// coordinate is `(x + 1) / 1024`), `EMPTY`-ish where nothing is stored.
    pub fn lookup(&self, x: f32, z: f32) -> f32 {
        let (cx, cz) = (x / 2.0, z / 2.0);
        let (x0, z0) = (cx.floor(), cz.floor());
        let (fx, fz) = (cx - x0, cz - z0);
        let ix = |i: f32| (i as i64).rem_euclid(SIDE as i64) as usize;
        let g = |i: f32, j: f32| self.h[ix(j) * SIDE + ix(i)];
        let top = g(x0, z0) * (1.0 - fx) + g(x0 + 1.0, z0) * fx;
        let bot = g(x0, z0 + 1.0) * (1.0 - fx) + g(x0 + 1.0, z0 + 1.0) * fx;
        top * (1.0 - fz) + bot * fz
    }
}

#[cfg(test)]
#[path = "heightatlas_tests.rs"]
mod tests;
