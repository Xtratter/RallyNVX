//! Off-road game: pure CPU gameplay (no wgpu). Rendering lives in `crate::raster`.
pub mod app;
pub mod camera_follow;
#[cfg(feature = "desktop")]
pub mod desktop;
pub mod input;
pub mod math;
pub mod props;
pub mod terrain;
pub mod vehicle;
pub mod world;
