//! APT's movie clock and clip playback. The clock belongs to the top-level
//! animation, not each imported movie or each render frame (`tick`82a21c10).
use super::apt_timeline::{FrameEffect, Timeline, TimelineError};
use skate2_formats::apt::Movie;

#[derive(Clone, Debug, Default)]
pub struct MovieClock {
    remainder_ms: u32,
}
impl MovieClock {
    pub fn remainder_ms(&self) -> u32 {
        self.remainder_ms
    }

    /// Native tick loops through complete authored intervals, retaining the
    /// fractional remainder. Saved-input mode deliberately permits one tick.
    /// The caller runs interval timers, display lists, actions, input and
    /// linker (in that order) once for each returned step, before rendering.
    pub fn advance(
        &mut self,
        elapsed_ms: u32,
        frame_ms: std::num::NonZeroU32,
        saved_input_mode: bool,
    ) -> u32 {
        let total = self.remainder_ms.wrapping_add(elapsed_ms);
        let available = total / frame_ms.get();
        let steps = if saved_input_mode {
            available.min(1)
        } else {
            available
        };
        self.remainder_ms = total - steps * frame_ms.get();
        steps
    }
}

#[derive(Clone, Debug)]
pub struct ClipPlayback {
    timeline: Timeline,
    playing: bool,
}
impl Default for ClipPlayback {
    fn default() -> Self {
        Self {
            timeline: Timeline::default(),
            playing: true,
        }
    }
}
impl ClipPlayback {
    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }
    pub fn playing(&self) -> bool {
        self.playing
    }
    pub fn stop(&mut self) {
        self.playing = false;
    }
    pub fn play(&mut self) {
        self.playing = true;
    }

    /// `_gotoAndX`82a1cf48: even a same-frame gotoAndPlay resumes playback;
    /// the display list itself decides whether target actions must run.
    pub fn goto_label(
        &mut self,
        movie: &Movie,
        label: &str,
        play: bool,
    ) -> Result<Vec<FrameEffect>, TimelineError> {
        let effects = self.timeline.goto_label(movie, label)?;
        self.playing = play;
        Ok(effects)
    }

    /// Numeric ActionScript frame arguments are one-based; the retained
    /// display list uses zero-based indices, like its authored frame table.
    pub fn goto_number(&mut self, movie: &Movie, frame: usize, play: bool)
        -> Result<Vec<FrameEffect>, TimelineError> {
        // _gotoAndX82a1d00c ignores numeric zero, including playback state.
        if frame == 0 { return Ok(Vec::new()); }
        let effects = self.timeline.goto(movie, frame - 1)?;
        self.playing = play;
        Ok(effects)
    }

    /// One authored interval (`AptCIH::tick`82a1b580). A stopped parent still
    /// ticks its children: traversal is the display-list owner's job and must
    /// not return early just because this method emits no parent controls.
    pub fn tick(&mut self, movie: &Movie) -> Result<Vec<FrameEffect>, TimelineError> {
        if self.timeline.frame().is_none() {
            return self.timeline.goto(movie, 0);
        }
        if !self.playing || movie.frames.len() == 1 {
            return Ok(Vec::new());
        }
        let target = (self.timeline.frame().unwrap() + 1) % movie.frames.len();
        self.timeline.goto(movie, target)
    }
}

#[cfg(test)]
#[path = "apt_playback_tests.rs"]
mod tests;
