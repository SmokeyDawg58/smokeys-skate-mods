//! Source-backed plugin leaf construction and direct owner effects.
use super::Family;
use crate::living_world_pedestrian_entity::PedestrianConstruction;
use crate::pedestrian_graph::simple_behaviours::SimpleBehaviour;
impl PedestrianConstruction {
    /// SetInteractingObject82e6ff80, same dynamic-object monitor and rig-ignore owner.
    pub(crate) fn set_interacting_object(&mut self,id:Option<u32>,objects:&mut crate::living_world_grind_services::dmo_objects::DmoObjectMan)->Result<(),String>{
        if let Some(old)=self.interacting_object{
            let object=objects.object_mut(old).ok_or("old interacting DMO absent")?;
            if let Some(index)=object.monitoring_entities.iter().position(|v|*v==self.id){object.monitoring_entities.remove(index);}
            self.nav_rig_ignore_entity=None;
        }
        self.interacting_object=id;
        if let Some(id)=id{
            let object=objects.object_mut(id).ok_or("new interacting DMO absent")?;
            // RegisterEntityMonitor82d38488 returns false at count4; this
            // SetInteractingObject caller deliberately ignores that result.
            if object.monitoring_entities.len()<4&&!object.monitoring_entities.contains(&self.id){object.monitoring_entities.push(self.id);}
            self.nav_rig_ignore_entity=Some(crate::living_world_chase::TargetableId{kind:2,id:id as i32});
        }Ok(())
    }
}

pub(crate) enum DirectLeaf {
    LookAt{duration:f32},
    DisallowHandPropActions,
    DisableHeavyCollision,
    IgnoreStandingCollisions,
    DisableCollisionSliding,
    LockToCurrentPosition,
    OverridePluginCollision,
    ExitPlugin,
    Simple(SimpleBehaviour),
}
impl DirectLeaf {
    pub(crate) fn bind(family:Family,index:u16)->Result<Option<Self>,String>{
        let declarations=family.declarations();
        let declaration=declarations.behaviours.get(index as usize).ok_or("plugin behaviour index")?;
        let section=declaration.attributes;
        let fields=if section.count==0 { &[][..] } else {
            family.attributes().get(section.index as usize..section.index as usize+section.count as usize)
                .ok_or("plugin behaviour attribute section")?
        };
        let get=|name|fields.iter().find(|v|v.key==name).map(|v|v.value);
        Ok(Some(match declaration.name {
            // LookAt826eaf20 invokes Behaviour ctor then defaults30s.
            "LookAt"=>Self::LookAt {duration:get("duration").map_or(Ok(30.),|s|s.parse::<f32>().map_err(|e|e.to_string()))?},
            "DisallowHandPropActions"=>Self::DisallowHandPropActions,
            "DisableHeavyCollision"=>Self::DisableHeavyCollision,
            "IgnoreStandingCollisions"=>Self::IgnoreStandingCollisions,
            "DisableCollisionSliding"=>Self::DisableCollisionSliding,
            "LockToCurrentPosition"=>Self::LockToCurrentPosition,
            "OverridePluginCollision"=>Self::OverridePluginCollision,
            "ExitPlugin"=>Self::ExitPlugin,
            // Shared original SetSimpleTimer826ec700 payload and Begin.
            "SetSimpleTimer"=>Self::Simple(SimpleBehaviour::SetTimer {
                timer:crate::living_world_pedestrian_state::timer_type(get("timerName").unwrap_or("TimedRangeRandom"))
                    .ok_or("plugin timer name")?,
                length:get("length").map_or(Ok(0.),|s|s.parse::<f32>().map_err(|e|e.to_string()))?,
            }),
            _=>return Ok(None),
        }))
    }
    pub(crate) fn begin(&self,entity:&mut PedestrianConstruction,source:Option<u32>)->Result<(),String>{
        match self {
            Self::LookAt{duration}=>{
                //826fda78: null source returns before either effect.
                if let Some(source)=source {
                    entity.head_tracking.as_mut().ok_or("LookAt requires head tracking")?.push_behaviour_source(Some(source))?;
                    entity.state.as_mut().ok_or("LookAt requires AI state")?.set_timer(6,*duration).map_err(|e|e.to_string())?;
                }
            }
            //826ee8c8 raw lwz1a18/lbzCAB/ori10/stbCAB.
            Self::DisallowHandPropActions=>entity.state.as_mut().ok_or("plugin AI state absent")?.flags[3]|=0x10,
            //826ecdc8/826ece00: these are independent bits in state+ca9.
            Self::DisableHeavyCollision=>entity.state.as_mut().ok_or("plugin AI state absent")?.flags[1]|=0x20,
            Self::IgnoreStandingCollisions=>entity.state.as_mut().ok_or("plugin AI state absent")?.flags[1]|=0x10,
            //826ece38 clears Pedestrian+1a44 bit40.
            Self::DisableCollisionSliding=>entity.adjust_world_collisions=false,
            //82705d08 writes the retained PedestrianAIState+C60 byte.
            Self::OverridePluginCollision=>entity.state.as_mut().ok_or("plugin AI state absent")?.override_external_collision=true,
            //826ece98 locks the spatial adjust owner to GetPosition now.
            Self::LockToCurrentPosition=>{
                let position=entity.core.as_ref().ok_or("position lock requires core")?.transform[3];
                let adjust=entity.spatial_adjust.as_mut().ok_or("position lock requires spatial adjust")?;
                adjust.position_locked=true;
                adjust.locked_position=position;
            }
            Self::ExitPlugin=>return Err("ExitPlugin requires the retained source/sink route".into()),
            Self::Simple(value)=>{if !value.begin(entity)? {return Err("plugin simple Begin not handled".into());}}
        }
        Ok(())
    }
    pub(crate) fn end(&self,entity:&mut PedestrianConstruction)->Result<(),String>{
        match self {
            //826eafa8 pops even when Begin had no behaviour source.
            Self::LookAt{..}=>entity.head_tracking.as_mut().ok_or("LookAt requires head tracking")?.pop(),
            Self::DisallowHandPropActions=>entity.state.as_mut().ok_or("plugin AI state absent")?.flags[3]&=!0x10,
            Self::DisableHeavyCollision=>entity.state.as_mut().ok_or("plugin AI state absent")?.flags[1]&=!0x20,
            Self::IgnoreStandingCollisions=>entity.state.as_mut().ok_or("plugin AI state absent")?.flags[1]&=!0x10,
            //826ece50 sets bit40 unconditionally, rather than restoring it.
            Self::DisableCollisionSliding=>entity.adjust_world_collisions=true,
            Self::OverridePluginCollision=>entity.state.as_mut().ok_or("plugin AI state absent")?.override_external_collision=false,
            //826ecf18 only clears the lock; it retains the stored position.
            Self::LockToCurrentPosition=>entity.spatial_adjust.as_mut().ok_or("position lock requires spatial adjust")?.position_locked=false,
            Self::ExitPlugin=>{}, // actual empty End825187c0
            // SetSimpleTimer's End is the native empty825187c0.
            Self::Simple(SimpleBehaviour::SetTimer{..})=>{},
            Self::Simple(value)=>{if !value.end(entity)? {return Err("plugin simple End not handled".into());}}
        }
        Ok(())
    }
}
/// ExitPlugin826eb090 -> ExitBehaviourPlugin82cef3f8, for an actual pedestrian
/// sink and source Prop. Null source still requests removal from the sink.
pub(crate) fn exit_plugin(entity:&mut PedestrianConstruction,source:Option<u32>,
    props:&mut crate::living_world_grind_services::waypoints::PropMan)->Result<(),String>{
    if let Some(source)=source {
        let id=entity.id;
        props.entities.get_mut(source).ok_or("ExitPlugin source Prop absent")?
            .behaviour_sinks.retain(|sink|*sink!=id);
    }
    entity.plugin_removal_requested=true;
    Ok(())
}
/// LookAtTimeOut826f2598 uses the same retained timer6 as Begin.
pub(crate) fn look_at_timed_out(entity:&PedestrianConstruction)->Result<bool,String>{
    Ok(entity.state.as_ref().ok_or("LookAtTimeOut requires AI state")?.timer(6)<=0.)
}

