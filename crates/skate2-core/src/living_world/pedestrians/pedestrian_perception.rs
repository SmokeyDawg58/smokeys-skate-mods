//! Sole embedded PedestrianPerception, native entity+2720.
use crate::{attribute_runtime::AttributeRuntimeInstance,
    pedestrian_animation::sim_lod_pose::SimLodBoneGlobal};

#[derive(Clone,Copy)]
pub(crate) struct Audible {pub(crate) position:[f32;4],pub(crate) active:bool}
#[derive(Clone,Copy)]
pub(crate) struct SpectateEvent {
    pub(crate) active:bool,pub(crate) position:[f32;4],
    pub(crate) score:i32,pub(crate) wiped_out:bool,
}
#[derive(Clone,Copy)]
pub(crate) struct Detection {pub(crate) audible:bool,pub(crate) visible:bool}
impl Detection {pub(crate) fn detected(self)->bool{self.audible||self.visible}}
pub(crate) struct PedestrianPerception {
    pub(crate) last_audible:Audible,
    pub(crate) last_spectate_event:SpectateEvent,
    pub(crate) eye_position:Option<[f32;4]>,
    pub(crate) eye_direction:Option<[f32;4]>,
    pub(crate) parameters:Option<AttributeRuntimeInstance>,
}
impl PedestrianPerception {
    ///82e6b2b0 explicitly initializes both events, leaving eye vectors for Init.
    pub(crate) fn new()->Self {Self{
        last_audible:Audible{position:[0.;4],active:false},
        last_spectate_event:SpectateEvent{active:false,position:[0.;4],score:0,wiped_out:false},
        eye_position:None,eye_direction:None,parameters:None,
    }}
    /// RecordAudible82e6b368 / RecordSpectateEvent82e6b398 copy complete events.
    pub(crate) fn record_audible(&mut self,event:Audible){self.last_audible=event;}
    pub(crate) fn record_spectate_event(&mut self,event:SpectateEvent){self.last_spectate_event=event;}
    /// Clears retain the previous payload, exactly as82e6b388/82e6b3d0.
    pub(crate) fn clear_audible(&mut self){self.last_audible.active=false;}
    pub(crate) fn clear_spectate_event(&mut self){self.last_spectate_event.active=false;}
    pub(crate) fn has_audible(&self)->bool{self.last_audible.active}
    pub(crate) fn has_spectate_event(&self)->bool{self.last_spectate_event.active}
    ///Init82e73cd0 uses GetTargetableCategoryKey82e6ad90: core.mEntityKey,
    /// not the later reverse-lookup entity category. Missing parent collection
    /// leaves the null parameters; a present parent requires a valid RefSpec.
    pub(crate) fn initialize(&mut self,vault:&std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        entity_key:u64)->Result<(),String>{
        self.eye_position=Some([0.;4]);
        self.eye_direction=Some([0.;4]);
        let entity=vault.instance(0x60d7_03c7_7cab_1631,entity_key);
        if !entity.is_null() {
            let parameters=entity.try_ref_spec_instance(0xa278_62c1_107f_64ec).map_err(|e|e.to_string())?;
            self.parameters=parameters;
            if self.parameters.is_none(){return Err("native perception parameters assertion82e73dcc".into());}
        }
        Ok(())
    }
    ///82e6b3e0 uses SimLODBoneGlobal[0] translation and second basis vector.
    /// The caller must supply the actual published simulation pose.
    pub(crate) fn update(&mut self,pose:&SimLodBoneGlobal){
        self.eye_position=Some(pose.matrices[0][3]);
        self.eye_direction=Some(pose.matrices[0][1]);
    }
    ///CanDetect82e73df0: proximity overrides both senses; hearing wins before
    ///cone/visibility work, and only a cone hit invokes the real World query.
    pub(crate) fn can_detect<N:crate::reckoning_numeric::ReckoningNumeric+
        crate::simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric>(
        &self,centre:[f32;4],event:[f32;4],apparent_audible_distance:f32,apparent_visible_distance:f32,
        world:&mut crate::physical_player_assembly::PhysicalAssemblyWorld,
        ground:&crate::static_ground_pool::StaticGroundPool,numeric:&mut N
    )->Result<Detection,String> where N::Error:std::fmt::Debug {
        use crate::reckoning_numeric::{try_length,try_normalize_unsafe};
        let delta:[f32;4]=std::array::from_fn(|i|event[i]-centre[i]);
        let horizontal=[delta[0],0.,delta[2],0.];
        let square=numeric.dot3(horizontal,horizontal).map_err(|e|format!("perception proximity: {e:?}"))?;
        if 1.>square&&4.>delta[1]*delta[1] {return Ok(Detection{audible:true,visible:true});}
        let eye=self.eye_position.ok_or("perception eye position not initialized")?;
        let direction=self.eye_direction.ok_or("perception eye direction not initialized")?;
        let delta:[f32;4]=std::array::from_fn(|i|event[i]-eye[i]);
        let square=numeric.dot3(delta,delta).map_err(|e|format!("perception distance: {e:?}"))?;
        let scalar=|key|->Result<f32,String>{
            let Some(parameters)=&self.parameters else{return Ok(0.);};
            Ok(f32::from_bits(parameters.try_scalar_word(key,0).map_err(|e|e.to_string())?.unwrap_or(0)))
        };
        let audibility=scalar(0xd036_ffce_ec9e_7d03)?;
        let visibility=scalar(0xd238_ac1b_1053_61c6)?;
        let fov=scalar(0x7435_316f_4a5e_0b59)?*0.017453292;
        let audible=audibility*apparent_audible_distance;
        if audible*audible>=square{return Ok(Detection{audible:true,visible:false});}
        //geotest::PointInCone82e4e338 uses a two-refinement length followed by
        //independent unsafe normalization and abs(XMVectorCos(angle)).
        let distance=try_length(delta,numeric).map_err(|e|format!("perception cone length: {e:?}"))?;
        if distance>visibility*apparent_visible_distance{return Ok(Detection{audible:false,visible:false});}
        let unit=try_normalize_unsafe(delta,numeric).map_err(|e|format!("perception cone normalize: {e:?}"))?;
        let cosine=crate::reckoning_sin_cos::try_cosine(fov,&mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric)
            .unwrap_or_else(|never|match never{}).abs();
        let facing=numeric.dot3(unit,direction).map_err(|e|format!("perception cone direction: {e:?}"))?;
        if !(facing>cosine){return Ok(Detection{audible:false,visible:false});}
        Ok(Detection{audible:false,visible:world.try_pedestrian_line_of_sight(ground,eye,event,numeric)?})
    }
}

