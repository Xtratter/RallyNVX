//! Generated vehicle meshes (no files): the off-roader, the helicopter and the drone, plus the pose helpers that place
//! their moving parts. Vehicle local frame: +z forward, +y up, +x to the vehicle's left.
use bytemuck::{Pod, Zeroable};
use crate::game::{math::{v3, Q, V3}, vehicle::{Vehicle, VehicleSpec}};

pub const MAT_PAINT: u8 = 0;
pub const MAT_GLASS: u8 = 1;
pub const MAT_RUBBER: u8 = 2;
pub const MAT_PLASTIC: u8 = 3;
pub const MAT_CHROME: u8 = 4;
pub const MAT_EMISSIVE: u8 = 5;
pub const MAT_FOLIAGE: u8 = 6;
/// The lamps of a car model whose lamp state the shader shows (the headlight, tail, reverse and indicator units keep their shape; see `rcar::lamp_points`).
pub const MAT_CARLAMP: u8 = 7;
/// A front position lamp of a car model (the corner piece of the headlight unit, see `rcar::position_vertices`): lit with the low beam.
pub const MAT_CARPOS: u8 = 8;
/// The reflector interior of a front / of a rear lamp unit of a car model (textured; lit by the car's state in the shader).
pub const MAT_CARINT_FRONT: u8 = 11;
pub const MAT_CARINT_REAR: u8 = 12;
/// A brake disc of a car model: textured metal that glows when hot (see `Vehicle::brake_heat`).
pub const MAT_CARDISC: u8 = 13;
/// A suspension part of a car model: textured; the vertex colour carries its weight (red: 1 at the hub, 0 at the far end) and its corner (green: 0 front-left (+x), 1 front-right, 2 rear-left, 3 rear-right); the shader lifts it by the hub's offset times the weight.
pub const MAT_CARSUSP: u8 = 14;
/// The interior and the driver of a car model: textured from the interior atlas, darkened by a cabin factor, drawn only near the car (see `draw_interior`).
pub const MAT_CARINT: u8 = 15;

/// The near plane for a camera `dist` metres from the car: 0.08 m inside or right at the car (the cockpit parts are closer than 0.3 m), 0.3 m otherwise.
pub fn near_for(dist: f32) -> f32 { if dist < 2.5 { 0.08 } else { 0.3 } }

/// Beyond this camera distance (metres) the car's windows are solid dark glass and the interior is not drawn.
pub const INTERIOR_RANGE: f32 = 8.0;
/// Whether the interior is drawn for a camera `dist` metres from the car.
pub fn draw_interior(dist: f32) -> bool { dist < INTERIOR_RANGE }

/// The alpha of the car's glass: 0.2 + 0.3 fresnel near the car, rising to 0.95 between 4 and 8 m of camera distance (the cabin must not show through from afar);
/// a clear lamp cover (`clear` 1) is a quarter as strong. Mirrored in `raster.wgsl::fs_glass`.
pub fn glass_alpha(dist: f32, fresnel: f32, clear: f32) -> f32 {
    let t = ((dist - 4.0) / 4.0).clamp(0.0, 1.0); let t = t * t * (3.0 - 2.0 * t);
    let near = (0.2 + 0.3 * fresnel.clamp(0.0, 1.0)) * (1.0 - 0.75 * clear.clamp(0.0, 1.0));
    (near + (0.95 - near) * t * (1.0 - 0.75 * clear.clamp(0.0, 1.0))).clamp(0.0, 1.0)
}

/// One vertex: position, normal, colour (rgb) with the material id in the alpha byte. 28 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vert { pub pos: [f32; 3], pub nrm: [f32; 3], pub col: [u8; 4], pub uv: [u16; 2] }

/// The UV of a vertex without a texture (the shader then uses the vertex colour).
pub const NO_UV: [u16; 2] = [0xFFFF, 0xFFFF];

