//! The NIS load-screen transition on the presentation side:
//! `Sk8::PresVisualTransitionManager` (`BeginTransition` `0x827dfde8`,
//! `EndTransition` `0x827dfed8`, `TransitionDoneCallback` `0x827d0e28`) running a
//! `TransitionPresentation` effect (`StartTransition` `0x827db108`, `Update`
//! `0x827d0ee8`). `BeginTransition` maps `eTransitionType` 1 and 2 to
//! `VISUAL_TRANSITION_TYPE_BLACK_FADE`, 3 to `WHITE_FADE` and every other type to
//! `BLOOM_TO_WHITE`; the NIS flow only ever passes `TRANSITION_TYPE_NISLOADSCREEN`
//! (5), so only the bloom-to-white effect is modelled here.

/// `Sk8::eTransitionType`.
pub mod transition_type {
    pub const NONE: i32 = 0;
    pub const ENTER_GAME: i32 = 1;
    pub const LEAVE_GAME: i32 = 2;
    pub const TELEPORT: i32 = 3;
    pub const LOADSCREEN: i32 = 4;
    pub const NISLOADSCREEN: i32 = 5;
}

/// The completion callbacks the NIS flow hands the presentation transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Callback {
    /// `SimulationBasedTransitionEndCallback` (`0x827dffa8`):
    /// `cMsgVisualTransitionComplete` to the simulation.
    SimulationBasedEnd,
    /// `SceneManagerImpl::StopNISTransitionOutCallback` (`0x8286bc48`):
    /// `cMsgNISTransitionOutComplete` to the simulation.
    StopNisTransitionOut,
    /// `SceneManagerImpl::StopNISTransitionInCallback` (`0x8286e300`).
    StopNisTransitionIn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    None,
    In,
    Out,
}

/// How long the bloom takes each way: `TransitionPresentation::StartTransition` (`0x827db108`)
/// gives the original's effect a second; the port's takes a fifth of that.
const DURATION: f32 = 0.2;

/// `BloomToWhiteTransitionEffect` (0x10 bytes): its time and duration ([`DURATION`]),
/// `mFadeUp` for the IN direction.
#[derive(Clone, Copy, Debug, PartialEq)]
struct BloomToWhite {
    time: f32,
    duration: f32,
    fade_up: bool,
}

impl BloomToWhite {
    /// `Update` (`0x827d55c0`): the value it writes to both general post-fx
    /// blocks' `bloomToWhiteValue` and `AudioSystem::mNISFadeAmount`.
    fn update(&mut self, dt: f32) -> f32 {
        self.time -= dt;
        if self.time > 0.0 {
            if self.fade_up { (self.duration - self.time) / self.duration } else { self.time / self.duration }
        } else if self.fade_up {
            1.0
        } else {
            0.0
        }
    }

    /// `IsComplete` (`0x827d0f98`, shared by both effects).
    fn is_complete(&self) -> bool {
        self.time <= 0.0
    }
}

/// `Sk8::PresVisualTransitionManager` with its `TransitionPresentation` effect.
#[derive(Debug)]
pub struct PresTransitions {
    in_transition: bool,
    /// `mVisualTransitionType` is bloom-to-white once a transition began; it
    /// survives the done callback so `EndTransition` replays it.
    bloom: bool,
    direction: Direction,
    callback: Option<Callback>,
    effect: Option<BloomToWhite>,
    /// `mGameGeneralPostfxData.bloomToWhiteValue` (and the ICE block's).
    pub bloom_to_white: f32,
}

impl Default for PresTransitions {
    fn default() -> Self {
        Self { in_transition: false, bloom: false, direction: Direction::None, callback: None, effect: None, bloom_to_white: 0.0 }
    }
}

impl PresTransitions {
    /// `mIsInTransition`.
    #[must_use]
    pub fn in_transition(&self) -> bool {
        self.in_transition
    }

    /// `mVisualTransitionDirection`.
    #[must_use]
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// `BeginTransition(TRANSITION_TYPE_NISLOADSCREEN, callback)`: the bloom
    /// to white, direction IN. Asserts (and does nothing) while a transition
    /// runs.
    pub fn begin(&mut self, callback: Callback) {
        if self.in_transition {
            return;
        }
        self.callback = Some(callback);
        self.in_transition = true;
        self.bloom = true;
        self.direction = Direction::In;
        self.start(true);
    }

    /// `EndTransition(callback)`: the same visual type, direction OUT; with no
    /// visual type the callback runs at once (returned to the caller).
    #[must_use]
    pub fn end(&mut self, callback: Callback) -> Option<Callback> {
        if self.in_transition {
            return None;
        }
        self.callback = Some(callback);
        self.in_transition = true;
        if self.bloom {
            self.direction = Direction::Out;
            self.start(false);
            self.bloom = false;
            None
        } else {
            self.in_transition = false;
            self.direction = Direction::None;
            self.callback = None;
            Some(callback)
        }
    }

    /// `TransitionPresentation::StartTransition`: asserts no effect runs.
    fn start(&mut self, fade_up: bool) {
        if self.effect.is_none() {
            self.effect = Some(BloomToWhite { time: DURATION, duration: DURATION, fade_up });
        }
    }

    /// `TransitionPresentation::Update(dT)`: the effect advances; once complete
    /// it is deleted and `TransitionDoneCallback` clears the manager's
    /// transition (keeping its visual type) and returns the user callback.
    #[must_use]
    pub fn update(&mut self, dt: f32) -> Option<Callback> {
        let effect = self.effect.as_mut()?;
        self.bloom_to_white = effect.update(dt);
        if !effect.is_complete() {
            return None;
        }
        self.effect = None;
        self.in_transition = false;
        self.direction = Direction::None;
        self.callback.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bloom_in_then_out_takes_a_fifth_of_a_second_each() {
        let mut t = PresTransitions::default();
        t.begin(Callback::SimulationBasedEnd);
        assert!(t.in_transition());
        let mut done = None;
        for _ in 0..12 {
            done = t.update(1.0 / 60.0);
            assert_eq!(done, None);
        }
        assert!(t.bloom_to_white > 0.99 && t.bloom_to_white < 1.0);
        while done.is_none() {
            done = t.update(1.0 / 60.0);
        }
        assert_eq!(done, Some(Callback::SimulationBasedEnd));
        assert_eq!(t.bloom_to_white, 1.0);
        assert!(!t.in_transition());
        assert_eq!(t.end(Callback::StopNisTransitionOut), None);
        assert_eq!(t.direction(), Direction::Out);
        let _ = t.update(0.05);
        assert!((t.bloom_to_white - 0.75).abs() < 1e-6);
        assert_eq!(t.update(0.15), Some(Callback::StopNisTransitionOut));
        assert_eq!(t.bloom_to_white, 0.0);
        // No visual type left: the next end calls back at once.
        assert_eq!(t.end(Callback::StopNisTransitionOut), Some(Callback::StopNisTransitionOut));
    }
}
