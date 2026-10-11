// Forward photons from the sun through the prism into the receivers' fixed-point counters (see docs/superpowers/specs/2026-10-10-prism-design.md).
@group(0) @binding(0) var<uniform> P: Params;
@group(0) @binding(1) var<storage, read_write> COUNTS: array<atomic<u32>>;

fn to_local(v: vec3<f32>) -> vec3<f32> { return rot_y(v, PR_YAW + P.time * PR_SPIN); }
fn to_world(v: vec3<f32>) -> vec3<f32> { return rot_y(v, -(PR_YAW + P.time * PR_SPIN)); }

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (!has_sun()) { return; }
  rng = pcg(gid.x + P.seed * 747796405u);
  rng = pcg(rng ^ 2747636419u);
  let axis = -SUN_DIR;
  let d0 = sample_cone(axis, SUN_COS);
  // start on the disc perpendicular to the sun that covers the prism's bound sphere
  let b = make_basis(axis);
  let rr = PR_BOUND * sqrt(rnd());
  let ph = 6.2831853 * rnd();
  // far enough back to be outside the whole room, so walls and slits between the sun and the prism are tested
  let start = PR_C - axis * (PR_BOUND * 1.5 + 40.0) + (b[0] * cos(ph) + b[1] * sin(ph)) * rr;
  let lam = 380.0 + 400.0 * rnd();
  let n = ior_l(lam, PR_IOR, PR_DISP);
  let f0 = (n - 1.0) / (n + 1.0);

  // trace in the prism's own frame
  var o = to_local(start - PR_C);
  var d = to_local(d0);
  let c4 = prism_clip(o, d, PR_SIDE, PR_LEN);
  if (!(c4.y > max(c4.x, 0.0)) || c4.x < 0.0) { return; }
  // the sun beam must reach the prism: slits and walls between the sun and the entry point cut it (glass panes tint it)
  let beam = occ_all(start, d0, c4.x - 0.01, -1);
  if (all(beam == vec3<f32>(0.0))) { return; }
  let nin = prism_normal(c4.z);
  if (rnd() < schlick(clamp(-dot(d, nin), 0.0, 1.0), f0 * f0)) { return; }
  o = o + d * c4.x - nin * 1e-4;
  d = refract(d, nin, 1.0 / n);
  var out_ok = false;
  for (var k = 0; k < 5; k = k + 1) {
    let c = prism_clip(o, d, PR_SIDE, PR_LEN);
    let nout = prism_normal(c.w);
    let pos = o + d * c.y;
    let cosi = dot(d, nout);
    let sint = n * sqrt(max(0.0, 1.0 - cosi * cosi));
    var refl = sint > 1.0;
    if (!refl) { refl = rnd() < schlick(sqrt(max(0.0, 1.0 - sint * sint)), f0 * f0); }
    if (refl) {
      d = reflect(d, nout);
      o = pos - nout * 1e-4;
    } else {
      d = refract(d, -nout, n);
      o = pos + nout * 2e-3;
      out_ok = true;
      break;
    }
  }
  if (!out_ok) { return; }
  let wo = to_world(o) + PR_C;
  let wd = to_world(d);
  let hit = rx_hit(wo, wd);
  if (hit.x < 0.0) { return; }
  // occluders between the prism and the receiver (glass boxes tint, the rest blocks)
  let tr = occ_all(wo, wd, hit.y - 0.004, -1);
  let omega = 2.0 * PI * (1.0 - SUN_COS);
  let beam_area = PI * PR_BOUND * PR_BOUND;
  let e_perp = sun_rad() * omega; // irradiance on a surface facing the sun
  let phi = e_perp * beam_area / PHOTONS * wavelength_rgb(lam) * tr * beam;
  let idx = u32(hit.x) * 3u;
  atomicAdd(&COUNTS[idx], u32(phi.x * FIXED));
  atomicAdd(&COUNTS[idx + 1u], u32(phi.y * FIXED));
  atomicAdd(&COUNTS[idx + 2u], u32(phi.z * FIXED));
}
