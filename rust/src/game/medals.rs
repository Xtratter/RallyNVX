//! Medal grading and money for finished missions (pure functions).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Medal { None = 0, Bronze = 1, Silver = 2, Gold = 3 }

impl Medal {
    pub fn from_u8(v: u8) -> Medal { match v { 1 => Medal::Bronze, 2 => Medal::Silver, 3 => Medal::Gold, _ => Medal::None } }
}

fn by_time(t: f32, limit: f32) -> Medal {
    if t <= 0.6 * limit { Medal::Gold } else if t <= 0.8 * limit { Medal::Silver } else { Medal::Bronze }
}

pub fn grade_cargo(t: f32, limit: f32, integrity: f32) -> Medal {
    let by_i = if integrity >= 90.0 { Medal::Gold } else if integrity >= 60.0 { Medal::Silver } else { Medal::Bronze };
    by_time(t, limit).min(by_i)
}

pub fn grade_race(t: f32, limit: f32) -> Medal { by_time(t, limit) }

fn mult(m: Medal) -> f32 { match m { Medal::None => 0.0, Medal::Bronze => 1.0, Medal::Silver => 1.5, Medal::Gold => 2.2 } }

pub fn reward(base: f32, m: Medal) -> u32 { (base * mult(m)).round() as u32 }

/// Money for finishing with `new` when the best earlier medal for this mission was `best_before`.
pub fn payout(base: f32, new: Medal, best_before: Medal) -> u32 {
    if new > best_before { reward(base, new) - reward(base, best_before) } else { (reward(base, new) as f32 * 0.5).round() as u32 }
}

#[cfg(test)]
#[path = "medals_tests.rs"]
mod tests;
