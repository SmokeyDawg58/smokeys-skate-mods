//! LWPedestrianMan.UpdateMoodables82e83e40, over retained Actor/PhysOut owners.
use crate::{input_cvar_runtime::InputCvarReferences,
    reckoning_numeric::ReckoningVector,
    physical_player_assembly::PhysicalAssemblyWorld,
    hermes_game_sim_queue::{LivingWorldMessage,MoodEvent},
    living_world_chase::TargetableId,
    skateboard_force_runtime::fake_torque::{HostFakeTorqueNumeric,power_ten,natural_log,
        unconditional_refined_reciprocal}};
use std::sync::Arc;

impl super::SelectedRecipeFrame<'_> {
    /// dT is present in the native signature but unused here. Actor's own
    /// sound update owns timer decay; this pass only raises/reset levels.
    pub(crate) fn update_pedestrian_moodables(&mut self,world:&mut PhysicalAssemblyWorld,
        cvars:&InputCvarReferences,_dt:f32)->Result<(),String>{
        let owner=Arc::clone(world.grind_services.living_world());
        let living=owner.lock().map_err(|_|"moodables Manager lock")?;
        let Some(parameters)=living.pedestrian_group_perceptions.as_ref().filter(|p|!p.is_null()) else{return Ok(());};
        let mood=living.mood.as_ref().ok_or("moodables MoodMan absent")?;
        let noise_rate=mood.update_rate(5)?;
        let _skitch_rate=mood.update_rate(4)?;
        let scalar=|key|->Result<f32,String>{Ok(f32::from_bits(parameters.try_scalar_word(key,0)
            .map_err(|e|e.to_string())?.unwrap_or(0)))};
        // The source reads all six attributes before fetching SkaterManager.
        let impact_duration=scalar(0x18e6_eda0_8d46_b62e)?;
        let grind_duration=scalar(0xe9a1_088e_8ec8_d396)?;
        let reference_level=scalar(0x2eb3_7883_9ce9_bd47)?;
        let grind_level=scalar(0x6224_2536_0e7b_99b1)?;
        let impact_cutoff=scalar(0x9350_5dc0_4b99_9da9)?;
        let reference_force=scalar(0xd53b_adb0_c901_aeec)?;
        let mut numeric=HostFakeTorqueNumeric;
        let ln_ten=natural_log(10.,&mut numeric).unwrap();
        let reference_power=power_ten(reference_level*0.1,&mut numeric).unwrap();
        let force_scale=unconditional_refined_reciprocal(reference_force,&mut numeric).unwrap()*reference_power;
        // Preserve reference_force*(power/reference_force), including rounding.
        let noise_denominator=reference_force*force_scale;
        let actors=world.entity_roster.as_ref().ok_or("moodables SkaterManager roster absent")?
            .snapshot().map_err(|e|format!("moodables roster: {e:?}"))?;
        for actor in actors {
            let physical=actor.cached_physical_allocation().map_err(|e|format!("moodables physical: {e:?}"))?;
            let output=physical.owner().phys_out().lock().map_err(|_|"moodables PhysOut lock")?;
            let facts=output.try_pedestrian_moodable_facts()?;
            let mut prefix=actor.prefix().map_err(|e|format!("moodables Actor: {e:?}"))?;
            let fields=prefix.fields_mut().ok_or("moodables Actor fields absent")?;
            let landing=(0..4).any(|i|facts.audio.landed[i]&&!facts.audio.previous_landed[i]);
            if landing {
                let mut force=0.;
                for i in 0..4 {
                    if facts.audio.landed[i]&&!facts.audio.previous_landed[i]&&facts.audio.impact[i]>force {
                        force=facts.audio.impact[i];
                    }
                }
                if force>impact_cutoff {
                    let inverse_ln=unconditional_refined_reciprocal(ln_ten,&mut numeric).unwrap();
                    let level=(inverse_ln*natural_log(force_scale*force,&mut numeric).unwrap())*10.;
                    if level>=fields.sound_level {
                        fields.sound_level=level;
                        fields.sound_timer=raise_timer(fields.sound_timer,impact_duration);
                    }
                }
            }
            let grind_noise=facts.audio.grinding&&fields.grind_sound_timer<=0.;
            let mut noise=grind_noise;
            if grind_noise {
                fields.sound_level=grind_level;
                fields.sound_timer=raise_timer(fields.sound_timer,grind_duration);
            }
            let trick=facts.just_finished_grind||facts.qualifying_air_finished;
            if facts.qualifying_air_finished&&fields.sound_level>0. {noise=true;}
            // The HoM threshold is read only while its actual PhysOut flag is set.
            let hom=facts.hom_bail_in_progress&&facts.hom_score>=cvars.try_pedestrian_hom_slam_threshold()
                .map_err(|e|format!("moodables HoM threshold: {e:?}"))? as u32;
            let perpetrator=TargetableId{kind:0,id:fields.player_index as i32};
            let send=|event_type,loudness:f32|->Result<(),String>{
                // ITargetable.GetPosition825f9df0: actual skeleton root, read
                // for each message after its predicate/level computations.
                let position=output.try_skeleton_output().map_err(|e|format!("moodables position: {e:?}"))?
                    .skeleton_root_matrix.columns()[3].operand();
                living.game_sim_events.as_ref().ok_or("moodables GameSim broadcaster absent")?
                    .send(LivingWorldMessage::Mood(MoodEvent{event_type,
                        position:position.map(f32::to_bits),perpetrator,victim:TargetableId::INVALID,
                        loudness_distance_factor:loudness.to_bits(),visibility_distance_factor:1_f32.to_bits()}))
            };
            if facts.just_finished_grind {send(7,1.)?;}
            if trick {send(6,1.)?;}
            if noise {
                let power=power_ten(fields.sound_level*0.1,&mut numeric).unwrap();
                let ratio=power/noise_denominator;
                send(5,ratio*ratio)?;
            }
            if facts.skitching_vehicle {send(4,1.)?;}
            if grind_noise {fields.grind_sound_timer=raise_timer(fields.grind_sound_timer,noise_rate);}
            if hom {send(2,1.)?;}
        }
        Ok(())
    }
}
/// Native fsubs/fsel: NaN selects the requested duration, without f32::max's
/// differing NaN policy or an invented zero clamp.
fn raise_timer(old:f32,requested:f32)->f32{if old-requested>=0.{old}else{requested}}
