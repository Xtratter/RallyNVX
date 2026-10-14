# Pace notes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline).

**Goal:** a co-driver for rallies, text and voice.

**Spec:** `docs/superpowers/specs/2026-10-14-pace-notes-design.md`

## Review Focus

Notes with a loop path (the second lap must not jump back to the first); a path shorter than the lead distance; several notes due at once; a car reversing or reset (the window search must not lose it); TTS missing.

### Task 1: the generator `pacenotes.rs` and its tests
### Task 2: playback in the rally world, state slots, shared text, JNI
### Task 3: Kotlin bar, TextToSpeech, setting
### Task 4: docs, 2.23.0, device check, release question
