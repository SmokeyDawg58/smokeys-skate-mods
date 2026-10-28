//! Unregistered Normal child: SAME-owner Skeleton.Fill suffix82b589b8..59848.
//! The caller carries the original585b0 COM velocity and borrows actual `PhysOut`.
use super::{SkeletonNormalAssembly, SkeletonPreNormalSimulationBackend};
use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::part_contact_spy::DynamicAssemblyPartContactSpyError,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::{
        PhysicalPlayerPhysOut,
        construct::{
            skeleton_audio::SkeletonAudioOutputError,
            skeleton_fill_suffix::{SkeletonFillSuffixError, SkeletonFillSuffixSource},
            skeleton_ik_fill::SkeletonIkOutputError,
        },
    },
    physics::PointGraph,
    reckoning_numeric::ReckoningNumeric,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skeleton_collision_data::fill_audio::{SkeletonAudioError, SkeletonAudioServices},
    skeleton_collision_update_attributes::SkeletonCollisionUpdateAttributes,
    skeleton_ik_state::fill_phys_out::{SkeletonIkFillError, SkeletonIkFillServices},
    skeleton_physics_history::SkeletonPhysicsHistory,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum SkeletonFillLiveServiceError {
    Missing(&'static str),
    ProcessedPoisoned,
    ReckoningPoisoned,
    PartOutsideRecord(usize),
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    Part(DynamicAssemblyPartContactSpyError),
    Body(SimulationRigidBodyPoolError),
    Transform(super::part_transform_query::SkeletonPartTransformQueryError),
    Mirror(super::effective_animation_transform::SkeletonEffectiveAnimationUnknownMirror),
}
impl std::fmt::Display for SkeletonFillLiveServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(value) => write!(f, "Skeleton Fill missing {value}"),
            Self::ProcessedPoisoned => f.write_str("Skeleton Fill Processed lock poisoned"),
            Self::ReckoningPoisoned => f.write_str("Skeleton Fill Reckoning lock poisoned"),
            Self::PartOutsideRecord(part) => {
                write!(f, "Skeleton Fill record part {part} out of bounds")
            }
            Self::Attribute(e) => write!(f, "Skeleton Fill attribute: {e:?}"),
            Self::Cvar(e) => write!(f, "Skeleton Fill cvar: {e:?}"),
            Self::Overlay(e) => write!(f, "Skeleton Fill overlay: {e:?}"),
            Self::Part(e) => write!(f, "Skeleton Fill part: {e:?}"),
            Self::Body(e) => write!(f, "Skeleton Fill body: {e:?}"),
            Self::Transform(e) => write!(f, "Skeleton Fill transform: {e:?}"),
            Self::Mirror(e) => write!(f, "Skeleton Fill mirror: {e:?}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum SkeletonFillLiveError<N> {
    Service(SkeletonFillLiveServiceError),
    Ik(
        SkeletonIkFillError<
            SkeletonFillLiveServiceError,
            AttributeRuntimeError,
            SkeletonIkOutputError,
            N,
        >,
    ),
    Audio(
        SkeletonAudioError<
            SkeletonFillLiveServiceError,
            AttributeRuntimeError,
            SkeletonAudioOutputError,
            N,
        >,
    ),
}
impl<N: std::fmt::Debug> std::fmt::Display for SkeletonFillLiveError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Service(e) => e.fmt(f),
            Self::Ik(e) => e.fmt(f),
            Self::Audio(e) => e.fmt(f),
        }
    }
}

