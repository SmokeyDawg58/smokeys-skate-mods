//! Standalone lifecycle boundary for `Sk8::Behaviours::PlayAnimation`.
//!
//! This reconstructs the observable `CreateInstance`, `UpdateAttributes`,
//! `Begin`, `Update`, and `End` ordering. Animation playback, intent storage,
//! interface discovery internals, and generated-controller integration remain
//! explicit collaborators rather than inferred implementations.
//! Invalid raw `AttributeParam` discriminants are outside this seam because
//! the stored-data parser exposes a closed, validated source-type enum.
//! The failed-primary uppercase scratch conversion is backend-unobservable
//! and intentionally omitted; this is semantic, not instruction-trace, parity.

use std::{error::Error, fmt};

use crate::play_animation::{
    ANDALE_NULL_STRING_NAME_30, AnimationAttributeParamType, AnimationTransitionParams,
    AnimationTransitionType, FastString30, FastString36, PlayAnimationAttributeParam,
    PlayAnimationBehaviour, TRANSITION_BLEND_WITH_CURRENT_FRAME_MASK,
    TRANSITION_USE_CHANNEL_FROM_WEIGHTS_MASK,
};

pub const PLAY_ANIMATION_INSTANCE_SIZE: usize = 0x0c;
pub const PLAY_ANIMATION_INSTANCE_ALIGNMENT: usize = 0x10;
pub const PLAY_ANIMATION_INSTANCE_ALLOCATION_NAME: &[u8] = b"unknown";
pub const PLAY_ANIMATION_SEQUENCE_ID: i32 = -1;
pub const PLAY_ANIMATION_TIME_ZERO_BITS: u32 = 0;
pub const PLAY_ANIMATION_FALLBACK_SPEED_BITS: u32 = 1.0f32.to_bits();

#[derive(Debug, Eq, PartialEq)]
pub struct PlayAnimationRuntimeInstance {
    first_update_raw: u8,
}

impl PlayAnimationRuntimeInstance {
    #[must_use]
    pub const fn first_update_raw(&self) -> u8 {
        self.first_update_raw
    }

