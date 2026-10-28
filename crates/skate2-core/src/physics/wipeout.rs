//! Evidence-backed wipeout requests and `OffBoard` state selection.
//!
//! This module deliberately stops before ragdoll construction and force
//! application. Those systems still depend on unresolved runtime attributes
//! and authored-data selection.

use std::collections::BTreeSet;

use crate::physics::PhysicsState;

/// Number of entries in the reference `WipeoutType` enum and request arrays.
pub const WIPEOUT_TYPE_COUNT: usize = 30;

/// `cVarBoolRef` section containing the global wipeout-disable control.
///
/// Reference: the initializer for `kWipeoutDisableAllWipeouts` at
/// `0x82f9c320` passes this exact section to `cVarBoolRef::cVarBoolRef` at
/// `0x82a8adc0`.
pub const WIPEOUT_DISABLE_ALL_CVAR_SECTION: &str = "Physics_Cheats";

/// Plain-text `cVarBoolRef` label for the global wipeout-disable control.
///
/// Reference: the initializer at `0x82f9c320`. This is kept separate from the
/// 30 labels indexed by [`WipeoutType`].
pub const WIPEOUT_DISABLE_ALL_CVAR_LABEL: &str = "kWipeoutDisableAllWipeouts";

/// `cVarBoolRef` section containing all 30 per-type wipeout controls.
///
/// Reference: the per-type initializers at `0x82f9c550..0x82f9ceac` pass this
/// exact section to `cVarBoolRef::cVarBoolRef` at `0x82a8adc0`.
pub const WIPEOUT_ENABLE_CVAR_SECTION: &str = "Physics_Wipeouts";

/// Exact per-type `cVarBoolRef` labels in [`WipeoutType::ALL`] order.
///
/// The static initializers span `0x82f9c550..0x82f9ceac`; the matching target
/// values are assembled into the 30-entry table consumed by
/// `Wipeout::DoesPlayerRequestWipeout` at `0x82dd9988`.
pub const WIPEOUT_ENABLE_CVAR_LABELS: [&str; WIPEOUT_TYPE_COUNT] = [
    "kWipeoutSkeletonContactEnabled",
    "kWipeoutSkeletonDisplacementEnabled",
    "kWipeoutDeckAccelerationEnabled",
    "kWipeoutDeckFlippedEnabled",
    "kWipeoutBodyFlipEnabled",
    "kWipeoutBadAngleLandingEnabled",
    "kWipeoutHeavyLandingEnabled",
    "kWipeoutVehicleCollisionEnabled",
    "kWipeoutAirToHardGrindEnabled",
    "kWipeoutHardGrindEngagementEnabled",
    "kWipeoutBoardslideStopOnRailEnabled",
    "kWipeoutOffBalanceEnabled",
    "kWipeoutHungUpOnRailEnabled",
    "kWipeoutOffBalanceGrindEnabled",
    "kWipeoutHardHorizontalGrindEnabled",
    "kWipeoutUpsideDownFallingEnabled",
    "kWipeoutOpposingContactsEnabled",
    "kWipeoutFailureToInvExitEnabled",
    "kWipeoutSkeletonSquashedEnabled",
    "kWipeoutLeaningContactEnabled",
    "kWipeoutDeckOpposingContactsEnabled",
    "kWipeoutSensitiveSkeletonCollEnabled",
    "kWipeoutBadSkateboardEnabled",
    "kWipeoutTooFastOffBoardEnabled",
    "kWipeoutFailureToGoOffboardEnabled",
    "kWipeoutOB_BadGroundEnabled",
    "kWipeoutOB_DeathLeapEnabled",
    "kWipeoutOB_LandingTooFastEnabled",
    "kWipeoutOB_NotUprightEnabled",
    "kWipeoutOB_LandedOnDeckEnabled",
];

/// Stable indices used by `Sk8::Physics::Wipeout` for its request arrays.
///
/// The order is corroborated by the enum symbols and by
/// `Wipeout::DoesPlayerRequestWipeout` at `0x82dd9988`, which constructs and
/// scans a matching 30-entry enable table.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum WipeoutType {
    SkeletonContact = 0,
    SkeletonDisplacement = 1,
    DeckAcceleration = 2,
    DeckFlipped = 3,
    BodyFlip = 4,
    BadAngleLanding = 5,
    HeavyLanding = 6,
    VehicleCollision = 7,
    AirToHardGrind = 8,
    HardGrindEngagement = 9,
    BoardslideStopOnRail = 10,
    OffBalance = 11,
    HungUpOnRail = 12,
    OffBalanceGrind = 13,
    HardHorizontalGrind = 14,
    UpsideDownFalling = 15,
    OpposingContacts = 16,
    FailureToInvExit = 17,
    SkeletonSquashed = 18,
    LeaningContact = 19,
    DeckOpposingContacts = 20,
    SensitiveSkeletonColl = 21,
    BadSkateboard = 22,
    TooFastOffBoard = 23,
    FailureToGoOffboard = 24,
    OffBoardBadGround = 25,
    OffBoardDeathLeap = 26,
    OffBoardLandingTooFast = 27,
    OffBoardNotUpright = 28,
    OffBoardLandedOnDeck = 29,
}

