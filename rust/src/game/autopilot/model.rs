//! What the car can really do, measured by the car itself: a full-throttle run and a full-brake stop on a flat ground of each surface, in a copy of the
//! world with the same car, tuning, aids and gearbox. The speed plan of the route uses these tables (the engine's pull and the brake's bite at every
//! speed, with the gearbox, the traction control, ABS, the weight and the drag in them) instead of a guess from the spec.
use crate::game::input::Input;
use crate::game::surface::Surface;
use crate::game::world::World;

/// The table step (m/s) and its length: speeds 0..=(N-1)*DV.
pub const DV: f32 = 2.0;
pub const N: usize = 26;

#[derive(Clone, Debug)]
pub struct CarModel {
    /// The acceleration (m/s^2) the engine gives at the speed `i * DV` straight on, and the deceleration the brakes give.
    pub acc: [f32; N], pub brk: [f32; N],
    /// The largest lateral acceleration (m/s^2) the car holds at the speed `i * DV` (a steering ramp at a held speed: the tyres, the downforce and the steering lock are all in it).
    pub lat: [f32; N],
    /// The speed the full-throttle run ended at (the top speed on this ground).
    pub top: f32,
}

fn at(tab: &[f32; N], v: f32) -> f32 {
    let x = (v.max(0.0) / DV).min((N - 1) as f32); let i = x.floor() as usize; let j = (i + 1).min(N - 1);
    tab[i] + (tab[j] - tab[i]) * (x - i as f32)
}

impl CarModel {
    pub fn accel(&self, v: f32) -> f32 { at(&self.acc, v) }
    pub fn brake(&self, v: f32) -> f32 { at(&self.brk, v) }

    /// The old guess from the spec: a flat pull of half the engine force, a brake of the grip.
    pub fn nominal(accel: f32, mu: f32, top: f32) -> CarModel { CarModel { acc: [accel; N], brk: [mu * 9.81; N], lat: [mu * 9.81; N], top } }
    pub fn lateral(&self, v: f32) -> f32 { at(&self.lat, v) }

    /// Runs the two tests on `surface` in a copy of `w`'s setup.
    pub fn measure(w: &World, surface: Surface) -> CarModel {
        let mut p = w.probe(surface);
        let dt = 1.0 / 60.0;
        let (mut sum, mut cnt) = ([0.0f32; N], [0u32; N]);
        let mut prev = (0.0f32, 0.0f32); let mut t = 0.0f32; let mut best = 0.0f32; let mut still = 0.0f32;
        // the pull: full throttle from a standstill, the acceleration averaged over a quarter second
        while t < 40.0 {
            p.step(&Input { throttle: 1.0, ..Default::default() }, dt); t += dt;
            let v = p.vehicle.speed();
            if t - prev.0 >= 0.25 { let a = (v - prev.1) / (t - prev.0); let m = 0.5 * (v + prev.1); let k = ((m / DV).round() as usize).min(N - 1); if t > 1.0 || v > 1.0 { sum[k] += a; cnt[k] += 1; } prev = (t, v); }
            if v > best + 0.02 { best = v; still = 0.0 } else { still += dt; if still > 3.0 { break; } }
            if v > (N - 1) as f32 * DV - 0.5 { break; }
        }
        let top = p.vehicle.speed().max(best * 0.98);
        let mut acc = [0.0f32; N];
        for k in 0..N { acc[k] = if cnt[k] > 0 { sum[k] / cnt[k] as f32 } else { f32::NAN }; }
        fill(&mut acc, 0.3);
        // the bite: full brake from the speed reached, the deceleration averaged the same way
        let (mut sum, mut cnt) = ([0.0f32; N], [0u32; N]);
        let mut prev = (0.0f32, p.vehicle.speed()); let mut t = 0.0f32;
        while t < 20.0 && p.vehicle.speed() > 0.5 {
            p.step(&Input { brake: true, ..Default::default() }, dt); t += dt;
            let v = p.vehicle.speed();
            if t - prev.0 >= 0.25 { let a = (prev.1 - v) / (t - prev.0); let m = 0.5 * (v + prev.1); let k = ((m / DV).round() as usize).min(N - 1); sum[k] += a; cnt[k] += 1; prev = (t, v); }
        }
        let mut brk = [0.0f32; N];
        for k in 0..N { brk[k] = if cnt[k] > 0 { sum[k] / cnt[k] as f32 } else { f32::NAN }; }
        fill(&mut brk, 1.0);
        let lat = measure_lat(w, surface, top);
        CarModel { acc, brk, lat, top }
    }
}

