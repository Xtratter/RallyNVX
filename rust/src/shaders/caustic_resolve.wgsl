// Turns the photon counters into an exponentially averaged irradiance map (E / pi per cell) and clears the counters.
@group(0) @binding(0) var<uniform> P: Params;
@group(0) @binding(1) var<storage, read_write> COUNTS: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> MAP: array<vec4<f32>>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  if (i >= TOTAL_CELLS) { return; }
  let r = f32(atomicExchange(&COUNTS[i * 3u], 0u));
  let g = f32(atomicExchange(&COUNTS[i * 3u + 1u], 0u));
  let b = f32(atomicExchange(&COUNTS[i * 3u + 2u], 0u));
  let e = vec3<f32>(r, g, b) / FIXED / cell_area(i) / PI;
  let a = select(0.15, 1.0, P.history_reset != 0u);
  MAP[i] = vec4<f32>(mix(MAP[i].xyz, e, a), 1.0);
}
