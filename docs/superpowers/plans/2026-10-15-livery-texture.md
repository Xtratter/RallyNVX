# Livery texture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Peugeot's livery is drawn from a real texture atlas (UV-mapped) instead of blurred vertex colours.

**Architecture:** `RCA2` model files carry a UV per vertex; `tools/carconv` packs the body's base-colour textures into a 2048x2048 atlas (JPEG) and remaps UVs; the game decodes the JPEG (`zune-jpeg`), builds mips, binds it at binding 8/9 of the main bind group and `fs_model` multiplies the albedo by it where the vertex has a UV.

**Tech Stack:** Rust (`~/bin/t [filter]` runs `cargo test --lib` in `rust/`; the converter has its own crate, `cd tools/carconv && cargo test`), WGSL (`shaders_validate`), Gradle for the final build. No device use without the user's yes (hard rule).

**Spec:** `docs/superpowers/specs/2026-10-15-livery-texture-design.md`

## Global Constraints
- Vertex struct stays `#[repr(C)]` Pod: `Vert { pos: [f32;3], nrm: [f32;3], col: [u8;4], uv: [u16;2] }` = 32 bytes; the "no texture" sentinel is `uv = [0xFFFF, 0xFFFF]` (`NO_UV`).
- The old `RCAR` format must still parse (UV = `NO_UV`); new files start with `RCA2`.
- 30 fps target; the model pipeline gets one extra texture fetch; the APK grows by about 2 MB.
- Source model: `/sdcard/claude/peugeot-206-wrc-wwwvecarzcom.zip` (unzip into an empty dir under `$TMPDIR`, run the converter from another dir); the livery is the image of material `CRAPAINT_EXT` (image 0, 6144x4096); small parts use 512-pixel images.
- Lamp points, covers, bounds and physics must not change (existing tests pin them).
- Docs and changelog EN + RU; commit trailer `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.

## Review Focus
- A v1 `RCAR` file or a generated model still renders (sentinel UV, no texture fetch effect).
- A broken or missing `rally_car_livery.jpg`: the car falls back to vertex colours, no panic.
- UV at the atlas edge or exactly 1.0 must not be mistaken for the sentinel.
- A part that uses a texture too small or too repeated for the atlas tile: no crash, colour stays plausible.
- Trees and props (all `NO_UV`) look unchanged.

---

### Task 1: Vertex UV and the `RCA2` format (backward compatible)

**Files:** Modify `rust/src/raster/vehicle_models.rs` (`Vert`, `NO_UV`, 2 literals), `rust/src/raster/nature_models.rs` (1 literal), `rust/src/raster/rcar.rs` (`mesh`, `parse`, 2 literals, tests), `rust/src/raster/gpu.rs` (`MODEL_ATTRS`), `rust/src/shaders/raster.wgsl` (`vs_model`, `MOut`).

**Interfaces:** Produces `pub const NO_UV: [u16; 2] = [0xFFFF, 0xFFFF]`; `Vert.uv`; `rcar::parse` accepts `RCAR` (28-byte vertices) and `RCA2` (32-byte vertices with `u16x2` UV after the colour); `MOut.uv: vec2<f32>` (location 6) in WGSL, `1.0, 1.0` for the sentinel.

- [ ] **Step 1: failing tests** (append to `rcar.rs` tests)
```rust
    fn rca2(verts: &[([f32; 3], [u16; 2])]) -> Vec<u8> {
        let mut d = b"RCA2".to_vec();
        for _ in 0..2 {   // body and wheel: one triangle each
            d.extend_from_slice(&(verts.len() as u32).to_le_bytes()); d.extend_from_slice(&3u32.to_le_bytes());
            for (p, uv) in verts { for f in p.iter().chain([0.0f32, 1.0, 0.0].iter()) { d.extend_from_slice(&f.to_le_bytes()); } d.extend_from_slice(&[10, 20, 30, 0]); d.extend_from_slice(&uv[0].to_le_bytes()); d.extend_from_slice(&uv[1].to_le_bytes()); }
            for i in 0..3u32 { d.extend_from_slice(&i.to_le_bytes()); }
        }
        d
    }
    #[test] fn rca2_files_carry_uvs_and_the_sentinel() {
        let (b, _) = parse(&rca2(&[([0.0, 0.0, 0.0], [100, 200]), ([1.0, 0.0, 0.0], [0xFFFF, 0xFFFF]), ([0.0, 1.0, 0.0], [0, 65534])])).unwrap();
        assert_eq!(b.verts[0].uv, [100, 200]); assert_eq!(b.verts[1].uv, crate::raster::vehicle_models::NO_UV); assert_eq!(b.verts[2].uv, [0, 65534]);
        assert_eq!(b.verts[0].col, [10, 20, 30, 0]);
    }
    #[test] fn rcar_v1_still_parses_with_no_uv_and_rca2_garbage_is_an_error() {
        let (b, _) = parse(PEUGEOT_V1).unwrap();
        assert!(b.verts.iter().all(|v| v.uv == crate::raster::vehicle_models::NO_UV));
        let good = rca2(&[([0.0; 3], [1, 2]), ([1.0, 0.0, 0.0], [3, 4]), ([0.0, 1.0, 0.0], [5, 6])]);
        assert!(parse(&good[..good.len() - 5]).is_err(), "truncated");
        assert!(parse(b"RCA3").is_err());
    }
