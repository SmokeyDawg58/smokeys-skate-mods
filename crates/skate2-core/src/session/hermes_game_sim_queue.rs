//! Simulation's real GameSimulationBroadcaster/queue4, shared with LW through
//! InitLWMan82702eb8. Queue allocation829826a4=24576, ctor828a8728..873c.
use crate::hermes_challenge_queue::storage::{Queue,QueueError,PopError};
use std::sync::{Arc,Mutex};
/// cMsgMoodEvent: native float payload retained as words, including NaN bits.
/// BroadcastCollisionEvent82711df8 -> SendMsg82711520, size80.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) struct MoodEvent {
    pub(crate) event_type:i32,
    pub(crate) position:[u32;4],
    pub(crate) perpetrator:crate::living_world_chase::TargetableId,
    pub(crate) victim:crate::living_world_chase::TargetableId,
    pub(crate) loudness_distance_factor:u32,
    pub(crate) visibility_distance_factor:u32,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum LivingWorldMessage {
    SecuritySpawned(u32),SecurityCulled(u32),RequestCull(u32),
    WaypointGroupAdded(u64),WaypointGroupRemoved(u64),
    Mood(MoodEvent),
    ChaseStart(u32), ChaseEnd(u32),
    ChaserState{entity:u32,state:i32},
    SetSimRate { sim_time:u32, ticks:i32 },
}
impl LivingWorldMessage {
    pub(crate) fn kind(self)->u32{match self{
        Self::SecuritySpawned(_)=>(-994188055_i32) as u32,
        Self::SecurityCulled(_)=>(-141156312_i32) as u32,
        Self::RequestCull(_)=>1004304572,
        Self::WaypointGroupAdded(_)=>480852204,
        Self::WaypointGroupRemoved(_)=>1452361618,
        Self::Mood(_)=>(-164509450_i32) as u32,
        Self::ChaseStart(_)=>662517041,
        Self::ChaseEnd(_)=>(-1049887331_i32) as u32,
        Self::ChaserState{..}=>(-268789995_i32) as u32,
        Self::SetSimRate{..}=>0x3fbf_5e43,
    }}
    fn size(self)->usize{match self{Self::Mood(_)=>80,Self::WaypointGroupAdded(_)|Self::WaypointGroupRemoved(_)|Self::SetSimRate{..}|Self::ChaserState{..}=>24,_=>20}}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) struct GameSimMessage {
    pub(crate) kind:u32,pub(crate) port:u32,pub(crate) size:u32,pub(crate) id:u32,
    pub(crate) payload:LivingWorldMessage,
}
/// Concrete subscriber ownership is supplied at the original Flush boundary.
/// FE handlers826063e8/82606410 call MapBackEnd.Add/RemoveSecurityIcon;
/// challenge handler825a5798 marks pending spawns and rebinds authored waypoint
/// entities. RequestCull also reaches the registered Lua message handlers.
pub(crate) trait GameSimSubscribers {
    fn deliver(&mut self,message:&GameSimMessage)->Result<(),String>;
}
pub(crate) struct GameSimEventRoute {
    queue:Queue<GameSimMessage>,
    port:u32,
    dispatch_tick:Mutex<u32>,
    mood_subscribed: std::sync::atomic::AtomicBool,
    pub(crate) minimap_icons: Mutex<Vec<(bool,u32,crate::ui::minimap::Icon)>>,
    /// cChallengeCharacterManager.ReceiveWaypointGroupAddedMessage825a5798 while a
    /// career subscribes: the groups added since the career took them.
    pub(crate) challenge_waypoint_groups: Mutex<Option<Vec<u64>>>,
    /// `FrontEndManager::mChaseArrowMan`, which `ReceiveChaseStateMsg` (0x82634db8) adds to.
    pub(crate) chase_arrows: Mutex<crate::ui::chase_arrows::ChaseArrowMan>,
}
impl GameSimEventRoute {
    pub(crate) fn try_construct()->Result<Arc<Self>,QueueError>{
        Ok(Arc::new(Self{queue:Queue::try_construct(24576)?,
            port:skate2_formats::state_graph::state_graph_string_hash32(b"GameSimulationBroadcaster"),
            dispatch_tick:Mutex::new(0),mood_subscribed:std::sync::atomic::AtomicBool::new(false),
            minimap_icons:Mutex::new(Vec::new()),challenge_waypoint_groups:Mutex::new(None),
            chase_arrows:Mutex::default()}))
    }
    /// MoodMan.Init82ce96f4..9788 registers its ambient Mood handler on this
    /// same broadcaster. Delivery remains at the original Flush boundary.
    pub(crate) fn register_mood_man(&self) {
        self.mood_subscribed.store(true, std::sync::atomic::Ordering::Release);
    }
    pub(crate) fn mood_man_registered(&self) -> bool {
        self.mood_subscribed.load(std::sync::atomic::Ordering::Acquire)
    }
    pub(crate) fn send(&self,payload:LivingWorldMessage)->Result<(),String>{
        self.queue.enqueue(GameSimMessage{kind:payload.kind(),port:self.port,size:payload.size() as u32,id:0,payload},payload.size())
            .map_err(|e|format!("GameSim queue4: {e:?}"))
    }
    /// Dispatch824eccf8 -> Flush82982968 -> Pop829827b8. Newly appended
    /// messages participate in the same flush; tick increments only afterward.
    pub(crate) fn dispatch(&self,subscribers:&mut impl GameSimSubscribers)->Result<(),String>{
        self.dispatch_with(|message| subscribers.deliver(message))?;
        Ok(())
    }
    /// Both the tick and paused frontend supply the complete registered
    /// subscriber owners. Queue.Pop retains an event on a checked failure.
    pub(crate) fn dispatch_with(&self,mut deliver:impl FnMut(&GameSimMessage)->Result<(),String>)->Result<(),String>{
        loop {
            match self.queue.pop(|message|deliver(&message)){
                Ok(true)=>{},Ok(false)=>break,
                Err(PopError::Queue(e))=>return Err(format!("GameSim queue4: {e:?}")),
                Err(PopError::Delivery(e))=>return Err(e),
            }
        }
        let mut tick=self.dispatch_tick.lock().map_err(|_|"GameSim dispatch tick lock")?;
        *tick=tick.wrapping_add(1);Ok(())
    }
}
impl crate::living_world_grind_services::LivingWorldGrindServices {
    /// OnSpawnComplete82e85f18, AFTER actual map and execution admissions.
    pub(crate) fn pedestrian_on_spawn_complete(&self,id:u32)->Result<(),String>{
        let pedestrian=self.pedestrian_entities.as_ref().and_then(|entities|entities.get(id))
            .ok_or("OnSpawnComplete requires admitted pedestrian")?;
        if security_message_enabled(pedestrian)?{
            self.game_sim_events.as_ref().ok_or("Simulation GameSim broadcaster absent")?
                .send(LivingWorldMessage::SecuritySpawned(id))?;
        }
        Ok(())
    }
    /// Event suffix ONLY: callers first complete native base/animation/handprop/
    /// chase operations in OnCullRequested82e85fd0. The unconditional RequestCull
    /// precedes the security attribute read and optional second enqueue.
    pub(crate) fn pedestrian_cull_messages(&self,id:u32)->Result<(),String>{
        let route=self.game_sim_events.as_ref().ok_or("Simulation GameSim broadcaster absent")?;
        route.send(LivingWorldMessage::RequestCull(id))?;
        let pedestrian=self.pedestrian_entities.as_ref().and_then(|entities|entities.get(id))
            .ok_or("OnCullRequested event suffix requires still-admitted pedestrian")?;
        if security_message_enabled(pedestrian)?{route.send(LivingWorldMessage::SecurityCulled(id))?;}
        Ok(())
    }
}
pub(crate) fn security_message_enabled(pedestrian:&crate::living_world_pedestrian_entity::PedestrianConstruction)->Result<bool,String>{
    // Raw82e85f30..4c forms full key79d412ca0b043f37. This callback's
    // LWBasicEntity this+20 bias makes +1940 actual mAttribulatorData+1960.
    let entity=&pedestrian.resources.as_ref().ok_or("pedestrian resources absent")?.entity;
    Ok(entity.try_get_data(0x79d4_12ca_0b04_3f37,0).map_err(|e|e.to_string())?
        .is_some_and(|data|data.bytes().first().is_some_and(|&value|value!=0)))
}
