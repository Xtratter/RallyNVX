use super::*;
use crate::game::terrain::Terrain;

fn len(p: &[(f32, f32)]) -> f32 { p.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).sum() }

#[test] fn on_flat_ground_the_path_is_nearly_straight_but_not_a_ruler_line() {
    let t = Terrain::flat();
    let g = Grid::build(&t, 32.0, 1600.0);
    let p = g.find((-1000.0, -300.0), (900.0, 400.0), true);
    assert_eq!((p[0], *p.last().unwrap()), ((-1000.0, -300.0), (900.0, 400.0)));
    let straight = ((1900.0f32).powi(2) + 700.0f32.powi(2)).sqrt();
    let l = len(&p);
    assert!(l >= straight - 1.0 && l < straight * 1.35, "{l} vs {straight}");
}
#[test] fn the_path_is_deterministic_and_stays_inside_the_grid() {
    let t = Terrain::new(5);
    let g = Grid::build(&t, 32.0, 2000.0);
    let a = g.find((-900.0, 200.0), (1200.0, -700.0), true);
    let b = g.find((-900.0, 200.0), (1200.0, -700.0), true);
    assert_eq!(a, b);
    assert!(a.iter().all(|p| p.0.abs() <= 2000.0 && p.1.abs() <= 2000.0));
}
#[test] fn the_path_keeps_out_of_lakes_and_cities_when_it_can() {
    let mut checked = 0;
    for seed in 1..40u64 {
        let t = Terrain::new(seed);
        if t.water_level().is_none() { continue; }
        let g = Grid::build(&t, 32.0, 2400.0);
        let p = g.find((-1500.0, -1500.0), (1500.0, 1500.0), true);
        let wet = p.iter().filter(|q| g.is_wet(q.0, q.1)).count();
        assert!(wet * 20 <= p.len(), "seed {seed}: {wet}/{} points in water", p.len());
        checked += 1;
        if checked >= 4 { break; }
    }
    assert!(checked >= 2, "the probe needs levels with lakes");
}
#[test] fn a_steep_hill_makes_the_path_wind_instead_of_climbing_straight() {
    let t = Terrain::new(11);
    let g = Grid::build(&t, 32.0, 2000.0);
    let p = g.find((-1500.0, -1500.0), (1500.0, 1500.0), false);
    let straight = (3000.0f32 * 2.0f32.sqrt()).abs();
    assert!(len(&p) > straight * 1.01 && len(&p) < straight * 1.8, "{}", len(&p));
}
