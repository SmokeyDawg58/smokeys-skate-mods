//! `Sk8::LivingWorld::WaypointMan` (0x620 bytes): the spectator waypoint groups a challenge
//! activates (`ActivateSpectatorWaypoint` 0x82ce9180), whose share of forced spectators spawn on
//! `Manager::Update1` (`Update` 0x82ce8e00, `UpdateSpectatorActivationInfo` 0x82ce71b8), and their
//! release (`DeactivateSpectatorWaypoint` 0x82ce8ed8, `WaypointGroup::Deactivate` 0x82e65718).
use super::{PropMan, WaypointRef};

/// `PendingSpectatorActivation` is a `fixed_vector` of 32 without overflow.
const MAX_PENDING: usize = 32;
/// `Prop` +0x68's active bit (`Prop::Activate` 0x82e66ab8).
const PROP_ACTIVE: u8 = 0x80;

/// `fsel`: `b` when `a` is at least zero, else `c` (NaN takes `c`).
fn fsel(a: f32, b: f32, c: f32) -> f32 {
    if a >= 0.0 { b } else { c }
}

/// `tSpectatorActivationInfo` (0x30 bytes).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SpectatorActivation {
    /// `CategoryGroup`: the `livingworld_categorygroups` collection the spectators come from.
    pub(crate) category_group: u64,
    /// `WaypointActivated`, which nothing sets: every update activates the group again.
    pub(crate) waypoint_activated: bool,
    pub(crate) num_force_spawn: u32,
    pub(crate) force_spawn_remaining: u32,
    pub(crate) waypoint_guid: u64,
}

#[derive(Default)]
pub(crate) struct WaypointMan {
    /// `PendingSpectatorActivation`, in queue order.
    pub(crate) pending: Vec<SpectatorActivation>,
    /// `NumForceSpectators`.
    pub(crate) num_force_spectators: u32,
}

impl WaypointMan {
    pub(crate) const fn new() -> Self {
        Self { pending: Vec::new(), num_force_spectators: 0 }
    }

    /// `Init` 0x82ce7f50: the queue empties.
    pub(crate) fn init(&mut self) {
        self.pending.clear();
    }

    /// `ActivateSpectatorWaypoint` after its `RequestCensusManagedCull()`: the info's share (a
    /// percentage from 5 on, else a fraction; clamped to 0..1, then to 0.8 while fewer than
    /// `kMaxNumForceSpectators` are forced, else none) of the group's waypoints, truncated, is
    /// forced and queued. With none forced the group only activates: true.
    pub(crate) fn activate(
        &mut self,
        num_waypoints: u32,
        guid: u64,
        force_spawn_percentage: f32,
        category_group: u64,
        max_force_spectators: i32,
    ) -> Result<bool, String> {
        let mut share = force_spawn_percentage;
        if share >= 5.0 {
            share *= f32::from_bits(0x3c23_d70a);
        }
        let floor = fsel(-share, 0.0, share);
        let share = fsel(1.0 - floor, floor, 1.0);
        #[expect(clippy::cast_sign_loss, reason = "the native's unsigned compare")]
        let share = if self.num_force_spectators < max_force_spectators as u32 {
            let floor = fsel(-share, 0.0, share);
            fsel(f32::from_bits(0x3f4c_cccd) - floor, floor, f32::from_bits(0x3f4c_cccd))
        } else {
            0.0
        };
        #[expect(clippy::cast_precision_loss, clippy::cast_possible_truncation, reason = "the native's (float) and fctiwz")]
        let count = (num_waypoints as f32 * share) as i32;
        #[expect(clippy::cast_sign_loss, reason = "the native adds the word")]
        {
            self.num_force_spectators = self.num_force_spectators.wrapping_add(count as u32);
        }
        if count <= 0 {
            return Ok(true);
        }
        if self.pending.len() == MAX_PENDING {
            return Err("WaypointMan PendingSpectatorActivation native capacity 32 exceeded".into());
        }
        #[expect(clippy::cast_sign_loss, reason = "positive")]
        let count = count as u32;
        self.pending.push(SpectatorActivation { category_group, waypoint_activated: false, num_force_spawn: count, force_spawn_remaining: count, waypoint_guid: guid });
        Ok(false)
    }

