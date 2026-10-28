//! Shared native Challenge queue7 producers, not a mission/event dispatcher.
//! EventManager.Init82982638 -> EventSys.Init82985538 constructs its retained
//! 65536-byte queue. The caller retains this owner before player registration.
use std::sync::{Arc, Mutex};

const QUEUE_BYTES: usize = 0x1_0000;
#[path = "hermes_queue_storage.rs"]
pub(crate) mod storage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkaterNotification {
    Created,
    Destroyed,
    LocalChanged,
}

/// Native message header plus the copied Actor player ID. No Actor pointer is
/// retained by the source message: callbacks sample `GetPlayerID` before enqueue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChallengeMessage {
    pub(crate) kind: u32,
    pub(crate) port: u32,
    pub(crate) size: u32,
    pub(crate) id: u32,
    pub(crate) payload: ChallengePayload,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ChallengePayload {
    Skater { notification: SkaterNotification, player_id: i32 },
    /// `cMsgPlayerRadiusEnter`/`cMsgPlayerRadiusExit`: `fDistance` and `fPlayerID`.
    PlayerRadius { enter: bool, distance: f32, player_id: i32 },
    LocalChaseStart,
    LocalChaseEnd,
    GuardKnockedDown { entity_id: u32 },
    /// `cMsgSetMarker` (`set`) or `cMsgUseMarker`: `fPlayerID`.
    Marker { set: bool, player_id: i32 },
}
impl ChallengeMessage {
    fn new(payload: ChallengePayload) -> Self {
        Self {
            kind: match payload {
                ChallengePayload::Skater { notification: SkaterNotification::Created, .. } => (-340_057_085_i32) as u32,
                ChallengePayload::Skater { notification: SkaterNotification::Destroyed, .. } => (-465_644_327_i32) as u32,
                ChallengePayload::Skater { notification: SkaterNotification::LocalChanged, .. } => 2_104_924_600,
                ChallengePayload::PlayerRadius { enter: true, .. } => (-397_789_218_i32) as u32,
                ChallengePayload::PlayerRadius { enter: false, .. } => 2_010_261_299,
                ChallengePayload::LocalChaseStart => 661_379_046,
                ChallengePayload::LocalChaseEnd => 1_771_027_901,
                ChallengePayload::GuardKnockedDown { .. } => (-2_000_196_102_i32) as u32,
                ChallengePayload::Marker { set: true, .. } => (-116_504_286_i32) as u32,
                ChallengePayload::Marker { set: false, .. } => 799_139_884,
            },
            port: 0,
            size: match payload {
                ChallengePayload::Skater { .. }
                | ChallengePayload::GuardKnockedDown { .. }
                | ChallengePayload::Marker { .. } => 20,
                ChallengePayload::PlayerRadius { .. } => 24,
                _ => 16,
            },
            id: 0,
            payload,
        }
    }
}

pub(crate) type ChallengeQueueEntry=storage::Entry<ChallengeMessage>;
pub(crate) type ChallengeQueueError=storage::QueueError;
impl std::fmt::Display for ChallengeQueueError {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{write!(f,"Hermes queue: {self:?}")}
}
pub(crate) struct ChallengeEventQueue { storage:storage::Queue<ChallengeMessage> }
impl ChallengeEventQueue {
    pub(crate) fn try_construct()->Result<Arc<Self>,ChallengeQueueError>{
        Ok(Arc::new(Self{storage:storage::Queue::try_construct(QUEUE_BYTES)?}))
    }
    fn enqueue(&self,message:ChallengeMessage)->Result<(),ChallengeQueueError>{self.storage.enqueue(message,message.size as usize)}
    /// The queued messages in order, each removed once taken.
    pub(crate) fn try_take_all(&self)->Result<Vec<ChallengeMessage>,ChallengeQueueError>{
        let mut messages=Vec::new();
        loop{
            match self.storage.pop(|message|{messages.push(message);Ok::<(),std::convert::Infallible>(())}){
                Ok(true)=>{}
                Ok(false)=>return Ok(messages),
                Err(storage::PopError::Queue(error))=>return Err(error),
                Err(storage::PopError::Delivery(never))=>match never{},
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn snapshot(&self)->Result<(usize,usize,Vec<ChallengeQueueEntry>),ChallengeQueueError>{self.storage.snapshot()}
}

/// `Sk8ChallengeSystem` constructor825a7444..7478 retains port Hash32("Challenge"),
/// queueID7 and mutex. Hash32's native seed matches the existing string helper.
pub(crate) struct ChallengeEventBroadcaster {
    queue: Arc<ChallengeEventQueue>,
    port: u32,
    mutex: Mutex<()>,
}
impl ChallengeEventBroadcaster {
    pub(crate) fn construct(queue: Arc<ChallengeEventQueue>) -> Self {
        Self {
            queue,
            port: skate2_formats::state_graph::state_graph_string_hash32(b"Challenge"),
            mutex: Mutex::new(()),
        }
    }

    /// Callback825b83f8/825b85c8 -> `SendMsg825b6d40` -> Enqueue829832f8.
    pub(crate) fn send(
        &self,
        notification: SkaterNotification,
        player_id: i32,
    ) -> Result<(), ChallengeQueueError> {
        self.send_payload(ChallengePayload::Skater { notification, player_id })
    }

    /// RegisterChaser8271bacc / UnRegisterChaser82718b74: header-only native
    /// messages, sent through this same Challenge broadcaster and queue7.
    pub(crate) fn send_local_chase(&self, started: bool) -> Result<(), ChallengeQueueError> {
        self.send_payload(if started { ChallengePayload::LocalChaseStart } else { ChallengePayload::LocalChaseEnd })
    }

    /// `AISkaterManager::UpdateSkaterDistanceCallbackMessages` 0x824c8470:
    /// `SendMsg<cMsgPlayerRadiusEnter>` with either kind.
    pub(crate) fn send_player_radius(&self, enter: bool, distance: f32, player_id: i32) -> Result<(), ChallengeQueueError> {
        self.send_payload(ChallengePayload::PlayerRadius { enter, distance, player_id })
    }

    /// PedestrianColliding.Begin82716b1c..b54 copies the actual guard LW ID.
    pub(crate) fn send_guard_knocked_down(&self,entity_id:u32)->Result<(),ChallengeQueueError>{
        self.send_payload(ChallengePayload::GuardKnockedDown{entity_id})
    }

    /// `cMsgSetMarker` from `PlayerUI::UpdateSessionMarker` 0x828af3f0 (on the Simulation's
    /// GameSimulationBroadcaster) or `cMsgUseMarker` from `SkaterManager::OnCommand`
    /// 0x82600920 (on its own broadcaster). Their subscriber, `cChallengeObjectivesManager`
    /// (`Init` 0x82d53680), takes them here, where the career drains every update.
    pub(crate) fn send_marker(&self, set: bool, player_id: i32) -> Result<(), ChallengeQueueError> {
        self.send_payload(ChallengePayload::Marker { set, player_id })
    }

    fn send_payload(&self, payload: ChallengePayload) -> Result<(), ChallengeQueueError> {
        let mut message = ChallengeMessage::new(payload);
        let _guard = self
            .mutex
            .lock()
            .map_err(|_| ChallengeQueueError::Poisoned)?;
        message.port = self.port;
        self.queue.enqueue(message)
    }
}

#[cfg(test)]
#[path = "hermes_challenge_queue_tests.rs"]
mod tests;