/// Immutable same-Vault views follow existing Collision.Update binding policy.
/// No payload is read when constructing this adapter. Global replacement/editor
/// races remain outside the existing immutable-Vault admission contract.
struct Globals<'a> {
    vault: &'a Arc<AttributeRuntimeVault>,
    references: &'a InputCvarReferences,
    overlay: &'a DebugOverlaySlot,
}
impl Globals<'_> {
    fn skeleton(&self) -> AttributeRuntimeInstance {
        self.vault.instance(
            SkeletonCollisionUpdateAttributes::SKELETON_CLASS,
            SkeletonCollisionUpdateAttributes::DEFAULT_COLLECTION,
        )
    }
    fn require_overlay(&self) -> Result<(), SkeletonFillLiveServiceError> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(SkeletonFillLiveServiceError::Overlay)
    }
}
fn layout_word(bytes: &[u8], offset: usize) -> f32 {
    f32::from_be_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked generated layout"),
    )
}
fn record_vector(
    record: &crate::skeleton_physics_history::SkeletonPhysicsRecord,
    part: usize,
    field: fn(usize) -> crate::skeleton_physics_history::lane_knowledge::RecordVectorField,
) -> Result<SkeletonRootVector, SkeletonFillLiveServiceError> {
    if part >= 24 {
        return Err(SkeletonFillLiveServiceError::PartOutsideRecord(part));
    }
    Ok(record.vector_typed(field(part)))
}

struct IkServices<'a, 'b> {
    history: &'a SkeletonPhysicsHistory,
    processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    globals: &'a Globals<'b>,
}
impl SkeletonIkFillServices for IkServices<'_, '_> {
    type Error = SkeletonFillLiveServiceError;
    type Global = AttributeRuntimeInstance;
    fn capture_skeleton_global(&mut self) -> Result<Self::Global, Self::Error> {
        Ok(self.globals.skeleton())
    }
    fn hand_grab_margin(&mut self, global: &Self::Global) -> Result<[f32; 4], Self::Error> {
        let layout = global
            .try_cached_layout(0x70)
            .map_err(Self::Error::Attribute)?;
        let bytes = layout
            .as_ref()
            .map_or(&[0; 0x70][..], AttributeRuntimeData::bytes);
        Ok(std::array::from_fn(|lane| {
            layout_word(bytes, 0x20 + lane * 4)
        }))
    }
    fn physics_column(
        &mut self,
        part: usize,
        column: usize,
    ) -> Result<SkeletonRootVector, Self::Error> {
        let matrix = self
            .history
            .record()
            .transform_typed(part)
            .ok_or(Self::Error::PartOutsideRecord(part))?;
        matrix
            .columns()
            .get(column)
            .copied()
            .ok_or(Self::Error::Missing("source matrix column outside0..4"))
    }

    fn delta_time(&mut self) -> Result<f32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::ProcessedPoisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(Self::Error::Missing("Processed93c timestep"))
    }
    fn physics_velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        record_vector(
            self.history.record(),
            part,
            crate::skeleton_physics_history::lane_knowledge::RecordVectorField::Velocity,
        )
    }
    fn draw_contact_info(&mut self) -> Result<bool, Self::Error> {
        self.globals
            .references
            .try_draw_contact_info()
            .map_err(Self::Error::Cvar)
    }
    fn debug_overlay_noop(&mut self) -> Result<(), Self::Error> {
        self.globals.require_overlay()
    }
}
struct AudioServices<'a, 'b> {
    history: &'a SkeletonPhysicsHistory,
    globals: &'a Globals<'b>,
}
impl SkeletonAudioServices for AudioServices<'_, '_> {
    type Error = SkeletonFillLiveServiceError;
    fn velocity_delta(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        record_vector(
            self.history.record(),
            part,
            crate::skeleton_physics_history::lane_knowledge::RecordVectorField::VelocityDelta,
        )
    }
    fn position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        record_vector(
            self.history.record(),
            part,
            crate::skeleton_physics_history::lane_knowledge::RecordVectorField::Position,
        )
    }
    fn mass(&mut self, part: usize) -> Result<f32, Self::Error> {
        self.history
            .part_masses()
            .get(part)
            .copied()
            .ok_or(Self::Error::PartOutsideRecord(part))
    }
    fn draw_audio_info(&mut self) -> Result<bool, Self::Error> {
        self.globals
            .references
            .try_draw_audio_collision_info()
            .map_err(Self::Error::Cvar)
    }
    fn debug_overlay_noop(&mut self) -> Result<(), Self::Error> {
        self.globals.require_overlay()
    }
}