/// The cornering grip at several speeds: at each a steering ramp from straight to full lock over 6 s with the speed held, the largest lateral acceleration (yaw rate x speed, smoothed over 0.4 s) kept.
fn measure_lat(w: &World, surface: Surface, top: f32) -> [f32; N] {
    const SPEEDS: [f32; 8] = [6.0, 10.0, 14.0, 18.0, 22.0, 26.0, 30.0, 34.0];
    let dt = 1.0 / 60.0;
    let mut pts: Vec<(f32, f32)> = Vec::new();
    for &vt in SPEEDS.iter().filter(|&&v| v < top * 0.92) {
        let mut p = w.probe(surface);
        let f = p.vehicle.forward(); p.vehicle.vel = f * vt;
        let (mut t, mut peak, mut avg) = (0.0f32, 0.0f32, 0.0f32);
        while t < 8.0 {
            let v = p.vehicle.speed();
            let throttle = (0.45 + 0.3 * (vt - v)).clamp(0.0, 1.0); let brake = v > vt + 2.0;
            let steer = if t < 1.5 { 0.0 } else { ((t - 1.5) / 6.0).min(1.0) };
            p.step(&Input { throttle, steer, brake, ..Default::default() }, dt); t += dt;
            let a = (p.vehicle.omega.dot(p.vehicle.up()) * p.vehicle.speed()).abs();
            avg += (a - avg) * (dt / 0.4).min(1.0);
            if t > 2.0 { peak = peak.max(avg); }
        }
        pts.push((vt, peak));
    }
    let mut lat = [f32::NAN; N];
    for (k, x) in lat.iter_mut().enumerate() {
        let v = k as f32 * DV;
        let lo = pts.iter().rev().find(|p| p.0 <= v); let hi = pts.iter().find(|p| p.0 >= v);
        *x = match (lo, hi) { (Some(a), Some(b)) if b.0 > a.0 => a.1 + (b.1 - a.1) * (v - a.0) / (b.0 - a.0), (Some(a), _) => a.1, (None, Some(b)) => b.1, _ => f32::NAN };
    }
    fill(&mut lat, 2.0);
    lat
}

/// Fills the unmeasured (NaN) entries from their neighbours, and raises every entry to at least `floor`.
fn fill(t: &mut [f32; N], floor: f32) {
    let known: Vec<usize> = (0..N).filter(|&i| t[i].is_finite()).collect();
    if known.is_empty() { for x in t.iter_mut() { *x = floor; } return; }
    for i in 0..N {
        if t[i].is_finite() { continue; }
        let lo = known.iter().rev().find(|&&k| k < i); let hi = known.iter().find(|&&k| k > i);
        t[i] = match (lo, hi) { (Some(&a), Some(&b)) => t[a] + (t[b] - t[a]) * (i - a) as f32 / (b - a) as f32, (Some(&a), None) => t[a], (None, Some(&b)) => t[b], _ => floor };
    }
    for x in t.iter_mut() { *x = x.max(floor); }
}

/// The measured models of the surfaces the route runs on, each with the grip `mu` (car x ground) it was measured at; a route point takes the nearest.
#[derive(Clone, Debug)]
pub struct Models { pub list: Vec<(f32, CarModel)>, /** the steering lock (rad), the speed it halves at (m/s) and the wheelbase (m): the tightest bend the car can turn at a given speed */ pub steer: (f32, f32, f32) }

impl Models {
    /// Measures the car on asphalt and on gravel (the stage's worst case) in a copy of `w`'s setup.
    pub fn measure(w: &World) -> Models {
        let key = |s: Surface| w.car_spec.mu * crate::game::grip::grip(s, 1.0).mu;
        let sp = &w.car_spec;
        Models { list: [Surface::Asphalt, Surface::Gravel].iter().map(|&s| (key(s), CarModel::measure(w, s))).collect(), steer: (sp.steer_max, sp.steer_speed_ref, (sp.wheels[0].z - sp.wheels[2].z).abs().max(1.0)) }
    }
    /// The speed (m/s) up to which the steering can follow a bend of curvature `kappa` (the lock shrinks with speed): the largest `v` with `tan(lock / (1 + v / ref)) / wheelbase >= kappa`.
    pub fn steer_speed(&self, kappa: f32, v_max: f32) -> f32 {
        let (lock, rf, wb) = self.steer;
        let reach = |v: f32| (lock / (1.0 + v / rf)).tan() / wb;
        if kappa <= 1e-5 || reach(v_max) >= kappa { return v_max; }
        if reach(0.0) < kappa { return 3.0; }
        let (mut lo, mut hi) = (0.0f32, v_max);
        for _ in 0..20 { let mid = 0.5 * (lo + hi); if reach(mid) >= kappa { lo = mid } else { hi = mid } }
        lo.max(3.0)
    }

