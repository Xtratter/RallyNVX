use super::*;
use crate::game::math::v3;

fn line() -> Line { Line { pos: v3(10.0, 0.0, 20.0), dir: v3(0.0, 0.0, 1.0), width: 8.0, kind: LineKind::Check } }

#[test] fn crossing_forwards_inside_the_width() {
    let l = line();
    assert!(l.crossed(v3(10.0, 0.0, 19.0), v3(10.0, 0.0, 21.0)));
    assert!(l.crossed(v3(13.0, 0.0, 19.9), v3(13.5, 0.0, 20.1)));
    assert!(!l.crossed(v3(10.0, 0.0, 21.0), v3(10.0, 0.0, 19.0)), "backwards");
    assert!(!l.crossed(v3(40.0, 0.0, 19.0), v3(40.0, 0.0, 21.0)), "far outside the width");
    assert!(!l.crossed(v3(10.0, 0.0, 15.0), v3(10.0, 0.0, 19.9)), "short of the line");
}
#[test] fn local_coordinates() {
    let l = Line { dir: v3(1.0, 0.0, 0.0), ..line() };
    let (along, across) = l.local(v3(15.0, 0.0, 22.0));
    assert!((along - 5.0).abs() < 1e-5 && (across + 2.0).abs() < 1e-5, "{along} {across}");   // right of +x is -z
}