struct Source<'a> {
    normal: &'a mut SkeletonNormalAssembly,
    backend: &'a SkeletonPreNormalSimulationBackend,
    owner: &'a PhysicalPlayerAssemblyOwner,
    processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    globals: Globals<'a>,
}
impl<N: ReckoningNumeric> SkeletonFillSuffixSource<N> for Source<'_> {
    type Error = SkeletonFillLiveError<N::Error>;
    fn angular_velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        let observation = self
            .normal
            .session
            .try_part_contact_spy_observation(self.backend, part)
            .map_err(|e| Self::Error::Service(SkeletonFillLiveServiceError::Part(e)))?;
        let xyz = self
            .backend
            .collision_body_pool()
            .try_phys_out_angular_velocity(observation)
            .map_err(|e| Self::Error::Service(SkeletonFillLiveServiceError::Body(e)))?;
        Ok(SkeletonRootVector { xyz, w: None })
    }
    fn hook_to_hips(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self.normal.post_physics_records.hook_to_hips())
    }
    fn proxy_in_collision(&mut self) -> Result<bool, Self::Error> {
        Ok(self.normal.state.collision.is_proxy_in_collision != 0)
    }
    fn physics_velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        record_vector(
            self.normal.physics_history.record(),
            part,
            crate::skeleton_physics_history::lane_knowledge::RecordVectorField::Velocity,
        )
        .map_err(Self::Error::Service)
    }
    fn effective_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        let processed = self
            .processed
            .lock()
            .map_err(|_| Self::Error::Service(SkeletonFillLiveServiceError::ProcessedPoisoned))?;
        self.normal
            .try_effective_animation_to_world(&processed)
            .map_err(|e| Self::Error::Service(SkeletonFillLiveServiceError::Mirror(e)))
    }
    fn part_transform(&mut self, part: usize) -> Result<SkeletonRootMatrix, Self::Error> {
        self.normal
            .try_query_part_transform(part)
            .map_err(|e| Self::Error::Service(SkeletonFillLiveServiceError::Transform(e)))
    }
    fn fill_ik(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        numeric: &mut N,
    ) -> Result<(), Self::Error> {
        // Resolve actual runtime only when native56590 is reached. Separate
        // fields permit history reads while the SAME IK foot histories mutate.
        let ik = self
            .normal
            .current_animation
            .ik_runtime_mut()
            .ok_or(Self::Error::Service(SkeletonFillLiveServiceError::Missing(
                "constructed IK",
            )))?;
        ik.try_fill_phys_out(
            &mut IkServices {
                history: &self.normal.physics_history,
                processed: self.processed,
                globals: &self.globals,
            },
            output,
            numeric,
        )
        .map_err(Self::Error::Ik)
    }
    fn reckoning_up(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        //59658 resolves Skeleton+3df4,5965c reads actual Reckoning+1b0.
        // Do not substitute Processed's prior-output copy at+220.
        let reckoning =
            self.owner.reckoning().lock().map_err(|_| {
                Self::Error::Service(SkeletonFillLiveServiceError::ReckoningPoisoned)
            })?;
        Ok(reckoning
            .reckoning()
            .ok_or(Self::Error::Service(SkeletonFillLiveServiceError::Missing(
                "actual Reckoning+1b0",
            )))?
            .up())
    }
    /// Shoulder twist 82b5963c..59644: Skeleton+0 `mProcessedPhysIn` +160
    /// `mBodyZAxis`, not the flattened +170 direction.
    fn processed_body_z(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Service(SkeletonFillLiveServiceError::ProcessedPoisoned))?
            .toolkit_state()
            .body_z()
            .ok_or(Self::Error::Service(SkeletonFillLiveServiceError::Missing(
                "Processed bodyZ160",
            )))
    }
    fn compliant_just_collided(&mut self) -> Result<bool, Self::Error> {
        Ok(self.normal.state.collision.compliant_just_collided != 0)
    }
    fn fill_audio_collision(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        numeric: &mut N,
    ) -> Result<(), Self::Error> {
        let mut attributes = SkeletonCollisionUpdateAttributes::bind(self.globals.vault);
        self.normal
            .state
            .collision
            .try_fill_audio_collision(
                &mut attributes,
                &mut AudioServices {
                    history: &self.normal.physics_history,
                    globals: &self.globals,
                },
                output,
                numeric,
            )
            .map_err(Self::Error::Audio)
    }
    fn ik_contact_offset(&mut self, index: usize) -> Result<f32, Self::Error> {
        let ik = self
            .normal
            .current_animation
            .ik_runtime()
            .ok_or(Self::Error::Service(SkeletonFillLiveServiceError::Missing(
                "constructed IK",
            )))?;
        ik.state()
            .contact_offsets()
            .get(index)
            .copied()
            .ok_or(Self::Error::Service(SkeletonFillLiveServiceError::Missing(
                "source IK offset index",
            )))
    }
    fn feet_compression_threshold(&mut self) -> Result<f32, Self::Error> {
        let value = self
            .globals
            .skeleton()
            .try_get_data(0x2f1f_013c_9907_9afb, 0)
            .map_err(|e| Self::Error::Service(SkeletonFillLiveServiceError::Attribute(e)))?;
        let Some(value) = value else {
            return Ok(0.0);
        }; // native DefaultDataArea4
        let bytes = value.bytes();
        let array = bytes.try_into().map_err(|_| {
            Self::Error::Service(SkeletonFillLiveServiceError::Attribute(
                AttributeRuntimeError::ScalarSize { size: bytes.len() },
            ))
        })?;
        Ok(f32::from_be_bytes(array))
    }
    fn feet_compression_curve(&mut self) -> Result<PointGraph<4>, Self::Error> {
        let layout = self
            .globals
            .skeleton()
            .try_cached_layout(0x70)
            .map_err(|e| Self::Error::Service(SkeletonFillLiveServiceError::Attribute(e)))?;
        let bytes = layout
            .as_ref()
            .map_or(&[0; 0x70][..], AttributeRuntimeData::bytes);
        Ok(PointGraph {
            x: std::array::from_fn(|i| layout_word(bytes, 0x50 + i * 4)),
            y: std::array::from_fn(|i| layout_word(bytes, 0x60 + i * 4)),
        })
    }
    fn want_respawn(&mut self) -> Result<bool, Self::Error> {
        Ok(self.normal.general_update.want_respawn())
    }
    fn clear_want_respawn(&mut self) -> Result<(), Self::Error> {
        self.normal.general_update.clear_want_respawn_after_fill();
        Ok(())
    }
}

impl SkeletonNormalAssembly {
    /// Pool caller checks same board/Skeleton/constructor owner and same
    /// Processed Arc before borrowing `PhysOut`. No latch or owner lookup is
    /// repeated inside the mandatory IK/audio child callbacks.
    #[expect(
        clippy::too_many_arguments,
        reason = "same actual owners and late source services remain explicit"
    )]
    pub(crate) fn try_fill_output_suffix_live<N: ReckoningNumeric>(
        &mut self,
        backend: &SkeletonPreNormalSimulationBackend,
        owner: &PhysicalPlayerAssemblyOwner,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        vault: &Arc<AttributeRuntimeVault>,
        references: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
        output: &mut PhysicalPlayerPhysOut,
        captured_com_velocity: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), SkeletonFillSuffixError<SkeletonFillLiveError<N::Error>, N::Error>> {
        output.try_fill_skeleton_suffix(
            &mut Source {
                normal: self,
                backend,
                owner,
                processed,
                globals: Globals {
                    vault,
                    references,
                    overlay,
                },
            },
            captured_com_velocity,
            numeric,
        )
    }
}