#[derive(Clone, Default)]
pub struct Mesh3 { pub verts: Vec<Vert>, pub idx: Vec<u32> }
impl Mesh3 {
    pub fn tris(&self) -> usize { self.idx.len() / 3 }
    pub fn append(&mut self, o: &Mesh3) {
        let base = self.verts.len() as u32;
        self.verts.extend_from_slice(&o.verts);
        self.idx.extend(o.idx.iter().map(|i| i + base));
    }
    pub fn bounds(&self) -> (V3, V3) {
        let mut lo = v3(f32::MAX, f32::MAX, f32::MAX); let mut hi = v3(f32::MIN, f32::MIN, f32::MIN);
        for v in &self.verts { for (k, c) in v.pos.iter().enumerate() { let (l, h) = match k { 0 => (&mut lo.x, &mut hi.x), 1 => (&mut lo.y, &mut hi.y), _ => (&mut lo.z, &mut hi.z) }; if *c < *l { *l = *c; } if *c > *h { *h = *c; } } }
        (lo, hi)
    }
    pub(crate) fn vert(&mut self, pos: V3, nrm: V3, col: [f32; 3], mat: u8) -> u32 {
        let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        self.verts.push(Vert { pos: [pos.x, pos.y, pos.z], nrm: [nrm.x, nrm.y, nrm.z], col: [c(col[0]), c(col[1]), c(col[2]), mat], uv: NO_UV });
        (self.verts.len() - 1) as u32
    }
}

#[derive(Clone, Copy)] pub(crate) enum Axis { X, Y, Z }

pub(crate) fn add_box(m: &mut Mesh3, centre: V3, half: V3, rot: Q, col: [f32; 3], mat: u8) {
    let faces: [(V3, V3, V3); 6] = [
        (v3(1., 0., 0.), v3(0., 1., 0.), v3(0., 0., 1.)), (v3(-1., 0., 0.), v3(0., 0., 1.), v3(0., 1., 0.)),
        (v3(0., 1., 0.), v3(0., 0., 1.), v3(1., 0., 0.)), (v3(0., -1., 0.), v3(1., 0., 0.), v3(0., 0., 1.)),
        (v3(0., 0., 1.), v3(1., 0., 0.), v3(0., 1., 0.)), (v3(0., 0., -1.), v3(0., 1., 0.), v3(1., 0., 0.)),
    ];
    for (n, u, v) in faces {
        let ids: Vec<u32> = [(-1.0f32, -1.0f32), (1., -1.), (1., 1.), (-1., 1.)].iter().map(|(su, sv)| {
            let local = v3((n.x + u.x * su + v.x * sv) * half.x, (n.y + u.y * su + v.y * sv) * half.y, (n.z + u.z * su + v.z * sv) * half.z);
            m.vert(centre + rot.rotate(local), rot.rotate(n), col, mat)
        }).collect();
        m.idx.extend_from_slice(&[ids[0], ids[1], ids[2], ids[0], ids[2], ids[3]]);
    }
}

pub(crate) fn add_cylinder(m: &mut Mesh3, centre: V3, axis: Axis, radius: f32, len: f32, segs: usize, col: [f32; 3], mat: u8) {
    let (a, b, c) = match axis { Axis::X => (v3(0., 1., 0.), v3(0., 0., 1.), v3(1., 0., 0.)), Axis::Y => (v3(0., 0., 1.), v3(1., 0., 0.), v3(0., 1., 0.)), Axis::Z => (v3(1., 0., 0.), v3(0., 1., 0.), v3(0., 0., 1.)) };
    let ring = |m: &mut Mesh3, off: f32, side: bool| -> Vec<u32> {
        (0..=segs).map(|k| { let th = std::f32::consts::TAU * k as f32 / segs as f32; let n = a * th.cos() + b * th.sin();
            m.vert(centre + n * radius + c * off, if side { n } else { c * off.signum() }, col, mat) }).collect()
    };
    let (r0, r1) = (ring(m, -len / 2.0, true), ring(m, len / 2.0, true));
    for k in 0..segs { m.idx.extend_from_slice(&[r0[k], r1[k], r0[k + 1], r0[k + 1], r1[k], r1[k + 1]]); }
    for off in [-len / 2.0, len / 2.0] {
        let cc = m.vert(centre + c * off, c * off.signum(), col, mat);
        let rr = ring(m, off, false);
        for k in 0..segs { m.idx.extend_from_slice(&[cc, rr[k], rr[k + 1]]); }
    }
}

