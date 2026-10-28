//! Live decision order: `DoesPlayerRequest82dd9988`, Runout82dffe88, Wipeout82e00a50.
//! These callbacks read retained owners; no request or enable snapshot is invented.

pub(crate) trait WipeoutDecisionSource {
    type Error;
    type EnableTarget;
    fn disable_all(&mut self) -> Result<bool, Self::Error>;
    fn capture_enable(&mut self, index: usize) -> Result<Self::EnableTarget, Self::Error>;
    fn read_enable(&mut self, target: &Self::EnableTarget) -> Result<bool, Self::Error>;
    fn requested(&mut self, index: usize) -> Result<bool, Self::Error>;
    /// Native requires `DebugOverlay` even when display-debug is false.
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error>;
    fn display_debug(&mut self) -> Result<bool, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnableRead {
    Capture(usize),
    Value(usize),
}
// Retained cvar target identity and scalar load are distinct operations. This
// domain sampling schedule preserves native relinking/value-mutation timing.
const ENABLE_READS: [EnableRead; 60] = {
    use EnableRead::{Capture as C, Value as V};
    [
        C(0),
        C(1),
        C(2),
        C(3),
        C(4),
        C(5),
        C(6),
        C(7),
        C(8),
        C(9),
        C(13),
        C(10),
        C(11),
        C(12),
        V(0),
        V(1),
        V(2),
        V(3),
        V(4),
        V(5),
        V(6),
        V(7),
        V(8),
        V(9),
        V(10),
        V(11),
        V(12),
        V(13),
        C(14),
        V(14),
        C(15),
        V(15),
        C(17),
        C(18),
        C(19),
        C(20),
        C(21),
        C(22),
        V(17),
        C(23),
        V(18),
        C(24),
        V(19),
        C(25),
        V(20),
        V(21),
        C(26),
        V(22),
        C(27),
        C(16),
        C(28),
        C(29),
        V(26),
        V(23),
        V(16),
        V(24),
        V(25),
        V(27),
        V(28),
        V(29),
    ]
};

/// All enabled values load before the first request, even for an empty ledger.
pub(crate) fn try_does_player_request<S: WipeoutDecisionSource>(
    source: &mut S,
) -> Result<bool, S::Error> {
    if source.disable_all()? {
        return Ok(false);
    }
    let mut targets: [Option<S::EnableTarget>; 30] = std::array::from_fn(|_| None);
    let mut enabled = [false; 30];
    for read in ENABLE_READS {
        match read {
            EnableRead::Capture(index) => targets[index] = Some(source.capture_enable(index)?),
            EnableRead::Value(index) => {
                enabled[index] = source.read_enable(
                    targets[index]
                        .as_ref()
                        .expect("source target precedes scalar"),
                )?;
            }
        }
    }
    let mut any = false;
    for (index, enabled) in enabled.into_iter().enumerate() {
        let requested = source.requested(index)?; // one retained request read per entry
        any |= requested && enabled;
        if requested {
            source.require_debug_overlay()?;
            // Enabled and ignored paths call the shared empty825187c0 at
            //9d04/9d5c and9d8c. The display scalar is still sampled every time.
            let _ = source.display_debug()?;
        }
    }
    Ok(any)
}

pub(crate) trait RunoutDecisionSource {
    type Error;
    fn request_count(&mut self) -> Result<u32, Self::Error>;
    fn requested(&mut self, index: usize) -> Result<bool, Self::Error>;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error>;
    fn flags_8cc(&mut self) -> Result<u32, Self::Error>;
    /// Actual Processed+220 dot global J, through caller's existing dot numeric.
    /// A direct Y read is not equivalent for unusual X/Z (NaN * zero).
    fn upright_dot(&mut self) -> Result<f32, Self::Error>;
}
///82dffe88: event count equality, two lazy flags, then ordered abs-dot threshold.
pub(crate) fn try_is_runout_requested<S: RunoutDecisionSource>(
    source: &mut S,
) -> Result<bool, S::Error> {
    if source.request_count()? != 1 {
        return Ok(false);
    }
    if !source.requested(2)? && !source.requested(24)? {
        return Ok(false);
    }
    if source.flags_8c4()? & 0x20 != 0 {
        return Ok(false);
    }
    if source.flags_8cc()? & 0x1000_0000 != 0 {
        return Ok(false);
    }
    let magnitude = f32::from_bits(source.upright_dot()?.to_bits() & 0x7fff_ffff);
    Ok(magnitude > f32::from_bits(0x3f4f_5c29))
}

pub(crate) trait HiLodWipeoutDecisionSource {
    type Error;
    fn runout(&mut self) -> Result<bool, Self::Error>;
    fn player_request(&mut self) -> Result<bool, Self::Error>;
    fn flags_8d0(&mut self) -> Result<u32, Self::Error>;
}
///82e00a50: only a false runout reaches cvars; animation is read last and lazily.
pub(crate) fn try_is_wipeout_requested<S: HiLodWipeoutDecisionSource>(
    source: &mut S,
) -> Result<bool, S::Error> {
    if source.runout()? {
        return Ok(false);
    }
    if source.player_request()? {
        return Ok(true);
    }
    Ok(source.flags_8d0()? & 0x40 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Decisions {
        disabled: bool,
        requests: [bool; 30],
        count: u32,
        dot: f32,
        log: Vec<String>,
        fail_capture: Option<usize>,
    }
    impl WipeoutDecisionSource for Decisions {
        type Error = &'static str;
        type EnableTarget = usize;
        fn disable_all(&mut self) -> Result<bool, Self::Error> {
            self.log.push("disable".into());
            Ok(self.disabled)
        }
        fn capture_enable(&mut self, index: usize) -> Result<usize, Self::Error> {
            self.log.push(format!("capture{index}"));
            if self.fail_capture == Some(index) {
                Err("capture")
            } else {
                Ok(index)
            }
        }
        fn read_enable(&mut self, index: &usize) -> Result<bool, Self::Error> {
            self.log.push(format!("value{index}"));
            Ok(*index != 29)
        }
        fn requested(&mut self, index: usize) -> Result<bool, Self::Error> {
            self.log.push(format!("request{index}"));
            Ok(self.requests[index])
        }
        fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
            self.log.push("overlay".into());
            Ok(())
        }
        fn display_debug(&mut self) -> Result<bool, Self::Error> {
            self.log.push("display".into());
            Ok(false)
        }
    }
    impl RunoutDecisionSource for Decisions {
        type Error = &'static str;
        fn request_count(&mut self) -> Result<u32, Self::Error> {
            self.log.push("count".into());
            Ok(self.count)
        }
        fn requested(&mut self, index: usize) -> Result<bool, Self::Error> {
            WipeoutDecisionSource::requested(self, index)
        }
        fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
            self.log.push("8c4".into());
            Ok(0)
        }
        fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
            self.log.push("8cc".into());
            Ok(0)
        }
        fn upright_dot(&mut self) -> Result<f32, Self::Error> {
            self.log.push("dot".into());
            Ok(self.dot)
        }
    }
    #[test]
    fn disabled_skips_captures_but_empty_ledger_does_not() {
        let mut source = Decisions {
            disabled: true,
            ..Decisions::default()
        };
        assert!(!try_does_player_request(&mut source).unwrap());
        assert_eq!(source.log, ["disable"]);
        source.disabled = false;
        source.log.clear();
        source.fail_capture = Some(26);
        assert_eq!(try_does_player_request(&mut source), Err("capture"));
        assert!(!source.log.iter().any(|s| s.starts_with("request")));
        assert_eq!(
            &source.log[source.log.len() - 3..],
            ["value20", "value21", "capture26"]
        );
    }
    #[test]
    fn all_controls_load_before_requests_and_scan_does_not_short_circuit() {
        let mut source = Decisions::default();
        source.requests[0] = true;
        source.requests[29] = true;
        assert!(try_does_player_request(&mut source).unwrap());
        assert_eq!(&source.log[61..64], ["request0", "overlay", "display"]);
        assert_eq!(
            &source.log[source.log.len() - 3..],
            ["request29", "overlay", "display"]
        );
        assert_eq!(
            source.log.iter().filter(|s| s.starts_with("value")).count(),
            30
        );
        let values: Vec<_> = source
            .log
            .iter()
            .filter(|s| s.starts_with("value"))
            .map(String::as_str)
            .collect();
        assert_eq!(
            &values[15..],
            [
                "value15", "value17", "value18", "value19", "value20", "value21", "value22",
                "value26", "value23", "value16", "value24", "value25", "value27", "value28",
                "value29"
            ]
        );
    }
    #[test]
    fn runout_count_is_events_and_threshold_is_ordered_strict() {
        let mut source = Decisions {
            count: 2,
            dot: 1.0,
            ..Decisions::default()
        };
        source.requests[2] = true;
        assert!(!try_is_runout_requested(&mut source).unwrap());
        assert_eq!(source.log, ["count"]);
        source.count = 1;
        for (dot, expected) in [
            (1.0, true),
            (-1.0, true),
            (f32::from_bits(0x3f4f_5c29), false),
            (f32::NAN, false),
        ] {
            source.dot = dot;
            source.log.clear();
            assert_eq!(try_is_runout_requested(&mut source).unwrap(), expected);
            assert_eq!(source.log, ["count", "request2", "8c4", "8cc", "dot"]);
        }
    }
    struct HiLod {
        runout: bool,
        player: bool,
        log: Vec<&'static str>,
    }
    impl HiLodWipeoutDecisionSource for HiLod {
        type Error = ();
        fn runout(&mut self) -> Result<bool, ()> {
            self.log.push("runout");
            Ok(self.runout)
        }
        fn player_request(&mut self) -> Result<bool, ()> {
            self.log.push("player");
            Ok(self.player)
        }
        fn flags_8d0(&mut self) -> Result<u32, ()> {
            self.log.push("animation");
            Ok(0x40)
        }
    }
    #[test]
    fn outer_runout_and_player_requests_keep_animation_lazy() {
        for (runout, player, expected, events) in [
            (true, true, false, vec!["runout"]),
            (false, true, true, vec!["runout", "player"]),
            (false, false, true, vec!["runout", "player", "animation"]),
        ] {
            let mut s = HiLod {
                runout,
                player,
                log: Vec::new(),
            };
            assert_eq!(try_is_wipeout_requested(&mut s).unwrap(), expected);
            assert_eq!(s.log, events);
        }
    }
}
