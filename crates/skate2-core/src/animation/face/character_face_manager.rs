//! Retained `CharacterFaceManager` load, completion and copied-data ownership.
//! Reference: 0x82d04cf0, 0x82d008d0, 0x82d01868, 0x82d01920, 0x82d01a18.
//! The platform fulfills an issued archive request; completion does not imply
//! that the presentation callback has already been dispatched.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "load producer is exercised through original-data schedules pending native app startup"
    )
)]

use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

#[cfg(test)]
#[path = "character_face_manager_tests.rs"]
mod tests;

pub(crate) struct CharacterFaceManager {
    occurrence: Arc<()>,
    holders: BTreeMap<u32, FaceHolder>,
    requests: VecDeque<FaceRequest>,
    finished: VecDeque<u32>,
}
struct FaceHolder {
    bytes: Option<Box<[u8]>>,
    references: i32,
}
#[derive(Clone, Debug)]
pub(crate) struct FaceLoadTicket {
    owner: Arc<()>,
    hash: u32,
    pub(crate) archive_path: Vec<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FaceRequest {
    pub(crate) asset_id: u64,
    pub(crate) callback: u64,
    face_hash: u32,
}
#[derive(Debug)]
pub(crate) struct FaceLoadAdmission {
    pub(crate) already_loaded: bool,
    /// Only the first request starts an IO operation.
    pub(crate) load: Option<FaceLoadTicket>,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CharacterFaceError {
    ForeignLoad,
    MissingHolder,
    AlreadyComplete,
    LoadFailed,
    NotLoaded,
}
impl CharacterFaceManager {
    pub(crate) fn new() -> Self {
        Self {
            occurrence: Arc::new(()),
            holders: BTreeMap::new(),
            requests: VecDeque::new(),
            finished: VecDeque::new(),
        }
    }

    /// Case-sensitive unsigned-byte hash, seed -1, wrapping multiply33.
    /// The native map uses only this hash; collisions share a holder.
    pub(crate) fn face_hash(name: &[u8]) -> u32 {
        name.iter()
            .copied()
            .take_while(|byte| *byte != 0)
            .fold(u32::MAX, |hash, byte| {
                hash.wrapping_mul(33).wrapping_add(u32::from(byte))
            })
    }

    /// Reference: `LoadFaceData` 0x82d04cf0. Retains an actual request before
    /// issuing the first load; duplicate pending requests share its holder.
    pub(crate) fn begin_load(
        &mut self,
        name: &[u8],
        asset_id: u64,
        callback: u64,
    ) -> FaceLoadAdmission {
        let hash = Self::face_hash(name);
        if let Some(holder) = self.holders.get_mut(&hash) {
            holder.references = holder.references.wrapping_add(1);
            if holder.bytes.is_some() {
                return FaceLoadAdmission {
                    already_loaded: true,
                    load: None,
                };
            }
        }
        self.requests.push_back(FaceRequest {
            asset_id,
            callback,
            face_hash: hash,
        });
        if let Some(holder) = self.holders.get_mut(&hash) {
            holder.references = self
                .requests
                .iter()
                .filter(|request| request.face_hash == hash)
                .fold(0_i32, |count, _| count.wrapping_add(1));
            return FaceLoadAdmission {
                already_loaded: false,
                load: None,
            };
        }
        self.holders.insert(
            hash,
            FaceHolder {
                bytes: None,
                references: 1,
            },
        );
        FaceLoadAdmission {
            already_loaded: false,
            load: Some(FaceLoadTicket {
                owner: Arc::clone(&self.occurrence),
                hash,
                archive_path: Self::archive_path(name),
            }),
        }
    }

    /// The archive request `LoadFaceData` issues for the named face.
    pub(crate) fn archive_path(name: &[u8]) -> Vec<u8> {
        let mut path = b"data\\big/anim.big|data/anim/faces/".to_vec();
        path.extend(name.iter().copied().take_while(|byte| *byte != 0));
        path.extend_from_slice(b".abin");
        // Native snprintf has a 256-byte destination including terminator.
        path.truncate(255);
        for byte in &mut path {
            if *byte == b'/' {
                *byte = b'\\';
            }
        }
        path
    }

    /// Reference: `FaceHolder::OnComplete` 0x82d008d0. IO transfers its buffer;
    /// stores buffer and size, publishes loaded, then queues the SIM event.
    pub(crate) fn complete_load(
        &mut self,
        ticket: &FaceLoadTicket,
        complete: bool,
        bytes: Box<[u8]>,
    ) -> Result<(), CharacterFaceError> {
        if !Arc::ptr_eq(&self.occurrence, &ticket.owner) {
            return Err(CharacterFaceError::ForeignLoad);
        }
        let holder = self
            .holders
            .get_mut(&ticket.hash)
            .ok_or(CharacterFaceError::MissingHolder)?;
        if holder.bytes.is_some() {
            return Err(CharacterFaceError::AlreadyComplete);
        }
        if !complete || bytes.is_empty() {
            return Err(CharacterFaceError::LoadFailed);
        }
        holder.bytes = Some(bytes);
        self.finished.push_back(ticket.hash);
        Ok(())
    }

    /// Reference: 0x82d01a18. Deliver in original request-list order. Failed
    /// delivery retains the current request and the remaining event for retry.
    pub(crate) fn dispatch_finished<E>(
        &mut self,
        mut enqueue_presentation: impl FnMut(FaceRequest) -> Result<(), E>,
    ) -> Result<(), E> {
        while let Some(hash) = self.finished.front().copied() {
            let mut index = 0;
            while index < self.requests.len() {
                let request = self.requests[index];
                if request.face_hash == hash {
                    enqueue_presentation(request)?;
                    self.requests.remove(index);
                } else {
                    index += 1;
                }
            }
            self.finished.pop_front();
        }
        Ok(())
    }
    pub(crate) fn has_face_data(&self, name: &[u8]) -> bool {
        self.holders
            .get(&Self::face_hash(name))
            .is_some_and(|holder| holder.bytes.is_some())
    }
    /// Reference: `GetFaceData` 0x82d01920. Returns a fresh allocation containing
    /// every retained byte, so `DataBase` `E_DELETE` never owns the load buffer.
    pub(crate) fn copy_face_data(&self, name: &[u8]) -> Result<Box<[u8]>, CharacterFaceError> {
        let holder = self
            .holders
            .get(&Self::face_hash(name))
            .ok_or(CharacterFaceError::MissingHolder)?;
        holder.bytes.clone().ok_or(CharacterFaceError::NotLoaded)
    }
}