pub(crate) fn add_sphere(m: &mut Mesh3, centre: V3, radii: V3, rings: usize, segs: usize, col: [f32; 3], mat: u8) {
    let mut ids = Vec::new();
    for r in 0..=rings { let t = std::f32::consts::PI * r as f32 / rings as f32; for s in 0..=segs {
        let a = std::f32::consts::TAU * s as f32 / segs as f32;
        let n = v3(t.sin() * a.cos(), t.cos(), t.sin() * a.sin());
        ids.push(m.vert(centre + v3(n.x * radii.x, n.y * radii.y, n.z * radii.z), n, col, mat));
    } }
    let w = segs + 1;
    for r in 0..rings { for s in 0..segs { let i = r * w + s; m.idx.extend_from_slice(&[ids[i], ids[i + w], ids[i + 1], ids[i + 1], ids[i + w], ids[i + w + 1]]); } }
}

/// A smooth lofted body through stations `(z, half_width, y_bottom, y_top)` with superellipse cross-sections.
pub(crate) fn loft(stations: &[(f32, f32, f32, f32)], ring: usize, exp: f32, col: [f32; 3], mat: u8) -> Mesh3 {
    let mut pts: Vec<V3> = Vec::new();
    for &(z, hw, y0, y1) in stations {
        let (yc, hh) = ((y0 + y1) * 0.5, (y1 - y0) * 0.5);
        for k in 0..ring { let (s, c) = (std::f32::consts::TAU * k as f32 / ring as f32).sin_cos(); let e = 2.0 / exp;
            pts.push(v3(hw * c.signum() * c.abs().powf(e), yc + hh * s.signum() * s.abs().powf(e), z)); }
    }
    let mut tris: Vec<[usize; 3]> = Vec::new();
    for st in 0..stations.len() - 1 { for k in 0..ring {
        let (a, b, c, d) = (st * ring + k, st * ring + (k + 1) % ring, (st + 1) * ring + k, (st + 1) * ring + (k + 1) % ring);
        tris.push([a, c, b]); tris.push([b, c, d]);
    } }
    // end caps: a centre vertex per end
    let first_c = pts.len(); pts.push(pts[..ring].iter().fold(V3::ZERO, |s, p| s + *p) * (1.0 / ring as f32));
    let last_c = pts.len(); let off = (stations.len() - 1) * ring; pts.push(pts[off..off + ring].iter().fold(V3::ZERO, |s, p| s + *p) * (1.0 / ring as f32));
    for k in 0..ring { tris.push([first_c, k, (k + 1) % ring]); tris.push([last_c, off + (k + 1) % ring, off + k]); }
    let centre = pts.iter().fold(V3::ZERO, |s, p| s + *p) * (1.0 / pts.len() as f32);
    let mut nrm = vec![V3::ZERO; pts.len()];
    for t in &tris {
        let n = (pts[t[1]] - pts[t[0]]).cross(pts[t[2]] - pts[t[0]]);
        let ctr = (pts[t[0]] + pts[t[1]] + pts[t[2]]) * (1.0 / 3.0);
        let n = if n.dot(ctr - centre) < 0.0 { -n } else { n };
        for &i in t { nrm[i] = nrm[i] + n; }
    }
    let mut m = Mesh3::default();
    for (p, n) in pts.iter().zip(&nrm) { let n = if n.len() > 1e-9 { n.norm() } else { v3(0., 1., 0.) }; m.vert(*p, n, col, mat); }
    for t in &tris { m.idx.extend_from_slice(&[t[0] as u32, t[1] as u32, t[2] as u32]); }
    m
}

pub(crate) fn scaled(st: &[(f32, f32, f32, f32)], sx: f32, sy: f32, sz: f32) -> Vec<(f32, f32, f32, f32)> { st.iter().map(|&(z, w, a, b)| (z * sz, (w * sx).max(0.06), a * sy, b * sy)).collect() }

pub struct CarModel { pub body: Mesh3, pub wheel: Mesh3, pub disc: Option<Mesh3>, pub caliper: Option<Mesh3>, pub alt_wheels: [Option<Mesh3>; 2], pub glass: Option<Mesh3>, pub interior: Option<Mesh3>, pub steering: Option<Steering>, pub driver: Option<Mesh3>,
    /// The windscreen wipers (the driver's side first): each mesh is stored shifted by `-pivot` so that it sweeps about the origin, `axis` is the sweep axis.
    pub wipers: [Option<Wiper>; 2],
    /// The driver's gloves, stored shifted by the steering wheel's pivot so that they turn with the wheel.
    pub gloves: Option<Mesh3>,
    /// The windscreen's plane in the body's frame, where the running raindrops live (found from the glass mesh).
    pub windscreen: Option<super::glassdrops::Windscreen>,
    /// How far (radians) the wipers sweep: the largest angle at which both blades stay on the glass.
    pub wiper_sweep: f32 }

