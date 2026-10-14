use super::*;

#[test] fn default_mode_follows_the_quality() {
    assert_eq!(default_mode(2), RenderMode::Hybrid);
    for q in [-1, 0, 1, 3, 99] { assert_eq!(default_mode(q), RenderMode::Raster, "quality {q}"); }
}
fn feed(m: &mut FallbackMonitor, fps: f32, secs: f32, hybrid: bool, auto: bool) -> bool {
    let mut fired = false;
    for _ in 0..(secs / 0.5) as usize { fired |= m.update(fps, 0.5, hybrid, auto); }
    fired
}
#[test] fn the_monitor_fires_only_when_enabled_in_hybrid_after_four_slow_seconds() {
    assert!(!feed(&mut FallbackMonitor::new(), 20.0, 30.0, true, false), "auto off");
    assert!(!feed(&mut FallbackMonitor::new(), 20.0, 30.0, false, true), "not hybrid");
    let mut m = FallbackMonitor::new();
    assert!(!feed(&mut m, 45.0, 3.5, true, true));
    assert!(m.update(45.0, 0.5, true, true), "the 8th slow sample fires");
}
#[test] fn one_good_sample_resets_and_firing_happens_once_until_reset() {
    let mut m = FallbackMonitor::new();
    assert!(!feed(&mut m, 40.0, 3.5, true, true));
    assert!(!m.update(60.0, 0.5, true, true));
    assert!(!feed(&mut m, 40.0, 3.5, true, true), "the count restarted");
    assert!(m.update(40.0, 0.5, true, true));
    assert!(!feed(&mut m, 10.0, 20.0, true, true), "does not fire twice");
    m.reset();
    assert!(feed(&mut m, 10.0, 5.0, true, true));
}
#[test] fn hitches_and_bad_numbers_are_bounded() {
    let mut m = FallbackMonitor::new();
    assert!(!m.update(5.0, 100.0, true, true), "a single huge dt counts as one second at most");
    assert!(!m.update(f32::NAN, 0.5, true, true) && !m.update(10.0, f32::NAN, true, true));
    let mut m = FallbackMonitor::new();
    let mut n = 0; while !m.update(30.0, 1.0, true, true) { n += 1; assert!(n < 20); }
    assert_eq!(n, 3, "four one-second slow samples fire on the fourth");
}
#[test] fn warm_up_ignores_the_first_seconds_and_a_new_warm_up_restarts_the_count() {
    let mut m = FallbackMonitor::new();
    m.warm_up(6.0);
    assert!(!feed(&mut m, 10.0, 6.0, true, true), "tile streaming at the start is not a verdict on Hybrid");
    assert!(!feed(&mut m, 10.0, 3.5, true, true), "the slow count starts after the warm-up");
    assert!(m.update(10.0, 0.5, true, true));
    m.reset(); m.warm_up(2.0);
    assert!(!feed(&mut m, 10.0, 2.0, true, true));
    assert!(feed(&mut m, 10.0, 4.0, true, true));
}
