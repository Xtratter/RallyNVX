//! Dust billboards: the particles as instance data, sorted far to near for alpha blending.
use crate::game::{dust::Particle, math::V3};

/// Position + size, colour + alpha (32 bytes, the instance vertex layout of the dust pipeline).
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DustInst { pub p: [f32; 4], pub c: [f32; 4] }

pub const MAX_INST: usize = 1024;

pub fn instances(ps: &[Particle], cam: V3) -> Vec<DustInst> {
    let mut v: Vec<(f32, DustInst)> = ps.iter()
        .filter(|p| p.alpha > 0.003 && p.size > 0.0 && p.size.is_finite() && p.pos.is_finite() && p.alpha.is_finite())
        .map(|p| { let d = (p.pos - cam).len(); (d, DustInst { p: [p.pos.x, p.pos.y, p.pos.z, p.size], c: [p.color[0], p.color[1], p.color[2], p.alpha] }) })
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    v.truncate(MAX_INST);
    v.into_iter().map(|x| x.1).collect()
}

#[cfg(test)]
#[path = "dustgen_tests.rs"]
mod tests;