/// The steering wheel of a car model: its mesh is stored shifted by `-pivot` so that it turns about the origin; `pivot` is where the wheel's centre is in the body frame,
/// `axis` its axis (a unit vector towards the driver).
#[derive(Clone)]
pub struct Steering { pub mesh: Mesh3, pub pivot: V3, pub axis: V3 }

/// The angle the steering wheel is turned by for a front-wheel steer angle (radians): 5.5 times, never beyond 3.8 rad (about 1.2 turns lock to lock).
pub fn steering_wheel_angle(steer: f32) -> f32 { if steer.is_finite() { (-steer * 5.5).clamp(-3.8, 3.8) } else { 0.0 } }

/// How far the wipers sweep (radians) from their parked position.
pub const WIPER_SWEEP: f32 = 1.0;

/// The wipers' cycle in `rain` (0..1) as `(period, wipe)`: the period in seconds and the share of it spent wiping (the rest is a pause at the park position). A drizzle gets an
/// intermittent wipe (a 1.6 s sweep every 5 s), a steady rain the slow speed (1.8 s a cycle) and a downpour the fast one (1.15 s), as car wipers do.
pub fn wiper_cycle(rain: f32) -> (f32, f32) {
    if rain > 0.6 { (1.15, 1.0) } else if rain > 0.25 { (1.8, 1.0) } else { (5.0, 0.32) }
}

/// The angle of the wipers (radians from parked) at the cycle phase `ph` (0..1) for a cycle that wipes during the share `wipe` of it: the blade swings up and back as sin squared
/// (what a crank and a linkage give: slow at the turns, fastest mid-glass) and then waits parked.
pub fn wiper_angle_at(ph: f32, wipe: f32) -> f32 {
    if !(ph.is_finite() && wipe.is_finite()) || wipe <= 0.0 { return 0.0; }
    let u = (ph.rem_euclid(1.0) / wipe).min(1.0);
    let s = (std::f32::consts::PI * u).sin();
    WIPER_SWEEP * s * s
}

/// The angle of the wipers at `time` (seconds) in `rain` (0..1): parked while it is dry, else the sweep of `wiper_angle_at`.
pub fn wiper_angle(time: f32, rain: f32) -> f32 {
    if !(time.is_finite() && rain.is_finite()) || rain < 0.05 { return 0.0; }
    let (period, wipe) = wiper_cycle(rain);
    wiper_angle_at(time / period, wipe)
}

/// The wipers' clock: it runs while it rains and, when the rain stops, finishes the wipe it is in before the blades park (so they never jump).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WiperState { pub phase: f32, pub period: f32, pub wipe: f32 }
impl Default for WiperState { fn default() -> Self { WiperState { phase: 0.0, period: 5.0, wipe: 0.32 } } }
impl WiperState {
    /// Advances by `dt` seconds in `rain`; `true` while the wipers are working or finishing a wipe.
    pub fn advance(&mut self, dt: f32, rain: f32) -> bool {
        let raining = rain.is_finite() && rain >= 0.05;
        if raining { let (p, w) = wiper_cycle(rain); self.period = p; self.wipe = w; }
        if !raining && self.phase == 0.0 { return false; }
        if !raining && self.phase >= self.wipe { self.phase = 0.0; return false; }   // paused between wipes: park at once
        self.phase += dt.clamp(0.0, 0.5) / self.period;
        if self.phase >= 1.0 { self.phase = if raining { self.phase.fract() } else { 0.0 }; }
        if !raining && self.phase >= self.wipe { self.phase = 0.0; }
        true
    }
}

/// One windscreen wiper: its mesh shifted by `-pivot` (it sweeps about the origin), the sweep `axis` (out of the glass), the direction of the blade when parked (`rest`, in the
/// plane of the sweep) and how far the tip reaches from the spindle (`reach`, metres).
#[derive(Clone)]
pub struct Wiper { pub mesh: Mesh3, pub pivot: V3, pub axis: V3, pub rest: V3, pub reach: f32 }

