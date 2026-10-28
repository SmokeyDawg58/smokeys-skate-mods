//! Guard predicates over actual target owners, preserving lazy native reads.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    living_world_chase::TargetableId, living_world_guard_graph::{self, GuardConditionFacts}};

impl FrameServices<'_> {
    /// Actor.IsChaseeLocalPlayer / Chasee.GetGroupEndChaseReason82de1de0.
    pub(crate) fn chase_group_end_reason(&self, entity: &PedestrianConstruction,
        target: TargetableId) -> Result<Option<i32>, String> {
        let Some(stored) = self.with_chasee(entity, target, |s| s.map(|s| s.group_end_reason))? else {
            return Ok(None);
        };
        let local = if let Some(actor) = self.actor(target)? {
            self.world.local_skater_identity.as_ref().ok_or("chase local identity absent")?
                .try_is_local_player(|| actor.prefix().map_err(|e| e.to_string())?.fields()
                    .map(|f| f.player_index as i32).ok_or_else(|| "chase Actor fields absent".to_owned()))
                .map_err(|e| e.to_string())?
        } else { false };
        Ok(Some(self.chase.group_end_reason(stored, local)))
    }
    /// SecurityGuard.PreStateGraphUpdate82e9f5a8 suffix, after the entire
    /// inherited pedestrian prestate. Debug zone labels have no gameplay work.
    pub(crate) fn security_guard_prestate_suffix(&mut self,
        entity: &mut PedestrianConstruction, dt: f32,
    ) -> Result<(), String> {
        if entity.security_guard.is_none() { return Ok(()); }
        if entity.target_iterators.mesh_current.is_some() {
            let threshold = self.cvars.try_security_guard_mesh_cost_threshold()
                .map_err(|e| format!("guard mesh threshold: {e:?}"))?;
            entity.target_iterators.set_mesh_cost_threshold(threshold);
        }
        let target = entity.state.as_ref().ok_or("guard prestate state absent")?.chaser.chasee;
        if self.with_chasee(entity, target, |chasee| chasee.is_some())? {
            let position = self.target_position(entity, target)?.ok_or("guard chasee position absent")?;
            entity.security_guard.as_mut().expect("checked guard").update_guardee_velocity(
                Some([position[0], position[1], position[2]]), dt,
                &mut crate::reckoning_numeric::HostReckoningNumeric)
                .map_err(|e| format!("guard chasee velocity: {e:?}"))?;
            let position = self.target_position(entity, target)?.ok_or("guard chasee second position absent")?;
            entity.security_guard.as_mut().expect("checked guard").previous_guardee_position =
                [position[0], position[1], position[2]];
        }
        Ok(())
    }

    /// ConvertTargetableTypeToITargetable82de3658. All four concrete classes
    /// expose ITargetable; invalid IDs and missing manager entries return null.
    pub(crate) fn target_exists(&self, entity: &PedestrianConstruction, target: TargetableId)
        -> Result<bool, String> {
        if !target.is_valid() { return Ok(false); }
        Ok(match target.kind {
            0 => self.actor(target)?.is_some(),
            1 => target.id as u32 == entity.id || self.pedestrian(target).is_some(),
            2 => self.dmos.object(target.id as u32).is_some(),
            3 => self.vehicles.get(target.id as u32).is_some(),
            _ => return Err(format!("native targetable type assertion: {}", target.kind)),
        })
    }

    /// ITargetable.IsWipedOut: Actor825edad8 reads completed PhysOut.State;
    /// Pedestrian82e6eba8 receives adjusted ITargetable+184, so +1960 is
    /// actual entity+1a18 (state), NOT the decompiler's attribute collection.
    pub(crate) fn target_wiped_out(&self, entity: &PedestrianConstruction, target: TargetableId)
        -> Result<Option<bool>, String> {
        if !self.target_exists(entity, target)? { return Ok(None); }
        match target.kind {
            0 => {
                let actor = self.actor(target)?.ok_or("resolved target Actor disappeared")?;
                let allocation = actor.cached_physical_allocation().map_err(|e| format!("target physical: {e:?}"))?;
                let output = allocation.owner().phys_out().lock().map_err(|_| "target PhysOut lock")?;
                Ok(Some(output.try_state_output().map_err(|e| format!("target state: {e:?}"))?.is_wiping_out))
            }
            1 => {
                let target = if target.id as u32 == entity.id { entity }
                    else { self.pedestrian(target).ok_or("resolved target pedestrian disappeared")? };
                Ok(Some(target.state.as_ref().ok_or("target pedestrian state absent")?.flags[1] & 4 != 0))
            }
            // Actual ITargetable vtables82326b00/8232cf70 slot24 both bind
            // folded82784530 (li r3,0; blr). This is native behavior.
            2 | 3 => Ok(Some(false)),
            _ => unreachable!("target_exists validated type"),
        }
    }

    pub(crate) fn evaluate_guard_condition(&mut self, name: &str, reason: Option<i32>,
        entity: &PedestrianConstruction) -> Result<bool, String> {
        let state = entity.state.as_ref().ok_or("guard condition requires AI state")?;
        // Only the selected predicate consumes these facts. Unread fields are
        // scratch, never fabricated values published to an entity or cache.
        let mut facts = GuardConditionFacts {
            current_waypoint_exists: false, take_down_sink_exists: false,
            take_down_sink_wiped_out: false, disable_takedowns: false,
            take_down_type_exists: false, can_taze_unreachable: false,
            chasee_group_end_reason: None,
        };
        match name {
            "IsPatrolling" => facts.current_waypoint_exists = entity.current_waypoint.is_some(),
            "HasDrawnTazer" | "HasLineOfSightToTazeTarget" |
            "TakeDownAttemptSuccessful" | "TakeDownAttemptFailed" => {},
            "CanTazeUnreachableTargets" => {
                // Pedestrian82e6f738 reads live authored chase layout byte9.
                let chase = &entity.resources.as_ref().ok_or("guard chase resources absent")?.chase;
                if chase.is_null() { return Err("CanTazeUnreachableTargets requires chase collection".into()); }
                facts.can_taze_unreachable = chase.try_cached_layout_field(9, 1)
                    .map_err(|e| e.to_string())?.ok_or("guard chase layout absent")?.bytes()[0] != 0;
            }
            "HasTakeDownTargetable" => facts.take_down_sink_exists = self.target_exists(entity, state.take_down_sink)?,
            "CanAttemptTakeDownTargetable" => {
                //8270c960: cvar -> source/sink -> wipeout -> selected type.
                facts.disable_takedowns = self.cvars.try_pedestrian_disable_takedowns().map_err(|e| e.to_string())?;
                if !facts.disable_takedowns {
                    facts.take_down_sink_exists = self.target_exists(entity, state.take_down_sink)?;
                    if facts.take_down_sink_exists {
                        facts.take_down_sink_wiped_out = self.target_wiped_out(entity, state.take_down_sink)?
                            .ok_or("resolved takedown sink disappeared")?;
                        if !facts.take_down_sink_wiped_out { facts.take_down_type_exists = state.take_down_type.is_some(); }
                    }
                }
            }
            "IsEndChaseReason" => {
                facts.chasee_group_end_reason = self.chase_group_end_reason(entity, state.chaser.chasee)?;
            }
            _ => return Err(format!("unknown guard condition {name}")),
        }
        living_world_guard_graph::state_condition(name, state, &facts, reason)?
            .ok_or_else(|| format!("guard predicate absent: {name}"))
    }
}
