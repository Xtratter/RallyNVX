//! The shoulder of the road as room to cut a corner: how far beyond each edge a car may go where the ground is passable (no water, no steep bank or ditch), at every point of a route.
use crate::game::math::V3;
use crate::game::terrain::Terrain;

/// The most that is used (m): the dirt shoulder of the stages is about 2 m wide.
pub const MAX_EXT: f32 = 2.0;

/// For every centre point the usable width beyond the edge on the side along the normal `(-tz, tx)` (first) and against it (second), in steps of 0.5 m.
/// A step is usable when its ground is not water and its height is within 0.45 m per metre of the height at the edge (a bank or a ditch stops it); `half` is the half width of the road.
pub fn extents(t: &Terrain, centre: &[V3], half: &[f32]) -> Vec<(f32, f32)> {
    let n = centre.len();
    (0..n).map(|i| {
        if t.in_city(centre[i].x, centre[i].z, 15.0) || half[i] <= 1.0 { return (0.0, 0.0); }
        let (a, b) = (centre[i.saturating_sub(1)], centre[(i + 1).min(n - 1)]);
        let (tx, tz) = (b.x - a.x, b.z - a.z); let l = tx.hypot(tz).max(1e-3);
        let nor = (-tz / l, tx / l);
        let side = |sgn: f32| -> f32 {
            let at = |d: f32| (centre[i].x + sgn * nor.0 * (half[i] + d), centre[i].z + sgn * nor.1 * (half[i] + d));
            let (ex, ez) = at(0.0); let h0 = t.height(ex, ez);
            let mut ok = 0.0;
            for k in 1..=(MAX_EXT / 0.5) as i32 {
                let d = k as f32 * 0.5; let (x, z) = at(d);
                if t.is_water(x, z, 0.6) || (t.height(x, z) - h0).abs() > 0.45 * d { break; }
                ok = d;
            }
            ok
        };
        (side(1.0), side(-1.0))
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::math::v3;
    #[test] fn a_flat_bare_ground_gives_the_whole_shoulder_and_no_road_gives_none() {
        let t = Terrain::flat();
        let c: Vec<V3> = (0..10).map(|i| v3(0.0, 0.0, i as f32 * 4.0)).collect();
        let e = extents(&t, &c, &vec![4.0; 10]);
        assert!(e.iter().all(|x| x.0 == MAX_EXT && x.1 == MAX_EXT), "{e:?}");
        assert!(extents(&t, &c, &vec![0.0; 10]).iter().all(|x| *x == (0.0, 0.0)), "off the roads nothing");
    }
}
