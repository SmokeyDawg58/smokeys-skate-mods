//! Pedestrian::UpdateIntent82e94f70 over the existing queue and animation data.
use super::{ActiveNavigation, PedestrianConstruction};
use crate::{
    input_cvar_runtime::InputCvarReferences,
    living_world_mesh_store::MeshStore,
    pedestrian_navigation_modifiers::{ORDER, intent::NavigationIntentAdapter},
    reckoning_numeric::ReckoningNumeric,
};

impl PedestrianConstruction {
    pub(crate) fn update_intent<N: ReckoningNumeric>(
        &mut self, mesh: &MeshStore, cvars: &InputCvarReferences, numeric: &mut N,
    ) -> Result<(), String> where N::Error: std::fmt::Debug {
        // First active intent/speed recommendation wins; a target-only
        // recommendation does not end this scan. Native asserts !managed.
        for kind in ORDER {
            let modifiers = self.navigation_modifiers.as_ref().ok_or("intent modifiers absent")?;
            if !modifiers.active(kind) { continue; }
            if let Some(packet) = modifiers.suggested_intent(kind)? {
                let queue = self.intent_queue.as_mut().ok_or("intent queue absent")?;
                if queue.packet(packet)?.managed {
                    return Err("UpdateIntent modifier suggested a managed packet".into());
                }
                queue.suggest(packet, &mut self.animation.as_mut()
                    .ok_or("intent animation absent")?.ai_input)?;
                break;
            }
            if let Some(speed) = modifiers.suggested_speed(kind) {
                self.navigation_mut()?.seek_speed = speed;
                break;
            }
        }
        let maximum_turn = self.current_target_max_turn_threshold().unwrap_or(180.);
        let tuning = cvars.try_pedestrian_navigation_intent_tuning()
            .map_err(|e| format!("navigation intent tuning: {e:?}"))?;
        let navigation = match self.active_navigation {
            ActiveNavigation::Mesh => &self.mesh_navigation.as_ref().ok_or("intent mesh navigation absent")?.base,
            ActiveNavigation::Physics => self.physics_navigation.as_ref().ok_or("intent physics navigation absent")?,
        };
        let core = self.core.as_ref().ok_or("intent core absent")?;
        let animation = self.animation.as_mut().ok_or("intent animation absent")?;
        let explicit_direction = self.navigation_intent.explicit_direction;
        let mut adapter = NavigationIntentAdapter {
            state: &mut self.navigation_intent, core, orientation: &core.target_transform,
            navigation, rig: self.nav_rig.as_ref().ok_or("intent rig absent")?,
            modifiers: self.navigation_modifiers.as_ref().ok_or("intent modifiers absent")?,
            spatial: self.spatial_adjust.as_mut().ok_or("intent spatial adjust absent")?,
            ai_state: self.state.as_mut().ok_or("intent state absent")?,
            mesh, tuning: &tuning, nav_rig_override: self.navigation_rig_override,
            smoothing_enabled: self.navigation_smoothing_enabled, maximum_turn, numeric,
        };
        self.intent_queue.as_mut().ok_or("intent queue absent")?
            .update(animation, &mut adapter, explicit_direction)
    }

    /// Pedestrian::ClearAllIntents 0x82e8d890: IntentGen::ClearAllIntents 0x82e8b028 (not for
    /// a high priority, with the animation data's AI graph input), then the motion graph
    /// input's AnimFrameOffset (tAnimData+0x70) is zeroed.
    pub(crate) fn clear_all_intents(&mut self) -> Result<(), String> {
        let animation = self.animation.as_mut().ok_or("ClearAllIntents animation absent")?;
        self.intent_queue.as_mut().ok_or("ClearAllIntents queue absent")?
            .clear(false, Some(&mut animation.ai_input))?;
        animation.motion_intents.frame_offset = 0.;
        Ok(())
    }

    /// Pedestrian::SuggestNextIntent(char*) 0x82e8d830 -> (FastString<30>) 0x82e98678: a new
    /// managed packet of one stage holding the intent (cIntentGenPacketStage 0x82e87230), then
    /// IntentGen::SuggestNextIntent 0x82e93c70. A rejected packet stays allocated, as native.
    pub(crate) fn suggest_next_intent(&mut self, name: &str) -> Result<(), String> {
        use crate::pedestrian_intent_packet::{IntentPacket, IntentStage};
        let mut stage = IntentStage::default();
        stage.add_intent(crate::play_animation::FastString30::encode(name)
            .map_err(|e| format!("intent name {name}: {e:?}"))?)?;
        let mut packet = IntentPacket::new(true);
        packet.add_stage(stage)?;
        let animation = self.animation.as_mut().ok_or("SuggestNextIntent animation absent")?;
        let queue = self.intent_queue.as_mut().ok_or("SuggestNextIntent queue absent")?;
        let id = queue.allocate(packet)?;
        queue.suggest(id, &mut animation.ai_input)?;
        Ok(())
    }
}
