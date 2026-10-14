use super::*;
use crate::game::{roads::{Road, RoadKind, RoadNet, RoadPt}, terrain::Terrain};

const DT: f32 = 1.0 / 60.0;

fn road_from(pts: &[V3], kind: RoadKind, width: f32) -> Road {
    let mut s = 0.0; let mut out = Vec::new();
    for (i, p) in pts.iter().enumerate() { if i > 0 { s += ((p.x - pts[i - 1].x).hypot(p.z - pts[i - 1].z)).max(0.0); } out.push(RoadPt { pos: *p, width, s }); }
    Road { kind, pts: out }
}
/// A polyline of straight and arc pieces; `turn` in degrees (positive left) per piece, `len` metres.
fn track(pieces: &[(f32, f32)]) -> Vec<V3> {
    let (mut x, mut z, mut h) = (0.0f32, -100.0f32, 0.0f32);   // heading 0 = +z
    let mut pts = vec![v3(x, 0.0, z)];
    for &(len, turn) in pieces {
        let n = (len / 4.0).ceil() as usize; let dh = turn.to_radians() / n as f32;
        for _ in 0..n { h -= dh; x += 4.0 * h.sin(); z += 4.0 * h.cos(); pts.push(v3(x, 0.0, z)); }
    }
    pts
}
fn seg_dist(a: V3, b: V3, p: V3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    let t = (((p.x - a.x) * dx + (p.z - a.z) * dz) / (dx * dx + dz * dz).max(1e-6)).clamp(0.0, 1.0);
    (a.x + dx * t - p.x).hypot(a.z + dz * t - p.z)
}
struct Run { max_off: f32, finished: bool, flipped: bool, recoveries: u32, time: f32, mean_speed: f32 }
/// Drives a car with the autopilot along the road until it is done (or `limit` seconds).
fn drive(pts: &[V3], kind: RoadKind, limit: f32) -> Run {
    let road = road_from(pts, kind, 8.0);
    let t = Terrain::flat().with_net(RoadNet::new(vec![road.clone()]));
    let car = VehicleSpec::offroader();
    let route = route_for_road(&road, &car, &cfg());
    let yaw = (pts[1].x - pts[0].x).atan2(pts[1].z - pts[0].z);
    let mut v = Vehicle::new(car, v3(pts[0].x, pts[0].y + 1.1, pts[0].z), yaw);
    let mut ap = Autopilot::on_route(route, car, &v);
    let mut loose = crate::game::grip::LooseMap::new(t.roads().unwrap());
    let (mut max_off, mut time, mut flipped, mut finished) = (0.0f32, 0.0f32, false, false);
    let mut dist = 0.0f32;
    while time < limit {
        let cmd = ap.drive(&v, DT);
        if cmd.done { finished = true; break; }
        if cmd.reset { v.reset_upright(&t); }
        if let Some((p, yaw)) = cmd.teleport { v = Vehicle::new(car, p, yaw); }
        let before = v.pos;
        v.step_with(&t, &[], &cmd.input, DT, &mut loose);
        dist += (v.pos - before).len();
        time += DT;
        if v.flipped() { flipped = true; }
        let off = pts.windows(2).map(|w| seg_dist(w[0], w[1], v.pos)).fold(f32::INFINITY, f32::min);
        max_off = max_off.max(off);
    }
    if std::env::var("DBG").is_ok() { eprintln!("end: pos {:?} speed {:.1} idx {} of {} t {:.0}", v.pos, v.speed(), ap.progress(), ap.route().length(), time); }
    Run { max_off, finished, flipped, recoveries: ap.recoveries, time, mean_speed: dist / time.max(0.1) }
}

#[test] fn a_straight_is_driven_and_stopped_at_the_end() {
    let r = drive(&track(&[(600.0, 0.0)]), RoadKind::Stage, 80.0);
    assert!(r.finished && !r.flipped && r.recoveries == 0, "finished {} t {} off {}", r.finished, r.time, r.max_off);
    assert!(r.max_off < 1.0, "{}", r.max_off);
}
#[test] fn an_s_bend_stays_on_the_road() {
    let r = drive(&track(&[(150.0, 0.0), (120.0, 90.0), (60.0, 0.0), (120.0, -90.0), (150.0, 0.0)]), RoadKind::Stage, 120.0);
    assert!(r.finished && !r.flipped && r.recoveries == 0, "finished {} t {} off {} flipped {}", r.finished, r.time, r.max_off, r.flipped);
    assert!(r.max_off < 3.0, "off {}", r.max_off);
}
#[test] fn a_hairpin_is_taken_without_leaving_the_road() {
    let r = drive(&track(&[(200.0, 0.0), (40.0, 180.0), (200.0, 0.0)]), RoadKind::Stage, 120.0);
    assert!(r.finished && !r.flipped && r.recoveries == 0, "finished {} t {} off {}", r.finished, r.time, r.max_off);
    assert!(r.max_off < 3.5, "off {}", r.max_off);
}
#[test] fn a_crest_on_a_straight_is_taken_and_the_car_lands_on_the_road() {
    let mut pts = track(&[(900.0, 0.0)]);
    for p in pts.iter_mut() { let d = p.z - 300.0; if d.abs() < 30.0 { p.y = 1.5 * 0.5 * (1.0 + (std::f32::consts::PI * d / 30.0).cos()); } }
    let r = drive(&pts, RoadKind::Stage, 90.0);
    assert!(r.finished && !r.flipped && r.recoveries == 0, "finished {} t {} off {} flipped {}", r.finished, r.time, r.max_off, r.flipped);
    assert!(r.max_off < 1.5, "{}", r.max_off);
}
#[test] fn a_stuck_car_is_put_back_on_the_route_and_a_flipped_one_is_righted() {
    let pts = track(&[(400.0, 0.0)]);
    let road = road_from(&pts, RoadKind::Stage, 8.0);
    let car = VehicleSpec::offroader();
    let mut v = Vehicle::new(car, v3(pts[10].x, 1.1, pts[10].z), 0.0);
    let mut ap = Autopilot::on_route(route_for_road(&road, &car, &cfg()), car, &v);
    let mut got = None;
    for _ in 0..(10.0 / DT) as usize { let c = ap.drive(&v, DT); if c.teleport.is_some() { got = c.teleport; break; } }
    let (p, _) = got.expect("a car that does not move for several seconds is teleported back onto the route");
    assert!(p.x.abs() < 1.0);
    let mut ap = Autopilot::on_route(route_for_road(&road, &car, &cfg()), car, &v);
    v.rot = crate::game::math::Q::axis_angle(v3(0.0, 0.0, 1.0), std::f32::consts::PI);
    assert!(v.flipped() && ap.drive(&v, DT).reset);
}
