//! ActorTracker82e675f8: specialized Prop, with one store5 allocation.
use super::PropMan;

pub(crate) struct ActorTrackerData {
    pub(crate) allocation:u32,
    pub(crate) recognizers:Vec<u32>,
}
impl PropMan {
    pub(crate) fn recognizer_count(&self)->Result<Option<usize>,String>{
        let Some(id)=self.actor_tracker else{return Ok(None);};
        Ok(Some(self.entities.get(id).and_then(|p|p.actor_tracker.as_ref())
            .ok_or("Manager ActorTracker payload absent")?.recognizers.len()))
    }
    /// RegisterRecognizer82e682f0: unique IDs, silently ignore capacity16.
    pub(crate) fn register_recognizer(&mut self,id:u32)->Result<(),String>{
        let Some(tracker)=self.actor_tracker else{return Ok(());};
        let data=self.entities.get_mut(tracker).and_then(|p|p.actor_tracker.as_mut())
            .ok_or("Manager ActorTracker payload absent")?;
        if data.recognizers.len()<16&&!data.recognizers.contains(&id){data.recognizers.push(id);}
        Ok(())
    }
    /// UnRegisterRecognizer82e66e88: erase the first matching ID, in order.
    pub(crate) fn unregister_recognizer(&mut self,id:u32)->Result<(),String>{
        let Some(tracker)=self.actor_tracker else{return Ok(());};
        let data=self.entities.get_mut(tracker).and_then(|p|p.actor_tracker.as_mut())
            .ok_or("Manager ActorTracker payload absent")?;
        if let Some(index)=data.recognizers.iter().position(|entry|*entry==id){data.recognizers.remove(index);}
        Ok(())
    }
}
impl crate::living_world_pedestrian_entity::PedestrianConstruction {
    /// CanRecognizePlayer82e6da90 -> Manager8271a1c0. The missing attribute's
    /// original one-byte DefaultDataArea is zero; no reputation invented here.
    pub(crate) fn can_recognize_player(&self)->Result<bool,String>{
        Ok(self.resources.as_ref().ok_or("recognition pedestrian resources absent")?.entity
            .try_get_data(0x2466_b253_f1c0_6fb4,0).map_err(|e|e.to_string())?
            .and_then(|data|data.bytes().first().copied()).unwrap_or(0)!=0)
    }
}