impl WipeoutType {
    /// Every request type in the reference iteration order.
    pub const ALL: [Self; WIPEOUT_TYPE_COUNT] = [
        Self::SkeletonContact,
        Self::SkeletonDisplacement,
        Self::DeckAcceleration,
        Self::DeckFlipped,
        Self::BodyFlip,
        Self::BadAngleLanding,
        Self::HeavyLanding,
        Self::VehicleCollision,
        Self::AirToHardGrind,
        Self::HardGrindEngagement,
        Self::BoardslideStopOnRail,
        Self::OffBalance,
        Self::HungUpOnRail,
        Self::OffBalanceGrind,
        Self::HardHorizontalGrind,
        Self::UpsideDownFalling,
        Self::OpposingContacts,
        Self::FailureToInvExit,
        Self::SkeletonSquashed,
        Self::LeaningContact,
        Self::DeckOpposingContacts,
        Self::SensitiveSkeletonColl,
        Self::BadSkateboard,
        Self::TooFastOffBoard,
        Self::FailureToGoOffboard,
        Self::OffBoardBadGround,
        Self::OffBoardDeathLeap,
        Self::OffBoardLandingTooFast,
        Self::OffBoardNotUpright,
        Self::OffBoardLandedOnDeck,
    ];

    /// Returns the stable request-array index.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Converts a request-array index without accepting out-of-range values.
    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        if index < WIPEOUT_TYPE_COUNT {
            Some(Self::ALL[index])
        } else {
            None
        }
    }

    /// Returns this type's exact plain-text `cVarBoolRef` label.
    ///
    /// Reference: the per-type initializers at `0x82f9c550..0x82f9ceac` and
    /// the table construction in `Wipeout::DoesPlayerRequestWipeout` at
    /// `0x82dd9988`.
    #[must_use]
    pub const fn enable_cvar_label(self) -> &'static str {
        WIPEOUT_ENABLE_CVAR_LABELS[self.index()]
    }
}

impl From<WipeoutType> for usize {
    fn from(value: WipeoutType) -> Self {
        value.index()
    }
}

impl TryFrom<usize> for WipeoutType {
    type Error = WipeoutTypeIndexError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::from_index(value).ok_or(WipeoutTypeIndexError { value })
    }
}

/// An index outside the reference 30-entry wipeout request table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WipeoutTypeIndexError {
    pub value: usize,
}

/// Runtime controls consumed when deciding whether any request is enabled.
///
/// `Wipeout::DoesPlayerRequestWipeout` at `0x82dd9988` first checks the global
/// disable control, then visits all 30 request and per-type-enable pairs in
/// reference order. The supplied debug reference build's compiled seeds are
/// available through [`Self::debug_reference_compiled_defaults`]. Callers may
/// still supply arbitrary snapshots because later debug-config and console
/// variable mutations are not frozen by that boot-time evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WipeoutEnableControls {
    pub disable_all: bool,
    pub per_type: [bool; WIPEOUT_TYPE_COUNT],
}

impl WipeoutEnableControls {
    #[must_use]
    pub const fn new(disable_all: bool, per_type: [bool; WIPEOUT_TYPE_COUNT]) -> Self {
        Self {
            disable_all,
            per_type,
        }
    }

    /// Returns the compiled seeds from the supplied debug reference build.
    ///
    /// The initializer at `0x82f9c320` supplies `false` for the global disable;
    /// all 30 per-type initializers at `0x82f9c550..0x82f9ceac` supply `true`.
    /// `cVarBoolRef::Link` at `0x82a89ca0` creates each target from that seed.
    /// This explicitly build-scoped constructor is not a claim about other
    /// builds or values after a later runtime mutation.
    #[must_use]
    pub const fn debug_reference_compiled_defaults() -> Self {
        Self::new(false, [true; WIPEOUT_TYPE_COUNT])
    }

    /// Applies already-resolved integer values using the reference bool rule.
    ///
    /// `cVarSerializer::Deserialize` at `0x82a8ad50` leaves a target unchanged
    /// when a label is missing. `cIniFile::GetBool` at `0x82a89620` maps a
    /// present integer zero to `false` and every nonzero integer to `true`.
    /// Parsing text into these integers intentionally remains outside this
    /// evidence-backed slice.
    pub fn apply_resolved_integer_overlay(&mut self, overlay: WipeoutEnableIntegerOverlay) {
        if let Some(value) = overlay.disable_all {
            self.disable_all = value != 0;
        }
        for wipeout_type in WipeoutType::ALL {
            let index = wipeout_type.index();
            if let Some(value) = overlay.per_type[index] {
                self.per_type[index] = value != 0;
            }
        }
    }

    #[must_use]
    pub const fn is_enabled(self, wipeout_type: WipeoutType) -> bool {
        self.per_type[wipeout_type.index()]
    }
}

/// Optional, already-resolved integer values for the 31 wipeout bool `cVar`s.
///
/// `None` represents a missing value and therefore preserves the prior target.
/// The global label is separate from the 30 entries indexed in
/// [`WipeoutType::ALL`] order. This type does not parse INI syntax and does not
/// imply that a boot snapshot remains unchanged during later runtime mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WipeoutEnableIntegerOverlay {
    pub disable_all: Option<i32>,
    pub per_type: [Option<i32>; WIPEOUT_TYPE_COUNT],
}

impl WipeoutEnableIntegerOverlay {
    #[must_use]
    pub const fn new(
        disable_all: Option<i32>,
        per_type: [Option<i32>; WIPEOUT_TYPE_COUNT],
    ) -> Self {
        Self {
            disable_all,
            per_type,
        }
    }