impl Wiper {
    /// The largest sweep (radians, at most `WIPER_SWEEP`) at which the whole blade stays on the glass: the arm is as long as the glass is high, and swung too far it would
    /// point out over the pillar. Scans the blade's length at each angle against the glass's outline, 2 cm in from its edge.
    pub fn max_sweep(&self, screen: &super::glassdrops::Windscreen) -> f32 {
        let axis = self.axis;
        let side = axis.cross(self.rest);
        let mut th = WIPER_SWEEP;
        while th > 0.2 {
            let (c, s) = (th.cos(), th.sin());
            let dir = self.rest * c + side * s;
            let on_glass = (5..=10).all(|k| {   // the outer half: the arm's root sits at the glass's lower edge, off it
                let d = self.reach * 0.1 * k as f32 * 0.93;
                let p = self.pivot + dir * d;
                let rel = [p.x - screen.origin[0], p.y - screen.origin[1], p.z - screen.origin[2]];
                screen.inside(rel[0] * screen.u[0] + rel[1] * screen.u[1] + rel[2] * screen.u[2], rel[0] * screen.v[0] + rel[1] * screen.v[1] + rel[2] * screen.v[2], 0.0)
            });
            if on_glass { return th; }
            th -= 0.02;
        }
        0.2
    }
    /// Builds a wiper from its mesh (already shifted so the spindle is the origin), finding the rest direction and the reach from the farthest vertex.
    pub fn new(mesh: Mesh3, pivot: V3, axis: V3) -> Wiper {
        let far = mesh.verts.iter().map(|v| v3(v.pos[0], v.pos[1], v.pos[2])).max_by(|a, b| a.len().total_cmp(&b.len())).unwrap_or(v3(1.0, 0.0, 0.0));
        let flat = far - axis * far.dot(axis);
        let rest = if flat.len() > 1e-4 { flat.norm() } else { v3(1.0, 0.0, 0.0) };
        Wiper { mesh, pivot, axis, rest, reach: far.len() + 0.03 }
    }
}

/// The engine's revs (0..1) a car at `speed` m/s with the top speed `max_speed` shows on its RPM LEDs (the same curve as the engine sound at a steady throttle).
pub fn dash_rpm(speed: f32, max_speed: f32) -> f32 {
    if !(speed.is_finite() && max_speed.is_finite()) { return 0.0; }
    let frac = (speed.abs() / max_speed.max(1.0)).clamp(0.0, 1.0);
    (0.18 + 0.6 * frac.powf(0.7)).min(1.0)
}

/// Whether the driver is drawn for a camera `dist` metres from the car's centre: not when the camera sits inside the car (the head would block the view).
pub fn draw_driver(dist: f32) -> bool { dist >= 1.2 }

impl CarModel {
    pub fn new(body: Mesh3, wheel: Mesh3) -> Self { Self { body, wheel, disc: None, caliper: None, alt_wheels: [None, None], glass: None, interior: None, steering: None, driver: None, wipers: [None, None], gloves: None, windscreen: None, wiper_sweep: WIPER_SWEEP } }
    /// The wheel mesh of a wheel set (0 tarmac, 1 gravel, 2 snow); a set the model does not have shows the tarmac wheel.
    pub fn wheel_for(&self, set: u8) -> &Mesh3 {
        match set { 1 | 2 => self.alt_wheels[set as usize - 1].as_ref().unwrap_or(&self.wheel), _ => &self.wheel }
    }
}

pub(crate) const CHROME: [f32; 3] = [0.75, 0.77, 0.80];
pub(crate) const RUBBER: [f32; 3] = [0.04, 0.04, 0.045];

/// The generated body for the car's style (the wheel is shared in shape, the rally one has a lighter rim).
pub fn build_car(spec: &VehicleSpec) -> CarModel {
    match spec.style {
        crate::game::vehicle::CarStyle::Suv => build_suv(spec),
        crate::game::vehicle::CarStyle::Rally => super::rally_model::build_rally(spec),
    }
}

