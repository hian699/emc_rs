#![allow(dead_code)]

pub fn level_calculator(total_xp: u64) -> u64 {
    ((total_xp as f64).sqrt() / 10.0).floor() as u64
}
