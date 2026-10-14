//! Getting from anywhere onto the special stage: a field path found with the roads' A* (lakes and slopes are expensive).
use super::cfg::AutoCfg;
use super::route::Route;
use crate::game::math::v3;
use crate::game::roads::{path::Grid, shape};
use crate::game::surface::Surface;
use crate::game::terrain::Terrain;
use crate::game::vehicle::VehicleSpec;

/// The grid cell of the planner (coarse: the path is smoothed afterwards).
pub const CELL: f32 = 32.0;

/// A route over the open ground from `from` to `to` (world x, z), driven at the field speed. `None` when the points
/// are closer than 8 m (nothing to plan).
pub fn field_route(t: &Terrain, car: &VehicleSpec, from: (f32, f32), to: (f32, f32), c: &AutoCfg) -> Option<Route> {
    if (from.0 - to.0).hypot(from.1 - to.1) < 8.0 { return None; }
    let grid = Grid::build(t, CELL, 3800.0);
    let mut raw = grid.find(from, to, true);
    if raw.len() < 2 { raw = vec![from, to]; }
    let smooth = shape::smooth_curve(&raw);
    let pts: Vec<_> = smooth.iter().map(|p| v3(p.0, t.height(p.0, p.1), p.1)).collect();
    if pts.len() < 7 { return None; }
    let width = vec![0.0; pts.len()];   // no corridor: the field path is driven as it is
    Some(Route::build(&pts, &width, super::limits_for(car, Surface::Grass, c.field_speed, false), c))
}

#[cfg(test)]
#[path = "join_tests.rs"]
mod tests;
