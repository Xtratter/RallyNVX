//! The primitive part shared by the city generator and the renderer.
use crate::game::math::{Q, V3};

/// One primitive: `mesh` indexes the shared meshes (0 cube, 1 cylinder, 2 cone, 3 sphere).
#[derive(Clone, Copy, Debug)]
pub struct Part { pub mesh: usize, pub rot: Q, pub pos: V3, pub scale: V3, pub color: [f32; 3], pub glow: bool, pub facade: bool, pub street: bool }