/// A tyre with a rim: `rim` is the rim colour, `bolts` the number of rim bolts.
pub(crate) fn build_wheel(spec: &VehicleSpec, rim: [f32; 3], bolts: usize) -> Mesh3 {
    let r = spec.radius; let w = spec.look.wheel_width;
    let mut wheel = Mesh3::default();
    add_cylinder(&mut wheel, V3::ZERO, Axis::X, r - 0.03, w, 20, RUBBER, MAT_RUBBER);
    for k in 0..20 {
        let a = std::f32::consts::TAU * k as f32 / 20.0;
        let rot = Q::axis_angle(v3(1., 0., 0.), a);
        add_box(&mut wheel, rot.rotate(v3(0.0, 0.0, r - 0.03)), v3(w * 0.5, 0.03, r * 0.07), rot, RUBBER, MAT_RUBBER);
    }
    add_cylinder(&mut wheel, V3::ZERO, Axis::X, r * 0.62, w * 1.02, 20, rim, MAT_CHROME);
    for k in 0..bolts {
        let a = std::f32::consts::TAU * k as f32 / bolts as f32;
        let rot = Q::axis_angle(v3(1., 0., 0.), a);
        add_box(&mut wheel, rot.rotate(v3(w * 0.52, 0.0, r * 0.3)), v3(0.015, 0.025, 0.025), rot, [0.3, 0.3, 0.32], MAT_CHROME);
    }
    wheel
}

fn build_suv(spec: &VehicleSpec) -> CarModel {
    let (sx, sy, sz) = (spec.size.x / 2.0, spec.size.y / 1.4, spec.size.z / 4.2);
    let l = spec.look;
    let mut body = Mesh3::default();
    body.append(&loft(&scaled(&[(-2.0, 0.80, 0.40, 0.95), (-1.8, 0.96, 0.30, 1.00), (-0.5, 1.00, 0.30, 1.05), (0.9, 1.00, 0.30, 1.00), (1.7, 0.96, 0.30, 0.88), (2.0, 0.80, 0.42, 0.78)], sx, sy, sz), 16, 4.0, l.body, MAT_PAINT));
    body.append(&loft(&scaled(&[(-0.9, 0.80, 0.98, 1.56), (-0.4, 0.84, 0.98, 1.58), (0.5, 0.84, 0.98, 1.50), (0.95, 0.76, 0.98, 1.30)], sx, sy, sz), 12, 3.0, l.glass, MAT_GLASS));
    add_box(&mut body, v3(0.0, 1.60 * sy, -0.1 * sz), v3(0.78 * sx, 0.03 * sy, 0.95 * sz), Q::IDENT, l.body, MAT_PAINT);
    for z in [2.04, -2.04] { add_box(&mut body, v3(0.0, 0.52 * sy, z * sz), v3(0.92 * sx, 0.15 * sy, 0.07 * sz), Q::IDENT, l.accent, MAT_PLASTIC); }
    add_box(&mut body, v3(0.0, 0.70 * sy, 2.0 * sz), v3(0.38 * sx, 0.12 * sy, 0.04 * sz), Q::IDENT, CHROME, MAT_CHROME);
    for x in [0.62, -0.62] {
        add_box(&mut body, v3(x * sx, 0.72 * sy, 2.0 * sz), v3(0.14 * sx, 0.08 * sy, 0.04 * sz), Q::IDENT, [1.0, 0.95, 0.8], MAT_EMISSIVE);
        add_box(&mut body, v3(x * sx, 0.78 * sy, -2.0 * sz), v3(0.10 * sx, 0.07 * sy, 0.04 * sz), Q::IDENT, [0.9, 0.05, 0.03], MAT_EMISSIVE);
    }
    for (x, z) in [(0.7, 0.9), (-0.7, 0.9), (0.7, -0.7), (-0.7, -0.7)] { add_box(&mut body, v3(x * sx, 1.67 * sy, z * sz), v3(0.03 * sx, 0.03 * sy, 0.03 * sz), Q::IDENT, CHROME, MAT_CHROME); }
    for x in [0.7, -0.7] { add_box(&mut body, v3(x * sx, 1.70 * sy, 0.1 * sz), v3(0.03 * sx, 0.02 * sy, 0.85 * sz), Q::IDENT, CHROME, MAT_CHROME); }
    add_cylinder(&mut body, v3(0.0, 0.85 * sy, -2.1 * sz), Axis::Z, 0.34 * sy, 0.2 * sz, 16, RUBBER, MAT_RUBBER);
    add_cylinder(&mut body, v3(0.0, 0.85 * sy, -2.21 * sz), Axis::Z, 0.17 * sy, 0.03 * sz, 12, CHROME, MAT_CHROME);

    let wheel = build_wheel(spec, CHROME, 5);
    CarModel::new(body, wheel)
}

