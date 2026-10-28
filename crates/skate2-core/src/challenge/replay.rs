//! `DataSystem::DataMan` as the challenges read it: the frames the replay
//! recorder holds (`DataTracker`) and the pins on them
//! (`PinnedFrameManager`). The frames' data stays with the session that
//! submits it; this index decides which frames it keeps.
//!
//! The original budgets its compressed buffer in bytes (`desiredsize`
//! 12 MiB, `pinratio` 0.6); the port's frames are uncompressed, so the frame
//! count stands in for the size: at most `numframes` frames, and pins may
//! hold at most 0.6 of them. A frame takes the simulation tick it was
//! submitted on: the scripts pin from `Time.GetVisualGameTick` to
//! `ChallengeReplay.GetLastFrameNumberWritten`, one count.

use std::collections::VecDeque;

/// `DataManInitialization::numframes` (`DataSysMan::DataSysMan` `0x8277f600`).
pub const MAX_FRAMES: usize = 5400;
/// `DataManInitialization::pinratio`.
const PIN_RATIO: f32 = 0.6;
/// `PinnedFrameManager::mPins`.
const PIN_SLOTS: usize = 4;

/// `PinnedFrameManager::PinInfo`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pin {
    id: i32,
    start: u32,
    end: u32,
    /// `additionalframestopin`: frames still to pin as they are stored.
    future: i32,
}

/// `DataSystem::DataMan`'s tracker and pins.
#[derive(Clone, Debug, Default)]
pub struct ReplaySystem {
    /// The tracked frames, oldest first (`mTailFrameIndex` to
    /// `mLastValidFrameIndex`).
    frames: VecDeque<u32>,
    pins: [Option<Pin>; PIN_SLOTS],
    /// `mCurPinID`.
    next_pin: i32,
}

impl ReplaySystem {
    /// `DataTracker::ResetSystem` (`0x82996910`): no frames, no pins.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// `DataMan::InternalSubmit` (`0x82997968`): the frame joins the tracker
    /// under its number (a number not past the newest is not stored);
    /// `PinnedFrameManager::FrameStored` (`0x82993820`) extends every pin
    /// still waiting for future frames. Returns whether it was stored and the
    /// frames dropped to make room: the oldest no pin holds.
    pub fn store_frame(&mut self, frame: u32) -> (bool, Vec<u32>) {
        if self.frames.back().is_some_and(|&newest| frame <= newest) {
            return (false, Vec::new());
        }
        self.frames.push_back(frame);
        for pin in self.pins.iter_mut().flatten() {
            if pin.future > 0 {
                pin.end = frame;
                pin.future -= 1;
            }
        }
        let mut dropped = Vec::new();
        while self.frames.len() > MAX_FRAMES {
            let Some(at) = self.frames.iter().position(|&f| !self.pinned(f)) else { break };
            if let Some(f) = self.frames.remove(at) {
                dropped.push(f);
            }
        }
        (true, dropped)
    }

    fn pinned(&self, frame: u32) -> bool {
        self.pins.iter().flatten().any(|pin| (pin.start..=pin.end).contains(&frame))
    }

    /// Whether `frame` is held.
    #[must_use]
    pub fn holds(&self, frame: u32) -> bool {
        self.frames.binary_search(&frame).is_ok()
    }

    /// `DataMan::GetLastFrameNumberWritten` (`0x82992138`): 0 while the
    /// tracker is empty.
    #[must_use]
    pub fn last_frame_number_written(&self) -> u32 {
        self.frames.back().copied().unwrap_or(0)
    }

    /// `DataMan::GetOldestFrameNumberWritten` (`0x82990658`).
    #[must_use]
    pub fn oldest_frame_number_written(&self) -> u32 {
        self.frames.front().copied().unwrap_or(0)
    }

    /// `DataMan::GetNumFramesTracked` (`0x829921b8`).
    #[must_use]
    pub fn num_frames_tracked(&self) -> usize {
        self.frames.len()
    }

    /// `DataTracker::GetClosestIndexToFrameNumber(frame, false)`: from the
    /// nearer end, the first frame at or after `frame` (scanning forward) or
    /// the last at or before it (scanning back), clamped to the tracked range.
    fn closest(&self, frame: u32) -> Option<usize> {
        let (&oldest, &newest) = (self.frames.front()?, self.frames.back()?);
        let frame = frame.clamp(oldest, newest);
        Some(match self.frames.binary_search(&frame) {
            Ok(at) => at,
            Err(at) if frame - oldest < newest - frame => at,
            Err(at) => at.saturating_sub(1),
        })
    }