fn lock_available_waypoint(entity:&mut PedestrianConstruction,source:Option<u32>,
    props:&mut crate::living_world_grind_services::waypoints::PropMan)->Result<bool,String>{
    use crate::living_world_grind_services::waypoints::WaypointRef;
    let Some(prop)=source else{return Ok(false);};
    let source=props.entities.get(prop).ok_or("plugin source Prop absent")?;
    let Some(group)=source.waypoint.as_ref() else{return Ok(false);};
    // Raw827084e4 dispatches Entity vtable+58, GetFirstUnlockedWaypoint82e64b00.
    let Some(index)=group.first_unlocked() else{return Ok(false);};
    let reference=WaypointRef{prop,index};
    entity.set_current_waypoint(Some(reference));
    props.point_mut(reference)?.lock(entity.id)?;
    Ok(true)
}
/// LockAndTargetWayPoint.Begin82708418. The rig-enable write belongs only to
/// successful Begin acquisition; Update's retry intentionally omits it.
pub(crate) fn begin_lock_and_target_waypoint(entity:&mut PedestrianConstruction,source:Option<u32>,
    props:&mut crate::living_world_grind_services::waypoints::PropMan)->Result<(),String>{
    entity.navigation_mut()?.arrival_radius=0.2;
    if entity.current_waypoint.is_none() && lock_available_waypoint(entity,source,props)? {
        entity.navigation_rig_enabled=true;
    }
    Ok(())
}
/// Update82708548 targets an existing point, or acquires one for the next
/// update. Both branches finish by setting seek speed2 through real navigation.
pub(crate) fn update_lock_and_target_waypoint<N:crate::reckoning_numeric::ReckoningNumeric,
    A:crate::pedestrian_navigation::wander::WanderAvoidVolumes>(entity:&mut PedestrianConstruction,
    source:Option<u32>,facts:&crate::pedestrian_navigation::mesh::NavigationEntityFacts,
    world:&mut crate::living_world_pedestrian_entity::targeting::TargetingWorld<'_,A>,numeric:&mut N
)->Result<(),String> where N::Error:std::fmt::Debug {
    if let Some(reference)=entity.current_waypoint {
        let location=world.props.point(reference)?.location;
        entity.target_explicit(location,crate::pedestrian_navigation::NavigationTargetMode::Explicit,
            facts,world,numeric)?;
    }else{lock_available_waypoint(entity,source,world.props)?;}
    entity.navigation_mut()?.seek_speed=2.;
    Ok(())
}
/// UnlockWaypoint.Begin826ebf98 clears the actual reservation and rig bit.
pub(crate) fn unlock_waypoint(entity:&mut PedestrianConstruction,
    props:&mut crate::living_world_grind_services::waypoints::PropMan)->Result<(),String>{
    entity.clear_current_waypoint(props)?;
    entity.navigation_rig_enabled=false;
    Ok(())
}

