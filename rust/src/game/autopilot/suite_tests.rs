//! The AI suite (roadmap-ai-2026-10.md, stage A0): the autopilot's whole drives over cars x conditions x levels, one table. Slow; run with
//! `AP_STYLE=1 ~/bin/lim cargo test --release --lib ai_suite -- --ignored --nocapture`.
use crate::game::{input::Input, vehicle::VehicleSpec, world::World};

#[derive(Clone, Copy, Debug)]
pub enum Cond { Dry, Wet, Worn, Cold, Damaged }
impl Cond {
    pub const ALL: [Cond; 5] = [Cond::Dry, Cond::Wet, Cond::Worn, Cond::Cold, Cond::Damaged];
    pub fn name(self) -> &'static str { match self { Cond::Dry => "dry", Cond::Wet => "wet", Cond::Worn => "worn tyres", Cond::Cold => "cold tyres", Cond::Damaged => "damaged" } }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Cell { pub finished: bool, pub time: f32, pub edge: f32, pub flips: u32, pub damage: f32 }

/// One whole drive of the stage of level `seed` in a condition.
pub fn drive(car_id: &str, cond: Cond, seed: u64, style: i32) -> Cell {
    let car = VehicleSpec::by_id(car_id).expect("a car");
    let mut w = World::with_roads(seed, car);
    if !w.place_on_road(2) { return Cell::default(); }
    match cond {
        Cond::Dry => {}
        Cond::Wet => { w.climate = crate::game::weather::Climate::with_mode(seed, 3); }
        Cond::Worn => { for t in w.vehicle.tyres.iter_mut() { t.wear = 0.7; } }
        Cond::Cold => { for t in w.vehicle.tyres.iter_mut() { t.temp = 5.0; } w.vehicle.air_c = 3.0; }
        Cond::Damaged => { w.vehicle.damage.body = 0.3; w.vehicle.damage.engine = 0.3; w.vehicle.damage.susp = [0.2; 4]; }
    }
    if !w.set_autopilot_style(style) { return Cell::default(); }
    let (mut t, mut worst, mut flips, mut was) = (0.0f32, f32::MIN, 0u32, false);
    while w.autopilot_on() && t < 700.0 {
        w.step(&Input::default(), 1.0 / 60.0); t += 1.0 / 60.0;
        if t > 8.0 { if let Some(e) = w.terrain.road_edge_distance(w.vehicle.pos.x, w.vehicle.pos.z) { worst = worst.max(e); } }
        let f = w.vehicle.flipped(); if f && !was { flips += 1; } was = f;
    }
    Cell { finished: !w.autopilot_on(), time: t, edge: worst.max(-5.0), flips, damage: w.vehicle.damage.total() }
}

/// The table of a style: for every car and condition the mean time over `seeds`, the worst excursion, the flips and the damage taken.
pub fn table(cars: &[&str], seeds: &[u64], style: i32, par: usize) -> String {
    let mut out = format!("style {style}\n{:<10} {:<11} {:>8} {:>8} {:>6} {:>8}\n", "car", "condition", "time s", "edge m", "flips", "damage %");
    for cond in Cond::ALL {
        // the weather is a switch of the whole process: all the wet drives together, the others with a clear sky
        crate::game::weather::set_mode(if matches!(cond, Cond::Wet) { 3 } else { 1 });
        for car in cars {
            let jobs: Vec<(String, Cond, u64)> = seeds.iter().map(|&s| (car.to_string(), cond, s)).collect();
            let mut cells: Vec<Cell> = Vec::new();
            for chunk in jobs.chunks(par.max(1)) {
                let hs: Vec<_> = chunk.iter().cloned().map(|(c, k, s)| std::thread::spawn(move || drive(&c, k, s, style))).collect();
                cells.extend(hs.into_iter().map(|h| h.join().unwrap_or_default()));
            }
            let n = cells.len().max(1) as f32;
            let ok = cells.iter().filter(|c| c.finished).count();
            let time = cells.iter().map(|c| if c.finished { c.time } else { 700.0 }).sum::<f32>() / n;
            out += &format!("{:<10} {:<11} {:>8.0} {:>8.1} {:>6} {:>8.0}{}\n", car, cond.name(), time, cells.iter().map(|c| c.edge).fold(f32::MIN, f32::max), cells.iter().map(|c| c.flips).sum::<u32>(), 100.0 * cells.iter().map(|c| c.damage).sum::<f32>() / n, if ok < cells.len() { format!("  ({} of {} finished)", ok, cells.len()) } else { String::new() });
        }
    }
    crate::game::weather::set_mode(0);
    out
}

#[test] #[ignore] fn ai_suite() {
    let style: i32 = std::env::var("AP_STYLE").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let par: usize = std::env::var("AP_PAR").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
    let seeds: Vec<u64> = std::env::var("AP_SEEDS").map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect()).unwrap_or_else(|_| vec![1, 2, 3, 21]);
    let cars: Vec<String> = std::env::var("AP_CARS").map(|s| s.split(',').map(|x| x.to_string()).collect()).unwrap_or_else(|_| vec!["vulpes".into(), "offroader".into()]);
    let refs: Vec<&str> = cars.iter().map(|s| s.as_str()).collect();
    println!("{}", table(&refs, &seeds, style, par).replace("\n", "\n"));
}
