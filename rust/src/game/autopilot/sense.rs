//! What the car senses of itself and of the road around it, for a net that does not read the route's line: a fan of rays that measure how far the usable road lies in each
//! direction (the machine's "eyes"), the road's edge beside the car, and everything the car knows of itself (its speeds and accelerations, the slip of each wheel, the tyres'
//! temperatures and wear, the damage, the fuel, the engine's and the clutch's heat, the revs and the gear, the grip under the wheels, the pitch and the roll), plus a coarse hint
//! of where the route goes (like a co-driver's "left in 100 m"), not its line or its planned speeds.
use crate::game::{math::V3, terrain::Terrain, vehicle::Vehicle};

/// The angles of the rays from the car's heading (degrees, negative to the right-hand side as `v.right()` counts positive).
pub const RAY_DEG: [f32; 15] = [-80.0, -58.0, -40.0, -27.0, -17.0, -9.0, -3.5, 0.0, 3.5, 9.0, 17.0, 27.0, 40.0, 58.0, 80.0];
/// How far a ray looks (m).
pub const RANGE: f32 = 80.0;
/// The size of the sensors' feature vector.
pub const N: usize = 15 + 3 + 29 + 4;

/// What the engine side of the world tells the autopilot each step (the drivetrain is not a part of the vehicle).
#[derive(Clone, Copy, Debug, Default)]
pub struct Telemetry { pub rpm: f32, pub gear: i32, pub fuel_l: f32, pub coolant_c: f32, pub clutch_c: f32, pub throttle: f32, pub brake: f32 }

/// The usable ground: the road and 2 m of shoulder beyond its edge.
fn usable(t: &Terrain, x: f32, z: f32) -> bool { t.road_edge_distance(x, z).map_or(false, |e| e < 2.0) && !t.is_water(x, z, 0.3) }

/// How far (m, up to `RANGE`) the ground stays usable along the ray from (x, z) in the direction (dx, dz): steps of 2 m to 40 m, then of 4 m.
fn ray(t: &Terrain, x: f32, z: f32, dx: f32, dz: f32) -> f32 {
    let mut d = 0.0f32;
    while d < RANGE {
        let step = if d < 40.0 { 2.0 } else { 4.0 };
        d += step;
        if !usable(t, x + dx * d, z + dz * d) { return d - step; }
    }
    RANGE
}

