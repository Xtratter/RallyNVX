//! Stateless helpers for the Android menu (no GPU, no game thread).
use std::path::Path;

use super::meta::Profile;
use super::garage::Upgrade;
use super::vehicle::VehicleSpec;

/// While the rally career does not exist yet (nothing earns money), buying cars and upgrades is free: the garage shows
/// price 0 and purchases leave the balance alone. Flip this back when the rally mode pays prize money.
pub const FREE_GARAGE: bool = true;

/// `[money, selected_index, car_count]` then per car (file order, sorted by id) `CAR_FIELDS` numbers: `[owned(0/1), price, upgrade_cost,
/// engine, tyres, suspension, brakes, splitter, wing, underbody, wing_angle, wheel_set, downforce kg at 150 km/h, drag change %, top speed km/h]`.
pub const CAR_FIELDS: usize = 15;
pub fn garage_info(path: &Path) -> Vec<f32> {
    let p = Profile::load_or_new(path);
    let cars = VehicleSpec::all();
    let sel = cars.iter().position(|(id, _)| *id == p.garage.selected).unwrap_or(0);
    let mut out = vec![p.money as f32, sel as f32, cars.len() as f32];
    for (id, s) in &cars {
        let l = p.levels(id);
        let (price, cost) = if FREE_GARAGE { (0, 0) } else { (s.price, s.upgrade_cost) };
        let st = super::aero::stats(&super::garage::apply(s, l));
        out.extend_from_slice(&[p.owns(id) as u8 as f32, price as f32, cost as f32, l[0] as f32, l[1] as f32, l[2] as f32, l[3] as f32, l[4] as f32, l[5] as f32, l[6] as f32, l[7] as f32, l[8] as f32, st.downforce_kg_150, st.drag_delta_pct, st.top_kmh]);
    }
    out
}

/// Loads the profile, applies `f` and saves when it succeeded; false for a bad car index or a refused action.
fn garage_act(path: &Path, index: usize, f: impl FnOnce(&mut Profile, &str) -> bool) -> bool {
    let cars = VehicleSpec::all();
    let Some((id, _)) = cars.get(index) else { return false };
    let mut p = Profile::load_or_new(path);
    let balance = p.money;
    if FREE_GARAGE { p.money = u32::MAX / 2; }   // every purchase is affordable ...
    let ok = f(&mut p, id);
    if FREE_GARAGE { p.money = balance; }          // ... and costs nothing
    if !ok { return false; }
    p.save(path).is_ok()
}
pub fn garage_buy_car(path: &Path, index: usize) -> bool { garage_act(path, index, |p, id| p.buy_car(id)) }
pub fn garage_select(path: &Path, index: usize) -> bool { garage_act(path, index, |p, id| p.select(id)) }
/// Sets the wing angle (0 low drag .. 2 high downforce) of the car at `index`.
pub fn garage_set_wheel_set(path: &Path, index: usize, set: usize) -> bool { garage_act(path, index, |p, id| p.set_wheel_set(id, set as u8)) }
pub fn garage_set_wing_angle(path: &Path, index: usize, angle: usize) -> bool { garage_act(path, index, |p, id| p.set_wing_angle(id, angle as u8)) }
pub fn garage_buy_upgrade(path: &Path, index: usize, upgrade: usize) -> bool {
    let Some(up) = Upgrade::from_index(upgrade) else { return false };
    garage_act(path, index, |p, id| p.buy_upgrade(id, up))
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
