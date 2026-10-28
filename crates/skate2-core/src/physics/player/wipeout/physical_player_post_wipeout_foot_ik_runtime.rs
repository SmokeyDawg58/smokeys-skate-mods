//! Child of `SkeletonNormalAssembly`. Concrete actual Parts/record/Vault binding.
use super::SkeletonNormalAssembly;
#[path = "physical_player_post_wipeout_ik_update_runtime.rs"]
pub(crate) mod ik_update;
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError,
    ragdoll::SkeletonPartIndex as Part,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_assembly::{SkeletonPreNormalAssemblySession, SkeletonPreNormalSimulationBackend},
    skeleton_ik::SkeletonIkDataIn,
    skeleton_ik_attributes::{SkeletonIkAttributeRead, SkeletonIkVaultAttributes},
    skeleton_ik_geometry::SkeletonIkGeometry,
    skeleton_ik_state::{
        SkeletonIkState,
        post_wipeout_foot::{FootIkError, FootIkInput, FootIkSource},
    },
    skeleton_ik_targets::SkeletonIkTargets,
    skeleton_physics_history::SkeletonPhysicsHistory,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};

/// Narrow shared-owner accessor return type; all fields are borrowed from the
/// same `current_animation.ik_runtime` and `current_animation.ik`. No reset/copy.
pub(crate) struct FootIkRetainedOwners<'a> {
    pub(crate) state: &'a mut SkeletonIkState,
    pub(crate) data_in: &'a SkeletonIkDataIn,
    pub(crate) targets: &'a SkeletonIkTargets,
    pub(crate) geometry: &'a SkeletonIkGeometry,
    pub(crate) attributes: &'a mut SkeletonIkVaultAttributes,
}
pub(crate) trait FootIkLiveDebug {
    type Error;
    fn draw_post_feet(&mut self) -> Result<bool, Self::Error>;
    fn draw_post_physics(&mut self) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
pub(crate) struct FootIkLiveInput<'a, D> {
    pub(crate) processed: &'a SkeletonProcessedContact,
    pub(crate) backend: &'a mut SkeletonPreNormalSimulationBackend,
    /// Existing global `physics_animation.default` instance, not Skeleton+4.
    pub(crate) physics_animation: &'a AttributeRuntimeInstance,
    pub(crate) deck: &'a Matrix,
    pub(crate) wipeout_starting: bool,
    pub(crate) debug: &'a mut D,
}
#[derive(Debug)]
pub(crate) enum FootIkLiveError<D> {
    Missing(&'static str),
    Attribute(AttributeRuntimeError),
    Part(DynamicAssemblyPartTransformError),
    Record(
        crate::skeleton_physics_history::SkeletonPhysicsHistoryError<
            std::convert::Infallible,
            std::convert::Infallible,
        >,
    ),
    Debug(D),
}
impl<D: std::fmt::Debug> std::fmt::Display for FootIkLiveError<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "foot IK missing {v}"),
            Self::Attribute(e) => write!(f, "foot IK attribute: {e:?}"),
            Self::Part(e) => write!(f, "foot IK Part: {e:?}"),
            Self::Record(e) => write!(f, "foot IK record: {e:?}"),
            Self::Debug(e) => write!(f, "foot IK debug: {e:?}"),
        }
    }
}
struct Source<'a, D> {
    /// Actual retained global plain animation, distinct from `physics_animation`.
    global_animation: Option<&'a AttributeRuntimeInstance>,
    session: &'a mut SkeletonPreNormalAssemblySession,
    history: &'a mut SkeletonPhysicsHistory,
    own_attributes: &'a mut SkeletonIkVaultAttributes,
    input: FootIkLiveInput<'a, D>,
}
impl<D: FootIkLiveDebug> FootIkSource for Source<'_, D> {
    type Error = FootIkLiveError<D::Error>;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
        let (v, k) = self.input.processed.known_flags_8c4();
        if k & 0x40000 == 0 {
            return Err(Self::Error::Missing("Processed8c4 bit40000"));
        }
        Ok(v)
    }
    fn state_timer(&mut self) -> Result<f32, Self::Error> {
        self.input
            .processed
            .physics_state_fields()
            .state_timer
            .ok_or(Self::Error::Missing("Processed974"))
    }
    fn minimum_deck_up_y(&mut self) -> Result<f32, Self::Error> {
        let key = 0x3b26_41b6_4fcf_3485;
        let data = self
            .input
            .physics_animation
            .try_get_data(key, 0)
            .map_err(Self::Error::Attribute)?;
        let Some(data) = data else {
            return Ok(0.0);
        };
        let bytes = data.bytes();
        if bytes.len() < 4 {
            return Err(Self::Error::Attribute(AttributeRuntimeError::PayloadSize {
                key,
                expected: 4,
                actual: bytes.len(),
            }));
        }
        Ok(f32::from_be_bytes(
            bytes[..4].try_into().expect("four bytes"),
        ))
    }
    fn deck_bounds_margin(&mut self) -> Result<Vector, Self::Error> {
        self.own_attributes
            .vector(0x6934_53cd_b831_119b)
            .map(Vector::known)
            .map_err(Self::Error::Attribute)
    }
    fn feet_relative_height(&mut self, wipeout: bool) -> Result<f32, Self::Error> {
        let animation = self.global_animation.ok_or(Self::Error::Missing(
            "GlobalAttributeCollection animation+0xd4",
        ))?;
        let key = if wipeout {
            0x38e9_1dfb_cb5e_99f3
        } else {
            0x8e12_e37f_f8c7_1792
        };
        // Generated animation getter: reached live lookup, missing field alone
        // uses DefaultDataArea4; a missing global owner is never a default.
        Ok(f32::from_bits(
            animation
                .try_scalar_word(key, 0)
                .map_err(Self::Error::Attribute)?
                .unwrap_or(0),
        ))
    }
    fn record(&mut self, part: Part) -> Result<Matrix, Self::Error> {
        Ok(self.history.post_wipeout_record_transform(part.index()))
    }
    fn set_part(&mut self, part: Part, matrix: &Matrix) -> Result<(), Self::Error> {
        self.session
            .try_set_part_transform_typed(self.input.backend, part.index(), matrix)
            .map_err(Self::Error::Part)
    }
    fn set_record(&mut self, part: Part, matrix: &Matrix) -> Result<(), Self::Error> {
        self.history
            .try_set_post_wipeout_record_transform(part.index(), matrix)
            .map_err(Self::Error::Record)
    }
    fn draw_post_feet_ik(&mut self) -> Result<bool, Self::Error> {
        self.input
            .debug
            .draw_post_feet()
            .map_err(Self::Error::Debug)
    }
    fn draw_post_phys_ik(&mut self) -> Result<bool, Self::Error> {
        self.input
            .debug
            .draw_post_physics()
            .map_err(Self::Error::Debug)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.input
            .debug
            .require_overlay()
            .map_err(Self::Error::Debug)
    }
}
impl SkeletonNormalAssembly {
    /// Root calls after actual `PostWipeout` branch selection and any wobble root
    /// adjustment. Does not include `HandIK`, `UpdatePostPhysics` IK or pose output.
    #[expect(
        clippy::type_complexity,
        reason = "preserve distinct source and numeric failures"
    )]
    pub(crate) fn try_post_wipeout_foot_live<
        D: FootIkLiveDebug,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        input: FootIkLiveInput<'_, D>,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), FootIkError<FootIkLiveError<D::Error>, N::Error, R::Error>> {
        let owners = self
            .current_animation
            .post_wipeout_foot_parts_mut()
            .ok_or(FootIkError::Missing("retained IK runtime"))?;
        let mut source = Source {
            global_animation: self
                .drives
                .as_ref()
                .and_then(super::drive_owner::SkeletonDriveRuntime::global_animation_instance),
            session: &mut self.session,
            history: &mut self.physics_history,
            own_attributes: owners.attributes,
            input,
        };
        owners.state.try_post_wipeout_foot_ik(
            FootIkInput {
                data_in: owners.data_in,
                targets: owners.targets,
                geometry: owners.geometry,
                deck: source.input.deck,
                wipeout_starting: source.input.wipeout_starting,
            },
            &mut source,
            numeric,
            rounding,
        )
    }
}
