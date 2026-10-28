//! Motion callbacks on the SAME borrowed AI frame world owners.
use super::live::ExternalOwners;
use crate::{pedestrian_graph::backend::frame::FrameServices,
    living_world_pedestrian_entity::PedestrianConstruction,
    native_skater_random::{NativeRandomGate,rw_math}};

impl ExternalOwners for FrameServices<'_> {
    fn next_random_u32(&mut self) -> Result<u32,String> {
        // RemapAnim82e8d9b8 uses Man.GetRandomObject, not rw::math globals.
        self.random.try_next(&NativeRandomGate::production_shared()).map_err(|e|e.to_string())
    }
    fn bernoulli_random(&mut self, probability:f32)->Result<bool,String>{
        Ok(rw_math::bernoulli_choice(probability,rw_math::global_next().map_err(|e|format!("Bernoulli RNG: {e:?}"))?))
    }
    fn drop_probability(&mut self,entity:&PedestrianConstruction)->Result<f32,String>{
        Ok(entity.hand_prop_usage.drop_on_large_avoid)
    }
    fn interaction_hand_prop(&mut self,entity:&PedestrianConstruction)->Result<Option<u64>,String>{
        let Some(point)=entity.current_waypoint else{return Ok(None)};
        // Waypoint.GetWaypointGroup -> complete Prop IProp. Validate the actual
        // point before borrowing its containing Prop's retained characteristics.
        self.props.point(point)?;
        let prop=self.props.entities.get(point.prop).ok_or("hand prop waypoint Prop absent")?;
        let data=prop.characteristics.as_ref().ok_or("hand prop Prop characteristics absent")?;
        const KEY:u64=0x2ca9_b96b_e15e_856f;
        const TYPE:u64=0xe4aa_c2ea_bc05_3d0a;
        let count=data.try_length(KEY).map_err(|e|e.to_string())?;
        let mut probabilities=Vec::with_capacity(count as usize);
        for index in 0..count {
            let entry=data.try_array_field_data(KEY,index,TYPE,32,24..28).map_err(|e|e.to_string())?;
            probabilities.push(entry.map_or(0.,|entry|f32::from_be_bytes(entry.bytes().try_into().expect("probability field"))));
        }
        // ChooseHandPropToSpawn82e689b8 calls DiscreteRandom even for no entries.
        let draw=rw_math::global_next().map_err(|e|format!("Prop hand prop RNG: {e:?}"))?;
        let Some(index)=crate::living_world_grind_services::census::targets::discrete_choice(&probabilities,draw)
            else{return Ok(None)};
        let key=data.try_array_field_data(KEY,index as u32,TYPE,32,8..16).map_err(|e|e.to_string())?
            .map_or(0,|entry|u64::from_be_bytes(entry.bytes().try_into().expect("RefSpec collection")));
        Ok(Some(key))
    }
    fn set_hand_prop_to_spawn(&mut self,entity:&mut PedestrianConstruction,key:u64)->Result<(),String>{
        // SetHandPropToSpawn82e7af20 assigns the actual RefSpec, then pending bit.
        entity.hand_prop_attributes=Some(self.vault.instance(0xec8b_7565_46e2_26f6,key));
        entity.state.as_mut().ok_or("hand prop AI state absent")?.flags[3]|=2;
        Ok(())
    }
    fn log_spawn_hand_prop(&mut self,entity:&PedestrianConstruction)->Result<(),String>{
        // LogBehaviourToDB826fd9d0 resolves the model name before JLogEvent's
        // existing, source-global uninitialized telemetry gate82a76fc0.
        let key=entity.core.as_ref().ok_or("hand prop log core absent")?.model_key;
        let model=self.vault.instance(crate::living_world_recipe_manager::LIVING_WORLD_MODELS_CLASS_ID,key);
        model.try_cached_layout_text(0).map_err(|e|e.to_string())?.ok_or("hand prop log model name absent")?;
        crate::actor_construction::input_generation::try_juice_telemetry_gate().map_err(|e|e.to_string())
    }
}