    /// An overlay in which every config label is missing.
    #[must_use]
    pub const fn all_missing() -> Self {
        Self::new(None, [None; WIPEOUT_TYPE_COUNT])
    }
}

/// The 30 request flags, associated values, and request-event count owned by
/// `Sk8::Physics::Wipeout`.
///
/// Request writes are visible throughout the reference post-physics/output
/// work until `Wipeout::ClearWipeoutRequests` at `0x82dd4570` clears all three
/// pieces of state during the next system-logic stage.
#[derive(Clone, Debug, PartialEq)]
pub struct WipeoutRequestLedger {
    requested: [bool; WIPEOUT_TYPE_COUNT],
    values: [f32; WIPEOUT_TYPE_COUNT],
    request_count: u32,
}

impl Default for WipeoutRequestLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl WipeoutRequestLedger {
    /// Constructs the same cleared array state produced by
    /// `Wipeout::ClearWipeoutRequests` at `0x82dd4570`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            requested: [false; WIPEOUT_TYPE_COUNT],
            values: [0.0; WIPEOUT_TYPE_COUNT],
            request_count: 0,
        }
    }

    /// Records one request event.
    ///
    /// Reference request sites set the indexed flag, replace its value, and
    /// increment the counter even when the same type was already requested;
    /// examples occur in `Wipeout::CheckForOffBoardWipeout` at `0x82ddec70`.
    pub fn request(&mut self, wipeout_type: WipeoutType, value: f32) {
        let index = wipeout_type.index();
        self.requested[index] = true;
        self.values[index] = value;
        self.request_count = self.request_count.wrapping_add(1);
    }

    /// Clears all flags, values, and the event counter in reference order.
    ///
    /// Reference: `Wipeout::ClearWipeoutRequests` at `0x82dd4570`.
    pub fn clear(&mut self) {
        self.request_count = 0;
        for wipeout_type in WipeoutType::ALL {
            let index = wipeout_type.index();
            self.requested[index] = false;
            self.values[index] = 0.0;
        }
    }

    /// `Wipeout::Reset` writes only the event count at +0xb4. The flags and
    /// values remain live until `ClearWipeoutRequests` runs separately.
    pub(crate) fn reset_count_only(&mut self) {
        self.request_count = 0;
    }

    #[must_use]
    pub const fn is_requested(&self, wipeout_type: WipeoutType) -> bool {
        self.requested[wipeout_type.index()]
    }

    #[must_use]
    pub const fn request_value(&self, wipeout_type: WipeoutType) -> f32 {
        self.values[wipeout_type.index()]
    }

    #[must_use]
    pub const fn request_count(&self) -> u32 {
        self.request_count
    }

    /// Reconstructs the non-debug result of
    /// `Wipeout::DoesPlayerRequestWipeout` at `0x82dd9988`.
    ///
    /// The loop intentionally does not short-circuit: the reference visits all
    /// 30 entries so its optional debug reporting can observe every request.
    #[must_use]
    pub fn does_player_request_wipeout(&self, controls: WipeoutEnableControls) -> bool {
        if controls.disable_all {
            return false;
        }

        let mut any_enabled_request = false;
        for wipeout_type in WipeoutType::ALL {
            let index = wipeout_type.index();
            any_enabled_request |= self.requested[index] && controls.per_type[index];
        }
        any_enabled_request
    }
}

/// Inputs combined by `PhysicalPlayerHiLOD::IsWipeoutRequested`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WipeoutRequestSources {
    pub runout_requested: bool,
    pub animation_requests_wipeout: bool,
}

/// Reconstructs `PhysicalPlayerHiLOD::IsWipeoutRequested` at `0x82e00a50`.
///
/// A runout suppresses both physics-ledger and animation requests. Otherwise
/// an enabled ledger request wins, followed by the processed animation request
/// (`ProcessedPhysIn` word 564, byte offset `0x8d0`, bit 6).
#[must_use]
pub fn is_wipeout_requested(
    ledger: &WipeoutRequestLedger,
    controls: WipeoutEnableControls,
    sources: WipeoutRequestSources,
) -> bool {
    if sources.runout_requested {
        return false;
    }
    if ledger.does_player_request_wipeout(controls) {
        return true;
    }
    sources.animation_requests_wipeout
}

/// An unresolved flag read by two OffBoard-family branches of
/// `PhysicalPlayerStateChanger::CalcSuggestedState` at `0x82ddbd58`.
///
/// The flag is `ProcessedPhysIn` word 564 at byte offset `0x8d0`, mask
/// `0x0004_0000` (bit 18). Its branch behavior is exact, but its original field
/// name and producer remain unknown, so this wrapper intentionally gives it no
/// gameplay interpretation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OpaqueProcessedWord564Bit18 {
    #[default]
    Unset,
    Set,
}

impl OpaqueProcessedWord564Bit18 {
    #[must_use]
    pub const fn is_set(self) -> bool {
        matches!(self, Self::Set)
    }
}

