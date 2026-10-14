# Side roads Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline).

**Goal:** junctions with dead-end side roads along the stage.

**Architecture:** `RoadKind::Side`, `roads/side.rs` generator called from `generate_with`, numbers in `roads.json`.

**Spec:** `docs/superpowers/specs/2026-10-14-side-roads-design.md`

## Global Constraints

Deterministic per seed; EN+RU docs; ships as v2.20.0 together with 2.19.1; ask the user for the phone for one check; tests stay green.

## Review Focus

A side road across a lake; a short stage with no room for junctions; a junction on a crest or in a bend; two junctions overlapping; the ditch/verge/loose layer on the side road.

### Task 1: the Side kind and the config
- [ ] `RoadKind::Side`, `is_gravel()`; replace `== Stage` gravel tests (surface verge, ditch, props rocks, loose map); roadgen ribbon code; `roads.json` `side` block + `SideCfg` in `roads/cfg.rs` with tests.
### Task 2: the generator
- [ ] `roads/side.rs`: `side_roads(stage: &Road, base: &Terrain, seed: u64, cfg: &SideCfg) -> Vec<Road>`; `build_road` takes `pin_start: Option<f32>` and the Side width rules; call it from `generate_with`; tests as in the spec.
### Task 3: docs, version, release with 2.19.1
- [ ] game.md EN+RU, changelog 2.20.0 EN+RU (including the panel), README bullet, version 2.20.0, notes `../releases/rallynvx-2.20.0.md`; phone check, then release after the user's go.
