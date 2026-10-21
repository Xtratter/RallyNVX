//! The automatic gearbox's mind: when to change gear, as pure functions of the situation and a small memory of the driver, so the rules can be tested without a car.
//! A real automatic stays in the highest gear that pulls, climbs later the harder the pedal was pressed, drops one or several gears when the pedal is floored (kick-down),
//! steps down under braking for the engine brake, holds its gear in a corner and does not hunt (a pause between two shifts).
use super::drivetrain::{engine_torque, ratio, rpm_in_gear};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Profile { Comfort, #[default] Sport, Race }

impl Profile {
    /// 0 comfort, 1 sport, 2 race; anything else is sport.
    pub fn from_i32(v: i32) -> Profile { match v { 0 => Profile::Comfort, 2 => Profile::Race, _ => Profile::Sport } }
}

/// The numbers of one profile: the up-shift revs at a light touch and at the floor, the down floor likewise, the pause (s) before an up-shift and a down-shift,
/// the revs under which a floored pedal kicks down, the most revs a shift may land on, the revs the engine brake aims at and the time a shift takes.
#[derive(Clone, Copy, Debug)]
pub struct Table { pub up_lo: f32, pub up_hi: f32, pub floor_lo: f32, pub floor_hi: f32, pub dwell_up: f32, pub dwell_down: f32, pub kick_rpm: f32, pub over_rev: f32, pub brake_rpm: f32, pub shift_time: f32 }

pub fn table(p: Profile) -> Table {
    match p {
        Profile::Comfort => Table { up_lo: 2600.0, up_hi: 6000.0, floor_lo: 1400.0, floor_hi: 2400.0, dwell_up: 1.2, dwell_down: 1.5, kick_rpm: 2800.0, over_rev: 6200.0, brake_rpm: 3200.0, shift_time: 0.30 },
        Profile::Sport => Table { up_lo: 3800.0, up_hi: 7300.0, floor_lo: 1900.0, floor_hi: 3600.0, dwell_up: 0.9, dwell_down: 1.2, kick_rpm: 3300.0, over_rev: 6600.0, brake_rpm: 4300.0, shift_time: 0.18 },
        Profile::Race => Table { up_lo: 5200.0, up_hi: 7500.0, floor_lo: 3500.0, floor_hi: 5200.0, dwell_up: 0.5, dwell_down: 0.6, kick_rpm: 4200.0, over_rev: 7000.0, brake_rpm: 5200.0, shift_time: 0.10 },
    }
}

/// What the box remembers of the driver: the pedal (a press counts at once, a lift fades over about a second), the aggressiveness (a stab, the brake, a corner raise it, it fades over about 8 s)
/// and the seconds since its last shift.
#[derive(Clone, Copy, Debug)]
pub struct Memory { pub load: f32, pub aggr: f32, prev_gas: f32, pub since_shift: f32 }
impl Default for Memory { fn default() -> Memory { Memory { load: 0.0, aggr: 0.0, prev_gas: 0.0, since_shift: 9.0 } } }

impl Memory {
    /// One step; returns the pedal's rate of change (1/s).
    pub fn update(&mut self, dt: f32, gas: f32, brake: bool, lat_g: f32) -> f32 {
        let dt = dt.clamp(1e-4, 0.1);
        let gas = if gas.is_finite() { gas.clamp(0.0, 1.0) } else { 0.0 };
        let rate = (gas - self.prev_gas) / dt;
        self.prev_gas = gas;
        self.load = if gas > self.load { gas } else { self.load + (gas - self.load) * (dt / 1.2).min(1.0) };
        let lat = if lat_g.is_finite() { (lat_g.abs() / 0.7).clamp(0.0, 1.0) * 0.7 } else { 0.0 };
        let target = (0.5 * gas + 0.5 * (rate / 4.0).clamp(0.0, 1.0)).max(if brake { 0.5 } else { 0.0 }).max(lat);
        self.aggr = if target > self.aggr { target } else { self.aggr + (target - self.aggr) * (dt / 8.0).min(1.0) };
        self.since_shift += dt;
        rate
    }
    pub fn shifted(&mut self) { self.since_shift = 0.0; }
}

/// What the box sees.
#[derive(Clone, Copy, Debug)]
pub struct Situation {
    pub gear: i32, pub rpm: f32,
    /// The pedal (0..1) and its rate (1/s).
    pub gas: f32, pub rate: f32,
    pub brake: bool,
    /// The car's speed (m/s, along the heading), the wheels' radius (m) and the final drive multiplier of the tuning.
    pub speed: f32, pub wheel_r: f32, pub gearing: f32,
    /// The lateral acceleration (g) and the grade (the sine of the pitch, positive uphill).
    pub lat_g: f32, pub slope: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice { Hold, Up(i32), Down(i32) }

/// The gear the box wants now (never a gear outside 1..=6; garbage in holds the gear).
pub fn choose(p: Profile, m: &Memory, s: &Situation) -> Choice {
    let finite = s.rpm.is_finite() && s.speed.is_finite() && s.wheel_r.is_finite() && s.gearing.is_finite() && s.gas.is_finite();
    if !finite || s.gear < 1 || s.gear > 6 || s.rpm < 0.0 || s.wheel_r <= 0.0 { return Choice::Hold; }
    let (rate, lat_g, slope) = (if s.rate.is_finite() { s.rate } else { 0.0 }, if s.lat_g.is_finite() { s.lat_g } else { 0.0 }, if s.slope.is_finite() { s.slope } else { 0.0 });
    let t = table(p);
    let at = |g: i32| rpm_in_gear(s.speed, s.wheel_r, g) * s.gearing;   // the revs the wheels would give the engine in gear g
    let eff = m.load.max(0.7 * m.aggr).clamp(0.0, 1.0);
    let floor = t.floor_lo + (t.floor_hi - t.floor_lo) * eff + (slope.max(0.0) * 3000.0).min(900.0);
    let up_at = t.up_lo + (t.up_hi - t.up_lo) * eff;
    let moving = s.speed > 4.0;
    let braking = s.brake && moving && s.gas < 0.1;
    let lifted = s.gas < 0.05 && moving;
    let g = s.gear;
    // a floored or stabbed pedal at low revs asks for the gear with the most force at the wheels (it may skip gears), without the pause of an ordinary shift
    let stab = s.gas > 0.85 || (rate > 3.0 && s.gas > 0.5);
    if stab && g > 1 && s.rpm < t.kick_rpm && m.since_shift >= 0.35 {
        let force = |h: i32| engine_torque(at(h), 1.0) * ratio(h);
        let cur = force(g);
        let best = (1..g).filter(|&h| at(h) < t.over_rev).max_by(|&a, &b| force(a).partial_cmp(&force(b)).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(b) = best { if force(b) > cur * 1.12 { return Choice::Down(b); } }
    }
    // braking at speed: down to the gear whose revs are nearest to the engine brake's band
    if braking && g > 1 && s.rpm < t.brake_rpm - 400.0 && m.since_shift >= t.dwell_down {
        let target = (1..g).filter(|&h| at(h) <= t.over_rev - 600.0).min_by(|&a, &b| (at(a) - t.brake_rpm).abs().partial_cmp(&(at(b) - t.brake_rpm).abs()).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(h) = target { return Choice::Down(h); }
    }
    // up: only with the pedal down, not braking, not cornering, not coasting downhill, and only if the next gear still has the revs to pull
    let downhill_coast = slope < -0.04 && s.gas < 0.3;
    if g < 6 && !braking && !lifted && lat_g.abs() <= 0.35 && !downhill_coast && s.rpm > up_at && at(g + 1) > floor + 500.0 && m.since_shift >= t.dwell_up { return Choice::Up(g + 1); }
    // down: under the floor (lugging ignores the pause)
    if g > 1 && s.rpm < floor && at(g - 1) < t.over_rev && (m.since_shift >= t.dwell_down || s.rpm < 1300.0) { return Choice::Down(g - 1); }
    Choice::Hold
}

#[cfg(test)]
#[path = "shiftlogic_tests.rs"]
mod tests;