/// Named `ProcessedPhysIn` flags used by the implemented `OffBoard` cases.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OffBoardStateFlag {
    /// `ProcessedPhysIn` word 561, byte offset `0x8c4`, bit 18.
    WipingOut,
    /// `ProcessedPhysIn` word 564, byte offset `0x8d0`, bit 6.
    AnimationRequestsWipeout,
    /// `ProcessedPhysIn` word 563, byte offset `0x8cc`, bit 23.
    GrabbingObject,
    /// `ProcessedPhysIn` word 563, byte offset `0x8cc`, bit 16.
    OffBoardComMotion,
    /// `ProcessedPhysIn` word 563, byte offset `0x8cc`, bit 17.
    OffBoardTrajectory,
    /// `ProcessedPhysIn` word 563, byte offset `0x8cc`, bit 8.
    OffBoardAir,
    /// `ProcessedPhysIn` word 563, byte offset `0x8cc`, bit 20.
    OffBoardStandingJump,
    /// `ProcessedPhysIn` word 565, byte offset `0x8d4`, bit 29.
    OffBoardJumpInfoReady,
    /// `ProcessedPhysIn` word 564, byte offset `0x8d0`, bit 7.
    LandOnBoard,
    /// `ProcessedPhysIn` word 565, byte offset `0x8d4`, bit 23.
    BoardMountInvalid,
}

/// Named, proven inputs used by the `OffBoard`, Pushing, Air, and Locomotion
/// cases of `PhysicalPlayerStateChanger::CalcSuggestedState` at `0x82ddbd58`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OffBoardStateInput {
    pub flags: BTreeSet<OffBoardStateFlag>,
    pub opaque_word_564_bit_18: OpaqueProcessedWord564Bit18,
}

impl OffBoardStateInput {
    #[must_use]
    pub fn new(
        flags: impl IntoIterator<Item = OffBoardStateFlag>,
        opaque_word_564_bit_18: OpaqueProcessedWord564Bit18,
    ) -> Self {
        Self {
            flags: flags.into_iter().collect(),
            opaque_word_564_bit_18,
        }
    }

    /// Returns the reference suggestion for the four fully decoded `OffBoard`
    /// cases. `None` makes the intentionally unsupported states explicit.
    ///
    /// Reference: `PhysicalPlayerStateChanger::CalcSuggestedState` at
    /// `0x82ddbd58`; the relevant branch block spans `0x82ddcafc` through
    /// `0x82ddcd10`. Checks remain in reference priority order.
    #[must_use]
    pub fn suggested_state(&self, current: PhysicsState) -> Option<PhysicsState> {
        match current {
            PhysicsState::OffBoard => Some(self.suggest_from_offboard()),
            PhysicsState::OffBoardPushing => Some(self.suggest_from_pushing()),
            PhysicsState::OffBoardAir => Some(self.suggest_from_air()),
            PhysicsState::OffBoardLocomotion => Some(self.suggest_from_locomotion()),
            _ => None,
        }
    }

    fn suggest_from_offboard(&self) -> PhysicsState {
        let flags = &self.flags;
        let opaque = self.opaque_word_564_bit_18;
        match Self::try_suggest_from_offboard(
            |flag| Ok::<_, std::convert::Infallible>(flags.contains(&flag)),
            || Ok::<_, std::convert::Infallible>(opaque.is_set()),
        ) {
            Ok(state) => state,
            Err(never) => match never {},
        }
    }

    /// One source-ordered OffBoard decision tree for the typed model and live
    /// Processed adapter. Fallible reads let the live caller keep unknown bits
    /// unknown until the original branch actually reaches them.
    pub(crate) fn try_suggest_from_offboard<E>(
        mut flag: impl FnMut(OffBoardStateFlag) -> Result<bool, E>,
        mut opaque_word_564_bit_18: impl FnMut() -> Result<bool, E>,
    ) -> Result<PhysicsState, E> {
        if flag(OffBoardStateFlag::WipingOut)? {
            return Ok(PhysicsState::WipeoutGround);
        }
        if flag(OffBoardStateFlag::GrabbingObject)? {
            return Ok(PhysicsState::OffBoardPushing);
        }
        if flag(OffBoardStateFlag::OffBoardComMotion)? {
            return Ok(PhysicsState::OffBoardLocomotion);
        }
        if flag(OffBoardStateFlag::OffBoardJumpInfoReady)?
            && (flag(OffBoardStateFlag::OffBoardStandingJump)?
                || flag(OffBoardStateFlag::OffBoardAir)?)
        {
            return Ok(PhysicsState::OffBoardJump);
        }
        if flag(OffBoardStateFlag::OffBoardTrajectory)? {
            return Ok(PhysicsState::OffBoard);
        }
        if opaque_word_564_bit_18()? && !flag(OffBoardStateFlag::BoardMountInvalid)? {
            return Ok(PhysicsState::PhysicsAir);
        }
        Ok(PhysicsState::WipeoutGround)
    }

    fn suggest_from_pushing(&self) -> PhysicsState {
        if self.has(OffBoardStateFlag::WipingOut) {
            return PhysicsState::WipeoutGround;
        }
        if self.has(OffBoardStateFlag::GrabbingObject) {
            return PhysicsState::OffBoardPushing;
        }
        PhysicsState::OffBoard
    }

