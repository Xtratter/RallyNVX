use super::*;
use crate::game::{input::Input, vehicle::*};
#[test] fn camera_stays_above_terrain_and_behind() {
    let t = Terrain::new(6);
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., t.height(0., 0.) + 1.1, 0.), 0.0);
    let mut c = FollowCam::new(&v);
    for i in 0..1200 {
        v.step(&t, &[], &Input { throttle: 1.0, steer: if (i / 200) % 2 == 0 { 0.4 } else { -0.4 }, ..Default::default() }, 1.0 / 60.0);
        c.update(&v, &t, 1.0 / 60.0);
        assert!(c.pos.y >= t.height(c.pos.x, c.pos.z) + 0.89, "step {i}");
        assert!(c.pos.is_finite() && c.target.is_finite());
    }
    assert!((c.pos - v.pos).len() > 3.0 && (c.pos - v.pos).len() < 14.0);
}
