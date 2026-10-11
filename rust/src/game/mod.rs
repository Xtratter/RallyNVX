//! Off-road game: pure CPU gameplay (no wgpu). Rendering lives in `crate::raster`.
pub mod app;
pub mod api;
pub mod camera_follow;
#[cfg(feature = "desktop")]
pub mod desktop;
pub mod flight;
pub mod input;
pub mod math;
pub mod medals;
pub mod meta;
pub mod missions;
pub mod props;
pub mod run;
pub mod sling;
pub mod terrain;
pub mod vehicle;
pub mod world;
