//! The co-driver: pace notes made from the shape of a stage (corners, crests, junctions, the finish) and their playback
//! by the car's progress. See docs/superpowers/specs/2026-10-14-pace-notes-design.md.
use super::math::V3;
use super::roads::shape::radius;

/// A corner call: direction, severity (0 hairpin, 1 tightest .. 5 gentlest), how it develops, and a second corner it runs into.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner { pub left: bool, pub sev: u8, pub tightens: bool, pub opens: bool, pub long: bool, pub into: Option<(bool, u8)> }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoteKind { Corner(Corner), Crest { big: bool }, Junction { left: bool }, Finish }

/// A call and where along the path (metres) the thing it describes begins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note { pub s: f32, pub kind: NoteKind }

const ENTER: f32 = 200.0;
const EXIT: f32 = 220.0;
const MIN_ANGLE: f32 = 20.0 * std::f32::consts::PI / 180.0;
const INTO_GAP: f32 = 40.0;
const FINISH_BEFORE: f32 = 140.0;

/// Severity of a corner by its smallest radius: 0 hairpin, 1..5, `None` for corners too fast to call.
pub fn severity(min_radius: f32) -> Option<u8> {
    Some(match min_radius { r if r < 18.0 => 0, r if r < 30.0 => 1, r if r < 45.0 => 2, r if r < 65.0 => 3, r if r < 95.0 => 4, r if r < 140.0 => 5, _ => return None })
}

/// The notes of a path (points about 4 m apart, `y` the height): `junctions` are (metres along the path, to the left).
pub fn generate(path: &[V3], junctions: &[(f32, bool)]) -> Vec<Note> {
    let n = path.len();
    let mut s = vec![0.0f32; n];
    for i in 1..n { s[i] = s[i - 1] + ((path[i].x - path[i - 1].x).hypot(path[i].z - path[i - 1].z)).max(1e-3); }
    let end = s.last().copied().unwrap_or(0.0);
    let mut notes: Vec<Note> = Vec::new();
    if n >= 12 {
        // radius and turning side at every point (three points 12 m apart); the car's left is +x when it faces +z, so a left turn has a negative cross product
        let (mut r, mut left) = (vec![f32::INFINITY; n], vec![false; n]);
        for i in 3..n - 3 {
            let (a, b, c) = (path[i - 3], path[i], path[i + 3]);
            r[i] = radius((a.x, a.z), (b.x, b.z), (c.x, c.z));
            left[i] = (b.x - a.x) * (c.z - b.z) - (b.z - a.z) * (c.x - b.x) < 0.0;
        }
        struct Raw { start: f32, end: f32, left: bool, min_r: f32, first: f32, last: f32, arc: f32 }
        let mut raws: Vec<Raw> = Vec::new();
        let mut i = 3;
        while i < n - 3 {
            if r[i] >= ENTER { i += 1; continue; }
            let (dir, start) = (left[i], i);
            let mut j = i;
            while j < n - 3 && r[j] < EXIT && left[j] == dir { j += 1; }
            let angle: f32 = (start..j).map(|k| (s[k + 1] - s[k]) / r[k].max(1.0)).sum();
            if angle >= MIN_ANGLE {
                let third = ((j - start) / 3).max(1);
                let mean = |a: usize, b: usize| { let v: Vec<f32> = r[a..b].iter().cloned().filter(|x| x.is_finite()).collect(); if v.is_empty() { ENTER } else { v.iter().sum::<f32>() / v.len() as f32 } };
                raws.push(Raw { start: s[start], end: s[j - 1], left: dir, min_r: r[start..j].iter().cloned().fold(f32::INFINITY, f32::min), first: mean(start, start + third), last: mean(j - third, j), arc: s[j - 1] - s[start] });
            }
            i = j.max(i + 1);
        }
        let mut k = 0;
        while k < raws.len() {
            let c = &raws[k];
            if let Some(sev) = severity(c.min_r) {
                let mut corner = Corner { left: c.left, sev, tightens: c.last < 0.75 * c.first, opens: c.last > 1.35 * c.first, long: c.arc > 80.0, into: None };
                if let Some(nx) = raws.get(k + 1) {
                    if nx.start - c.end < INTO_GAP { if let Some(s2) = severity(nx.min_r) { corner.into = Some((nx.left, s2)); k += 1; } }
                }
                notes.push(Note { s: c.start, kind: NoteKind::Corner(corner) });
            }
            k += 1;
        }
        // crests: the second difference of the height over +-5 points, a local minimum below the threshold
        if n >= 14 {
            let ky: Vec<f32> = (0..n).map(|i| if i < 5 || i + 5 >= n { 0.0 } else { let d = (s[i + 5] - s[i - 5]) * 0.5; (path[i - 5].y + path[i + 5].y - 2.0 * path[i].y) / (d * d).max(1.0) }).collect();
            let mut last_crest: Option<(f32, f32, usize)> = None;   // (s, ky, index into notes)
            for i in 6..n - 6 {
                if ky[i] < -0.0035 && ky[i] <= ky[i - 1] && ky[i] < ky[i + 1] {
                    let note = Note { s: s[i], kind: NoteKind::Crest { big: ky[i] < -0.008 } };
                    match last_crest {
                        Some((ls, lk, idx)) if s[i] - ls < 30.0 => { if ky[i] < lk { notes[idx] = note; last_crest = Some((s[i], ky[i], idx)); } }
                        _ => { notes.push(note); last_crest = Some((s[i], ky[i], notes.len() - 1)); }
                    }
                }
            }
        }
    }
    for &(at, left) in junctions { notes.push(Note { s: at, kind: NoteKind::Junction { left } }); }
    let finish_s = (end - FINISH_BEFORE).max(0.0);
    notes.retain(|n| n.s < finish_s);   // nothing is called after the finish call
    notes.push(Note { s: finish_s, kind: NoteKind::Finish });
    notes.sort_by(|a, b| a.s.partial_cmp(&b.s).unwrap_or(std::cmp::Ordering::Equal));
    notes
}

