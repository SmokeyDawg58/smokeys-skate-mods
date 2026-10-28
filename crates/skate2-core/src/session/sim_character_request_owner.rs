//! Retained request fields of `SimCharacterManager`; selected load/register path.
//! Constructor `8281cf70`, `Load8281a1f0`, `Register82817a08`, `OnLoaded82815418`.
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Load,
    Register,
    Unload,
}
struct Request {
    id: u32,
    kind: Kind,
    callback: Arc<()>,
    player: u32,
}
pub(crate) struct SimCharacterRequests {
    next: u32,
    pending: [Option<Request>; 256],
}
pub(crate) struct LoadTicket {
    id: u32,
    callback: Arc<()>,
    player: u32,
}
pub(crate) struct UnloadTicket(LoadTicket);
impl UnloadTicket {
    pub(crate) fn id(&self) -> u32 {
        self.0.id
    }
}
impl LoadTicket {
    pub(crate) fn id(&self) -> u32 {
        self.id
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RequestError {
    Capacity,
    Missing(u32),
    ForeignCallback,
    WrongKind,
}
impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capacity => f.write_str("all 256 character requests are occupied"),
            Self::Missing(id) => write!(f, "character request {id} absent"),
            Self::ForeignCallback => f.write_str("foreign character load callback"),
            Self::WrongKind => f.write_str("character request kind disagrees with callback"),
        }
    }
}
impl SimCharacterRequests {
    pub(crate) fn construct_fields() -> Self {
        Self {
            next: 0,
            pending: std::array::from_fn(|_| None),
        }
    }
    pub(crate) fn next_request_id(&self) -> u32 {
        self.next
    }
    fn allocate(
        &mut self,
        kind: Kind,
        callback: Arc<()>,
        player: u32,
    ) -> Result<u32, RequestError> {
        let slot = self
            .pending
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(RequestError::Capacity)?;
        let id = self.next;
        *slot = Some(Request {
            id,
            kind,
            callback,
            player,
        });
        self.next = self.next.wrapping_add(1);
        if self.next == u32::MAX {
            self.next = 0;
        }
        Ok(id)
    }
    pub(crate) fn begin_load(
        &mut self,
        callback: Arc<()>,
        player: u32,
    ) -> Result<LoadTicket, RequestError> {
        Ok(LoadTicket {
            id: self.allocate(Kind::Load, Arc::clone(&callback), player)?,
            callback,
            player,
        })
    }
    fn load_slot(&self, ticket: &LoadTicket) -> Result<usize, RequestError> {
        self.ticket_slot(ticket, Kind::Load)
    }
    fn ticket_slot(&self, ticket: &LoadTicket, kind: Kind) -> Result<usize, RequestError> {
        let index = self
            .pending
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|r| r.id == ticket.id))
            .ok_or(RequestError::Missing(ticket.id))?;
        let r = self.pending[index]
            .as_ref()
            .expect("located occupied request");
        if !Arc::ptr_eq(&r.callback, &ticket.callback) || r.player != ticket.player {
            return Err(RequestError::ForeignCallback);
        }
        if r.kind != kind {
            return Err(RequestError::WrongKind);
        }
        Ok(index)
    }
    /// The native callback, rather than a local player, owns this request.
    /// MAX is only the host's existing provenance field, never a player lookup.
    pub(crate) fn begin_ambient_load(
        &mut self,
        callback: Arc<()>,
    ) -> Result<LoadTicket, RequestError> {
        self.begin_load(callback, u32::MAX)
    }
    /// Unload82817910 uses the SAME 256 slots and next-request counter as Load.
    pub(crate) fn begin_unload(&mut self, callback: Arc<()>) -> Result<UnloadTicket, RequestError> {
        Ok(UnloadTicket(LoadTicket {
            id: self.allocate(Kind::Unload, Arc::clone(&callback), u32::MAX)?,
            callback,
            player: u32::MAX,
        }))
    }
    /// OnUnloaded828154a0 releases the slot only AFTER the consumer callback.
    pub(crate) fn finish_unloaded_callback(
        &mut self,
        ticket: &UnloadTicket,
    ) -> Result<(), RequestError> {
        let index = self.ticket_slot(&ticket.0, Kind::Unload)?;
        self.pending[index] = None;
        Ok(())
    }
    /// Called inside the loaded callback, while its source load slot is occupied.
    pub(crate) fn begin_registration(&mut self, ticket: &LoadTicket) -> Result<u32, RequestError> {
        self.load_slot(ticket)?;
        self.allocate(Kind::Register, Arc::clone(&ticket.callback), ticket.player)
    }
    /// Native callback returns before its old load request is released.
    pub(crate) fn finish_loaded_callback(
        &mut self,
        ticket: &LoadTicket,
    ) -> Result<(), RequestError> {
        let index = self.load_slot(ticket)?;
        self.pending[index] = None;
        Ok(())
    }
    /// OnRegistered releases the matching register slot after its callback.
    pub(crate) fn finish_registered_callback(
        &mut self,
        id: u32,
        callback: &Arc<()>,
        player: u32,
    ) -> Result<(), RequestError> {
        let ticket = LoadTicket {
            id,
            callback: Arc::clone(callback),
            player,
        };
        let index = self.ticket_slot(&ticket, Kind::Register)?;
        self.pending[index] = None;
        Ok(())
    }
}
#[cfg(test)]
#[path = "sim_character_request_owner_tests.rs"]
mod tests;