    #[must_use]
    pub const fn first_update(&self) -> bool {
        self.first_update_raw != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayAnimationInterfaceKind {
    Physical,
    Animatable,
    SkaterAnim,
    MotionGraph,
    SkaterMotionGraph,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayAnimationInterfaceResolution {
    Resolved,
    /// Checked form of the Actor cached-pointer assertion used by Physical and
    /// `SkaterAnim` resolution.
    /// Valid only for the Actor-backed Physical and `SkaterAnim` lookups.
    MissingActorCached,
    /// The UCOM result was null; native diagnoses this at the aggregate check.
    Missing,
    /// The cached interface and generic UCOM lookup did not identify the same
    /// object.
    IdentityMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TreeAttributeSample {
    pub type_raw: i8,
    pub value_bits: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RawAnimationTransitionParams {
    pub transition_type_raw: u32,
    pub blend_length_bits: u32,
    pub placement_raw: u32,
    pub flags_raw: u32,
}

impl From<AnimationTransitionParams> for RawAnimationTransitionParams {
    fn from(value: AnimationTransitionParams) -> Self {
        let mut flags_raw = 0;
        if value.blend_with_current_frame {
            flags_raw |= TRANSITION_BLEND_WITH_CURRENT_FRAME_MASK;
        }
        if value.use_channel_from_weights {
            flags_raw |= TRANSITION_USE_CHANNEL_FROM_WEIGHTS_MASK;
        }
        Self {
            transition_type_raw: value.transition_type as u32,
            blend_length_bits: value.blend_length_bits,
            placement_raw: value.placement as u32,
            flags_raw,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalOutputObservation {
    MissingPhysOut,
    MissingOffBoard,
    HoldingSkateboardRaw(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnimationDispatch {
    Play {
        name: FastString36,
        playback_speed_bits: u32,
        time_bits: u32,
    },
    Blend {
        name: FastString36,
        blend_length_bits: u32,
        playback_speed_bits: u32,
        time_bits: u32,
        blend_with_current_frame: bool,
        placement_raw: u32,
    },
    ChannelBlend {
        name: FastString36,
        blend_length_bits: u32,
        playback_speed_bits: u32,
        time_bits: u32,
        blend_with_current_frame: bool,
        placement_raw: u32,
        use_channel_from_weights: bool,
    },
    Sequence {
        name: FastString36,
        playback_speed_bits: u32,
        time_bits: u32,
        placement_raw: u32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayAnimationBackendOperation {
    AllocateInstance,
    ResolveInterface(PlayAnimationInterfaceKind),
    MotionIntent,
    FilteredMotionIntent,
    TreeAttribute,
    AddSettableAttribute,
    SelectedProSkater,
    SetConstructionAttribute,
    NextTransitionOverride,
    ClearTransitionOverride,
    IsSwitch,
    IsMirrored,
    PhysicalOutput,
    Dispatch,
}

#[derive(Debug)]
pub enum PlayAnimationRuntimeError<E> {
    /// Host collaborator failure; native interface methods do not expose this
    /// Rust error channel.
    Backend {
        operation: PlayAnimationBackendOperation,
        source: E,
    },
    MissingInstance,
    MissingActorCached(PlayAnimationInterfaceKind),
    InterfaceIdentityMismatch(PlayAnimationInterfaceKind),
    MissingInterfaces(Vec<PlayAnimationInterfaceKind>),
    NullTransitionOverride,
    NullPhysOut,
    NullOffBoard,
    InvalidTreeAttributeType {
        index: usize,
        raw: i8,
    },
    InvalidTransitionType(u32),
    FallbackDispatchFailed,
}

impl<E: fmt::Display> fmt::Display for PlayAnimationRuntimeError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend { operation, source } => {
                write!(formatter, "PlayAnimation {operation:?} collaborator failed: {source}")
            }
            Self::MissingInstance => {
                formatter.write_str("PlayAnimation Context instance pointer is null")
            }
            Self::MissingActorCached(kind) => {
                write!(formatter, "PlayAnimation {kind:?} Actor cached pointer is null")
            }
            Self::InterfaceIdentityMismatch(kind) => write!(
                formatter,
                "PlayAnimation cached and generic {kind:?} interfaces differ"
            ),
            Self::MissingInterfaces(kinds) => {
                write!(formatter, "PlayAnimation required interfaces are null: {kinds:?}")
            }
            Self::NullTransitionOverride => {
                formatter.write_str("PlayAnimation transition override pointer is null")
            }
            Self::NullPhysOut => formatter.write_str("PlayAnimation PhysOut pointer is null"),
            Self::NullOffBoard => {
                formatter.write_str("PlayAnimation PhysOut OffBoard pointer is null")
            }
            Self::InvalidTreeAttributeType { index, raw } => write!(
                formatter,
                "PlayAnimation AttributeParam {index} tree type {raw} reaches a native assertion"
            ),
            Self::InvalidTransitionType(raw) => write!(
                formatter,
                "PlayAnimation transition type {raw:#x} reaches a native assertion"
            ),
            Self::FallbackDispatchFailed => formatter.write_str(
                "PlayAnimation KeepDefaultAnim fallback returned false at the native assertion boundary",
            ),
        }
    }
}

impl<E: Error + 'static> Error for PlayAnimationRuntimeError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub trait PlayAnimationInstanceAllocator {
    type Error;

    /// Returns false for the native allocation-failure/null result.
    ///
    /// # Errors
    ///
    /// Returns a host allocator failure distinct from native allocation
    /// exhaustion.
    fn allocate(&mut self, size: usize, name: &[u8], alignment: usize)
    -> Result<bool, Self::Error>;
}

#[allow(clippy::missing_errors_doc)]
pub trait PlayAnimationLifecycleBackend {
    type Error;

    fn resolve_interface(
        &mut self,
        kind: PlayAnimationInterfaceKind,
    ) -> Result<PlayAnimationInterfaceResolution, Self::Error>;
    fn motion_intent(&mut self, name: FastString36) -> Result<Option<u32>, Self::Error>;
    fn filtered_motion_intent(&mut self, name: FastString36) -> Result<Option<u32>, Self::Error>;
    /// A `Some` result promises that the native tree query initialized both
    /// the signed type byte and first value word represented by the sample.
    /// This is a semantic boundary, not a model of the native 64-byte
    /// temporary: native initializes only buffer bytes 0..4 to positive zero,
    /// leaves buffer bytes 4..24 untouched, and seeds begin/end to -1, the
    /// name words to zero, flags to 6, type to 0, and sequence to -1.
    fn tree_attribute(
        &mut self,
        name: FastString30,
    ) -> Result<Option<TreeAttributeSample>, Self::Error>;
    fn add_settable_attribute(
        &mut self,
        name: FastString30,
        value_bits: u32,
        normalize: bool,
        sequence: i32,
    ) -> Result<(), Self::Error>;
    fn selected_pro_skater(&mut self) -> Result<FastString30, Self::Error>;
    fn set_construction_attribute(
        &mut self,
        name: FastString30,
        value: FastString30,
    ) -> Result<(), Self::Error>;
    /// `Some` with raw type zero is the valid native "no override" value.
    /// `None` represents the invalid null pointer that native dereferences.
    fn next_transition_override(
        &mut self,
    ) -> Result<Option<RawAnimationTransitionParams>, Self::Error>;
    fn clear_transition_override(&mut self) -> Result<(), Self::Error>;
    fn is_switch(&mut self) -> Result<bool, Self::Error>;
    fn is_mirrored(&mut self) -> Result<bool, Self::Error>;
    fn physical_output(&mut self) -> Result<PhysicalOutputObservation, Self::Error>;
    /// Returns the native bool result byte; every nonzero value is true.
    fn dispatch(&mut self, request: AnimationDispatch) -> Result<u8, Self::Error>;
}

impl PlayAnimationBehaviour {
    /// Reconstructs the allocation-visible portion of folded `CreateInstance`
    /// at `0x82cba730`.
    ///
    /// # Errors
    ///
    /// Returns a host allocator error. Native allocation exhaustion is the
    /// successful `None` result.
    pub fn create_runtime_instance<A: PlayAnimationInstanceAllocator>(
        &self,
        allocator: &mut A,
    ) -> Result<Option<PlayAnimationRuntimeInstance>, PlayAnimationRuntimeError<A::Error>> {
        let allocated = allocator
            .allocate(
                PLAY_ANIMATION_INSTANCE_SIZE,
                PLAY_ANIMATION_INSTANCE_ALLOCATION_NAME,
                PLAY_ANIMATION_INSTANCE_ALIGNMENT,
            )
            .map_err(|source| PlayAnimationRuntimeError::Backend {
                operation: PlayAnimationBackendOperation::AllocateInstance,
                source,
            })?;
        Ok(allocated.then_some(PlayAnimationRuntimeInstance {
            first_update_raw: 1,
        }))
    }

    /// Reconstructs `UpdateAttributes` at `0x82cc92d0`.
    ///
    /// # Errors
    ///
    /// Returns collaborator failures or checked native assertion boundaries.
    pub fn update_runtime_attributes<B: PlayAnimationLifecycleBackend>(
        &self,
        backend: &mut B,
        update_from_last_anim: bool,
    ) -> Result<(), PlayAnimationRuntimeError<B::Error>> {
        resolve_required_group(
            backend,
            &[
                PlayAnimationInterfaceKind::Animatable,
                PlayAnimationInterfaceKind::MotionGraph,
                PlayAnimationInterfaceKind::SkaterMotionGraph,
            ],
        )?;
        for (index, param) in self.params().iter().enumerate() {
            update_one_attribute(backend, index, param, update_from_last_anim)?;
        }
        Ok(())
    }

    /// Reconstructs `Begin` at `0x82cc9818`.
    ///
    /// # Errors
    ///
    /// Returns collaborator failures and checked forms of native assertion or
    /// non-null pointer boundaries. The instance latch is rearmed before any
    /// collaborator call and is not rolled back on error.
    pub fn begin_runtime<B: PlayAnimationLifecycleBackend>(
        &self,
        instance: Option<&mut PlayAnimationRuntimeInstance>,
        backend: &mut B,
    ) -> Result<(), PlayAnimationRuntimeError<B::Error>> {
        let instance = instance.ok_or(PlayAnimationRuntimeError::MissingInstance)?;
        instance.first_update_raw = 1;
        // Native copies all four packed names before its first interface lookup.
        let animation_names = RuntimeAnimationNames {
            base: self.anim_name,
            switch: self.switch_anim_name,
            mirror: self.mirror_anim_name,
            no_board: self.no_board_anim_name,
        };

        resolve_required_group(
            backend,
            &[
                PlayAnimationInterfaceKind::Physical,
                PlayAnimationInterfaceKind::Animatable,
                PlayAnimationInterfaceKind::SkaterAnim,
                PlayAnimationInterfaceKind::MotionGraph,
            ],
        )?;
        self.update_runtime_attributes(backend, true)?;

        let pro_skater_key = fast_string_30("ProSkater");
        let pro_skater =
            backend
                .selected_pro_skater()
                .map_err(|source| PlayAnimationRuntimeError::Backend {
                    operation: PlayAnimationBackendOperation::SelectedProSkater,
                    source,
                })?;
        backend
            .set_construction_attribute(pro_skater_key, pro_skater)
            .map_err(|source| PlayAnimationRuntimeError::Backend {
                operation: PlayAnimationBackendOperation::SetConstructionAttribute,
                source,
            })?;

        let mut transition = RawAnimationTransitionParams::from(self.transition);
        let transition_override = backend.next_transition_override().map_err(|source| {
            PlayAnimationRuntimeError::Backend {
                operation: PlayAnimationBackendOperation::NextTransitionOverride,
                source,
            }
        })?;
        let transition_override =
            transition_override.ok_or(PlayAnimationRuntimeError::NullTransitionOverride)?;
        if transition_override.transition_type_raw != 0 {
            transition = transition_override;
        }
        backend.clear_transition_override().map_err(|source| {
            PlayAnimationRuntimeError::Backend {
                operation: PlayAnimationBackendOperation::ClearTransitionOverride,
                source,
            }
        })?;

        let selected = self.select_runtime_animation_from(animation_names, backend)?;
        let dispatch = dispatch_request(transition, selected, self.playback_speed_bits)?;
        let succeeded =
            backend
                .dispatch(dispatch)
                .map_err(|source| PlayAnimationRuntimeError::Backend {
                    operation: PlayAnimationBackendOperation::Dispatch,
                    source,
                })?;
        if succeeded == 0 {
            let fallback = AnimationDispatch::Play {
                name: fast_string_36("KeepDefaultAnim"),
                playback_speed_bits: PLAY_ANIMATION_FALLBACK_SPEED_BITS,
                time_bits: PLAY_ANIMATION_TIME_ZERO_BITS,
            };
            let fallback_succeeded = backend.dispatch(fallback).map_err(|source| {
                PlayAnimationRuntimeError::Backend {
                    operation: PlayAnimationBackendOperation::Dispatch,
                    source,
                }
            })?;
            if fallback_succeeded == 0 {
                return Err(PlayAnimationRuntimeError::FallbackDispatchFailed);
            }
        }
        Ok(())
    }

    /// Reconstructs `Update` at `0x82ccb840`.
    ///
    /// # Errors
    ///
    /// Returns collaborator failures and checked native pointer/assertion
    /// boundaries. An attribute-update error leaves an already-zero latch zero.
    pub fn update_runtime<B: PlayAnimationLifecycleBackend>(
        &self,
        instance: Option<&mut PlayAnimationRuntimeInstance>,
        backend: &mut B,
    ) -> Result<(), PlayAnimationRuntimeError<B::Error>> {
        let instance = instance.ok_or(PlayAnimationRuntimeError::MissingInstance)?;
        if instance.first_update_raw == 0 {
            self.update_runtime_attributes(backend, false)?;
        }
        instance.first_update_raw = 0;
        Ok(())
    }

    /// Folded native `End` at `0x825187c0` is a one-instruction no-op.
    pub const fn end_runtime(&self) {}

    #[cfg(test)]
    fn select_runtime_animation<B: PlayAnimationLifecycleBackend>(
        &self,
        backend: &mut B,
    ) -> Result<FastString36, PlayAnimationRuntimeError<B::Error>> {
        self.select_runtime_animation_from(
            RuntimeAnimationNames {
                base: self.anim_name,
                switch: self.switch_anim_name,
                mirror: self.mirror_anim_name,
                no_board: self.no_board_anim_name,
            },
            backend,
        )
    }

    fn select_runtime_animation_from<B: PlayAnimationLifecycleBackend>(
        &self,
        names: RuntimeAnimationNames,
        backend: &mut B,
    ) -> Result<FastString36, PlayAnimationRuntimeError<B::Error>> {
        let mut selected = names.base;
        if self.has_switch_anim
            && backend
                .is_switch()
                .map_err(|source| PlayAnimationRuntimeError::Backend {
                    operation: PlayAnimationBackendOperation::IsSwitch,
                    source,
                })?
        {
            selected = names.switch;
        }
        if self.has_mirror_anim
            && backend
                .is_mirrored()
                .map_err(|source| PlayAnimationRuntimeError::Backend {
                    operation: PlayAnimationBackendOperation::IsMirrored,
                    source,
                })?
        {
            selected = names.mirror;
        }
        if self.has_no_board_anim {
            match backend.physical_output().map_err(|source| {
                PlayAnimationRuntimeError::Backend {
                    operation: PlayAnimationBackendOperation::PhysicalOutput,
                    source,
                }
            })? {
                PhysicalOutputObservation::MissingPhysOut => {
                    return Err(PlayAnimationRuntimeError::NullPhysOut);
                }
                PhysicalOutputObservation::MissingOffBoard => {
                    return Err(PlayAnimationRuntimeError::NullOffBoard);
                }
                PhysicalOutputObservation::HoldingSkateboardRaw(0) => {
                    selected = names.no_board;
                }
                PhysicalOutputObservation::HoldingSkateboardRaw(_) => {}
            }
        }
        Ok(selected)
    }
}

#[derive(Clone, Copy)]
struct RuntimeAnimationNames {
    base: FastString36,
    switch: FastString36,
    mirror: FastString36,
    no_board: FastString36,
}

fn resolve_required_group<B: PlayAnimationLifecycleBackend>(
    backend: &mut B,
    kinds: &[PlayAnimationInterfaceKind],
) -> Result<(), PlayAnimationRuntimeError<B::Error>> {
    let mut missing = Vec::new();
    for &kind in kinds {
        let resolution = backend.resolve_interface(kind).map_err(|source| {
            PlayAnimationRuntimeError::Backend {
                operation: PlayAnimationBackendOperation::ResolveInterface(kind),
                source,
            }
        })?;
        match resolution {
            PlayAnimationInterfaceResolution::Resolved => {}
            PlayAnimationInterfaceResolution::MissingActorCached => {
                return Err(PlayAnimationRuntimeError::MissingActorCached(kind));
            }
            PlayAnimationInterfaceResolution::Missing => missing.push(kind),
            PlayAnimationInterfaceResolution::IdentityMismatch => {
                return Err(PlayAnimationRuntimeError::InterfaceIdentityMismatch(kind));
            }
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(PlayAnimationRuntimeError::MissingInterfaces(missing))
    }
}

fn update_one_attribute<B: PlayAnimationLifecycleBackend>(
    backend: &mut B,
    index: usize,
    param: &PlayAnimationAttributeParam,
    update_from_last_anim: bool,
) -> Result<(), PlayAnimationRuntimeError<B::Error>> {
    match param.source_type {
        AnimationAttributeParamType::FromLastAnim => {
            if !update_from_last_anim {
                return Ok(());
            }
            let destination = renamed_or(param.attribute, param.rename);
            let sample = backend.tree_attribute(param.attribute).map_err(|source| {
                PlayAnimationRuntimeError::Backend {
                    operation: PlayAnimationBackendOperation::TreeAttribute,
                    source,
                }
            })?;
            match sample {
                Some(TreeAttributeSample {
                    type_raw: 0 | 2,
                    value_bits,
                }) => add_settable(backend, destination, value_bits, false),
                Some(TreeAttributeSample {
                    type_raw: 1 | 3, ..
                }) => Ok(()),
                Some(TreeAttributeSample { type_raw, .. }) => {
                    Err(PlayAnimationRuntimeError::InvalidTreeAttributeType {
                        index,
                        raw: type_raw,
                    })
                }
                None if param.use_default => {
                    add_settable(backend, destination, param.default_value_bits, false)
                }
                None => Ok(()),
            }
        }
        AnimationAttributeParamType::FromIntent => {
            let destination = renamed_or(fast_string_36_to_30(param.intent), param.rename);
            let value = backend.motion_intent(param.intent).map_err(|source| {
                PlayAnimationRuntimeError::Backend {
                    operation: PlayAnimationBackendOperation::MotionIntent,
                    source,
                }
            })?;
            add_optional_intent_value(backend, param, destination, value)
        }
        AnimationAttributeParamType::FromFilteredIntent => {
            let destination = renamed_or(fast_string_36_to_30(param.intent), param.rename);
            let value = backend
                .filtered_motion_intent(param.intent)
                .map_err(|source| PlayAnimationRuntimeError::Backend {
                    operation: PlayAnimationBackendOperation::FilteredMotionIntent,
                    source,
                })?;
            add_optional_intent_value(backend, param, destination, value)
        }
    }
}

fn add_optional_intent_value<B: PlayAnimationLifecycleBackend>(
    backend: &mut B,
    param: &PlayAnimationAttributeParam,
    destination: FastString30,
    value: Option<u32>,
) -> Result<(), PlayAnimationRuntimeError<B::Error>> {
    match value {
        Some(value_bits) => add_settable(backend, destination, value_bits, param.normalize),
        None if param.use_default => add_settable(
            backend,
            destination,
            param.default_value_bits,
            param.normalize,
        ),
        None => Ok(()),
    }
}

fn add_settable<B: PlayAnimationLifecycleBackend>(
    backend: &mut B,
    name: FastString30,
    value_bits: u32,
    normalize: bool,
) -> Result<(), PlayAnimationRuntimeError<B::Error>> {
    backend
        .add_settable_attribute(name, value_bits, normalize, PLAY_ANIMATION_SEQUENCE_ID)
        .map_err(|source| PlayAnimationRuntimeError::Backend {
            operation: PlayAnimationBackendOperation::AddSettableAttribute,
            source,
        })
}

fn dispatch_request<E>(
    transition: RawAnimationTransitionParams,
    name: FastString36,
    playback_speed_bits: u32,
) -> Result<AnimationDispatch, PlayAnimationRuntimeError<E>> {
    let blend_with_current_frame =
        transition.flags_raw & TRANSITION_BLEND_WITH_CURRENT_FRAME_MASK != 0;
    let use_channel_from_weights =
        transition.flags_raw & TRANSITION_USE_CHANNEL_FROM_WEIGHTS_MASK != 0;
    match transition.transition_type_raw {
        raw if raw == AnimationTransitionType::Play as u32 => Ok(AnimationDispatch::Play {
            name,
            playback_speed_bits,
            time_bits: PLAY_ANIMATION_TIME_ZERO_BITS,
        }),
        raw if raw == AnimationTransitionType::Blend as u32 => Ok(AnimationDispatch::Blend {
            name,
            blend_length_bits: transition.blend_length_bits,
            playback_speed_bits,
            time_bits: PLAY_ANIMATION_TIME_ZERO_BITS,
            blend_with_current_frame,
            placement_raw: transition.placement_raw,
        }),
        raw if raw == AnimationTransitionType::ChannelBlend as u32 => {
            Ok(AnimationDispatch::ChannelBlend {
                name,
                blend_length_bits: transition.blend_length_bits,
                playback_speed_bits,
                time_bits: PLAY_ANIMATION_TIME_ZERO_BITS,
                blend_with_current_frame,
                placement_raw: transition.placement_raw,
                use_channel_from_weights,
            })
        }
        raw if raw == AnimationTransitionType::Sequence as u32 => Ok(AnimationDispatch::Sequence {
            name,
            playback_speed_bits,
            time_bits: PLAY_ANIMATION_TIME_ZERO_BITS,
            placement_raw: transition.placement_raw,
        }),
        raw => Err(PlayAnimationRuntimeError::InvalidTransitionType(raw)),
    }
}

const fn fast_string_36_to_30(value: FastString36) -> FastString30 {
    FastString30 {
        words: [
            value.words[0],
            value.words[1],
            value.words[2],
            value.words[3],
            value.words[4],
        ],
    }
}

fn renamed_or(fallback: FastString30, rename: FastString30) -> FastString30 {
    if rename.words == ANDALE_NULL_STRING_NAME_30.words {
        fallback
    } else {
        rename
    }
}

fn fast_string_30(value: &str) -> FastString30 {
    FastString30::encode(value).expect("static PlayAnimation FastString30 literal is valid")
}

fn fast_string_36(value: &str) -> FastString36 {
    FastString36::encode(value).expect("static PlayAnimation FastString36 literal is valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play_animation::{AnimationTransitionPlacement, PlayAnimationParseError};
    use skate2_formats::state_graph::{StateGraphBinaryAttribute, state_graph_string_hash32};
    use std::collections::VecDeque;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestError;

    impl fmt::Display for TestError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("injected backend failure")
        }
    }

    impl Error for TestError {}

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Event {
        Resolve(PlayAnimationInterfaceKind),
        MotionIntent(FastString36),
        FilteredMotionIntent(FastString36),
        TreeAttribute(FastString30),
        AddSettable {
            name: FastString30,
            value_bits: u32,
            normalize: bool,
            sequence: i32,
        },
        SelectedProSkater,
        SetConstruction {
            name: FastString30,
            value: FastString30,
        },
        NextOverride,
        ClearOverride,
        IsSwitch,
        IsMirrored,
        PhysicalOutput,
        Dispatch(AnimationDispatch),
    }

    struct Backend {
        events: Vec<Event>,
        resolutions: VecDeque<PlayAnimationInterfaceResolution>,
        motion_values: VecDeque<Option<u32>>,
        filtered_values: VecDeque<Option<u32>>,
        tree_values: VecDeque<Option<TreeAttributeSample>>,
        selected_pro_skater: FastString30,
        transition_override: Option<RawAnimationTransitionParams>,
        switch: bool,
        mirrored: bool,
        physical: PhysicalOutputObservation,
        dispatch_results: VecDeque<u8>,
        fail_on: Option<PlayAnimationBackendOperation>,
    }

    impl Backend {
        fn new() -> Self {
            Self {
                events: Vec::new(),
                resolutions: VecDeque::new(),
                motion_values: VecDeque::new(),
                filtered_values: VecDeque::new(),
                tree_values: VecDeque::new(),
                selected_pro_skater: fast_string_30("Rodney"),
                transition_override: Some(RawAnimationTransitionParams {
                    transition_type_raw: 0,
                    blend_length_bits: 0,
                    placement_raw: 0,
                    flags_raw: 0,
                }),
                switch: false,
                mirrored: false,
                physical: PhysicalOutputObservation::HoldingSkateboardRaw(1),
                dispatch_results: VecDeque::new(),
                fail_on: None,
            }
        }

        fn finish<T>(
            &self,
            operation: PlayAnimationBackendOperation,
            value: T,
        ) -> Result<T, TestError> {
            if self.fail_on == Some(operation) {
                Err(TestError)
            } else {
                Ok(value)
            }
        }
    }

    impl PlayAnimationLifecycleBackend for Backend {
        type Error = TestError;

        fn resolve_interface(
            &mut self,
            kind: PlayAnimationInterfaceKind,
        ) -> Result<PlayAnimationInterfaceResolution, TestError> {
            self.events.push(Event::Resolve(kind));
            let value = self
                .resolutions
                .pop_front()
                .unwrap_or(PlayAnimationInterfaceResolution::Resolved);
            self.finish(PlayAnimationBackendOperation::ResolveInterface(kind), value)
        }

        fn motion_intent(&mut self, name: FastString36) -> Result<Option<u32>, TestError> {
            self.events.push(Event::MotionIntent(name));
            let value = self.motion_values.pop_front().unwrap_or(None);
            self.finish(PlayAnimationBackendOperation::MotionIntent, value)
        }

        fn filtered_motion_intent(&mut self, name: FastString36) -> Result<Option<u32>, TestError> {
            self.events.push(Event::FilteredMotionIntent(name));
            let value = self.filtered_values.pop_front().unwrap_or(None);
            self.finish(PlayAnimationBackendOperation::FilteredMotionIntent, value)
        }

        fn tree_attribute(
            &mut self,
            name: FastString30,
        ) -> Result<Option<TreeAttributeSample>, TestError> {
            self.events.push(Event::TreeAttribute(name));
            let value = self.tree_values.pop_front().unwrap_or(None);
            self.finish(PlayAnimationBackendOperation::TreeAttribute, value)
        }

        fn add_settable_attribute(
            &mut self,
            name: FastString30,
            value_bits: u32,
            normalize: bool,
            sequence: i32,
        ) -> Result<(), TestError> {
            self.events.push(Event::AddSettable {
                name,
                value_bits,
                normalize,
                sequence,
            });
            self.finish(PlayAnimationBackendOperation::AddSettableAttribute, ())
        }

        fn selected_pro_skater(&mut self) -> Result<FastString30, TestError> {
            self.events.push(Event::SelectedProSkater);
            self.finish(
                PlayAnimationBackendOperation::SelectedProSkater,
                self.selected_pro_skater,
            )
        }

        fn set_construction_attribute(
            &mut self,
            name: FastString30,
            value: FastString30,
        ) -> Result<(), TestError> {
            self.events.push(Event::SetConstruction { name, value });
            self.finish(PlayAnimationBackendOperation::SetConstructionAttribute, ())
        }

        fn next_transition_override(
            &mut self,
        ) -> Result<Option<RawAnimationTransitionParams>, TestError> {
            self.events.push(Event::NextOverride);
            self.finish(
                PlayAnimationBackendOperation::NextTransitionOverride,
                self.transition_override,
            )
        }

        fn clear_transition_override(&mut self) -> Result<(), TestError> {
            self.events.push(Event::ClearOverride);
            self.finish(PlayAnimationBackendOperation::ClearTransitionOverride, ())
        }

        fn is_switch(&mut self) -> Result<bool, TestError> {
            self.events.push(Event::IsSwitch);
            self.finish(PlayAnimationBackendOperation::IsSwitch, self.switch)
        }

        fn is_mirrored(&mut self) -> Result<bool, TestError> {
            self.events.push(Event::IsMirrored);
            self.finish(PlayAnimationBackendOperation::IsMirrored, self.mirrored)
        }

        fn physical_output(&mut self) -> Result<PhysicalOutputObservation, TestError> {
            self.events.push(Event::PhysicalOutput);
            self.finish(PlayAnimationBackendOperation::PhysicalOutput, self.physical)
        }

        fn dispatch(&mut self, request: AnimationDispatch) -> Result<u8, TestError> {
            self.events.push(Event::Dispatch(request));
            let result = self.dispatch_results.pop_front().unwrap_or(1);
            self.finish(PlayAnimationBackendOperation::Dispatch, result)
        }
    }

    #[derive(Default)]
    struct Allocator {
        calls: Vec<(usize, Vec<u8>, usize)>,
        allocated: bool,
        fail: bool,
    }

    impl PlayAnimationInstanceAllocator for Allocator {
        type Error = TestError;

        fn allocate(
            &mut self,
            size: usize,
            name: &[u8],
            alignment: usize,
        ) -> Result<bool, TestError> {
            self.calls.push((size, name.to_vec(), alignment));
            if self.fail {
                Err(TestError)
            } else {
                Ok(self.allocated)
            }
        }
    }

    fn attr_bits(
        key: &[u8],
        text: &[u8],
        float_bits: u32,
        boolean_raw: u8,
    ) -> StateGraphBinaryAttribute {
        StateGraphBinaryAttribute {
            key: key.to_vec(),
            key_hash: state_graph_string_hash32(key),
            text: text.to_vec(),
            float_bits,
            boolean_raw,
        }
    }

    fn attr(key: &[u8], text: &[u8]) -> StateGraphBinaryAttribute {
        attr_bits(key, text, 0, 0)
    }

    fn make_behaviour(extra: Vec<StateGraphBinaryAttribute>) -> PlayAnimationBehaviour {
        let mut attributes = vec![attr(b"anim", b"Base")];
        attributes.extend(extra);
        PlayAnimationBehaviour::from_attributes(&attributes).unwrap()
    }

    fn instance() -> PlayAnimationRuntimeInstance {
        PlayAnimationRuntimeInstance {
            first_update_raw: 1,
        }
    }

    fn event_tail_dispatch(backend: &Backend) -> AnimationDispatch {
        backend
            .events
            .iter()
            .rev()
            .find_map(|event| match event {
                Event::Dispatch(request) => Some(*request),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn create_instance_preserves_exact_allocation_request_and_null_result() {
        let behaviour = make_behaviour(Vec::new());
        let mut allocator = Allocator {
            allocated: true,
            ..Allocator::default()
        };
        let created = behaviour
            .create_runtime_instance(&mut allocator)
            .unwrap()
            .unwrap();
        assert_eq!(created.first_update_raw(), 1);
        assert_eq!(allocator.calls, [(0x0c, b"unknown".to_vec(), 0x10)]);

        allocator.allocated = false;
        assert_eq!(
            behaviour.create_runtime_instance(&mut allocator).unwrap(),
            None
        );
        allocator.fail = true;
        assert!(matches!(
            behaviour.create_runtime_instance(&mut allocator),
            Err(PlayAnimationRuntimeError::Backend {
                operation: PlayAnimationBackendOperation::AllocateInstance,
                source: TestError,
            })
        ));
        assert_eq!(allocator.calls.len(), 3);
        assert!(
            allocator
                .calls
                .iter()
                .all(|request| request == &(0x0c, b"unknown".to_vec(), 0x10))
        );
    }

    #[test]
    fn begin_empty_list_keeps_exact_resolution_and_dispatch_order() {
        let behaviour = make_behaviour(Vec::new());
        let mut missing_backend = Backend::new();
        assert!(matches!(
            behaviour.begin_runtime(None, &mut missing_backend),
            Err(PlayAnimationRuntimeError::MissingInstance)
        ));
        assert!(missing_backend.events.is_empty());

        let mut instance = instance();
        let mut backend = Backend::new();
        behaviour
            .begin_runtime(Some(&mut instance), &mut backend)
            .unwrap();
        let pro = fast_string_30("Rodney");
        assert_eq!(
            backend.events,
            [
                Event::Resolve(PlayAnimationInterfaceKind::Physical),
                Event::Resolve(PlayAnimationInterfaceKind::Animatable),
                Event::Resolve(PlayAnimationInterfaceKind::SkaterAnim),
                Event::Resolve(PlayAnimationInterfaceKind::MotionGraph),
                Event::Resolve(PlayAnimationInterfaceKind::Animatable),
                Event::Resolve(PlayAnimationInterfaceKind::MotionGraph),
                Event::Resolve(PlayAnimationInterfaceKind::SkaterMotionGraph),
                Event::SelectedProSkater,
                Event::SetConstruction {
                    name: fast_string_30("ProSkater"),
                    value: pro,
                },
                Event::NextOverride,
                Event::ClearOverride,
                Event::Dispatch(AnimationDispatch::Blend {
                    name: fast_string_36("Base"),
                    blend_length_bits: 0.2f32.to_bits(),
                    playback_speed_bits: 1.0f32.to_bits(),
                    time_bits: 0,
                    blend_with_current_frame: false,
                    placement_raw: AnimationTransitionPlacement::Over as u32,
                }),
            ]
        );
        assert!(instance.first_update());
    }

    #[test]
    fn begin_failure_prefixes_rearm_latch_and_preserve_group_rules() {
        let behaviour = make_behaviour(Vec::new());
        for failing_index in 0..4 {
            let mut instance = PlayAnimationRuntimeInstance {
                first_update_raw: 0,
            };
            let mut backend = Backend::new();
            backend.resolutions = (0..4)
                .map(|index| {
                    if index == failing_index {
                        PlayAnimationInterfaceResolution::IdentityMismatch
                    } else {
                        PlayAnimationInterfaceResolution::Resolved
                    }
                })
                .collect();
            assert!(matches!(
                behaviour.begin_runtime(Some(&mut instance), &mut backend),
                Err(PlayAnimationRuntimeError::InterfaceIdentityMismatch(_))
            ));
            assert_eq!(backend.events.len(), failing_index + 1);
            assert!(instance.first_update());
        }

        let mut instance = PlayAnimationRuntimeInstance {
            first_update_raw: 0,
        };
        let mut backend = Backend::new();
        backend.resolutions = [
            PlayAnimationInterfaceResolution::Missing,
            PlayAnimationInterfaceResolution::Resolved,
            PlayAnimationInterfaceResolution::Missing,
            PlayAnimationInterfaceResolution::Resolved,
        ]
        .into();
        assert!(matches!(
            behaviour.begin_runtime(Some(&mut instance), &mut backend),
            Err(PlayAnimationRuntimeError::MissingInterfaces(ref missing))
                if missing == &[
                    PlayAnimationInterfaceKind::Physical,
                    PlayAnimationInterfaceKind::SkaterAnim,
                ]
        ));
        assert_eq!(backend.events.len(), 4);
        assert!(instance.first_update());

        let mut backend = Backend::new();
        backend.resolutions = [
            PlayAnimationInterfaceResolution::Resolved,
            PlayAnimationInterfaceResolution::Resolved,
            PlayAnimationInterfaceResolution::MissingActorCached,
        ]
        .into();
        assert!(matches!(
            behaviour.begin_runtime(Some(&mut instance), &mut backend),
            Err(PlayAnimationRuntimeError::MissingActorCached(
                PlayAnimationInterfaceKind::SkaterAnim
            ))
        ));
        assert_eq!(backend.events.len(), 3);

        let mut backend = Backend::new();
        backend.fail_on = Some(PlayAnimationBackendOperation::ResolveInterface(
            PlayAnimationInterfaceKind::SkaterMotionGraph,
        ));
        assert!(matches!(
            behaviour.begin_runtime(Some(&mut instance), &mut backend),
            Err(PlayAnimationRuntimeError::Backend {
                operation: PlayAnimationBackendOperation::ResolveInterface(
                    PlayAnimationInterfaceKind::SkaterMotionGraph
                ),
                ..
            })
        ));
        assert_eq!(backend.events.len(), 7);
        assert!(instance.first_update());
    }

    #[test]
    fn latch_update_and_end_order_are_exact_including_error_state() {
        let behaviour = make_behaviour(Vec::new());
        let mut instance = instance();
        let mut backend = Backend::new();
        behaviour
            .update_runtime(Some(&mut instance), &mut backend)
            .unwrap();
        assert!(!instance.first_update());
        assert!(backend.events.is_empty());

        behaviour
            .update_runtime(Some(&mut instance), &mut backend)
            .unwrap();
        assert_eq!(
            backend.events,
            [
                Event::Resolve(PlayAnimationInterfaceKind::Animatable),
                Event::Resolve(PlayAnimationInterfaceKind::MotionGraph),
                Event::Resolve(PlayAnimationInterfaceKind::SkaterMotionGraph),
            ]
        );
        backend.events.clear();
        backend.fail_on = Some(PlayAnimationBackendOperation::ResolveInterface(
            PlayAnimationInterfaceKind::Animatable,
        ));
        assert!(matches!(
            behaviour.update_runtime(Some(&mut instance), &mut backend),
            Err(PlayAnimationRuntimeError::Backend { .. })
        ));
        assert!(!instance.first_update());
        behaviour.end_runtime();
        assert_eq!(backend.events.len(), 1);
        assert!(matches!(
            behaviour.update_runtime(None, &mut backend),
            Err(PlayAnimationRuntimeError::MissingInstance)
        ));
    }

    #[test]
    fn update_attributes_preserves_source_order_destinations_and_float_bits() {
        let mut behaviour = make_behaviour(Vec::new());
        behaviour
            .add_param(&[
                attr(b"from", b"intent"),
                attr(b"intent", b"Push"),
                attr_bits(b"normalize", b"", 0, 1),
            ])
            .unwrap();
        behaviour
            .add_param(&[
                attr(b"from", b"filteredIntent"),
                attr(b"filteredIntent", b"Tweak"),
                attr(b"rename", b""),
                attr_bits(b"defaultValue", b"", 0x7fc0_1234, 0),
                attr_bits(b"normalize", b"", 0, 1),
            ])
            .unwrap();
        behaviour
            .add_param(&[
                attr(b"from", b"lastAnim"),
                attr(b"attribute", b"Speed"),
                attr(b"rename", b"OldSpeed"),
            ])
            .unwrap();
        let mut backend = Backend::new();
        backend.motion_values.push_back(Some((-0.0f32).to_bits()));
        backend.filtered_values.push_back(None);
        backend.tree_values.push_back(Some(TreeAttributeSample {
            type_raw: 2,
            value_bits: f32::INFINITY.to_bits(),
        }));
        behaviour
            .update_runtime_attributes(&mut backend, true)
            .unwrap();
        assert_eq!(
            backend.events,
            [
                Event::Resolve(PlayAnimationInterfaceKind::Animatable),
                Event::Resolve(PlayAnimationInterfaceKind::MotionGraph),
                Event::Resolve(PlayAnimationInterfaceKind::SkaterMotionGraph),
                Event::MotionIntent(fast_string_36("Push")),
                Event::AddSettable {
                    name: fast_string_30("Push"),
                    value_bits: (-0.0f32).to_bits(),
                    normalize: true,
                    sequence: -1,
                },
                Event::FilteredMotionIntent(fast_string_36("Tweak")),
                Event::AddSettable {
                    name: fast_string_30(""),
                    value_bits: 0x7fc0_1234,
                    normalize: true,
                    sequence: -1,
                },
                Event::TreeAttribute(fast_string_30("Speed")),
                Event::AddSettable {
                    name: fast_string_30("OldSpeed"),
                    value_bits: f32::INFINITY.to_bits(),
                    normalize: false,
                    sequence: -1,
                },
            ]
        );
    }

    #[test]
    fn last_animation_gates_queries_and_checks_signed_tree_type() {
        let mut behaviour = make_behaviour(Vec::new());
        behaviour
            .add_param(&[
                attr(b"from", b"lastAnim"),
                attr(b"attribute", b"Speed"),
                attr_bits(b"defaultValue", b"", 0xff80_0001, 0),
                attr_bits(b"normalize", b"", 0, 1),
            ])
            .unwrap();
        let mut backend = Backend::new();
        behaviour
            .update_runtime_attributes(&mut backend, false)
            .unwrap();
        assert_eq!(backend.events.len(), 3);

        let mut backend = Backend::new();
        backend.tree_values.push_back(Some(TreeAttributeSample {
            type_raw: 0,
            value_bits: (-0.0f32).to_bits(),
        }));
        behaviour
            .update_runtime_attributes(&mut backend, true)
            .unwrap();
        assert!(backend.events.contains(&Event::AddSettable {
            name: fast_string_30("Speed"),
            value_bits: (-0.0f32).to_bits(),
            normalize: false,
            sequence: -1,
        }));

        for type_raw in [1, 3] {
            let mut backend = Backend::new();
            backend.tree_values.push_back(Some(TreeAttributeSample {
                type_raw,
                value_bits: 0xdead_beef,
            }));
            behaviour
                .update_runtime_attributes(&mut backend, true)
                .unwrap();
            assert!(
                !backend
                    .events
                    .iter()
                    .any(|event| matches!(event, Event::AddSettable { .. }))
            );
        }
        let mut backend = Backend::new();
        backend.tree_values.push_back(Some(TreeAttributeSample {
            type_raw: -1,
            value_bits: 0,
        }));
        assert!(matches!(
            behaviour.update_runtime_attributes(&mut backend, true),
            Err(PlayAnimationRuntimeError::InvalidTreeAttributeType { index: 0, raw: -1 })
        ));

        let mut backend = Backend::new();
        backend.tree_values.push_back(None);
        behaviour
            .update_runtime_attributes(&mut backend, true)
            .unwrap();
        assert!(backend.events.contains(&Event::AddSettable {
            name: fast_string_30("Speed"),
            value_bits: 0xff80_0001,
            normalize: false,
            sequence: -1,
        }));
    }

    #[test]
    fn intent_and_tree_misses_without_defaults_skip_writes_but_filtered_hit_writes() {
        let mut behaviour = make_behaviour(Vec::new());
        behaviour
            .add_param(&[attr(b"from", b"intent"), attr(b"intent", b"Missing")])
            .unwrap();
        behaviour
            .add_param(&[
                attr(b"from", b"filteredIntent"),
                attr(b"filteredIntent", b"Filtered"),
                attr(b"rename", b"FilteredOut"),
            ])
            .unwrap();
        behaviour
            .add_param(&[
                attr(b"from", b"lastAnim"),
                attr(b"attribute", b"AbsentTree"),
            ])
            .unwrap();
        let mut backend = Backend::new();
        backend.motion_values.push_back(None);
        backend.filtered_values.push_back(Some(0x7fa1_2345));
        backend.tree_values.push_back(None);
        behaviour
            .update_runtime_attributes(&mut backend, true)
            .unwrap();
        let writes = backend
            .events
            .iter()
            .filter_map(|event| match event {
                Event::AddSettable {
                    name,
                    value_bits,
                    normalize,
                    sequence,
                } => Some((*name, *value_bits, *normalize, *sequence)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            writes,
            [(fast_string_30("FilteredOut"), 0x7fa1_2345, false, -1)]
        );
        assert_eq!(
            backend.events[3..6],
            [
                Event::MotionIntent(fast_string_36("Missing")),
                Event::FilteredMotionIntent(fast_string_36("Filtered")),
                Event::AddSettable {
                    name: fast_string_30("FilteredOut"),
                    value_bits: 0x7fa1_2345,
                    normalize: false,
                    sequence: -1,
                },
            ]
        );
        assert_eq!(
            backend.events.last(),
            Some(&Event::TreeAttribute(fast_string_30("AbsentTree")))
        );
    }

    #[test]
    fn override_copy_clear_selection_and_channel_dispatch_are_exact() {
        let behaviour = make_behaviour(vec![
            attr(b"switchAnim", b"Switch"),
            attr(b"mirrorAnim", b"Mirror"),
            attr(b"noBoardAnim", b"NoBoard"),
            attr_bits(b"playBackSpeed", b"", 0x7fc0_9876, 0),
        ]);
        let mut instance = instance();
        let mut backend = Backend::new();
        backend.transition_override = Some(RawAnimationTransitionParams {
            transition_type_raw: 3,
            blend_length_bits: (-0.0f32).to_bits(),
            placement_raw: 0xdead_beef,
            flags_raw: TRANSITION_BLEND_WITH_CURRENT_FRAME_MASK
                | TRANSITION_USE_CHANNEL_FROM_WEIGHTS_MASK
                | 0x0123_4567,
        });
        backend.switch = true;
        backend.mirrored = true;
        backend.physical = PhysicalOutputObservation::HoldingSkateboardRaw(0);
        behaviour
            .begin_runtime(Some(&mut instance), &mut backend)
            .unwrap();
        let clear = backend
            .events
            .iter()
            .position(|event| *event == Event::ClearOverride)
            .unwrap();
        let switch = backend
            .events
            .iter()
            .position(|event| *event == Event::IsSwitch)
            .unwrap();
        assert!(clear < switch);
        assert_eq!(
            event_tail_dispatch(&backend),
            AnimationDispatch::ChannelBlend {
                name: fast_string_36("NoBoard"),
                blend_length_bits: (-0.0f32).to_bits(),
                playback_speed_bits: 0x7fc0_9876,
                time_bits: 0,
                blend_with_current_frame: true,
                placement_raw: 0xdead_beef,
                use_channel_from_weights: true,
            }
        );
    }

    #[test]
    fn optional_selection_precedence_and_null_physical_paths_are_exact() {
        let behaviour = make_behaviour(vec![
            attr(b"switchAnim", b"Switch"),
            attr(b"mirrorAnim", b"Mirror"),
            attr(b"noBoardAnim", b"NoBoard"),
        ]);
        for switch in [false, true] {
            for mirrored in [false, true] {
                for holding_raw in [0_u8, 1] {
                    let mut backend = Backend::new();
                    backend.switch = switch;
                    backend.mirrored = mirrored;
                    backend.physical = PhysicalOutputObservation::HoldingSkateboardRaw(holding_raw);
                    let selected = behaviour.select_runtime_animation(&mut backend).unwrap();
                    let expected = if holding_raw == 0 {
                        "NoBoard"
                    } else if mirrored {
                        "Mirror"
                    } else if switch {
                        "Switch"
                    } else {
                        "Base"
                    };
                    assert_eq!(selected, fast_string_36(expected));
                    assert_eq!(
                        backend.events,
                        [Event::IsSwitch, Event::IsMirrored, Event::PhysicalOutput]
                    );
                }
            }
        }
        for (observation, expect_phys_out) in [
            (PhysicalOutputObservation::MissingPhysOut, true),
            (PhysicalOutputObservation::MissingOffBoard, false),
        ] {
            let mut backend = Backend::new();
            backend.physical = observation;
            let error = behaviour
                .select_runtime_animation(&mut backend)
                .unwrap_err();
            assert!(if expect_phys_out {
                matches!(error, PlayAnimationRuntimeError::NullPhysOut)
            } else {
                matches!(error, PlayAnimationRuntimeError::NullOffBoard)
            });
        }

        let base_only = make_behaviour(Vec::new());
        let mut backend = Backend::new();
        assert_eq!(
            base_only.select_runtime_animation(&mut backend).unwrap(),
            fast_string_36("Base")
        );
        assert!(backend.events.is_empty());
    }

    #[test]
    fn all_transition_dispatches_keep_exact_raw_arguments() {
        let behaviour = make_behaviour(vec![attr_bits(
            b"playBackSpeed",
            b"",
            (-0.0f32).to_bits(),
            0,
        )]);
        for type_raw in 1..=4 {
            let mut instance = instance();
            let mut backend = Backend::new();
            backend.transition_override = Some(RawAnimationTransitionParams {
                transition_type_raw: type_raw,
                blend_length_bits: 0x7fc0_1111,
                placement_raw: 0x8000_0001,
                flags_raw: TRANSITION_BLEND_WITH_CURRENT_FRAME_MASK
                    | TRANSITION_USE_CHANNEL_FROM_WEIGHTS_MASK,
            });
            behaviour
                .begin_runtime(Some(&mut instance), &mut backend)
                .unwrap();
            let dispatch = event_tail_dispatch(&backend);
            match (type_raw, dispatch) {
                (
                    1,
                    AnimationDispatch::Play {
                        name,
                        playback_speed_bits,
                        time_bits,
                    },
                ) => {
                    assert_eq!(name, fast_string_36("Base"));
                    assert_eq!(playback_speed_bits, (-0.0f32).to_bits());
                    assert_eq!(time_bits, 0);
                }
                (
                    2,
                    AnimationDispatch::Blend {
                        name,
                        blend_length_bits,
                        playback_speed_bits,
                        time_bits,
                        blend_with_current_frame,
                        placement_raw,
                    },
                ) => {
                    assert_eq!(name, fast_string_36("Base"));
                    assert_eq!(blend_length_bits, 0x7fc0_1111);
                    assert_eq!(playback_speed_bits, (-0.0f32).to_bits());
                    assert_eq!(time_bits, 0);
                    assert!(blend_with_current_frame);
                    assert_eq!(placement_raw, 0x8000_0001);
                }
                (
                    3,
                    AnimationDispatch::ChannelBlend {
                        name,
                        blend_length_bits,
                        playback_speed_bits,
                        time_bits,
                        blend_with_current_frame,
                        placement_raw,
                        use_channel_from_weights,
                    },
                ) => {
                    assert_eq!(name, fast_string_36("Base"));
                    assert_eq!(blend_length_bits, 0x7fc0_1111);
                    assert_eq!(playback_speed_bits, (-0.0f32).to_bits());
                    assert_eq!(time_bits, 0);
                    assert!(blend_with_current_frame);
                    assert_eq!(placement_raw, 0x8000_0001);
                    assert!(use_channel_from_weights);
                }
                (
                    4,
                    AnimationDispatch::Sequence {
                        name,
                        playback_speed_bits,
                        time_bits,
                        placement_raw,
                    },
                ) => {
                    assert_eq!(name, fast_string_36("Base"));
                    assert_eq!(playback_speed_bits, (-0.0f32).to_bits());
                    assert_eq!(time_bits, 0);
                    assert_eq!(placement_raw, 0x8000_0001);
                }
                other => panic!("transition type/dispatch mismatch: {other:?}"),
            }
        }
    }

    #[test]
    fn zero_override_retains_stored_transition_and_invalid_nonzero_fails_after_clear() {
        let behaviour = make_behaviour(vec![
            attr_bits(b"time", b"", 0x7fc0_4567, 0),
            attr_bits(b"transitionUnder", b"", 0, 1),
            attr_bits(b"blendWithCurrentFrame", b"", 0, 1),
            attr_bits(b"playBackSpeed", b"", (-0.0f32).to_bits(), 0),
        ]);
        for raw in [0, 5, 0x8000_0000] {
            let mut instance = instance();
            let mut backend = Backend::new();
            backend.transition_override = Some(RawAnimationTransitionParams {
                transition_type_raw: raw,
                blend_length_bits: 0xdead_beef,
                placement_raw: 0x8000_0001,
                flags_raw: 0x4123_4567,
            });
            let result = behaviour.begin_runtime(Some(&mut instance), &mut backend);
            if raw == 0 {
                assert!(result.is_ok(), "zero override retains stored Blend");
                assert_eq!(
                    event_tail_dispatch(&backend),
                    AnimationDispatch::Blend {
                        name: fast_string_36("Base"),
                        blend_length_bits: 0x7fc0_4567,
                        playback_speed_bits: (-0.0f32).to_bits(),
                        time_bits: 0,
                        blend_with_current_frame: true,
                        placement_raw: AnimationTransitionPlacement::Under as u32,
                    }
                );
            } else {
                assert!(matches!(
                    result,
                    Err(PlayAnimationRuntimeError::InvalidTransitionType(value)) if value == raw
                ));
                assert!(backend.events.contains(&Event::ClearOverride));
                assert!(
                    !backend
                        .events
                        .iter()
                        .any(|event| matches!(event, Event::Dispatch(_)))
                );
            }
        }
    }

    #[test]
    fn null_override_precedes_clear_and_fallback_contract_is_exact() {
        let behaviour = make_behaviour(Vec::new());
        let mut instance = instance();
        let mut backend = Backend::new();
        backend.transition_override = None;
        assert!(matches!(
            behaviour.begin_runtime(Some(&mut instance), &mut backend),
            Err(PlayAnimationRuntimeError::NullTransitionOverride)
        ));
        assert!(backend.events.contains(&Event::NextOverride));
        assert!(!backend.events.contains(&Event::ClearOverride));

        let mut backend = Backend::new();
        backend.dispatch_results = [0, 0x80].into();
        behaviour
            .begin_runtime(Some(&mut instance), &mut backend)
            .unwrap();
        let dispatches = backend
            .events
            .iter()
            .filter_map(|event| match event {
                Event::Dispatch(dispatch) => Some(*dispatch),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(matches!(dispatches[0], AnimationDispatch::Blend { .. }));
        assert_eq!(
            dispatches[1],
            AnimationDispatch::Play {
                name: fast_string_36("KeepDefaultAnim"),
                playback_speed_bits: 1.0f32.to_bits(),
                time_bits: 0,
            }
        );

        let mut backend = Backend::new();
        backend.dispatch_results = [0, 0].into();
        assert!(matches!(
            behaviour.begin_runtime(Some(&mut instance), &mut backend),
            Err(PlayAnimationRuntimeError::FallbackDispatchFailed)
        ));
    }

    #[test]
    fn parser_error_type_remains_separate_from_lifecycle_errors() {
        let error = PlayAnimationBehaviour::from_attributes(&[]).unwrap_err();
        assert!(matches!(
            error,
            PlayAnimationParseError::MissingAttribute { key: b"anim", .. }
        ));
    }
}