    /// The nominal grip (car x ground) the model nearest to `mu` was measured at.
    pub fn key_at(&self, mu: f32) -> f32 {
        self.list.iter().min_by(|a, b| (a.0 - mu).abs().partial_cmp(&(b.0 - mu).abs()).unwrap_or(std::cmp::Ordering::Equal)).map_or(mu, |x| x.0)
    }
    pub fn at(&self, mu: f32) -> &CarModel {
        &self.list.iter().min_by(|a, b| (a.0 - mu).abs().partial_cmp(&(b.0 - mu).abs()).unwrap_or(std::cmp::Ordering::Equal)).expect("a model").1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::vehicle::VehicleSpec;
    #[test] fn the_table_interpolates_and_clamps() {
        let m = CarModel::nominal(4.0, 0.7, 50.0);
        assert!((m.accel(7.3) - 4.0).abs() < 1e-5 && (m.accel(1e6) - 4.0).abs() < 1e-5 && (m.brake(-3.0) - 6.867).abs() < 1e-3);
    }
    #[test] fn the_gaps_are_filled_and_the_floor_holds() {
        let mut t = [f32::NAN; N]; t[2] = 4.0; t[6] = 8.0; t[10] = -3.0; fill(&mut t, 0.3);
        assert!((t[4] - 6.0).abs() < 1e-5 && t[0] == 4.0 && t[10] == 0.3 && t[N - 1] == 0.3 && t.iter().all(|x| x.is_finite() && *x >= 0.3));
    }
    #[test] fn the_cornering_grip_is_measured_and_follows_the_state_of_the_car() {
        let car = VehicleSpec::by_id("vulpes").unwrap();
        let mk = || World::from_terrain(crate::game::terrain::Terrain::flat(), car, (0.0, 0.0), 0.0);
        let clean = CarModel::measure(&mk(), Surface::Asphalt);
        assert!(clean.lateral(20.0) > 0.4 * 9.81 && clean.lateral(20.0) < 2.0 * 9.81, "a plausible cornering grip: {} m/s2", clean.lateral(20.0));
        let gravel = CarModel::measure(&mk(), Surface::Gravel);
        assert!(gravel.lateral(20.0) < clean.lateral(20.0), "gravel grips less than asphalt");
        let mut worn = mk(); for t in worn.vehicle.tyres.iter_mut() { t.wear = 0.8; }
        assert!(CarModel::measure(&worn, Surface::Asphalt).lateral(20.0) < clean.lateral(20.0) * 0.97, "worn tyres measure less grip");
        let mut cold = mk(); for t in cold.vehicle.tyres.iter_mut() { t.temp = 0.0; }
        assert!(CarModel::measure(&cold, Surface::Asphalt).lateral(20.0) < clean.lateral(20.0) * 0.97, "cold tyres measure less grip");
        let mut wet = mk(); wet.vehicle.wet = 1.0; wet.climate = crate::game::weather::Climate::with_mode(1, 4);   // heavy rain: the world sets the car's wetness from its climate
        assert!(CarModel::measure(&wet, Surface::Asphalt).lateral(20.0) < clean.lateral(20.0) * 0.97, "a wet road measures less grip");
    }
    #[test] #[ignore] fn measure_the_cornering_of_the_cars() {
        for id in ["offroader", "vulpes"] {
            let w = World::from_terrain(crate::game::terrain::Terrain::flat(), VehicleSpec::by_id(id).unwrap(), (0.0, 0.0), 0.0);
            for s in [Surface::Asphalt, Surface::Gravel] {
                let t0 = std::time::Instant::now(); let m = CarModel::measure(&w, s);
                let key = w.car_spec.mu * crate::game::grip::grip(s, 1.0).mu;
                println!("{id} {s:?} ({} ms) nominal mu {key:.2}\n  lat g {:?}\n  ratio {:?}", t0.elapsed().as_millis(), (0..N).step_by(3).map(|i| (m.lat[i] / 9.81 * 100.0).round() / 100.0).collect::<Vec<_>>(), (0..N).step_by(3).map(|i| (m.lat[i] / (key * 9.81) * 100.0).round() / 100.0).collect::<Vec<_>>());
            }
        }
    }
    #[test] #[ignore] fn measure_the_cars() {
        for id in ["offroader", "vulpes"] {
            let w = World::from_terrain(crate::game::terrain::Terrain::flat(), VehicleSpec::by_id(id).unwrap(), (0.0, 0.0), 0.0);
            for s in [Surface::Asphalt, Surface::Gravel] {
                let t0 = std::time::Instant::now(); let m = CarModel::measure(&w, s);
                println!("{id} {s:?} top {:.1} m/s ({} ms)\n  acc {:?}\n  brk {:?}", m.top, t0.elapsed().as_millis(), m.acc.iter().map(|x| (x * 10.0).round() / 10.0).collect::<Vec<_>>(), m.brk.iter().map(|x| (x * 10.0).round() / 10.0).collect::<Vec<_>>());
            }
        }
    }
}