    fn suggest_from_air(&self) -> PhysicsState {
        match Self::try_suggest_from_air::<std::convert::Infallible>(|flag| Ok(self.has(flag))) {
            Ok(state) => state,
            Err(never) => match never {},
        }
    }
    /// Native502 selector82ddcc00..cc8c, lazy retained-field reads.
    pub(crate) fn try_suggest_from_air<E>(
        mut flag: impl FnMut(OffBoardStateFlag) -> Result<bool, E>,
    ) -> Result<PhysicsState, E> {
        if flag(OffBoardStateFlag::WipingOut)? {
            return Ok(PhysicsState::WipeoutGround);
        }
        if flag(OffBoardStateFlag::OffBoardComMotion)? {
            return Ok(PhysicsState::OffBoardLocomotion);
        }
        if flag(OffBoardStateFlag::OffBoardTrajectory)? {
            return Ok(PhysicsState::OffBoard);
        }
        if flag(OffBoardStateFlag::LandOnBoard)? {
            return Ok(PhysicsState::LandingOnDeck);
        }
        if !flag(OffBoardStateFlag::OffBoardAir)? {
            return Ok(if flag(OffBoardStateFlag::BoardMountInvalid)? {
                PhysicsState::WipeoutGround
            } else {
                PhysicsState::PhysicsAir
            });
        }
        Ok(PhysicsState::OffBoardAir)
    }

    fn suggest_from_locomotion(&self) -> PhysicsState {
        match Self::try_suggest_from_locomotion::<std::convert::Infallible>(
            |flag| Ok(self.has(flag)),
            || Ok(self.opaque_word_564_bit_18.is_set()),
        ) {
            Ok(state) => state,
            Err(never) => match never {},
        }
    }
    /// Native504 selector82ddcc90..cd1c; preserve lazy source read order.
    pub(crate) fn try_suggest_from_locomotion<E>(
        mut flag: impl FnMut(OffBoardStateFlag) -> Result<bool, E>,
        mut opaque: impl FnMut() -> Result<bool, E>,
    ) -> Result<PhysicsState, E> {
        if flag(OffBoardStateFlag::WipingOut)? || flag(OffBoardStateFlag::AnimationRequestsWipeout)?
        {
            return Ok(PhysicsState::WipeoutGround);
        }
        if flag(OffBoardStateFlag::GrabbingObject)? {
            return Ok(PhysicsState::OffBoardPushing);
        }
        if flag(OffBoardStateFlag::OffBoardTrajectory)? {
            return Ok(PhysicsState::OffBoard);
        }
        if flag(OffBoardStateFlag::OffBoardAir)? {
            return Ok(PhysicsState::OffBoardAir);
        }
        if flag(OffBoardStateFlag::LandOnBoard)? {
            return Ok(PhysicsState::LandingOnDeck);
        }
        if !flag(OffBoardStateFlag::OffBoardComMotion)? {
            return Ok(if opaque()? {
                PhysicsState::PhysicsGround
            } else {
                PhysicsState::WipeoutGround
            });
        }
        Ok(PhysicsState::OffBoardLocomotion)
    }

