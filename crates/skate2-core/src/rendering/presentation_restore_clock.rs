//! `DataSystem::RealTimeController` (dataman 3.00.00): the clock `DataSysMan::StepTime`
//! 0x8277c1d0 advances once per rendered frame with that frame's elapsed time
//! (`AppMan::RenderLoop` 0x82989fc0 passes `GameModule::Render` the stopwatch time,
//! clamped to `kMaxDtBeforeRenderDrop`). It picks the submitted frame
//! `DataSysMan::RestoreFrame` 0x8277f040 restores and how far each presentation blends
//! toward it from the frame before.

use std::collections::VecDeque;

/// The invalid frame number of `DataTracker` and `RealTimeController`.
pub const INVALID_FRAME: u32 = 0x01ff_ffff;

/// `kNormalUpdateRate`, which `Presentation::EnterRealTime` 0x827f1130 gives the clock.
pub const NORMAL_UPDATE_RATE: f32 = f32::from_bits(0x3c88_8889);

/// `AppMan::kMaxDtBeforeRenderDrop`'s default: the longest frame time the render loop
/// passes on.
pub const MAX_DT_BEFORE_RENDER_DROP: f32 = 1.9;

/// `ITimeController::TimeInfo` as `RealTimeController::GetTickCount` 0x8298fd30 fills it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeInfo {
    pub frame: u32,
    pub fraction: f32,
    /// `NormalSimRate / LastKnownSimRate`, zero without a positive rate.
    pub delta: f32,
}

/// The submitted frames the clock walks: `DataTracker`'s frame numbers.
pub trait FrameTrack {
    /// `DataMan::GetLastFrameNumberWritten` 0x82992138: zero while nothing is tracked.
    fn last_written(&self) -> u32;
    /// `DataTracker::SkipForward` 0x829930f8: `count` frames on from the frame closest to
    /// `start`, never onto the head, so it stops at the newest frame.
    fn skip_forward(&self, start: u32, count: u32) -> u32;
}

/// `RealTimeController::Implementation`.
#[derive(Clone, Debug)]
pub struct RealTimeController {
    frame_to_restore: u32,
    fractional: f32,
    last_frame: u32,
    last_fractional: f32,
    unaccounted_dt: f32,
    normal_sim_rate: f32,
    last_known_sim_rate: f32,
}

impl RealTimeController {
    /// `Implementation::Implementation` 0x82992848.
    pub fn new(frames: &impl FrameTrack, normal_update_rate: f32) -> Self {
        Self {
            frame_to_restore: INVALID_FRAME,
            fractional: 1.0,
            last_frame: frames.last_written(),
            last_fractional: 0.0,
            unaccounted_dt: 0.0,
            normal_sim_rate: normal_update_rate,
            last_known_sim_rate: normal_update_rate,
        }
    }

    /// `Implementation::Update` 0x82993f28. `current_rate` is the query's
    /// `GetCurrentUpdateRate`: `SimPresentation`'s last known simulation rate.
    pub fn update(&mut self, dt: f32, frames: &impl FrameTrack, current_rate: f32) {
        if dt <= 0.0 {
            return;
        }
        let written = frames.last_written();
        if written == INVALID_FRAME {
            return;
        }
        self.last_known_sim_rate = current_rate;
        if current_rate <= 0.0 {
            return;
        }
        let inverse = 1.0 / current_rate;
        let step = inverse * self.normal_sim_rate;
        self.unaccounted_dt = step.mul_add(dt, self.unaccounted_dt);
        let mut elapsed = 0_i32;
        if self.unaccounted_dt > current_rate {
            loop {
                elapsed += 1;
                self.unaccounted_dt -= current_rate;
                if !(self.unaccounted_dt > self.last_known_sim_rate) {
                    break;
                }
            }
        }
        let fraction = inverse * self.unaccounted_dt;
        if self.last_frame == INVALID_FRAME {
            self.last_frame = written;
        } else {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "fctiwz of the ceiling, which saturates as the cast does"
            )]
            let lag = (current_rate / self.normal_sim_rate).ceil() as i32;
            let budget = written
                .wrapping_sub(lag.cast_unsigned())
                .wrapping_sub(self.last_frame)
                .wrapping_add(3)
                .cast_signed();
            let skip = if elapsed > budget { elapsed } else { budget };
            let frame = frames.skip_forward(self.last_frame, skip.cast_unsigned());
            self.frame_to_restore = frame;
            self.fractional = if frame != self.last_frame {
                fraction
            } else if elapsed != 0 {
                1.0
            } else {
                let sum = self.last_fractional + fraction;
                if 1.0 - sum >= 0.0 { sum } else { 1.0 }
            };
        }
        self.last_fractional = self.fractional;
        self.last_frame = self.frame_to_restore;
    }

    /// `RealTimeController::GetTickCount` 0x8298fd30.
    pub fn time_info(&self) -> TimeInfo {
        let delta = if self.last_known_sim_rate > 0.0 {
            self.normal_sim_rate / self.last_known_sim_rate
        } else {
            0.0
        };
        TimeInfo {
            frame: self.frame_to_restore,
            fraction: self.fractional,
            delta,
        }
    }
}

/// The submitted presentation frames a `DataTracker` keeps, oldest first, numbered in
/// submission order.
#[derive(Debug)]
pub struct PresentationFrames<T> {
    frames: VecDeque<(u32, T)>,
    capacity: usize,
    next: u32,
}

impl<T> PresentationFrames<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            frames: VecDeque::with_capacity(capacity),
            capacity: capacity.max(1),
            next: 0,
        }
    }

    /// Track a newly submitted frame, dropping the oldest beyond the capacity.
    pub fn submit(&mut self, data: T) -> u32 {
        let number = self.next;
        self.next = if number + 1 >= INVALID_FRAME {
            0
        } else {
            number + 1
        };
        if self.frames.len() == self.capacity {
            self.frames.pop_front();
        }
        self.frames.push_back((number, data));
        number
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn get(&self, frame: u32) -> Option<&T> {
        self.frames
            .iter()
            .find(|(number, _)| *number == frame)
            .map(|(_, data)| data)
    }

    /// `DataTracker::ShuffleBack` 0x82990488: the frame submitted before `frame`.
    pub fn previous(&self, frame: u32) -> Option<&T> {
        let index = self
            .frames
            .iter()
            .position(|(number, _)| *number == frame)?;
        index
            .checked_sub(1)
            .and_then(|index| self.frames.get(index))
            .map(|(_, data)| data)
    }

    pub fn newest(&self) -> Option<&T> {
        self.frames.back().map(|(_, data)| data)
    }

    /// The tracked frames, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.frames.iter().map(|(number, data)| (*number, data))
    }
}

impl<T> FrameTrack for PresentationFrames<T> {
    fn last_written(&self) -> u32 {
        self.frames.back().map_or(0, |(number, _)| *number)
    }

    fn skip_forward(&self, start: u32, count: u32) -> u32 {
        if count == 0 || self.frames.is_empty() {
            return start;
        }
        let last = self.frames.len() - 1;
        let index = self
            .frames
            .iter()
            .position(|(number, _)| *number == start)
            .unwrap_or_else(|| {
                if self
                    .frames
                    .front()
                    .is_some_and(|(oldest, _)| start < *oldest)
                {
                    0
                } else {
                    last
                }
            });
        let index = index
            .saturating_add(usize::try_from(count).unwrap_or(usize::MAX))
            .min(last);
        self.frames[index].0
    }
}

#[cfg(test)]
#[path = "presentation_restore_clock_tests.rs"]
mod tests;