/// The feature vector of the sensors (see the module); `hint` are the car-frame (side, forward) unit components of the directions to the route points 40 m and 100 m ahead.
pub fn features(t: &Terrain, v: &Vehicle, tel: &Telemetry, prev_vel: V3, dt: f32, hint: [f32; 4]) -> Vec<f32> {
    let mut f = Vec::with_capacity(N);
    let (fwd, right) = (v.forward(), v.right());
    let (fx, fz) = { let l = fwd.x.hypot(fwd.z).max(1e-3); (fwd.x / l, fwd.z / l) };
    let (rx, rz) = { let l = right.x.hypot(right.z).max(1e-3); (right.x / l, right.z / l) };
    for deg in RAY_DEG {
        let a = deg.to_radians();
        // positive degrees to the right
        let (dx, dz) = (fx * a.cos() + rx * a.sin(), fz * a.cos() + rz * a.sin());
        f.push(ray(t, v.pos.x, v.pos.z, dx, dz) / RANGE);
    }
    let edge = |ox: f32| t.road_edge_distance(v.pos.x + rx * ox, v.pos.z + rz * ox).map_or(1.0, |e| (e / 6.0).clamp(-1.0, 1.0));
    f.push(edge(0.0)); f.push(edge(-4.0)); f.push(edge(4.0));
    let speed = v.vel.len();
    let (vf, vr) = (v.vel.x * fwd.x + v.vel.z * fwd.z, v.vel.x * right.x + v.vel.z * right.z);
    let acc = if dt > 1e-4 { (v.vel - prev_vel) * (1.0 / dt) } else { V3::ZERO };
    let (a_long, a_lat) = (acc.x * fwd.x + acc.z * fwd.z, acc.x * right.x + acc.z * right.z);
    f.push(speed / 30.0); f.push(vf / 30.0); f.push((vr / 10.0).clamp(-1.5, 1.5)); f.push((v.omega.dot(v.up()) / 2.0).clamp(-2.0, 2.0));
    f.push((a_long / 10.0).clamp(-1.5, 1.5)); f.push((a_lat / 10.0).clamp(-1.5, 1.5));
    for k in 0..4 { f.push(v.wheels[k].slip.clamp(-1.0, 1.0)); }
    for k in 0..4 { f.push((v.wheels[k].lat_slip / 5.0).clamp(-1.5, 1.5)); }
    f.push(v.wheels.iter().filter(|w| w.grounded).count() as f32 / 4.0);
    f.push((v.steer / v.spec.steer_max.max(0.1)).clamp(-1.5, 1.5));
    for k in 0..4 { f.push(((v.tyres[k].temp - 60.0) / 40.0).clamp(-2.0, 2.0)); }
    f.push(v.tyres.iter().map(|y| y.wear).sum::<f32>() / 4.0);
    f.push(v.damage.total());
    f.push(tel.fuel_l / 70.0); f.push((tel.coolant_c / 100.0).clamp(0.0, 1.5)); f.push((tel.clutch_c / 300.0).clamp(0.0, 1.5));
    f.push(tel.rpm / 8000.0); f.push(tel.gear as f32 / 6.0);
    f.push(tel.throttle); f.push(tel.brake);
    let mu = v.wheels.iter().map(|w| crate::game::grip::grip(w.surface, 1.0).mu).sum::<f32>() / 4.0;
    f.push(mu); f.push(v.wet);
    f.push(fwd.y.clamp(-1.0, 1.0) * 3.0); f.push(right.y.clamp(-1.0, 1.0) * 3.0);
    // the pedal and the steering the net gave last are in `tel` and `v.steer`; the route's coarse hint last
    f.extend_from_slice(&hint);
    f.truncate(N); while f.len() < N { f.push(0.0); }
    f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{math::v3, vehicle::VehicleSpec};
    #[test] fn the_feature_vector_has_its_size_and_is_finite_on_a_bare_ground() {
        let t = Terrain::flat();
        let v = Vehicle::new(VehicleSpec::offroader(), v3(0.0, 1.1, 0.0), 0.3);
        let f = features(&t, &v, &Telemetry::default(), V3::ZERO, 1.0 / 60.0, [0.0; 4]);
        assert_eq!(f.len(), N);
        assert!(f.iter().all(|x| x.is_finite() && x.abs() <= 3.01), "{f:?}");
        assert!(f[..15].iter().all(|x| *x == 0.0), "no road: no usable ground in any direction");
    }
    #[test] fn on_a_straight_road_the_rays_see_it_ahead_and_the_edges_beside_symmetrically() {
        use crate::game::roads::{Road, RoadKind, RoadNet, RoadPt};
        let pts: Vec<RoadPt> = (0..60).map(|i| RoadPt { pos: v3(0.0, 0.0, i as f32 * 4.0), width: 8.0, s: i as f32 * 4.0 }).collect();
        let t = Terrain::flat().with_net(RoadNet::new(vec![Road { kind: RoadKind::Stage, pts }]));
        let v = Vehicle::new(VehicleSpec::offroader(), v3(0.0, 1.1, 20.0), 0.0);   // heading +z, in the middle of the road
        let f = features(&t, &v, &Telemetry::default(), V3::ZERO, 1.0 / 60.0, [0.0; 4]);
        assert!(f[7] > 0.9, "straight ahead the road runs on: {}", f[7]);
        assert!(f[0] < 0.15 && f[14] < 0.15, "to the sides it ends at once: {} {}", f[0], f[14]);
        for k in 0..7 { assert!((f[k] - f[14 - k]).abs() < 0.12, "ray {k} against its mirror: {} {}", f[k], f[14 - k]); }
        assert!(f[15] < -0.3, "in the middle the edge is 4 m away, inside the road: {}", f[15]);
    }
}