    fn has(&self, flag: OffBoardStateFlag) -> bool {
        self.flags.contains(&flag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_DISABLED: WipeoutEnableControls =
        WipeoutEnableControls::new(false, [false; WIPEOUT_TYPE_COUNT]);
    const ALL_ENABLED: WipeoutEnableControls =
        WipeoutEnableControls::new(false, [true; WIPEOUT_TYPE_COUNT]);

    fn offboard_input(flags: impl IntoIterator<Item = OffBoardStateFlag>) -> OffBoardStateInput {
        OffBoardStateInput::new(flags, OpaqueProcessedWord564Bit18::Unset)
    }

    #[test]
    fn wipeout_type_order_and_conversions_are_exhaustive() {
        assert_eq!(WipeoutType::ALL.len(), WIPEOUT_TYPE_COUNT);
        for (index, wipeout_type) in WipeoutType::ALL.into_iter().enumerate() {
            assert_eq!(wipeout_type.index(), index);
            assert_eq!(WipeoutType::from_index(index), Some(wipeout_type));
            assert_eq!(WipeoutType::try_from(index), Ok(wipeout_type));
            assert_eq!(usize::from(wipeout_type), index);
        }
        assert_eq!(WipeoutType::from_index(WIPEOUT_TYPE_COUNT), None);
        assert_eq!(
            WipeoutType::try_from(WIPEOUT_TYPE_COUNT),
            Err(WipeoutTypeIndexError {
                value: WIPEOUT_TYPE_COUNT
            })
        );
        assert_eq!(WipeoutType::from_index(usize::MAX), None);
    }

    #[test]
    fn wipeout_enable_cvar_sections_and_indexed_labels_are_exact() {
        const EXPECTED_LABELS: [&str; WIPEOUT_TYPE_COUNT] = [
            "kWipeoutSkeletonContactEnabled",
            "kWipeoutSkeletonDisplacementEnabled",
            "kWipeoutDeckAccelerationEnabled",
            "kWipeoutDeckFlippedEnabled",
            "kWipeoutBodyFlipEnabled",
            "kWipeoutBadAngleLandingEnabled",
            "kWipeoutHeavyLandingEnabled",
            "kWipeoutVehicleCollisionEnabled",
            "kWipeoutAirToHardGrindEnabled",
            "kWipeoutHardGrindEngagementEnabled",
            "kWipeoutBoardslideStopOnRailEnabled",
            "kWipeoutOffBalanceEnabled",
            "kWipeoutHungUpOnRailEnabled",
            "kWipeoutOffBalanceGrindEnabled",
            "kWipeoutHardHorizontalGrindEnabled",
            "kWipeoutUpsideDownFallingEnabled",
            "kWipeoutOpposingContactsEnabled",
            "kWipeoutFailureToInvExitEnabled",
            "kWipeoutSkeletonSquashedEnabled",
            "kWipeoutLeaningContactEnabled",
            "kWipeoutDeckOpposingContactsEnabled",
            "kWipeoutSensitiveSkeletonCollEnabled",
            "kWipeoutBadSkateboardEnabled",
            "kWipeoutTooFastOffBoardEnabled",
            "kWipeoutFailureToGoOffboardEnabled",
            "kWipeoutOB_BadGroundEnabled",
            "kWipeoutOB_DeathLeapEnabled",
            "kWipeoutOB_LandingTooFastEnabled",
            "kWipeoutOB_NotUprightEnabled",
            "kWipeoutOB_LandedOnDeckEnabled",
        ];

        assert_eq!(WIPEOUT_DISABLE_ALL_CVAR_SECTION, "Physics_Cheats");
        assert_eq!(WIPEOUT_DISABLE_ALL_CVAR_LABEL, "kWipeoutDisableAllWipeouts");
        assert_eq!(WIPEOUT_ENABLE_CVAR_SECTION, "Physics_Wipeouts");
        assert_eq!(WIPEOUT_ENABLE_CVAR_LABELS, EXPECTED_LABELS);
        for (wipeout_type, expected_label) in WipeoutType::ALL.into_iter().zip(EXPECTED_LABELS) {
            assert_eq!(wipeout_type.enable_cvar_label(), expected_label);
            assert_eq!(
                WIPEOUT_ENABLE_CVAR_LABELS[wipeout_type.index()],
                expected_label
            );
        }
    }

    #[test]
    fn debug_reference_compiled_defaults_are_explicitly_build_scoped() {
        let controls = WipeoutEnableControls::debug_reference_compiled_defaults();

        assert!(!controls.disable_all);
        for wipeout_type in WipeoutType::ALL {
            assert!(controls.is_enabled(wipeout_type));
        }
    }

    #[test]
    fn all_missing_resolved_overlay_preserves_prior_values() {
        let mut per_type = [false; WIPEOUT_TYPE_COUNT];
        for wipeout_type in WipeoutType::ALL {
            per_type[wipeout_type.index()] = wipeout_type.index() % 2 == 0;
        }
        let original = WipeoutEnableControls::new(true, per_type);
        let mut controls = original;

        controls.apply_resolved_integer_overlay(WipeoutEnableIntegerOverlay::all_missing());

        assert_eq!(controls, original);
    }

    #[test]
    fn resolved_integer_overlay_maps_zero_and_every_nonzero_to_bool() {
        let mut values = [None; WIPEOUT_TYPE_COUNT];
        values[WipeoutType::SkeletonContact.index()] = Some(0);
        values[WipeoutType::SkeletonDisplacement.index()] = Some(1);
        values[WipeoutType::DeckAcceleration.index()] = Some(-1);
        values[WipeoutType::DeckFlipped.index()] = Some(i32::MIN);
        let mut controls = WipeoutEnableControls::new(true, [false; WIPEOUT_TYPE_COUNT]);

        controls.apply_resolved_integer_overlay(WipeoutEnableIntegerOverlay::new(Some(0), values));

        assert!(!controls.disable_all);
        assert!(!controls.is_enabled(WipeoutType::SkeletonContact));
        assert!(controls.is_enabled(WipeoutType::SkeletonDisplacement));
        assert!(controls.is_enabled(WipeoutType::DeckAcceleration));
        assert!(controls.is_enabled(WipeoutType::DeckFlipped));
        assert!(!controls.is_enabled(WipeoutType::BodyFlip));

        controls.apply_resolved_integer_overlay(WipeoutEnableIntegerOverlay::new(
            Some(-37),
            [None; WIPEOUT_TYPE_COUNT],
        ));
        assert!(controls.disable_all);
    }

    #[test]
    fn resolved_overlay_isolates_each_index_and_global_gate() {
        for requested_type in WipeoutType::ALL {
            let mut ledger = WipeoutRequestLedger::new();
            ledger.request(requested_type, 0.0);

            let mut enabled_value = [None; WIPEOUT_TYPE_COUNT];
            enabled_value[requested_type.index()] = Some(-1);
            let mut controls = ALL_DISABLED;
            controls.apply_resolved_integer_overlay(WipeoutEnableIntegerOverlay::new(
                None,
                enabled_value,
            ));

            assert!(ledger.does_player_request_wipeout(controls));
            for wipeout_type in WipeoutType::ALL {
                assert_eq!(
                    controls.is_enabled(wipeout_type),
                    wipeout_type == requested_type
                );
            }

            let mut disabled_value = [None; WIPEOUT_TYPE_COUNT];
            disabled_value[requested_type.index()] = Some(0);
            controls.apply_resolved_integer_overlay(WipeoutEnableIntegerOverlay::new(
                None,
                disabled_value,
            ));
            assert!(!ledger.does_player_request_wipeout(controls));

            controls.apply_resolved_integer_overlay(WipeoutEnableIntegerOverlay::new(
                Some(1),
                enabled_value,
            ));
            assert!(!ledger.does_player_request_wipeout(controls));
        }
    }

    #[test]
    fn request_replaces_value_and_counts_duplicate_events() {
        let mut ledger = WipeoutRequestLedger::new();
        ledger.request(WipeoutType::SkeletonContact, 1.25);
        ledger.request(WipeoutType::SkeletonContact, -3.5);

        assert!(ledger.is_requested(WipeoutType::SkeletonContact));
        assert_eq!(
            ledger.request_value(WipeoutType::SkeletonContact).to_bits(),
            (-3.5_f32).to_bits()
        );
        assert_eq!(ledger.request_count(), 2);
    }

    #[test]
    fn per_type_enable_controls_gate_requests_by_matching_index() {
        let mut ledger = WipeoutRequestLedger::new();
        ledger.request(WipeoutType::SkeletonContact, 0.0);
        ledger.request(WipeoutType::OffBoardLandedOnDeck, 0.0);

        assert!(!ledger.does_player_request_wipeout(ALL_DISABLED));

        let mut enabled = [false; WIPEOUT_TYPE_COUNT];
        enabled[WipeoutType::OffBoardLandedOnDeck.index()] = true;
        assert!(ledger.does_player_request_wipeout(WipeoutEnableControls::new(false, enabled)));

        assert!(
            !ledger.does_player_request_wipeout(WipeoutEnableControls::new(
                true,
                [true; WIPEOUT_TYPE_COUNT]
            ))
        );
    }

    #[test]
    fn clear_zeros_flags_values_and_event_count() {
        let mut ledger = WipeoutRequestLedger::new();
        let mut value = 0.5;
        for wipeout_type in WipeoutType::ALL {
            ledger.request(wipeout_type, value);
            value += 1.0;
        }
        ledger.clear();

        assert_eq!(ledger.request_count(), 0);
        for wipeout_type in WipeoutType::ALL {
            assert!(!ledger.is_requested(wipeout_type));
            assert_eq!(
                ledger.request_value(wipeout_type).to_bits(),
                0.0_f32.to_bits()
            );
        }
    }

    #[test]
    fn runout_suppresses_physics_and_animation_wipeout_requests() {
        let mut ledger = WipeoutRequestLedger::new();
        ledger.request(WipeoutType::HeavyLanding, 0.0);

        assert!(!is_wipeout_requested(
            &ledger,
            ALL_ENABLED,
            WipeoutRequestSources {
                runout_requested: true,
                animation_requests_wipeout: true,
            }
        ));
    }

    #[test]
    fn animation_request_is_consulted_after_enabled_physics_requests() {
        let ledger = WipeoutRequestLedger::new();
        assert!(is_wipeout_requested(
            &ledger,
            ALL_DISABLED,
            WipeoutRequestSources {
                runout_requested: false,
                animation_requests_wipeout: true,
            }
        ));

        let mut requested = WipeoutRequestLedger::new();
        requested.request(WipeoutType::VehicleCollision, 0.0);
        assert!(is_wipeout_requested(
            &requested,
            ALL_ENABLED,
            WipeoutRequestSources::default()
        ));
        assert!(!is_wipeout_requested(
            &requested,
            WipeoutEnableControls::new(true, [true; WIPEOUT_TYPE_COUNT]),
            WipeoutRequestSources::default()
        ));
    }

    #[test]
    fn offboard_branch_priorities_match_the_reference() {
        let mut input = OffBoardStateInput::new(
            [
                OffBoardStateFlag::WipingOut,
                OffBoardStateFlag::AnimationRequestsWipeout,
                OffBoardStateFlag::GrabbingObject,
                OffBoardStateFlag::OffBoardComMotion,
                OffBoardStateFlag::OffBoardTrajectory,
                OffBoardStateFlag::OffBoardAir,
                OffBoardStateFlag::OffBoardStandingJump,
                OffBoardStateFlag::OffBoardJumpInfoReady,
                OffBoardStateFlag::LandOnBoard,
                OffBoardStateFlag::BoardMountInvalid,
            ],
            OpaqueProcessedWord564Bit18::Set,
        );
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoard),
            Some(PhysicsState::WipeoutGround)
        );

        input.flags.remove(&OffBoardStateFlag::WipingOut);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoard),
            Some(PhysicsState::OffBoardPushing)
        );

        input.flags.remove(&OffBoardStateFlag::GrabbingObject);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoard),
            Some(PhysicsState::OffBoardLocomotion)
        );

        input.flags.remove(&OffBoardStateFlag::OffBoardComMotion);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoard),
            Some(PhysicsState::OffBoardJump)
        );

        input
            .flags
            .remove(&OffBoardStateFlag::OffBoardJumpInfoReady);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoard),
            Some(PhysicsState::OffBoard)
        );
    }

    #[test]
    fn offboard_opaque_branch_requires_set_bit_and_valid_mount() {
        let mut input = OffBoardStateInput::new([], OpaqueProcessedWord564Bit18::Set);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoard),
            Some(PhysicsState::PhysicsAir)
        );
        input.flags.insert(OffBoardStateFlag::BoardMountInvalid);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoard),
            Some(PhysicsState::WipeoutGround)
        );
        assert_eq!(
            OffBoardStateInput::default().suggested_state(PhysicsState::OffBoard),
            Some(PhysicsState::WipeoutGround)
        );
    }

    #[test]
    fn pushing_prioritizes_wipeout_then_grab_then_offboard() {
        let mut input = offboard_input([
            OffBoardStateFlag::WipingOut,
            OffBoardStateFlag::GrabbingObject,
        ]);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardPushing),
            Some(PhysicsState::WipeoutGround)
        );
        input.flags.remove(&OffBoardStateFlag::WipingOut);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardPushing),
            Some(PhysicsState::OffBoardPushing)
        );
        input.flags.remove(&OffBoardStateFlag::GrabbingObject);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardPushing),
            Some(PhysicsState::OffBoard)
        );
    }

    #[test]
    fn air_branch_priorities_and_ties_match_the_reference() {
        let mut input = offboard_input([
            OffBoardStateFlag::WipingOut,
            OffBoardStateFlag::OffBoardComMotion,
            OffBoardStateFlag::OffBoardTrajectory,
            OffBoardStateFlag::LandOnBoard,
            OffBoardStateFlag::OffBoardAir,
        ]);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardAir),
            Some(PhysicsState::WipeoutGround)
        );
        input.flags.remove(&OffBoardStateFlag::WipingOut);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardAir),
            Some(PhysicsState::OffBoardLocomotion)
        );
        input.flags.remove(&OffBoardStateFlag::OffBoardComMotion);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardAir),
            Some(PhysicsState::OffBoard)
        );
        input.flags.remove(&OffBoardStateFlag::OffBoardTrajectory);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardAir),
            Some(PhysicsState::LandingOnDeck)
        );
        input.flags.remove(&OffBoardStateFlag::LandOnBoard);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardAir),
            Some(PhysicsState::OffBoardAir)
        );
        input.flags.remove(&OffBoardStateFlag::OffBoardAir);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardAir),
            Some(PhysicsState::PhysicsAir)
        );
        input.flags.insert(OffBoardStateFlag::BoardMountInvalid);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardAir),
            Some(PhysicsState::WipeoutGround)
        );
    }

    #[test]
    fn locomotion_branch_priorities_and_opaque_polarity_match_reference() {
        let mut input = OffBoardStateInput::new(
            [
                OffBoardStateFlag::WipingOut,
                OffBoardStateFlag::AnimationRequestsWipeout,
                OffBoardStateFlag::GrabbingObject,
                OffBoardStateFlag::OffBoardTrajectory,
                OffBoardStateFlag::OffBoardAir,
                OffBoardStateFlag::LandOnBoard,
                OffBoardStateFlag::OffBoardComMotion,
            ],
            OpaqueProcessedWord564Bit18::Set,
        );
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardLocomotion),
            Some(PhysicsState::WipeoutGround)
        );
        input.flags.remove(&OffBoardStateFlag::WipingOut);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardLocomotion),
            Some(PhysicsState::WipeoutGround)
        );
        input
            .flags
            .remove(&OffBoardStateFlag::AnimationRequestsWipeout);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardLocomotion),
            Some(PhysicsState::OffBoardPushing)
        );
        input.flags.remove(&OffBoardStateFlag::GrabbingObject);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardLocomotion),
            Some(PhysicsState::OffBoard)
        );
        input.flags.remove(&OffBoardStateFlag::OffBoardTrajectory);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardLocomotion),
            Some(PhysicsState::OffBoardAir)
        );
        input.flags.remove(&OffBoardStateFlag::OffBoardAir);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardLocomotion),
            Some(PhysicsState::LandingOnDeck)
        );

        input.flags.remove(&OffBoardStateFlag::OffBoardComMotion);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardLocomotion),
            Some(PhysicsState::LandingOnDeck),
            "LandOnBoard is tested before the no-COM opaque branch"
        );

        input.flags.remove(&OffBoardStateFlag::LandOnBoard);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardLocomotion),
            Some(PhysicsState::PhysicsGround)
        );
        input.opaque_word_564_bit_18 = OpaqueProcessedWord564Bit18::Unset;
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardLocomotion),
            Some(PhysicsState::WipeoutGround)
        );

        input.flags.insert(OffBoardStateFlag::OffBoardComMotion);
        assert_eq!(
            input.suggested_state(PhysicsState::OffBoardLocomotion),
            Some(PhysicsState::OffBoardLocomotion)
        );
    }

    #[test]
    fn selector_explicitly_rejects_states_outside_implemented_slice() {
        assert_eq!(
            OffBoardStateInput::default().suggested_state(PhysicsState::OffBoardJump),
            None
        );
        assert_eq!(
            OffBoardStateInput::default().suggested_state(PhysicsState::PhysicsGround),
            None
        );
    }
}

#[cfg(test)]
mod locomotion_lazy_tests {
    use super::*;
    #[test]
    fn wipeout_short_circuits_unknown_later_fields() {
        let mut calls = Vec::new();
        let result = OffBoardStateInput::try_suggest_from_locomotion(
            |f| {
                calls.push(f);
                if f == OffBoardStateFlag::WipingOut {
                    Ok(true)
                } else {
                    Err(())
                }
            },
            || Err(()),
        );
        assert_eq!(result, Ok(PhysicsState::WipeoutGround));
        assert_eq!(calls, [OffBoardStateFlag::WipingOut]);
    }
    #[test]
    fn staying_locomotion_does_not_read_mount_bit() {
        let result = OffBoardStateInput::try_suggest_from_locomotion(
            |f| Ok::<_, ()>(f == OffBoardStateFlag::OffBoardComMotion),
            || Err(()),
        );
        assert_eq!(result, Ok(PhysicsState::OffBoardLocomotion));
    }
}
