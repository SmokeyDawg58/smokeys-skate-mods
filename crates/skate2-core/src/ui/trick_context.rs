//! TricksBackEnd's seven display slots. AddBigTrick825650b0,
//! UpdateContextTrickLanded825651d0 and UpdateContextTimers8255c438.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    Big { index: i32, achievement: i32, measurement: f32 },
    Entity { entity_type: i32, is_entity: bool, count: i32, context_type: i32 },
}
#[derive(Clone, Debug, Default)]
pub struct Slot {
    pub entry: Option<Entry>,
    timer: i32,
}
#[derive(Clone, Debug, Default)]
pub struct ContextDisplay {
    pub slots: [Slot; 7],
    active_big: usize,
    active_entity: usize,
    changed: bool,
}
impl ContextDisplay {
    pub fn clear(&mut self) { *self = Self::default(); }
    pub fn take_changed(&mut self) -> bool { std::mem::take(&mut self.changed) }
    pub fn add(&mut self, value: Entry) {
        let big = matches!(value, Entry::Big { .. });
        // Native limit check precedes even refresh of an existing entry.
        if (if big { self.active_big } else { self.active_entity }) >= 3 { return; }
        let mut found = false;
        for slot in &mut self.slots {
            match (slot.entry.as_mut(), &value) {
                (Some(Entry::Big { index, achievement, measurement }),
                    Entry::Big { index: next, achievement: level, measurement: amount }) if index == next => {
                    found = true; slot.timer = 2000;
                    if level > achievement { *achievement = *level; self.changed = true; }
                    if amount > measurement { *measurement = *amount; self.changed = true; }
                }
                (Some(Entry::Entity { entity_type, is_entity, count, context_type }),
                    Entry::Entity { entity_type: next, is_entity: entity, count: amount, context_type: context })
                    if entity_type == next && is_entity == entity && context_type == context => {
                    found = true; slot.timer = 2000;
                    if amount > count { *count = *amount; self.changed = true; }
                }
                _ => {}
            }
        }
        if !found {
            if let Some(slot) = self.slots.iter_mut().find(|s| s.entry.is_none()) {
                slot.entry = Some(value); slot.timer = 2000; self.changed = true;
                if big { self.active_big += 1; } else { self.active_entity += 1; }
            }
        }
    }
    pub fn update(&mut self, milliseconds: u32) {
        for slot in &mut self.slots {
            if let Some(entry) = &slot.entry {
                slot.timer = slot.timer.wrapping_sub(milliseconds as i32);
                if slot.timer <= 0 {
                    if matches!(entry, Entry::Big { .. }) { self.active_big -= 1; }
                    else { self.active_entity -= 1; }
                    slot.entry = None; self.changed = true;
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn big(index: i32, measurement: f32) -> Entry { Entry::Big {index,measurement,achievement:0} }
    #[test]
    fn native_limit_precedes_refresh_and_timers_expire_exactly() {
        let mut c=ContextDisplay::default();
        c.add(big(0,1.)); c.add(big(1,1.)); c.add(big(2,1.));
        c.update(1000); c.add(big(0,2.)); c.add(big(3,1.));
        assert_eq!(c.slots[0].entry,Some(big(0,1.)));
        assert!(c.slots[3].entry.is_none());
        c.update(999); assert!(c.slots[0].entry.is_some());
        c.update(1); assert!(c.slots.iter().all(|s|s.entry.is_none()));
        c.add(big(3,1.)); assert_eq!(c.slots[0].entry,Some(big(3,1.)));
    }
    #[test]
    fn repeat_refreshes_timer_and_only_raises_measurement() {
        let mut c=ContextDisplay::default();c.add(big(0,10.));c.take_changed();
        c.update(1500);c.add(big(0,5.));assert!(!c.take_changed());
        c.update(1500);assert_eq!(c.slots[0].entry,Some(big(0,10.)));
        c.add(big(0,12.));assert!(c.take_changed());
    }
}
