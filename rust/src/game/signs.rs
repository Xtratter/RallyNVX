//! Signs along the special stage that say what the co-driver says: corner warnings, chevrons, bumps, route arrows at
//! junctions, no-entry signs on side roads, distance boards, the start and finish boards. Made from the same notes as the
//! voice (see docs/superpowers/specs/2026-10-14-signs-design.md). Shapes follow the public road-sign conventions
//! (UNECE warning triangles) and the rally stage boards (FIA / NASA Rally Sport); there is no text on the signs.
use super::math::{v3, V3};
use super::pacenotes::{generate, Note, NoteKind};
use super::roads::{Road, RoadKind};
use super::terrain::Terrain;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignKind { CurveL, CurveR, SCurveL, SCurveR, HairpinL, HairpinR, Bump, ChevL, ChevR, RouteArrow, NoEntry, Dist1, Dist2, Dist3, FinishBoard, Stop, Start }

impl SignKind {
    pub const ALL: [SignKind; 17] = [SignKind::CurveL, SignKind::CurveR, SignKind::SCurveL, SignKind::SCurveR, SignKind::HairpinL, SignKind::HairpinR, SignKind::Bump, SignKind::ChevL, SignKind::ChevR,
        SignKind::RouteArrow, SignKind::NoEntry, SignKind::Dist1, SignKind::Dist2, SignKind::Dist3, SignKind::FinishBoard, SignKind::Stop, SignKind::Start];
    pub fn index(self) -> usize { Self::ALL.iter().position(|k| *k == self).unwrap_or(0) }
}

/// A sign standing on the ground at `pos`; its board faces the direction `yaw` (heading atan2(x, z) of the viewer's side).
#[derive(Clone, Copy, Debug)]
pub struct Sign { pub kind: SignKind, pub pos: V3, pub yaw: f32 }

/// How far beyond the road edge the posts stand (m).
const OFFSET: f32 = 3.2;

/// The stage road, its side roads and where they leave it: (junction s along the stage, to the left of the travel direction, side road index).
pub fn stage_junctions<'a>(stage: &Road, sides: &[&'a Road]) -> Vec<(f32, bool, &'a Road)> {
    let mut out = Vec::new();
    for side in sides.iter().filter(|r| r.pts.len() > 4) {
        let a = side.pts[0].pos;
        let Some(j) = stage.pts.iter().enumerate().min_by(|x, y| ((x.1.pos.x - a.x).hypot(x.1.pos.z - a.z)).partial_cmp(&((y.1.pos.x - a.x).hypot(y.1.pos.z - a.z))).unwrap()).map(|x| x.0) else { continue };
        let (p, q) = (stage.pts[j.saturating_sub(1)].pos, stage.pts[(j + 1).min(stage.pts.len() - 1)].pos);
        let (tx, tz) = (q.x - p.x, q.z - p.z);
        let v = side.pts[4].pos - a;
        out.push((stage.pts[j].s, v.x * tz - v.z * tx > 0.0, *side));
    }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// The stage's pace notes (corners, crests, junctions, the finish) and the path they belong to.
pub fn stage_notes(stage: &Road, sides: &[&Road]) -> (Vec<V3>, Vec<Note>) {
    let path: Vec<V3> = stage.pts.iter().map(|p| p.pos).collect();
    let junctions: Vec<(f32, bool)> = stage_junctions(stage, sides).iter().map(|j| (j.0, j.1)).collect();
    let notes = generate(&path, &junctions);
    (path, notes)
}

/// The point of a road at distance `s` with its unit direction of travel and half width.
fn at(road: &Road, s: f32) -> (V3, V3, f32) {
    let i = road.pts.iter().position(|p| p.s >= s).unwrap_or(road.pts.len() - 1).clamp(1, road.pts.len() - 1);
    let (a, b) = (road.pts[i - 1], road.pts[i]);
    let (dx, dz) = (b.pos.x - a.pos.x, b.pos.z - a.pos.z);
    let l = dx.hypot(dz).max(1e-3);
    (b.pos, v3(dx / l, 0.0, dz / l), b.width * 0.5)
}

/// All the signs of a level (the stage only); empty without roads.
pub fn stage_signs(t: &Terrain) -> Vec<Sign> {
    let Some(net) = t.roads() else { return Vec::new() };
    let Some(stage) = net.roads.iter().find(|r| r.kind == RoadKind::Stage) else { return Vec::new() };
    if stage.pts.len() < 20 { return Vec::new(); }
    let sides: Vec<&Road> = net.roads.iter().filter(|r| r.kind == RoadKind::Side).collect();
    let (_, notes) = stage_notes(stage, &sides);
    let len = stage.length();
    let mut out: Vec<Sign> = Vec::new();
    // a sign on the right (`side` +1) or the left (-1) of `road` at `s`, its board turned to a driver coming from `s - look`
    let put = |out: &mut Vec<Sign>, road: &Road, kind: SignKind, s: f32, side: f32, look: f32| {
        let s = s.clamp(2.0, road.length() - 1.0);
        let (p, tan, half) = at(road, s);
        let right = v3(-tan.z, 0.0, tan.x);
        let pos = p + right * (side * (half + OFFSET));
        let (from, _, _) = at(road, (s - look).max(0.0));
        let to_driver = from - pos;
        let ground = t.height(pos.x, pos.z);
        if t.is_water(pos.x, pos.z, 0.5) { return; }
        out.push(Sign { kind, pos: v3(pos.x, ground, pos.z), yaw: to_driver.x.atan2(to_driver.z) });
    };
    for n in &notes {
        match n.kind {
            NoteKind::Corner(c) => {
                let kind = if c.sev == 0 { if c.left { SignKind::HairpinL } else { SignKind::HairpinR } }
                    else if let Some((l2, _)) = c.into { let _ = l2; if c.left { SignKind::SCurveL } else { SignKind::SCurveR } }
                    else if c.left { SignKind::CurveL } else { SignKind::CurveR };
                put(&mut out, stage, kind, n.s - 70.0, 1.0, 40.0);
                if c.sev <= 3 {
                    let k = ((c.arc / 18.0).round() as i32).clamp(2, 4);
                    for i in 1..=k {
                        let s = n.s + c.arc * i as f32 / (k + 1) as f32;
                        put(&mut out, stage, if c.left { SignKind::ChevL } else { SignKind::ChevR }, s, if c.left { 1.0 } else { -1.0 }, 40.0);
                    }
                }
            }
            NoteKind::Crest { .. } => put(&mut out, stage, SignKind::Bump, n.s - 60.0, 1.0, 40.0),
            _ => {}
        }
    }
    for (s, left, side) in stage_junctions(stage, &sides) {
        put(&mut out, stage, SignKind::RouteArrow, s - 6.0, if left { 1.0 } else { -1.0 }, 40.0);   // on the far side from the side road
        put(&mut out, side, SignKind::NoEntry, 16.0, 1.0, -30.0);   // on the side road, turned to the stage
    }
    let finish = len - 12.0;
    for (k, kind) in [(300.0, SignKind::Dist3), (200.0, SignKind::Dist2), (100.0, SignKind::Dist1)] { put(&mut out, stage, kind, finish - k, 1.0, 40.0); }
    put(&mut out, stage, SignKind::FinishBoard, finish, 1.0, 40.0);
    put(&mut out, stage, SignKind::FinishBoard, finish, -1.0, 40.0);
    put(&mut out, stage, SignKind::Stop, len - 4.0, 1.0, 40.0);
    put(&mut out, stage, SignKind::Start, 25.0, 1.0, 30.0);
    out
}

#[cfg(test)]
#[path = "signs_tests.rs"]
mod tests;
