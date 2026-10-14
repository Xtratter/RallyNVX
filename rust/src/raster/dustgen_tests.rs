use super::*;
use crate::game::{dust::Particle, math::v3};

fn p(x: f32, z: f32, a: f32) -> Particle { Particle { pos: v3(x, 1.0, z), size: 1.0, color: [0.7, 0.6, 0.5], alpha: a } }

#[test] fn instances_are_sorted_far_to_near_and_carry_the_data() {
    let cam = v3(0.0, 2.0, 0.0);
    let v = instances(&[p(0.0, 5.0, 0.4), p(0.0, 20.0, 0.3), p(0.0, 10.0, 0.2)], cam);
    let dist: Vec<f32> = v.iter().map(|i| (i.p[2]).abs()).collect();
    assert_eq!(dist, vec![20.0, 10.0, 5.0]);
    assert_eq!((v[2].c[3], v[2].p[3]), (0.4, 1.0));
    assert_eq!(&v[0].c[..3], &[0.7, 0.6, 0.5]);
}
#[test] fn invisible_nonfinite_and_excess_particles_are_dropped() {
    let cam = v3(0.0, 2.0, 0.0);
    let mut ps = vec![p(0.0, 5.0, 0.0), p(f32::NAN, 5.0, 0.5), Particle { size: f32::INFINITY, ..p(0.0, 6.0, 0.5) }];
    ps.extend((0..2000).map(|k| p(0.0, 7.0 + k as f32 * 0.01, 0.3)));
    let v = instances(&ps, cam);
    assert_eq!(v.len(), 1024, "capped, the zero-alpha and non-finite ones skipped");
    assert!(v.iter().all(|i| i.p.iter().chain(&i.c).all(|x| x.is_finite())));
    assert!(instances(&[], cam).is_empty());
}
