#![allow(dead_code)]

pub fn milisecond_calculator(input: &str) -> Option<u64> {
    let mut total_ms: u64 = 0;
    let mut buffer = String::new();

    for ch in input.chars() {
        if ch.is_ascii_digit() {
            buffer.push(ch);
            continue;
        }

        if buffer.is_empty() {
            return None;
        }

        let value = buffer.parse::<u64>().ok()?;
        buffer.clear();

        let factor = match ch {
            'd' | 'D' => 86_400_000,
            'h' | 'H' => 3_600_000,
            'm' | 'M' => 60_000,
            's' | 'S' => 1_000,
            _ => return None,
        };

        total_ms = total_ms.saturating_add(value.saturating_mul(factor));
    }

    if !buffer.is_empty() {
        total_ms = total_ms.saturating_add(buffer.parse::<u64>().ok()?);
    }

    Some(total_ms)
}
