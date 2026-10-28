//! TransferExpressionOperator.GetActivation82cefd38 over the original parsed tree.
use crate::behaviour_graph_runtime::{TransferExpression,TransferExpressionChild,TransferOperator,TransferCondition};

///826fd020 consumes the Simulation stream even for zero/one probabilities.
///It is independent of the global RW Bernoulli used by sink acceptance.
pub(crate) fn random_condition(condition:&TransferCondition,
    random:&mut crate::native_skater_random::SimulationCommandRandom,
)->Result<Option<bool>,String>{
    match condition {
        TransferCondition::PluginTransferOdds{percentage_bits}=>{
            let sample=random.try_range(0.,1.).map_err(|e|format!("plugin transfer simulation RNG: {e}"))?;
            Ok(Some(f32::from_bits(*percentage_bits)>sample))
        }
        _=>Ok(None),
    }
}

impl crate::living_world_grind_services::waypoints::Prop {
    ///IsActive82e66aa8 and CanTransferBehaviourPlugin82e653a8.
    ///The source-interface vtable+0x44 is GetBehaviourSinkCount82a96838,
    ///not the decompiler's misidentified GetSpawnOffset. Copies may outlive
    ///sink registration, so their count is not the admission limit.
    pub(crate) fn can_offer_behaviour(&self)->bool{
        self.flags&0x80!=0 && self.transferable.as_ref().is_some_and(|template|
            self.behaviour_sinks.len() < template.declaration.maximum_participants as usize)
        // Native also tests the static GetEditTake function ADDRESS for null;
        // it is present, and does not query whether an editor take is active.
    }
}

/// Waypoint predicates consume the existing source Prop's group/reservations.
/// Missing source/group is native false. The entity position is fetched only
/// when an unlocked point actually needs a distance test.
pub(crate) fn waypoint_condition<N:crate::reckoning_numeric::ReckoningNumeric>(
    condition:&TransferCondition,
    source:Option<&crate::living_world_grind_services::waypoints::Prop>,
    mut position:impl FnMut()->Result<[f32;4],String>,numeric:&mut N,
    cvars:&crate::input_cvar_runtime::InputCvarReferences,
)->Result<Option<bool>,String> where N::Error:std::fmt::Debug {
    match condition {
        TransferCondition::InFrontOfFirstWaypoint=>{
            //82711b38: first point regardless of reservation, flat vectors,
            //signed angle around +Y. Both angular boundaries are strict.
            let Some(point)=source.and_then(|p|p.waypoint.as_ref()).and_then(|g|g.points.first())
                else{return Ok(Some(false));};
            let at=position()?;
            let delta=[at[0]-point.location[0],0.,at[2]-point.location[2],0.];
            let direction=[point.orientation[0],0.,point.orientation[2],0.];
            let angle=crate::reckoning_numeric::try_signed_angle(direction,delta,[0.,1.,0.,0.],numeric)
                .map_err(|e|format!("transfer waypoint angle: {e:?}"))?;
            let limit=cvars.try_in_front_of_waypoint_angle().map_err(|e|format!("transfer waypoint tuning: {e:?}"))?;
            Ok(Some(angle<limit||angle>std::f32::consts::TAU-limit))
        }
        TransferCondition::HasFirstWaypointAvailable=>{
            //82706508: count, point0, then its actual lock flag.
            Ok(Some(source.and_then(|p|p.waypoint.as_ref()).and_then(|g|g.points.first())
                .is_some_and(|point|point.locking_entity.is_none())))
        }
        TransferCondition::NearAvailableWaypoint{distance_bits}=>{
            //82706610 uses full XYZ distance, strictly below authored distance.
            let Some(group)=source.and_then(|p|p.waypoint.as_ref()) else{return Ok(Some(false));};
            let threshold=f32::from_bits(*distance_bits);
            for point in &group.points {
                if point.locking_entity.is_some(){continue;}
                let at=position()?;
                let delta=std::array::from_fn(|i|at[i]-point.location[i]);
                let distance=crate::reckoning_numeric::try_length(delta,numeric)
                    .map_err(|e|format!("transfer waypoint distance: {e:?}"))?;
                if threshold>distance{return Ok(Some(true));}
            }
            Ok(Some(false))
        }
        _=>Ok(None),
    }
}

