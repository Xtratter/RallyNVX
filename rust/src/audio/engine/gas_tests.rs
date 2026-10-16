use super::*;

#[test] fn a_filled_volume_reads_back_its_pressure_and_temperature() {
    let g = Gas::air(0.002, 2.0 * P_ATM, 350.0);
    assert!((g.pressure() - 2.0 * P_ATM).abs() < 1.0 && (g.temperature() - 350.0).abs() < 1e-6, "{} {}", g.pressure(), g.temperature());
    assert!((g.o2 / g.n - 0.21).abs() < 1e-9 && g.fuel == 0.0);
}
#[test] fn compressing_ten_to_one_follows_the_adiabat() {
    let mut g = Gas::air(5e-4, P_ATM, 300.0);
    let (p0, t0) = (g.pressure(), g.temperature());
    for _ in 0..1000 { let dv = -(g.v - 5e-5) / 200.0; g.change_volume(dv.max(-g.v * 0.01)); if g.v <= 5.0001e-5 { break; } }
    let cr = 5e-4 / g.v;
    assert!(cr > 9.0, "compressed to {cr}:1");
    assert!((g.temperature() / t0 - cr.powf(GAMMA - 1.0)).abs() < 0.01 * cr.powf(GAMMA - 1.0), "T ratio {}", g.temperature() / t0);
    assert!((g.pressure() / p0 - cr.powf(GAMMA)).abs() < 0.01 * cr.powf(GAMMA), "P ratio {}", g.pressure() / p0);
}
#[test] fn orifice_flow_chokes_at_the_critical_ratio_and_is_continuous_there() {
    let (a, p0, t0) = (1e-4, 3.0 * P_ATM, 300.0);
    let crit = (2.0 / (GAMMA + 1.0)).powf(GAMMA / (GAMMA - 1.0));
    let below = orifice_mass_flow(a, p0, t0, p0 * (crit - 1e-4));
    let above = orifice_mass_flow(a, p0, t0, p0 * (crit + 1e-4));
    assert!((below - above).abs() < 0.01 * below, "{below} vs {above}");
    assert!((orifice_mass_flow(a, p0, t0, 1.0) - below).abs() < 1e-12, "choked: the back pressure no longer matters");
    // the textbook number: 0.0404 * p0 * A / sqrt(T) kg/s for air
    assert!((below - 0.0404 * p0 * a / t0.sqrt()).abs() < 0.02 * below, "{below}");
    assert_eq!(orifice_mass_flow(a, P_ATM, 300.0, 2.0 * P_ATM), 0.0, "no flow uphill");
}
#[test] fn flow_equalises_conserves_moles_and_energy_and_never_overshoots() {
    let mut a = Gas::air(1e-3, 8.0 * P_ATM, 600.0); let mut b = Gas::air(2e-3, P_ATM, 300.0);
    let (n0, e0) = (a.n + b.n, a.e + b.e);
    for _ in 0..20000 { flow(&mut a, &mut b, 3e-4, 2e-5); assert!(a.pressure() >= b.pressure() * 0.999, "overshoot: {} {}", a.pressure(), b.pressure()); }
    assert!((a.pressure() - b.pressure()).abs() < 0.02 * a.pressure(), "{} {}", a.pressure(), b.pressure());
    assert!(((a.n + b.n) - n0).abs() < 1e-12 && ((a.e + b.e) - e0).abs() < 1e-6 * e0, "moles {} energy {}", a.n + b.n - n0, a.e + b.e - e0);
    // the other way round
    let mut c = Gas::air(1e-3, P_ATM, 300.0); let mut d = Gas::air(1e-3, 3.0 * P_ATM, 300.0);
    assert!(flow(&mut c, &mut d, 1e-4, 1e-4) < 0.0 && d.pressure() < 3.0 * P_ATM, "from the high side to the low one");
}
#[test] fn the_mixture_travels_with_the_gas() {
    let mut a = Gas::air(1e-3, 4.0 * P_ATM, 300.0); a.fuel = a.n * 0.05;
    let mut b = Gas::air(1e-3, P_ATM, 300.0);
    flow(&mut a, &mut b, 1e-4, 1e-4);
    assert!(b.fuel > 0.0 && (b.fuel / b.n) > 0.0 && a.fuel > 0.0, "{} {}", a.fuel, b.fuel);
}
