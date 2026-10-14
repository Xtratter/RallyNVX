# Rally core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline).

**Goal:** the Rally entry with stages, timer, penalties, results and records.

**Spec:** `docs/superpowers/specs/2026-10-14-rally-core-design.md`

## Global Constraints

Free drive unchanged; EN+RU strings and docs; numbers in code constants documented in the spec; tests green; the phone only for one check at the end.

## Review Focus

A line crossed from the wrong side; the car teleported (reset) across a line; lines skipped by a reset to the road; the countdown while the autopilot is on; leaving the activity mid-run; records written for an assisted run.

### Task 1: lines (generalise, stage lines, drawing)
### Task 2: the runner `rally.rs`
### Task 3: World integration, JNI, state slots
### Task 4: Kotlin menu, HUD, results, records
### Task 5: docs, version 2.22.0, device check, release question
