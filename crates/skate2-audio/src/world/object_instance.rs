//! The OBJECTINSTANCE state (`eMAINMAPSTATES` 11): `CSTATEMGR_ObjectInstance`,
//! `CSTATE_ObjectInstance`, `SFXCTL_3DObjectInstancePos` (0x000b0000) and `SFXObj_ObjectInstance`
//! (0x000b0000): up to five static world instances near the local skater whose attributes name an
//! `aud_whooshby`, each of which can trigger a whoosh.

use crate::nicotine::{DMixIoId, MixMap};
use crate::player::pos3d::{MicrophoneMan, Obj3d, ObjRefs, Pos3dShared};
use crate::player::SfxCtx;
use crate::system::WorldAudioState;
use crate::vault::Vault;
use crate::vmx::{Vec3, ZERO};
use crate::world::whoosh::{self, WhooshManager};

/// `eMAINMAPSTATES` 11.
const STATE: u32 = 11;

/// `Sk8::AudioPresentation::SpacialInstanceData` (`tInstanceProperties`, 32 bytes) as
/// `AudioPresentation::DoSimSubmit` 0x827fb2f8 fills it: each of up to ten
/// `SpatialInstanceManager::BoundingBoxTest` results within 10 m of the local skater, its position
/// and the whoosh key (`RefSpec` at layout 0 of its attribute collection, 0 without one).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpacialInstanceData {
    pub position: Vec3,
    pub whoosh_key: u64,
}

/// One `CSTATE_ObjectInstance` (0x60 bytes) with its controller and object.
struct ObjectInstanceState {
    attached: bool,
    spacial_instance: SpacialInstanceData,
    position: Obj3d,
    object_io: Option<DMixIoId>,
    /// `SFXObj_ObjectInstance::mPlayedWhoosh`, which only the constructor clears.
    played_whoosh: bool,
}

/// `CSTATEMGR_ObjectInstance`.
pub struct ObjectInstanceManager {
    states: Vec<ObjectInstanceState>,
    num_instances_objects_around: i32,
}

impl ObjectInstanceManager {
    /// `EnterGamePlay` 0x82550c60: five states.
    #[must_use]
    pub fn new(vault: &Vault, map: Option<&MixMap>) -> Self {
        let states = (0..5u32)
            .map(|instance| {
                let id = (STATE << 16) | (instance << 11);
                let mut position = Obj3d::new(vault);
                position.io = map.and_then(|map| map.dmixio(0x6000_0000 | id));
                ObjectInstanceState {
                    attached: false,
                    spacial_instance: SpacialInstanceData::default(),
                    position,
                    object_io: map.and_then(|map| map.dmixio(0x4000_0000 | id)),
                    played_whoosh: false,
                }
            })
            .collect();
        Self { states, num_instances_objects_around: 0 }
    }

    /// `CSTATE_Base::Detach` / `CSTATE_ObjectInstance::Detach` 0x82551b18 (the object's `Destroy`
    /// does nothing).
    fn detach(state: &mut ObjectInstanceState, ctx: &mut SfxCtx<'_>) {
        state.attached = false;
        if let Some(io) = state.object_io {
            ctx.map.set_output_enabled(io, false);
            ctx.map.clear_input(io);
        }
        state.position.detach();
        if let Some(io) = state.position.io {
            ctx.map.clear_input(io);
        }
        state.spacial_instance = SpacialInstanceData { position: ZERO, whoosh_key: 0 };
    }

    /// `UpdateParams` 0x82558300: the first five instances; a state whose position and key (low
    /// word) are still listed keeps them, the others detach, and free states attach to the listed
    /// instances with a key; then `CSTATEMGR_Base::UpdateParams` (`SFXObj_ObjectInstance::UpdateParams`
    /// 0x82549ed8 checks for a whoosh; the position controller has no vectors assigned).
    pub fn update_params(
        &mut self,
        t: f32,
        world: &WorldAudioState,
        mics: &MicrophoneMan,
        shared: &mut Pos3dShared,
        mut whooshes: Option<&mut WhooshManager>,
        ctx: &mut SfxCtx<'_>,
    ) {
        let instances = &world.spacial_instances;
        self.num_instances_objects_around = instances.len() as i32;
        let count = self.num_instances_objects_around.clamp(0, 5) as usize;
        let mut list = [SpacialInstanceData::default(); 5];
        list[..count].copy_from_slice(&instances[..count]);
        for state in &mut self.states {
            if !state.attached {
                continue;
            }
            let held = state.spacial_instance;
            let found = list[..count].iter().position(|entry| {
                entry.position[0] == held.position[0]
                    && entry.position[1] == held.position[1]
                    && entry.position[2] == held.position[2]
                    && held.whoosh_key as u32 == entry.whoosh_key as u32
            });
            match found {
                Some(slot) => list[slot].whoosh_key = (count as u64) << 32,
                None => Self::detach(state, ctx),
            }
        }
        if self.num_instances_objects_around > 0 {
            for entry in &list[..count] {
                if entry.whoosh_key as u32 == 0 {
                    continue;
                }
                if let Some(state) = self.states.iter_mut().find(|state| !state.attached) {
                    state.attached = true;
                    for io in [state.position.io, state.object_io].into_iter().flatten() {
                        ctx.map.set_output_enabled(io, true);
                    }
                    state.spacial_instance = *entry;
                }
            }
        }
        for state in &mut self.states {
            if !state.attached {
                continue;
            }
            state.position.update_params(t, ObjRefs::default(), mics, shared, ctx.map);
            let instance = state.spacial_instance;
            whoosh::check_for_whoosh(&mut state.played_whoosh, instance.whoosh_key, instance.position, mics, whooshes.as_deref_mut(), ctx.vault);
        }
    }

    pub fn process_update(&mut self, _ctx: &mut SfxCtx<'_>) {}
}
