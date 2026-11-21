//! smoothing and spinner glyphs. no state, no allocation, no dependency to
//! justify. exponential easing is a one-liner and it beats a bouncing ball.

/// move `cur` toward `target` by a fraction of the remaining distance. frame
/// rate independent, so a 144 Hz screen and a 60 Hz one agree on what "fast"
/// means. the snap threshold exists to stop asymptotic creep at the last 0.0004.
pub fn approach(cur: f32, target: f32, dt: f32, rate: f32) -> f32 {
    let k = 1.0 - (-rate * dt).exp();
    let next = cur + (target - cur) * k;
    if (next - target).abs() < 0.0005 {
        target
    } else {
        next
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn spinner(elapsed_ms: u128) -> &'static str {
    SPINNER[((elapsed_ms / 80) % SPINNER.len() as u128) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approach_converges_and_snaps() {
        let mut v = 0.0;
        for _ in 0..600 {
            v = approach(v, 1.0, 0.016, 12.0);
        }
        assert_eq!(v, 1.0);
    }

    #[test]
    fn approach_is_monotonic() {
        let a = approach(0.0, 1.0, 0.016, 10.0);
        let b = approach(a, 1.0, 0.016, 10.0);
        assert!(0.0 < a && a < b && b < 1.0);
    }

    #[test]
    fn easing_bounds() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        assert_eq!(ease_out_cubic(2.0), 1.0);
    }

    #[test]
    fn spinner_cycles() {
        assert_eq!(spinner(0), spinner(800));
        assert_ne!(spinner(0), spinner(80));
    }
}