```
  `PEUGEOT_V1` is a copy of the current v1 asset kept for this test: `cp rust/assets/rally_car.rcar rust/assets/rally_car_v1.rcar` and `const PEUGEOT_V1: &[u8] = include_bytes!("../../assets/rally_car_v1.rcar");` in the tests (a test-only file; it is removed when Task 2 regenerates the main asset, then the test keeps using the v1 copy).
- [ ] **Step 2:** `~/bin/t rcar` → FAIL (no `uv`, no `NO_UV`).
- [ ] **Step 3: implement.** `vehicle_models.rs`: `pub const NO_UV: [u16; 2] = [0xFFFF, 0xFFFF];` and `pub struct Vert { pub pos: [f32; 3], pub nrm: [f32; 3], pub col: [u8; 4], pub uv: [u16; 2] }`; add `uv: NO_UV` to every `Vert { ... }` literal (`vehicle_models.rs` x2, `nature_models.rs`, `rcar.rs` tests). In `rcar.rs`: `fn mesh(d, o, stride: usize)`; vertex reading `uv: if stride == 32 { [u16::from_le_bytes([d[a+28], d[a+29]]), u16::from_le_bytes([d[a+30], d[a+31]])] } else { NO_UV }`, the size check uses `nv * stride`, the offset advance `*o += stride`; `parse` selects the stride from the magic (`RCAR` 28, `RCA2` 32, else error). `gpu.rs`: `MODEL_ATTRS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 8 => Unorm16x2];`. `raster.wgsl`: `MOut` gets `@location(6) uv: vec2<f32>`; `vs_model` gets the parameter `@location(8) uv: vec2<f32>` and sets `o.uv = uv;` (`vs_model_shadow` needs no change).
- [ ] **Step 4:** `~/bin/t` (all) and `~/bin/t shaders_validate` → PASS.
- [ ] **Step 5: commit** `feat: vertex UVs and the RCA2 model format (v1 still parses)`

### Task 2: Converter: atlas packing, UV output, JPEG; regenerate the asset

**Files:** Create `tools/carconv/src/atlas.rs`; Modify `tools/carconv/src/main.rs`; Modify `rust/assets/rally_car.rcar`, Create `rust/assets/rally_car_livery.jpg`.

**Interfaces:** Produces `atlas::pack(sizes: &[(usize, u32, u32)], atlas: u32) -> Vec<Rect>` with `Rect { id: usize, x: u32, y: u32, w: u32, h: u32 }` (pixels in the atlas, tiles keep the source aspect, `w,h >= 1`, no overlap, inside `atlas`).

- [ ] **Step 1: failing tests** in `tools/carconv/src/atlas.rs`
```rust
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
        assert!(big.w >= 1200, "the livery keeps most of the area: {big:?}");
    }
    #[test] fn an_impossible_request_still_returns_tiles_that_fit() {
        let r = pack(&[(0, 8192, 8192), (1, 8192, 8192), (2, 8192, 8192)], 256);
        for a in &r { assert!(a.x + a.w <= 256 && a.y + a.h <= 256); }
    }
}
```
- [ ] **Step 2:** `cd tools/carconv && cargo test atlas` → FAIL (no `pack`; add `mod atlas;` to `main.rs`).
- [ ] **Step 3: implement `pack`:** binary-search a global scale `s` in `(0, 1]` (tiles are never enlarged): for a scale, size each tile `(max(1, round(w*s)), max(1, round(h*s)))` and shelf-pack them sorted by height descending with a 2-pixel gutter; take the largest scale that fits (search 20 iterations); return the rectangles of that scale.
- [ ] **Step 4: converter changes.** (a) `V` gets `uv: [f32; 2]` and `tex: Option<usize>`; in `walk` store the raw `uvs[i]` (wrapped with `fract`) and the texture image index for classes PAINT/PLASTIC/CHROME/RUBBER, colour = the `base_color_factor` (no per-vertex sampling) and `tex = None`, the old sampling, for EMISSIVE and GLASS. (b) After `merge`, collect the used image indices with their sizes (`Textures::get`), `atlas::pack(.., 2048)`, then for each vertex with a texture set `uv = [(rect.x + 1 + u*(rect.w-2)) / 2048, (rect.y + 1 + v*(rect.h-2)) / 2048]` (inset one texel). (c) `simplify` weights: add the two UV components to the attribute vector with weight 1.0 each (extend the `attrs` and the weights slice to 8 entries). (d) `write` emits `RCA2` with the 32-byte vertex (`uv` as `u16` = `round(uv*65534)`, vertices without a texture get `0xFFFF`). (e) Compose the atlas image: resize each used texture with `image::imageops::resize(.., Lanczos3)` to its tile and copy it in; save as JPEG quality 90 to the path given as a new 5th argument `<livery.jpg>`. The converter stays runnable as `carconv <in.glb> <out.rcar> <body_tris> <wheel_tris> <livery.jpg>`.
- [ ] **Step 5: run it** (in an empty dir under `$TMPDIR`, from another dir): unzip the source zip, `cd tools/carconv && cargo build --release`, then `target/release/carconv $TMPDIR/peu/source/*.glb $TMPDIR/new.rcar 12000 1500 $TMPDIR/livery.jpg 2>&1 | tail -30`. Expected: same bounds as before (`body bounds: x -0.954..0.954 y 0.077..1.501 z -1.870..2.088`), a 2048 atlas, the file about 0.45 MB. Copy `new.rcar` to `rust/assets/rally_car.rcar` and `livery.jpg` to `rust/assets/rally_car_livery.jpg`; look at the JPEG (resize to 1000 px and read it): the CRAPAINT livery must be sharp and complete.
- [ ] **Step 6: game-side test** (append to `rcar.rs` tests, using the NEW asset): `the_embedded_peugeot_has_uvs_on_the_paint_and_none_on_lamps`: parse `PEUGEOT`, assert that more than 70 % of the vertices of material 0 have `uv != NO_UV`, none of the vertices of material 5 have one, and every non-sentinel UV component is `< 0xFFFF`. `~/bin/t rcar` and `~/bin/t rally_model` → PASS (lamp points, covers and bounds unchanged; if a lamp test moves by more than its tolerance, stop and investigate: the converter changed geometry).
- [ ] **Step 7: commit** `feat: the Peugeot asset with UVs and a livery atlas (RCA2)`

### Task 3: Decode, upload and sample the livery in the game

**Files:** Create `rust/src/raster/livery.rs`; Modify `rust/Cargo.toml` (`zune-jpeg = "0.5"`), `rust/src/raster/mod.rs`, `rust/src/raster/gpu.rs` (layout entries 8/9, bind group, texture), `rust/src/shaders/raster.wgsl` (bindings, `fs_model`).

**Interfaces:** Produces `livery::decode(jpeg: &[u8]) -> Option<(u32, u32, Vec<u8>)>` (RGBA8), `livery::mips(rgba: &[u8], w: u32, h: u32) -> Vec<(u32, u32, Vec<u8>)>` (level 0 first, down to 1x1, box filter), `pub const LIVERY: &[u8] = include_bytes!("../../assets/rally_car_livery.jpg")`.

- [ ] **Step 1: failing tests** in `livery.rs`
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn the_embedded_atlas_decodes_to_2048_square_rgba() {
        let (w, h, px) = decode(LIVERY).expect("livery decodes");
        assert_eq!((w, h), (2048, 2048)); assert_eq!(px.len(), 2048 * 2048 * 4); assert!(px.chunks(4).step_by(997).all(|p| p[3] == 255));
    }
    #[test] fn garbage_is_none_not_a_panic() { assert!(decode(b"").is_none() && decode(b"not a jpeg").is_none() && decode(&LIVERY[..100]).is_none()); }
    #[test] fn the_mip_chain_halves_down_to_one_pixel_and_averages() {
        let (w, h, px) = (4u32, 4u32, vec![255u8; 4 * 4 * 4]);
        let m = mips(&px, w, h);
        assert_eq!(m.iter().map(|l| (l.0, l.1)).collect::<Vec<_>>(), vec![(4, 4), (2, 2), (1, 1)]);
        let mut chk = vec![0u8; 4 * 4 * 4]; for p in chk.chunks_mut(4) { p.copy_from_slice(&[200, 100, 0, 255]); } chk[0] = 0;
        assert!(mips(&chk, 4, 4)[1].2[0] < 200 && mips(&chk, 4, 4)[1].2[0] > 100, "the average is between");
        assert_eq!(mips(&vec![255u8; 2048 * 2048 * 4], 2048, 2048).len(), 12);
    }
}
```
- [ ] **Step 2:** `~/bin/t livery` → FAIL.
- [ ] **Step 3: implement** `decode` with `zune_jpeg::JpegDecoder::new(bytes)` + `decode()` (RGB, expanded to RGBA with alpha 255; any error or `dimensions() == None` → `None`; guard against absurd sizes: more than 8192 per side → `None`); `mips` (each level the average of 2x2 of the previous, in sRGB bytes, odd sizes clamp). Add `pub mod livery;` in `mod.rs`.
- [ ] **Step 4: GPU wiring** (`gpu.rs`): in `new`, build the texture from `livery::decode(livery::LIVERY)` (on `None`: a 1x1 white texture and `log::warn!`), `Rgba8UnormSrgb`, `mip_level_count = mips.len()`, `TEXTURE_BINDING | COPY_DST`, upload every level with `queue.write_texture`; a sampler `{ mag: Linear, min: Linear, mipmap: Linear, address: ClampToEdge, anisotropy_clamp: 4 }`; add to `bgl_main` entries `binding: 8` (Texture 2D filterable float) and `binding: 9` (Sampler Filtering) and to the bind group; keep the texture alive in `_keep`/a field. Every other layout that is built from `bgl_main` (dust, fluid) is unaffected because they only reference the layout object.
- [ ] **Step 5: shader** (`raster.wgsl`): `@group(0) @binding(8) var livery_tex: texture_2d<f32>;` `@group(0) @binding(9) var livery_smp: sampler;`. At the top of `fs_model` (before any early return, uniform control flow): `let lv = textureSample(livery_tex, livery_smp, clamp(i.uv, vec2<f32>(0.0), vec2<f32>(1.0)));` `let has_uv = i.uv.x < 0.9999;` `let albedo = select(i.col, i.col * lv.rgb, has_uv);` and use `albedo` instead of `i.col` in the glass? no: only in the paint (0), plastic (default), chrome (4) and rubber (2) branches (`shade(albedo * occl, ...)`). Lamps and covers keep `i.col`.
- [ ] **Step 6:** `~/bin/t shaders_validate`, `~/bin/t` (all), then `./gradlew --no-daemon testDebugUnitTest assembleRelease -q` → PASS, APK builds.
- [ ] **Step 7: commit** `feat: the livery atlas is decoded, mipped and sampled on the car`

### Task 4: Docs, licence, notes

**Files:** `docs/vehicles.md`, `rust/assets/ASSET-LICENSE.md`, `CHANGELOG.md`, `CHANGELOG.ru.md` (a `2.30.0` entry), `~/projects/releases/rallynvx-2.30.0.md`.

- [ ] **Step 1:** `docs/vehicles.md` (EN + RU paragraphs): the `RCA2` format, the atlas and how to convert a new model (`carconv in.glb out.rcar 12000 1500 livery.jpg`, which materials are textured); `ASSET-LICENSE.md`: the atlas JPEG is a derivative under the same CC BY-NC-SA 4.0 terms, and the change list now says textures are kept (downscaled into an atlas) instead of baked into vertex colours; changelog EN + RU: "Changed: the Peugeot's livery is a real texture (a 2048 atlas) instead of blurred vertex colours; sharp stickers and logos".
- [ ] **Step 2:** `~/bin/t` and the Gradle build once more; commit `docs: livery texture`. Then ask the user for the phone (a day shot of the car, side and front, compared with the old build) and for the go before a release.
