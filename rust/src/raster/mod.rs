//! Raster renderer (render mode C): tiled terrain, instanced props and vehicle, sky, sun shadow map.
pub mod meshes;

#[cfg(any(target_os = "android", feature = "desktop"))]
mod gpu;
#[cfg(any(target_os = "android", feature = "desktop"))]
pub use gpu::Raster;
