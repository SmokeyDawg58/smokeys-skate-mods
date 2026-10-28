//! Optional source S3 automatic pushing after the actual S2 listener Fill.
//! Input still belongs to this Actor; the same ActionGraph and physical owner
//! consume the resulting intents. No button, history or physical state is forged.
use super::ActorConstruction;
use crate::{
    actor_graph_construction::ActorGraphIntents,
    reckoning::grounded_input::{GroundInputRead, GroundScalar},
};
use skate3_core::input::riding_intentions::{PushPreferences, automatic_push};
use std::sync::Arc;

impl ActorConstruction {
    pub(super) fn try_apply_source_auto_push(&self) -> Result<(), String> {
        // This extension is absent until a real physical owner and selected
        // profile exist. Preserve the base listener's lazy physical queries.
        let Ok(physical) = self.cached_physical_allocation() else {
            return Ok(());
        };
        let owner = physical.owner();
        let tuning = owner
            .skate3_tuning
            .lock()
            .map_err(|_| "auto push tuning lock")?
            .clone();
        let Some(tuning) = tuning.filter(|t| t.auto_push_enabled) else {
            return Ok(());
        };
        // The full source player supplies its own controller preferences.
        // Resolve this before taking the Actor lock, matching solve lock order.
        if owner
            .skate3
            .lock()
            .map_err(|_| "auto push source owner lock")?
            .is_some()
        {
            return Ok(());
        }
        let (regular, switched) = {
            let output = owner
                .phys_out()
                .lock()
                .map_err(|_| "auto push output lock")?;
            if output
                .try_state_output()
                .map_err(|e| format!("auto push state: {e:?}"))?
                .physics_state
                != 100
            {
                return Ok(());
            }
            let a = output
                .try_animation_output()
                .map_err(|e| format!("auto push stance: {e:?}"))?;
            (a.is_regular_stance, a.is_riding_switch)
        };
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| "auto push reckoning lock")?
                .processed(),
        );
        let speed = processed
            .lock()
            .map_err(|_| "auto push processed lock")?
            .scalar(GroundScalar::DeckSpeed)
            .map_err(|e| format!("auto push speed: {e:?}"))?;
        if !(speed < tuning.auto_push_speed_threshold) {
            return Ok(());
        }
        let mut state = self.prefix().map_err(|e| e.to_string())?;
        let flags = state
            .input_generation_flags
            .ok_or("auto push Actor flags")?;
        if flags & 0x0400_0000 != 0 {
            return Ok(());
        }
        let constructor_ai_null = state
            .fields
            .as_ref()
            .ok_or("auto push Actor fields")?
            .ai_controller_null;
        if !state
            .physics_auxiliary
            .as_ref()
            .map_or(constructor_ai_null, |a| a.ai_controller.is_none())
        {
            return Ok(());
        }
        let listener = Arc::clone(
            state
                .listener
                .as_ref()
                .filter(|_| state.listener_published)
                .ok_or("auto push retained listener")?,
        );
        let intents = state
            .graphs
            .as_mut()
            .and_then(|g| g.action_mut())
            .and_then(|a| a.intents.as_mut())
            .ok_or("auto push ActionGraph intents")?;
        let name = ActorGraphIntents::native_name;
        if intents.get(&name(b"ForceBrake")).is_some() {
            return Ok(());
        }
        let manual = [b"RightPush".as_slice(), b"LeftPush", b"NewPush"]
            .into_iter()
            .any(|n| intents.get(&name(n)).is_some());
        let stick = listener
            .lock()
            .map_err(|_| "auto push listener lock")?
            .source_auto_push_stick()
            .ok_or("auto push Derived input")?;
        // Existing Update5 classification: mongo = right == (switch == stance).
        // Select its regular foot using observed character stance, not a bit
        // alias of the unrelated S3 Animation156 storage.
        let Some(right) = automatic_push(
            stick,
            manual,
            true,
            PushPreferences {
                automatic_push_enabled: true,
                automatic_push_right: switched != regular,
            },
        ) else {
            return Ok(());
        };
        let side: &[u8] = if right { b"RightPush" } else { b"LeftPush" };
        for n in [b"NewPush".as_slice(), side, b"Pushing"] {
            intents
                .insert_or_assign(name(n), 1.)
                .map_err(|e| format!("auto push intent: {e:?}"))?;
        }
        crate::diagnostics::test_log::event(
            "skate3_auto_push",
            format_args!(
                "solver=skate2 profile={} right={right} regular={regular} switch={switched} speed={speed} threshold={} stick={stick:?}",
                tuning.profile, tuning.auto_push_speed_threshold
            ),
        );
        Ok(())
    }
}