/// Per-occurrence bool instance allocated by the folded CreateInstance827126f8.
/// Keep this separate from immutable delegate payloads: overlapping controllers
/// must restore their own captured value in native End order.
pub(crate) struct SavedSwitchInstance { previous: bool }
impl SavedSwitchInstance {
    pub(crate) fn new()->Self {Self{previous:false}}
}
pub(crate) enum SavedSwitch {
    DisableAllMoods,
    ChangeNavModifier{kind:crate::pedestrian_navigation_modifiers::ModifierKind, enabled:bool},
}
impl SavedSwitch {
    pub(crate) fn bind(family:Family,index:u16)->Result<Option<Self>,String>{
        use crate::pedestrian_navigation_modifiers::ModifierKind as Kind;
        let declarations=family.declarations();
        let declaration=declarations.behaviours.get(index as usize).ok_or("plugin behaviour index")?;
        if declaration.name=="DisableAllMoods" {return Ok(Some(Self::DisableAllMoods));}
        if declaration.name!="ChangeNavModifierSetting" {return Ok(None);}
        let section=declaration.attributes;
        let fields=family.attributes().get(section.index as usize..section.index as usize+section.count as usize)
            .ok_or("plugin modifier attributes absent")?;
        let get=|name|fields.iter().find(|v|v.key==name).map(|v|v.value);
        //826ee5f8: these five case-insensitive names are the complete mapping;
        // DynamicObject is intentionally not accepted by this native behavior.
        let name=get("modifier").unwrap_or("");
        let kind=if name.eq_ignore_ascii_case("ChaseGlue"){Kind::ChaseGlue}
            else if name.eq_ignore_ascii_case("Pedestrian"){Kind::General}
            else if name.eq_ignore_ascii_case("SkaterAvoidance"){Kind::Skater}
            else if name.eq_ignore_ascii_case("VehicleAvoidance"){Kind::Vehicle}
            else if name.eq_ignore_ascii_case("BigBlackAvoidance"){Kind::BigBlack}
            else{return Err(format!("native ChangeNavModifierSetting assertion: {name}"));};
        let enabled=match get("set_to").unwrap_or("false") {
            "true"|"1"=>true,"false"|"0"=>false,
            value=>return Err(format!("plugin modifier boolean {value}")),
        };
        Ok(Some(Self::ChangeNavModifier{kind,enabled}))
    }
    pub(crate) fn begin(&self,instance:&mut SavedSwitchInstance,entity:&mut PedestrianConstruction)->Result<(),String>{
        match self {
            Self::DisableAllMoods=>{
                let state=entity.state.as_mut().ok_or("plugin mood state absent")?;
                instance.previous=state.flags[2]&8!=0; //826ff310
                state.flags[2]|=8;
            }
            Self::ChangeNavModifier{kind,enabled}=>{
                let modifiers=entity.navigation_modifiers.as_mut().ok_or("plugin navigation modifiers absent")?;
                instance.previous=modifiers.enabled(*kind); //826ff568
                modifiers.set_enabled(*kind,*enabled);
            }
        }
        Ok(())
    }
    pub(crate) fn end(&self,instance:&SavedSwitchInstance,entity:&mut PedestrianConstruction)->Result<(),String>{
        match self {
            Self::DisableAllMoods=>{
                let state=entity.state.as_mut().ok_or("plugin mood state absent")?;
                state.flags[2]=(state.flags[2]&!8)|(u8::from(instance.previous)<<3); //826ff340
            }
            Self::ChangeNavModifier{kind,..}=>entity.navigation_modifiers.as_mut()
                .ok_or("plugin navigation modifiers absent")?.set_enabled(*kind,instance.previous), //826ff608
        }
        Ok(())
    }
}
