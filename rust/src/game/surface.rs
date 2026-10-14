//! What the ground is made of at a point (grip, dust and sound will read it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface { Grass, Dirt, Sand, Rock, Gravel, Asphalt, Water }

use super::materials::terrain_weights;
use super::terrain::Terrain;

/// The ground at (x, z): the road kind on a road, dirt on its 2 m shoulder, asphalt in a city, water in a lake, else the
/// strongest terrain material.
pub fn surface_at(t: &Terrain, x: f32, z: f32) -> Surface {
    if let (Some(h), Some(net)) = (t.road_hit(x, z), t.roads()) {
        if h.dist <= h.half { return net.roads[h.road].kind.surface(); }
        if h.dist <= h.half + 2.0 { return Surface::Dirt; }
    }
    if t.in_city(x, z, 0.0) { return Surface::Asphalt; }
    if t.is_water(x, z, 0.0) { return Surface::Water; }
    if t.is_flat_test() { return Surface::Asphalt; }
    let w = terrain_weights(t.height(x, z), t.normal(x, z), x, z);
    let k = (0..4).max_by_key(|&i| w[i]).unwrap();
    [Surface::Grass, Surface::Dirt, Surface::Sand, Surface::Rock][k]
}
