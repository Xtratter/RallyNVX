# Fluid dust Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline). Branch `exp/dust-fluid`.

**Spec:** `docs/superpowers/specs/2026-10-14-dust-fluid-design.md`

### Task 1: the CPU reference `game/fluid.rs` (grid, advection, projection, solids, wrap-around window, settling, emitters) with tests
### Task 2: `rust/src/shaders/fluid.wgsl` (compute entry points and the volume render) with a naga test; `fluid` numbers in `surfaces.json`
### Task 3: `raster/fluid_gpu.rs` (textures, pipelines, step and draw), dust level 3, WorldView data, depth kept and sampled, settings row and fallback
### Task 4: device runs: look, fps, tuning
### Task 5: docs, notes, memory; the decision with the user (keep, tune, or return to A/B)