    /// `DataTracker::SkipForward` (`0x829930f8`): `count` tracked frames on
    /// from the one closest to `frame`, stopping at the newest.
    #[must_use]
    pub fn skip_forward(&self, frame: u32, count: u32) -> u32 {
        if count == 0 {
            return frame;
        }
        let Some(at) = self.closest(frame) else { return frame };
        let to = at.saturating_add(count as usize).min(self.frames.len() - 1);
        self.frames[to]
    }

    /// `DataTracker::SkipBackward` (`0x82993050`): `count` tracked frames
    /// back, stopping at the oldest.
    #[must_use]
    pub fn skip_backward(&self, frame: u32, count: u32) -> u32 {
        if count == 0 {
            return frame;
        }
        let Some(at) = self.closest(frame) else { return frame };
        self.frames[at.saturating_sub(count as usize)]
    }

    /// The tracked frame after `frame` (`GetNextFrameNumber`), if any.
    #[must_use]
    pub fn next_frame(&self, frame: u32) -> Option<u32> {
        let at = self.frames.binary_search(&frame).ok()?;
        self.frames.get(at + 1).copied()
    }

    /// The tracked frame before `frame` (`GetPreviousFrameNumber`), if any.
    #[must_use]
    pub fn previous_frame(&self, frame: u32) -> Option<u32> {
        let at = self.frames.binary_search(&frame).ok()?;
        at.checked_sub(1).and_then(|at| self.frames.get(at)).copied()
    }

    /// `DataMan::RequestPin` (`0x82995040`) through
    /// `PinnedFrameManager::RequestPin` (`0x829935b8`): a free slot pins the
    /// frames from the one closest to `from` up to the last at or before `to`,
    /// then up to `future` frames after it already stored; the rest are
    /// pinned as they are stored. -1 when no slot is free, the range is empty,
    /// or the pins would hold more than `pinratio` of the tracker
    /// (`DataTracker::PinFrames` `0x82992af8`).
    pub fn request_pin(&mut self, from: u32, to: u32, future: i32) -> i32 {
        let Some(slot) = self.pins.iter().position(Option::is_none) else { return -1 };
        let Some(start) = self.closest(from) else { return -1 };
        if self.frames[start] >= to {
            return -1;
        }
        // ShuffleForwardToClosestIndexToFrameNumber (0x82991358).
        let mut end = start;
        while self.frames.get(end + 1).is_some_and(|&f| f <= to) {
            end += 1;
        }
        let mut future = future;
        while future > 0 && end + 1 < self.frames.len() {
            end += 1;
            future -= 1;
        }
        let range = end - start + 1 + usize::try_from(future.max(0)).unwrap_or(0);
        let pinned = self.frames.iter().filter(|&&f| self.pinned(f)).count();
        #[expect(clippy::cast_precision_loss, reason = "the ratio test")]
        if (pinned + range) as f32 / MAX_FRAMES as f32 >= PIN_RATIO {
            return -1;
        }
        let id = self.next_pin;
        self.next_pin = self.next_pin.wrapping_add(1);
        self.pins[slot] = Some(Pin { id, start: self.frames[start], end: self.frames[end], future });
        id
    }

    /// `PinnedFrameManager::ReleasePin` (`0x82993760`).
    pub fn release_pin(&mut self, id: i32) {
        if let Some(slot) = self.pins.iter_mut().find(|pin| pin.is_some_and(|pin| pin.id == id)) {
            *slot = None;
        }
    }

    /// `DataMan::GetNumActivePins` (`0x829921c0`).
    #[must_use]
    pub fn num_active_pins(&self) -> usize {
        self.pins.iter().flatten().count()
    }

    /// `DataMan::GetPinStartFrame` (`0x82993e30`).
    #[must_use]
    pub fn pin_start_frame(&self, id: i32) -> Option<u32> {
        self.pins.iter().flatten().find(|pin| pin.id == id).map(|pin| pin.start)
    }

