//! WorldPresentation::Update 827d3af8, ordinary (non-NIS) time branch.
//! The presentation's restore supplies the moment each frame shows: the restored
//! frame's completed ticks and the fraction toward it. A tick drawn without the
//! restore supplies its ticks and zero, and its repeated renders do not advance
//! simulation-driven foliage or water animation.
pub(crate) struct WorldAnimationClock {
    previous_frame: f32,
    previous_fraction: f32,
    seconds: f32,
    last_delta: f32,
}

impl Default for WorldAnimationClock {
    fn default() -> Self {
        // WorldPresentation ctor 827e9df0 suppresses the first update.
        Self {
            previous_frame: -15.0,
            previous_fraction: 0.0,
            seconds: 0.0,
            last_delta: 0.0,
        }
    }
}

impl WorldAnimationClock {
    pub(crate) fn update(&mut self, frame: u32, fraction: f32) -> f32 {
        let frame = frame as f32;
        let difference = frame - self.previous_frame;
        let delta = if difference >= 15.0 || difference <= -15.0 {
            0.0
        } else {
            ((fraction - self.previous_fraction + difference) * (1.0_f32 / 60.0)).max(0.0)
        };
        self.seconds += delta;
        self.last_delta = delta;
        self.previous_frame = frame;
        self.previous_fraction = fraction;
        self.seconds
    }

    /// The NIS branch (`0x827d3b6c..3bc8`): the active scene's `ScenePlayer::GetTimeDelta`, as
    /// the front end's scene advances the world's foliage and water.
    pub(crate) fn advance_scene(&mut self, delta: f32) -> f32 {
        self.seconds += delta;
        self.last_delta = delta;
        self.seconds
    }

    pub(crate) fn seconds(&self) -> f32 {
        self.seconds
    }
    pub(crate) fn last_delta(&self) -> f32 {
        self.last_delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_frames_rewinds_and_large_jumps_follow_native_time_gate() {
        let mut clock = WorldAnimationClock::default();
        assert_eq!(clock.update(0, 0.0), 0.0);
        let first = clock.update(1, 0.0);
        assert_eq!(first, 1.0 / 60.0);
        assert_eq!(clock.update(1, 0.0), first);
        assert_eq!(clock.update(0, 0.0), first);
        assert_eq!(clock.update(15, 0.0), first);
        assert_eq!(clock.update(0, 0.0), first);
        let half = clock.update(0, 0.5);
        assert_eq!(half, first + 0.5 / 60.0);
        assert_eq!(clock.update(1, 0.0), half + 0.5 / 60.0);
    }
}