impl crate::living_world_pedestrian_entity::PedestrianConstruction {
    ///82706920 invokes the same pedestrian IsOnRoadNetwork virtual as the
    ///ordinary graph predicate; keep NavRig score and RAIN ownership shared.
    pub(crate) fn transfer_entity_condition(&self,condition:&TransferCondition)
        ->Result<Option<bool>,String>{
        if matches!(condition,TransferCondition::IsOnRoad){
            return crate::pedestrian_graph::conditions::entity_condition("IsOnRoad",self);
        }
        self.transfer_hand_prop_condition(condition)
    }

    /// Concrete non-spatial transfer predicates for the pedestrian sink.
    /// None means another predicate owner must handle it, never success.
    pub(crate) fn transfer_hand_prop_condition(&self,condition:&TransferCondition)
        ->Result<Option<bool>,String>{
        Ok(match condition {
            TransferCondition::HasHandProp=>Some(self.has_hand_prop()?), //82706d78
            TransferCondition::HasDisposableHandProp=>Some(self.has_hand_prop()?&&self.is_hand_prop_disposable()?), //82706e00
            TransferCondition::CanSitWithHandProp=>Some(!self.has_hand_prop()?||self.can_sit_with_hand_prop()?), //82706f38
            _=>None,
        })
    }

    /// CanUseBehaviourSource82e86140 reads the processed controller root hash,
    /// not the prop ID, asset hash, or current plugin state.
    pub(crate) fn can_use_behaviour_source(&self,source:&super::load::ProcessedTransferable)
        ->Result<bool,String>{
        let hash=source.root_state_hash()?;
        Ok(self.state.as_ref().ok_or("plugin eligibility AI state absent")?.plugin_timer_expired(hash))
    }

    /// DecideToUseBehaviourSource82e9ac50 after the source expression accepts.
    /// Raw cmpld82e9ad48 compares the complete RefSpec collection key. This
    /// typed record also contains relocated pointers, so read only its fields.
    pub(crate) fn decide_to_use_behaviour_source(&mut self,source_category:u64,
        source:&super::load::ProcessedTransferable,cvars:&crate::input_cvar_runtime::InputCvarReferences,
    )->Result<bool,String>{
        const KEY:u64=0xf893_23e4_20a6_aaa3;
        const TYPE:u64=0xe4aa_c2ea_bc05_3d0a;
        let attributes=&self.resources.as_ref().ok_or("plugin eligibility resources absent")?.entity;
        for index in 0..attributes.try_length(KEY).map_err(|e|e.to_string())? {
            let category=attributes.try_array_field_data(KEY,index,TYPE,32,8..16).map_err(|e|e.to_string())?
                .map_or(0,|d|u64::from_be_bytes(d.bytes().try_into().expect("category key")));
            let probability=attributes.try_array_field_data(KEY,index,TYPE,32,24..28).map_err(|e|e.to_string())?
                .map_or(0.,|d|f32::from_be_bytes(d.bytes().try_into().expect("probability")));
            if category!=source_category {continue;}
            let word=crate::native_skater_random::rw_math::global_next().map_err(|e|format!("plugin probability RNG: {e:?}"))?;
            let accepted=crate::native_skater_random::rw_math::bernoulli_choice(probability,word);
            if !accepted {
                let hash=source.root_state_hash()?;
                let retry=cvars.try_plugin_retry_timer().map_err(|e|format!("plugin retry tuning: {e:?}"))?;
                self.state.as_mut().ok_or("plugin eligibility AI state absent")?.reset_plugin_timer(hash,retry);
            }
            return Ok(accepted);
        }
        Ok(false)
    }
}

pub(crate) fn evaluate(expression:&TransferExpression,
    condition:&mut impl FnMut(&TransferCondition)->Result<bool,String>)->Result<bool,String>{
    fn child(value:&TransferExpressionChild,condition:&mut impl FnMut(&TransferCondition)->Result<bool,String>)
        ->Result<bool,String>{match value{
            TransferExpressionChild::Expression(value)=>evaluate(value,condition),
            TransferExpressionChild::Condition(value)=>condition(value),
        }}
    match expression.operator {
        TransferOperator::None|TransferOperator::And=>{
            for value in &expression.children{if !child(value,condition)?{return Ok(false);}}
            Ok(true)
        }
        TransferOperator::Or=>{
            for value in &expression.children{if child(value,condition)?{return Ok(true);}}
            Ok(false)
        }
        TransferOperator::Not=>{
            if expression.children.len()!=1{return Err("native NOT transfer expression arity assertion".into());}
            Ok(!child(&expression.children[0],condition)?)
        }
    }
}
