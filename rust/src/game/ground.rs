//! The surface aircraft and cargo touch: plain terrain, or terrain plus city roofs.
use super::city::City;
use super::math::{v3, V3};
use super::terrain::Terrain;

pub trait Ground {
    fn height(&self, x: f32, z: f32) -> f32;
    fn normal(&self, x: f32, z: f32) -> V3;
}
impl Ground for Terrain {
    fn height(&self, x: f32, z: f32) -> f32 { Terrain::height(self, x, z) }
    fn normal(&self, x: f32, z: f32) -> V3 { Terrain::normal(self, x, z) }
}

pub struct WorldGround<'a> { pub terrain: &'a Terrain, pub cities: &'a [City] }
impl WorldGround<'_> {
    fn roof(&self, x: f32, z: f32) -> Option<f32> { self.cities.iter().filter(|c| c.inside(x, z)).find_map(|c| c.roof_at(x, z)) }
}
impl Ground for WorldGround<'_> {
    fn height(&self, x: f32, z: f32) -> f32 { let h = self.terrain.height(x, z); self.roof(x, z).map_or(h, |r| r.max(h)) }
    fn normal(&self, x: f32, z: f32) -> V3 {
        match self.roof(x, z) { Some(r) if r >= self.terrain.height(x, z) => v3(0.0, 1.0, 0.0), _ => self.terrain.normal(x, z) }
    }
}

#[cfg(test)]
#[path = "ground_tests.rs"]
mod tests;