fn round_dist(d: f32) -> u32 {
    let step = if d < 100.0 { 10.0 } else if d < 300.0 { 50.0 } else { 100.0 };
    ((d / step).round() * step) as u32
}

impl Note {
    /// The call as a sentence; `dist` is the distance to the next note (added when over 60 m).
    pub fn text(&self, dist: Option<f32>, ru: bool) -> String {
        let side = |left: bool| if ru { if left { "лево" } else { "право" } } else if left { "left" } else { "right" };
        let corner = |left: bool, sev: u8| if sev == 0 { if ru { format!("шпилька {}", side(left)) } else { format!("hairpin {}", side(left)) } } else { format!("{} {}", side(left), sev) };
        let body = match self.kind {
            NoteKind::Corner(c) => {
                let mut t = corner(c.left, c.sev);
                if c.long { t += if ru { " длинный" } else { " long" }; }
                if c.tightens { t += if ru { " затягивается" } else { " tightens" }; }
                if c.opens { t += if ru { " открывается" } else { " opens" }; }
                if let Some((l2, s2)) = c.into { t += &format!("{}{}", if ru { " в " } else { " into " }, corner(l2, s2)); }
                t
            }
            NoteKind::Crest { big } => (if big { if ru { "Прыжок" } else { "Jump" } } else if ru { "Гребень" } else { "Crest" }).to_string(),
            NoteKind::Junction { left } => if ru { format!("Перекрёсток {}", if left { "слева" } else { "справа" }) } else { format!("Junction {}", if left { "left" } else { "right" }) },
            NoteKind::Finish => return if ru { format!("Финиш через {}", round_dist(FINISH_BEFORE - 40.0)) } else { format!("Finish in {}", round_dist(FINISH_BEFORE - 40.0)) },
        };
        let mut t = body;
        if let Some(c) = t.chars().next() { t = c.to_uppercase().collect::<String>() + &t[c.len_utf8()..]; }
        match dist { Some(d) if d > 60.0 => format!("{t}, {}", round_dist(d)), _ => t }
    }
}

/// The notes of one stage and the car's progress through them.
pub struct Playback {
    path: Vec<V3>,
    s: Vec<f32>,
    pub notes: Vec<Note>,
    next: usize,
    idx: usize,
    cooldown: f32,
}

impl Playback {
    pub fn new(path: Vec<V3>, notes: Vec<Note>) -> Playback {
        let mut s = vec![0.0f32; path.len()];
        for i in 1..path.len() { s[i] = s[i - 1] + ((path[i].x - path[i - 1].x).hypot(path[i].z - path[i - 1].z)).max(1e-3); }
        Playback { path, s, notes, next: 0, idx: 0, cooldown: 0.0 }
    }

    /// The distance along the path of the car's nearest path point.
    pub fn progress(&self) -> f32 { self.s.get(self.idx).copied().unwrap_or(0.0) }
    pub fn next_index(&self) -> usize { self.next }

    /// Metres from the car to the start of the next note not yet called (0 when none is left).
    pub fn to_next(&self) -> f32 { self.notes.get(self.next).map_or(0.0, |n| (n.s - self.progress()).max(0.0)) }

    /// The car is at `pos` doing `speed` m/s: returns the index of a note to call now (at most one per 1.5 s, in order, once).
    pub fn update(&mut self, pos: V3, speed: f32, dt: f32) -> Option<usize> {
        if self.path.len() < 2 { return None; }
        let nearest = |lo: usize, hi: usize, path: &[V3]| (lo..=hi.min(path.len() - 1)).map(|i| (i, (path[i].x - pos.x).hypot(path[i].z - pos.z))).fold((lo, f32::INFINITY), |a, b| if b.1 < a.1 { b } else { a });
        let (mut i, mut d) = nearest(self.idx.saturating_sub(10), self.idx + 120, &self.path);
        if d > 40.0 { let all = nearest(0, self.path.len() - 1, &self.path); if all.1 < d { i = all.0; d = all.1; } }
        let _ = d;
        self.idx = i;
        self.cooldown = (self.cooldown - dt).max(0.0);
        let car_s = self.s[i];
        let lead = (5.0 * speed.max(0.0)).max(80.0);
        // a note passed without being called (the car was too quick or was reset ahead) is dropped
        while self.next < self.notes.len() && self.notes[self.next].s < car_s - 30.0 { self.next += 1; }
        if self.next < self.notes.len() && self.cooldown <= 0.0 && self.notes[self.next].s - car_s <= lead {
            let k = self.next;
            self.next += 1;
            self.cooldown = 1.5;
            return Some(k);
        }
        None
    }

    /// The text of note `k` with the distance to the following note.
    pub fn text(&self, k: usize, ru: bool) -> String {
        let dist = self.notes.get(k + 1).map(|n| n.s - self.notes[k].s);
        self.notes[k].text(dist, ru)
    }
}

#[cfg(test)]
#[path = "pacenotes_tests.rs"]
mod tests;
