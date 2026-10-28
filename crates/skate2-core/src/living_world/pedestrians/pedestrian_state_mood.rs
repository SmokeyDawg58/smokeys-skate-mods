//! Retained mood timers: UpdateMood82e9e278, timer functor82e8dd10.
use super::{PedestrianStateStorage, MoodEntry};

impl PedestrianStateStorage {
    /// IsMoodSuppressed82e827d0 has distinct ordered tests for the two targets.
    pub(crate) fn is_mood_suppressed(&self, mood: &MoodEntry) -> bool {
        let restriction = |target: crate::living_world_chase::TargetableId| {
            if target.kind == -1 || target.id < 0 { return None; }
            self.knowledge.iter().find(|entry| entry.target == target).map(|entry| entry.mood_restriction)
        };
        let perpetrator = restriction(mood.perpetrator).is_some_and(|v|v > 0.0);
        restriction(mood.victim).map(|v| !(v <= 0.0) || perpetrator).unwrap_or(perpetrator)
    }
    pub(crate) fn merge_mood(&mut self, incoming: &MoodEntry) -> bool {
        let Some(entry) = self.moods.iter_mut().find(|entry| entry.perpetrator == incoming.perpetrator
            && entry.victim == incoming.victim && entry.event_type == incoming.event_type) else { return false; };
        // Source fsel selects the incoming age on an ordered nonnegative
        // difference; it is not Rust min's unordered-value policy.
        if entry.since_refresh - incoming.since_refresh >= 0.0 { entry.since_refresh = incoming.since_refresh; }
        entry.exposure += incoming.exposure;
        entry.occurrences = entry.occurrences.wrapping_add(incoming.occurrences);
        entry.position = incoming.position;
        true
    }
    /// AddNewMood82e9e528 admits ten entries. Only an Actor-related event can
    /// displace the least-important existing entry when the storage is full.
    pub(crate) fn add_new_mood<E>(&mut self, incoming: &MoodEntry,
        mut priority: impl FnMut(i32) -> Result<u32, E>) -> Result<bool, E> {
        if self.moods.len() >= 10 {
            if incoming.perpetrator.kind != 0 && incoming.victim.kind != 0 { return Ok(false); }
            let mut selected = None;
            let mut least_priority = 0;
            let mut oldest = -1.0;
            for (index, entry) in self.moods.iter().enumerate() {
                if priority(entry.event_type)? > least_priority
                    || (priority(entry.event_type)? == least_priority && entry.since_refresh > oldest) {
                    least_priority = priority(entry.event_type)?;
                    oldest = entry.since_refresh;
                    selected = Some(index);
                }
            }
            let Some(index) = selected else { return Ok(false); };
            self.moods.remove(index);
        }
        self.moods.push(incoming.clone());
        Ok(true)
    }
    pub(crate) fn update_mood<E>(
        &mut self,
        dt: f32,
        mut expire_time: impl FnMut(i32) -> Result<f32, E>,
    ) -> Result<(), E> {
        // All entries age before the first category lookup. Cooldowns cross
        // below zero for one frame, just like the native old-value branch.
        for entry in &mut self.moods {
            for (_, timer) in &mut entry.probability_timers {
                *timer -= dt;
            }
            entry.probability_timers.retain(|(_, timer)| *timer > 0.0);
            entry.since_refresh += dt;
            entry.cooldown = if entry.cooldown > 0.0 {
                entry.cooldown - dt
            } else {
                0.0
            };
        }
        // Native find_if and remove_copy_if use different ordered comparisons.
        // Preserve that distinction for unordered values and keep source order.
        let mut first = None;
        for (index, entry) in self.moods.iter().enumerate() {
            if entry.since_refresh > expire_time(entry.event_type)? {
                first = Some(index);
                break;
            }
        }
        if let Some(mut output) = first {
            for input in output + 1..self.moods.len() {
                let entry = &self.moods[input];
                if entry.since_refresh <= expire_time(entry.event_type)? {
                    self.moods[output] = entry.clone();
                    output += 1;
                }
            }
            self.moods.truncate(output);
        }
        Ok(())
    }
}