impl super::PedestrianConstruction {
    ///GetTargetableCentreOfMass82e7a970 is entity GetPosition plus (0,.75,0).
    pub(crate) fn perception_centre(&self)->Result<[f32;4],String>{
        let position=self.core.as_ref().ok_or("perception entity core absent")?.transform[3];
        Ok([position[0]+0.,position[1]+0.75,position[2]+0.,position[3]+0.])
    }
}

impl crate::living_world_grind_services::LivingWorldGrindServices {
    /// LWPedestrianMan.UpdateAudibles82e838b8. Caller supplies the actual
    /// SkaterManager roster in index order; each event visits the sole hash map.
    pub(crate) fn update_pedestrian_audibles<N:crate::reckoning_numeric::ReckoningNumeric>(
        &mut self,actors:&[std::sync::Arc<crate::actor_construction::ActorConstruction>],numeric:&mut N
    )->Result<(),String> where N::Error:std::fmt::Debug {
        use crate::{physical_player_phys_out::PhysOutSystemVector,reckoning_numeric::{ReckoningVector,try_length}};
        let roster=self.pedestrian_entities.as_deref_mut().ok_or("perception pedestrian roster absent")?;
        // Native v2 persists across actor iterations. Once loud, later actors
        // skip the entire audible branch, but still broadcast scoring events.
        let mut loud=false;
        for actor in actors {
            let physical=actor.cached_physical_allocation().map_err(|e|format!("perception physical: {e:?}"))?;
            let output=physical.owner().phys_out().lock().map_err(|_|"perception PhysOut lock")?;
            if !loud {
                let audio=output.try_pedestrian_audio()?;
                let landed=(0..4).any(|i|audio.landed[i]&&!audio.previous_landed[i]);
                let mut strength=0.;
                if landed {
                    for i in 0..4 {
                        if audio.landed[i]&&!audio.previous_landed[i]&&audio.impact[i]>strength {
                            strength=audio.impact[i];
                        }
                    }
                }
                loud=audio.wheel_slide>0.3||audio.grinding||audio.wiping_out;
                if loud||landed {
                    // Actor.GetPosition825f9df0 returns skeleton root translation.
                    let actor_position=output.try_skeleton_output().map_err(|e|format!("perception actor position: {e:?}"))?
                        .skeleton_root_matrix.columns()[3].operand();
                    for id in roster.ids() {
                        let pedestrian=roster.get_mut(id).ok_or("perception roster member absent")?;
                        let position=pedestrian.core.as_ref().ok_or("perception pedestrian core absent")?.transform[3];
                        let delta=std::array::from_fn(|i|position[i]-actor_position[i]);
                        if !(crate::reckoning_numeric::host_dot3(delta,delta)<=5625.) {continue;}
                        let distance=try_length(delta,numeric).map_err(|e|format!("perception distance: {e:?}"))?;
                        if loud||landed&&strength>((distance*0.013333334)*0.4)+0.1 {
                            let position=output.try_camera_system_vector(PhysOutSystemVector::CentreOfMass)
                                .map_err(|e|format!("perception audible position: {e:?}"))?.operand();
                            pedestrian.perception.record_audible(Audible{position,active:true});
                        }
                    }
                }
            }
            //82e83d20..e14: banked change or wipeout, no distance/vision gate.
            if let Some(event)=output.try_pedestrian_spectate_event()? {
                for id in roster.ids() {
                    roster.get_mut(id).ok_or("spectate roster member absent")?.perception.record_spectate_event(event);
                }
            }
        }
        Ok(())
    }
}