    /// `DeactivateSpectatorWaypoint`'s queue part: every activation it walks past gives back its
    /// forced count, the matching one (by the GUID's low word) included, which leaves the queue.
    pub(crate) fn deactivate(&mut self, guid: u64) {
        for index in 0..self.pending.len() {
            let entry = &self.pending[index];
            self.num_force_spectators = self.num_force_spectators.wrapping_sub(entry.num_force_spawn);
            if entry.waypoint_guid as u32 == guid as u32 {
                self.pending.remove(index);
                return;
            }
        }
    }
}

impl PropMan {
    /// `ILWManager::GetWaypointByGUID`: the Prop of the waypoint group registered under the GUID.
    pub(crate) fn waypoint_group_by_guid(&self, guid: u64) -> Option<u32> {
        let &prop = self.waypoint_guids.get(&guid)?;
        self.entities.get(prop)?.waypoint.as_ref()?;
        Some(prop)
    }

    /// `Prop::Activate` 0x82e66ab8.
    pub(crate) fn activate_prop(&mut self, prop: u32) -> Result<(), String> {
        self.entities.get_mut(prop).ok_or("activated Prop absent")?.flags |= PROP_ACTIVE;
        Ok(())
    }

    /// `WaypointGroup::Deactivate(op 0)` 0x82e65718: the group inactive, each locked waypoint's
    /// pedestrian (returned in waypoint order, for `RequestExplicitCull`) released from it.
    pub(crate) fn deactivate_waypoint_group(&mut self, prop: u32) -> Result<Vec<u32>, String> {
        let entity = self.entities.get_mut(prop).ok_or("deactivated Prop absent")?;
        entity.flags &= !PROP_ACTIVE;
        let group = entity.waypoint.as_mut().ok_or("deactivated Prop is not a waypoint group")?;
        let mut released = Vec::new();
        for point in &mut group.points {
            if let Some(pedestrian) = point.locking_entity.take() {
                released.push(pedestrian);
            }
        }
        Ok(released)
    }

    /// `WaypointGroup::GetFirstUnlockedWaypoint` 0x82e64b00 with its location and orientation.
    pub(crate) fn first_unlocked_waypoint(&self, prop: u32) -> Result<Option<(WaypointRef, [f32; 4], [f32; 4])>, String> {
        let group = self.group(prop)?;
        let Some(index) = group.first_unlocked() else { return Ok(None) };
        let point = &group.points[index as usize];
        Ok(Some((WaypointRef { prop, index }, point.location, point.orientation)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX: i32 = 24;

    #[test]
    fn a_percentage_forces_its_share_up_to_eight_tenths() {
        let mut man = WaypointMan::new();
        assert!(!man.activate(10, 7, 50.0, 3, MAX).unwrap());
        assert_eq!(man.pending[0].num_force_spawn, 5);
        assert!(!man.activate(10, 8, 1.0, 3, MAX).unwrap());
        assert_eq!(man.pending[1].num_force_spawn, 8, "a fraction is capped at 0.8");
        assert_eq!(man.num_force_spectators, 13);
        assert!(man.activate(10, 9, 0.0, 3, MAX).unwrap(), "none forced only activates");
    }

    #[test]
    fn the_force_limit_stops_new_spectators() {
        let mut man = WaypointMan::new();
        man.num_force_spectators = 24;
        assert!(man.activate(10, 1, 50.0, 3, MAX).unwrap());
        assert!(man.pending.is_empty());
    }

    #[test]
    fn a_percentage_scales_and_truncates_in_single_precision() {
        let mut man = WaypointMan::new();
        man.activate(10, 1, 20.0, 3, MAX).unwrap();
        assert_eq!(man.pending[0].num_force_spawn, 1, "20 * 0.01f rounds below 0.2");
    }

    #[test]
    fn deactivation_gives_back_every_walked_count() {
        let mut man = WaypointMan::new();
        man.activate(10, 1, 0.25, 3, MAX).unwrap();
        man.activate(10, 2, 0.5, 3, MAX).unwrap();
        man.activate(10, 3, 0.75, 3, MAX).unwrap();
        assert_eq!(man.num_force_spectators, 14);
        man.deactivate(2);
        assert_eq!(man.num_force_spectators, 7);
        assert_eq!(man.pending.iter().map(|entry| entry.waypoint_guid).collect::<Vec<_>>(), [1, 3]);
    }
}
