//! PC wall-clock admission. Physics always receives its unchanged fixed step.
//! Offline stalls never become an unbounded queue of accelerated gameplay.
use std::time::Duration;

pub(super) struct Charge {
    pub due: Duration,
    pub discarded: Duration,
}

pub(super) fn charge(
    retained: Duration,
    elapsed: Duration,
    step: Duration,
    redraw_budget: u32,
) -> Charge {
    let total = retained.saturating_add(elapsed);
    // Focus controls input, not time. Offline and online simulation both
    // keep running in the background, with bounded debt after a real stall.
    let due = total.min(step.saturating_mul(redraw_budget));
    Charge {
        due,
        discarded: total.saturating_sub(due),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const STEP: Duration = Duration::from_nanos(16_666_667);

    #[test]
    fn background_ticks_continue_at_the_same_wall_clock_rate() {
        let mut retained=Duration::ZERO;
        let mut ticks=0;
        for _ in 0..120 {
            retained=charge(retained,STEP/2,STEP,4).due;
            while retained>=STEP {retained-=STEP;ticks+=1;}
        }
        assert_eq!(ticks,59); // Nanosecond division retains the final fractional slot.
        assert!(retained<STEP);
        let stalled=charge(retained,Duration::from_secs(60),STEP,4);
        assert_eq!(stalled.due,STEP*4);
        assert!(stalled.discarded>Duration::from_secs(59));
    }

    #[test]
    fn long_asset_or_window_stall_cannot_build_a_fast_forward_queue() {
        let stalled = charge(STEP, Duration::from_secs(20), STEP, 4);
        assert_eq!(stalled.due, STEP * 4);
        assert!(stalled.discarded > Duration::from_secs(19));
        let next = charge(Duration::ZERO, STEP, STEP, 4);
        assert_eq!(next.due, STEP);
    }

    #[test]
    fn ordinary_substep_jitter_retains_the_same_fixed_tick() {
        let first = charge(
            Duration::ZERO,
            Duration::from_millis(8),
            STEP,
            4,
        );
        assert!(first.due < STEP);
        let second = charge(first.due, STEP - first.due, STEP, 4);
        assert_eq!(second.due, STEP);
        assert_eq!(second.discarded, Duration::ZERO);
    }

    #[test]
    fn online_loading_debt_is_bounded_without_pausing_background_play() {
        let result = charge(STEP, Duration::from_secs(10), STEP, 4);
        assert_eq!(result.due, STEP * 4);
        assert!(result.discarded > Duration::from_secs(9));
        let ordinary = charge(Duration::ZERO, STEP, STEP, 4);
        assert_eq!(ordinary.due, STEP);
        assert_eq!(ordinary.discarded, Duration::ZERO);
    }
}