/// The wheels' poses; `spin` is how far the front wheels have turned, `rear_spin` the rear ones (they stand still while the handbrake locks them).
pub fn car_wheel_poses(v: &Vehicle, spin: f32, rear_spin: f32) -> [(Q, V3); 4] {
    let s = &v.spec;
    let mut out = [(Q::IDENT, V3::ZERO); 4];
    for w in 0..4 {
        let local = s.wheels[w] - v3(0.0, s.rest - v.comp[w], 0.0);
        let steer = if w < 2 { Q::axis_angle(v3(0., 1., 0.), -v.steer) } else { Q::IDENT };
        // the rally wheel mesh has its rim face on the +x side: wheels on the -x side turn the mesh around (after the spin)
        let flip = if s.style == crate::game::vehicle::CarStyle::Rally && s.wheels[w].x < 0.0 { Q::axis_angle(v3(0., 1., 0.), std::f32::consts::PI) } else { Q::IDENT };
        out[w] = (v.rot.mul(steer).mul(Q::axis_angle(v3(1., 0., 0.), if w < 2 { spin } else { rear_spin })).mul(flip), v.pos + v.rot.rotate(local));
    }
    out
}

/// How far each hub has moved up from its static place, in the body mesh's corner order (0 front-left (+x), 1 front-right, 2 rear-left, 3 rear-right), metres, clamped to +-0.25:
/// the wheel's compression minus the static compression the body mesh sits at. The shader moves the suspension parts of a corner by it (`MAT_CARSUSP`).
pub fn susp_offsets(v: &Vehicle) -> [f32; 4] {
    let s = &v.spec;
    let c0 = (s.mass * 9.81 / 4.0 / s.spring).min(s.rest);
    let mut out = [0.0f32; 4];
    for w in 0..4 {
        let corner = (if s.wheels[w].z > 0.0 { 0 } else { 2 }) + (if s.wheels[w].x > 0.0 { 0 } else { 1 });
        let d = v.comp[w] - c0;
        out[corner] = if d.is_finite() { d.clamp(-0.25, 0.25) } else { 0.0 };
    }
    out
}

/// The vertices and indices the car's signed distance field (the dust wraps the car) is built from: the body and the glass (without the glass the window holes would let the dust into the cabin).
pub fn sdf_input(car: &CarModel) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut verts: Vec<[f32; 3]> = car.body.verts.iter().map(|v| v.pos).collect();
    let mut idx = car.body.idx.clone();
    if let Some(g) = &car.glass { let base = verts.len() as u32; verts.extend(g.verts.iter().map(|v| v.pos)); idx.extend(g.idx.iter().map(|i| i + base)); }
    (verts, idx)
}

/// The brake calipers: the wheels' poses with no spin (a caliper follows the steering and the suspension, not the rolling).
pub fn car_caliper_poses(v: &Vehicle) -> [(Q, V3); 4] { car_wheel_poses(v, 0.0, 0.0) }

/// The body mesh is drawn 0.75 body-heights below the suspension attachment height (its own frame starts at y = 0.30)
/// so the sills clear the ground by about half a metre; the attachment height itself sits above the centre of mass.
pub fn car_body_pose(v: &Vehicle) -> (Q, V3) {
    if v.spec.style == crate::game::vehicle::CarStyle::Rally {
        // the rally body frame starts at the ground: the mounts' height minus the ride height at rest (spring static compression)
        let s = &v.spec;
        let c0 = (s.mass * 9.81 / 4.0 / s.spring).min(s.rest);
        return (v.rot, v.pos + v.rot.rotate(v3(0.0, s.wheels[0].y - (s.rest + s.radius - c0), 0.0)));
    }
    let sy = v.spec.size.y / 1.4;
    (v.rot, v.pos + v.rot.rotate(v3(0.0, v.spec.wheels[0].y - 0.75 * sy, 0.0)))
}

#[cfg(test)]
#[path = "vehicle_models_tests.rs"]
mod tests;