    /// `DataMan::GetPinEndFrame` (`0x82993e78`).
    #[must_use]
    pub fn pin_end_frame(&self, id: i32) -> Option<u32> {
        self.pins.iter().flatten().find(|pin| pin.id == id).map(|pin| pin.end)
    }

    /// Whether the pin still waits for frames to be stored.
    #[must_use]
    pub fn pin_waits_for_frames(&self, id: i32) -> bool {
        self.pins.iter().flatten().any(|pin| pin.id == id && pin.future > 0)
    }

    /// `DataTracker::EraseAllButLastNFrames` (`0x82994368`): the tail moves
    /// to the frame `n - 1` before the newest.
    pub fn erase_all_but_last_n_frames(&mut self, n: i32) {
        let keep = usize::try_from(n.max(1)).unwrap_or(1);
        while self.frames.len() > keep {
            self.frames.pop_front();
        }
    }

    /// The frames held, oldest first.
    pub fn frames(&self) -> impl Iterator<Item = u32> + '_ {
        self.frames.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(n: u32) -> ReplaySystem {
        let mut replay = ReplaySystem::default();
        for frame in 0..n {
            replay.store_frame(frame);
        }
        replay
    }

    #[test]
    fn frames_keep_their_ticks_and_the_empty_tracker_reads_zero() {
        let mut replay = ReplaySystem::default();
        assert_eq!(replay.last_frame_number_written(), 0);
        assert_eq!(replay.store_frame(40), (true, Vec::new()));
        assert_eq!(replay.store_frame(40), (false, Vec::new()), "the same tick again");
        assert_eq!(replay.store_frame(41), (true, Vec::new()));
        assert_eq!(replay.last_frame_number_written(), 41);
        assert_eq!(replay.oldest_frame_number_written(), 40);
    }

    #[test]
    fn pins_hold_their_range_and_extend_into_future_frames() {
        let mut replay = stored(100);
        let pin = replay.request_pin(10, 50, 20);
        assert_eq!(pin, 0);
        assert_eq!((replay.pin_start_frame(pin), replay.pin_end_frame(pin)), (Some(10), Some(70)));
        let late = replay.request_pin(90, 99, 30);
        assert_eq!(replay.pin_end_frame(late), Some(99));
        assert!(replay.pin_waits_for_frames(late));
        for frame in 100..130 {
            replay.store_frame(frame);
        }
        assert_eq!(replay.pin_end_frame(late), Some(129));
        assert!(!replay.pin_waits_for_frames(late));
        assert_eq!(replay.request_pin(50, 50, 0), -1, "an empty range");
    }

    #[test]
    fn the_oldest_unpinned_frames_go_first() {
        let mut replay = stored(10);
        let pin = replay.request_pin(0, 5, 0);
        let count = u32::try_from(MAX_FRAMES).unwrap_or(u32::MAX);
        for frame in 10..10 + count {
            replay.store_frame(frame);
        }
        assert!(replay.holds(0) && replay.holds(5), "pinned frames stay");
        assert!(!replay.holds(6));
        assert_eq!(replay.num_frames_tracked(), MAX_FRAMES);
        replay.release_pin(pin);
        replay.store_frame(10 + count);
        assert!(!replay.holds(0));
    }

    #[test]
    fn four_slots_and_the_pin_ratio_limit_pins() {
        let mut replay = stored(u32::try_from(MAX_FRAMES).unwrap_or(u32::MAX));
        assert_eq!(replay.request_pin(0, 3500, 0), -1, "over 0.6 of the tracker");
        let pins: Vec<i32> = (0..4).map(|i| replay.request_pin(i * 10, i * 10 + 5, 0)).collect();
        assert_eq!(pins, [0, 1, 2, 3]);
        assert_eq!(replay.request_pin(100, 110, 0), -1, "no slot");
        replay.release_pin(1);
        assert_eq!(replay.request_pin(100, 110, 0), 4);
    }

    #[test]
    fn skips_stop_at_the_ends() {
        let replay = stored(20);
        assert_eq!(replay.skip_forward(5, 3), 8);
        assert_eq!(replay.skip_forward(18, 5), 19);
        assert_eq!(replay.skip_backward(2, 5), 0);
        let mut replay = replay;
        replay.erase_all_but_last_n_frames(2);
        assert_eq!(replay.frames().collect::<Vec<_>>(), [18, 19]);
    }
}
