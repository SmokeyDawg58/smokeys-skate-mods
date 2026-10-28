//! Deferred collection per-update arena count.
//!
//! `cAssetStreamManager::GetActiveCollectionRegIndex` (`0x82d54230`) chooses
//! the number of arena registration messages emitted by one deferred update.
//! It divides the active collection's total arena count by a stream-specific
//! maximum-tick cVar, clamps that quotient up to the global minimum arenas per
//! tick, then clamps the result down to the number of arenas still remaining.
//! This module models only that pure calculation plus the Presentation
//! producer's subsequent per-update boundary scan. Pending-record selection,
//! registry lookup, message sends, callbacks, and update synchronization stay
//! in their owning boundaries.
//!
//! `cAssetStreamManager::UpdatePresCollectionDefer`
//! (`0x82d5b5f0..0x82d5b8a0`) includes the first selected Arena without reading
//! its `cArena::mCost` only when the returned quota is signed
//! positive (`cmpwi`/`ble` at `0x82d5b6a4..0x82d5b6a8`, then
//! `0x82d5b6b4..0x82d5b704`). For each remaining quota position, it loads
//! `cAsset::mAssetData`, checks it for null, and reads the unsigned
//! `cArena::mCost` field at `+0x14` at `0x82d5b710..0x82d5b74c`. A nonzero
//! cost stops before that Arena; a zero cost advances and includes it at
//! `0x82d5b750..0x82d5b784`. Completion is tested from the index actually
//! reached at `0x82d5b790..0x82d5b79c`.
//!
//! The fallible provider and quota-versus-remaining rejection below are
//! explicit safe-host boundaries. Arena construction is recovered separately:
//! `ProcessArena` clears `cArena::mCost`, and `ProcessInstanceList` wrapping-
//! increments it for admitted Environment instances. This pure selector does
//! not itself own that processed Arena, nor model native null-pointer failure,
//! messages, proxy/callback suffixes, manager state, or dispatch.

use crate::game_stream::StreamKind;
use std::fmt;

/// Native lower and upper bounds registered for `kMinDeferredArenasPerTick`.
pub const MIN_DEFERRED_ARENAS_PER_TICK_RANGE: std::ops::RangeInclusive<u32> = 1..=100;

/// Native lower and upper bounds registered for each maximum-tick cVar.
pub const MAX_DEFERRED_COLLECTION_TICKS_RANGE: std::ops::RangeInclusive<u32> = 1..=600;

/// One of the four cVars consumed by the deferred batch calculation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferredCollectionBatchControl {
    MinArenasPerTick,
    MaxSimulationCollectionTicks,
    MaxPresentationCollectionTicks,
    MaxTextureCollectionTicks,
}

/// A deferred batch cVar value outside its native registered range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeferredCollectionBatchControlError {
    pub control: DeferredCollectionBatchControl,
    pub value: u32,
    pub minimum: u32,
    pub maximum: u32,
}

impl fmt::Display for DeferredCollectionBatchControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "deferred collection batch control {:?} value {} is outside {}..={}",
            self.control, self.value, self.minimum, self.maximum
        )
    }
}

impl std::error::Error for DeferredCollectionBatchControlError {}

/// Safe typed snapshot of the four live native batch cVars.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeferredCollectionBatchControls {
    min_arenas_per_tick: u32,
    max_simulation_collection_ticks: u32,
    max_presentation_collection_ticks: u32,
    max_texture_collection_ticks: u32,
}

impl DeferredCollectionBatchControls {
    /// Defaults registered by the retail executable.
    pub const NATIVE_DEFAULT: Self = Self {
        min_arenas_per_tick: 10,
        max_simulation_collection_ticks: 20,
        max_presentation_collection_ticks: 10,
        max_texture_collection_ticks: 5,
    };

    /// Validates one sampled value for each native cVar.
    ///
    /// # Errors
    ///
    /// Returns the first value outside the exact range registered for that
    /// cVar. In particular, zero is rejected before it could reach the native
    /// post-division logical/unsigned `<= 0` trap check. High-bit values are
    /// separately excluded by the registered upper bound.
    /// Validation is eager for all four values even though one native call
    /// reads only the selected stream's tick cVar and the global minimum.
    pub fn new(
        min_arenas_per_tick: u32,
        max_simulation_collection_ticks: u32,
        max_presentation_collection_ticks: u32,
        max_texture_collection_ticks: u32,
    ) -> Result<Self, DeferredCollectionBatchControlError> {
        validate_control(
            DeferredCollectionBatchControl::MinArenasPerTick,
            min_arenas_per_tick,
            &MIN_DEFERRED_ARENAS_PER_TICK_RANGE,
        )?;
        for (control, value) in [
            (
                DeferredCollectionBatchControl::MaxSimulationCollectionTicks,
                max_simulation_collection_ticks,
            ),
            (
                DeferredCollectionBatchControl::MaxPresentationCollectionTicks,
                max_presentation_collection_ticks,
            ),
            (
                DeferredCollectionBatchControl::MaxTextureCollectionTicks,
                max_texture_collection_ticks,
            ),
        ] {
            validate_control(control, value, &MAX_DEFERRED_COLLECTION_TICKS_RANGE)?;
        }
        Ok(Self {
            min_arenas_per_tick,
            max_simulation_collection_ticks,
            max_presentation_collection_ticks,
            max_texture_collection_ticks,
        })
    }

    #[must_use]
    pub const fn min_arenas_per_tick(self) -> u32 {
        self.min_arenas_per_tick
    }

    #[must_use]
    pub const fn max_collection_ticks(self, stream: StreamKind) -> u32 {
        self.max_collection_ticks_for_raw_stream_type(stream as u32)
    }

    /// Preserves the native raw branch: one is Simulation, zero is
    /// Presentation, and every other value selects Texture controls.
    #[must_use]
    pub const fn max_collection_ticks_for_raw_stream_type(self, stream_type_raw: u32) -> u32 {
        match stream_type_raw {
            1 => self.max_simulation_collection_ticks,
            0 => self.max_presentation_collection_ticks,
            _ => self.max_texture_collection_ticks,
        }
    }

    /// Reconstructs `GetActiveCollectionRegIndex` for valid active progress.
    ///
    /// `active_max_index` is the collection's total arena count, not its
    /// remaining count. The latter is used only for the final clamp.
    ///
    /// # Errors
    ///
    /// Returns an error when the active index has advanced beyond its maximum;
    /// the native update calls this function only while index is below maximum.
    /// Equality returns zero as a quiescent safe-host extension.
    pub fn arenas_for_update(
        self,
        stream: StreamKind,
        active_index: u32,
        active_max_index: u32,
    ) -> Result<u32, DeferredCollectionProgressError> {
        self.arenas_for_update_raw(stream as u32, active_index, active_max_index)
    }

    /// Raw-stream-type form of [`Self::arenas_for_update`].
    ///
    /// This retains the executable's Texture fallback for every value other
    /// than zero and one.
    ///
    /// # Errors
    ///
    /// Returns an error when the active index has advanced beyond its maximum.
    pub fn arenas_for_update_raw(
        self,
        stream_type_raw: u32,
        active_index: u32,
        active_max_index: u32,
    ) -> Result<u32, DeferredCollectionProgressError> {
        let Some(remaining) = active_max_index.checked_sub(active_index) else {
            return Err(DeferredCollectionProgressError {
                active_index,
                active_max_index,
            });
        };
        let quotient =
            active_max_index / self.max_collection_ticks_for_raw_stream_type(stream_type_raw);
        let target = if signed_word(quotient) < signed_word(self.min_arenas_per_tick) {
            self.min_arenas_per_tick
        } else {
            quotient
        };
        Ok(if signed_word(remaining) > signed_word(target) {
            target
        } else {
            remaining
        })
    }
}

impl Default for DeferredCollectionBatchControls {
    fn default() -> Self {
        Self::NATIVE_DEFAULT
    }
}

/// Invalid active deferred-collection progress supplied by the host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeferredCollectionProgressError {
    pub active_index: u32,
    pub active_max_index: u32,
}

impl fmt::Display for DeferredCollectionProgressError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "deferred collection active index {} exceeds maximum {}",
            self.active_index, self.active_max_index
        )
    }
}

impl std::error::Error for DeferredCollectionProgressError {}

/// Pure result of one Presentation deferred Arena boundary scan.
///
/// This value reports selection only; no Arena message or receiver effect has
/// occurred.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PresentationDeferredBatchSelection {
    /// Arena quota returned by [`DeferredCollectionBatchControls`].
    pub requested_arena_count: u32,
    /// Arenas selected before the first nonzero following Arena cost.
    pub selected_arena_count: u32,
    /// Active index after advancing by `selected_arena_count`.
    pub next_index: u32,
    /// Whether this scan selected at least one Arena and advanced exactly to
    /// the active maximum, making the native terminal suffix eligible.
    pub completed_this_update: bool,
}

/// Refusal from the pure Presentation deferred Arena boundary scan.
#[derive(Debug, Eq, PartialEq)]
pub enum PresentationDeferredBatchSelectionError<E> {
    /// The active index was already beyond the active maximum.
    Progress(DeferredCollectionProgressError),
    /// Exceptional signed-clamp behavior produced a quota larger than the
    /// unsigned remaining Arena count.
    QuotaExceedsRemaining {
        /// Quota returned by the shared deferred batch calculation.
        quota: u32,
        /// Unsigned `active_max_index - active_index`.
        remaining: u32,
    },
    /// The lazy Arena-cost provider refused the exact following ordinal.
    FollowingArenaCost {
        /// Authored collection ordinal whose `cArena::mCost` was requested.
        ordinal: u32,
        /// Provider-specific refusal.
        source: E,
    },
}

impl<E: fmt::Display> fmt::Display for PresentationDeferredBatchSelectionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Progress(source) => source.fmt(formatter),
            Self::QuotaExceedsRemaining { quota, remaining } => write!(
                formatter,
                "Presentation deferred Arena quota {quota} exceeds unsigned remaining count {remaining}"
            ),
            Self::FollowingArenaCost { ordinal, source } => write!(
                formatter,
                "could not read Presentation deferred Arena {ordinal} cost: {source}"
            ),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error
    for PresentationDeferredBatchSelectionError<E>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Progress(source) => Some(source),
            Self::FollowingArenaCost { source, .. } => Some(source),
            Self::QuotaExceedsRemaining { .. } => None,
        }
    }
}

/// Selects the exact Arena prefix for one Presentation deferred update.
///
/// When the shared quota is signed positive, the active ordinal is included
/// unconditionally. A zero or high-bit quota selects nothing, preserving the
/// native signed `cmpwi` gate. `following_arena_cost` is then called
/// lazily and in increasing ordinal order only for additional candidates. A
/// nonzero cost ends the prefix without including or advancing past that
/// candidate.
/// An already-complete input is a quiescent safe-host result, not terminal
/// authority: native entry equality skips both this scan and the terminal
/// suffix and proceeds directly to Setup.
///
/// # Errors
///
/// Returns a progress error for an index beyond the maximum, a safe-host
/// refusal if a signed-positive PPC clamp result exceeds unsigned remaining
/// progress, or the provider error paired with the exact ordinal it refused.
/// No provider call occurs for a signed-nonpositive quota or the unconditional
/// first ordinal.
pub fn select_presentation_deferred_batch<F, E>(
    controls: DeferredCollectionBatchControls,
    stream_type_raw: u32,
    active_index: u32,
    active_max_index: u32,
    following_arena_cost: F,
) -> Result<PresentationDeferredBatchSelection, PresentationDeferredBatchSelectionError<E>>
where
    F: FnMut(u32) -> Result<u32, E>,
{
    let quota = controls
        .arenas_for_update_raw(stream_type_raw, active_index, active_max_index)
        .map_err(PresentationDeferredBatchSelectionError::Progress)?;
    select_presentation_deferred_batch_with_quota(
        quota,
        active_index,
        active_max_index,
        following_arena_cost,
    )
}

fn select_presentation_deferred_batch_with_quota<F, E>(
    quota: u32,
    active_index: u32,
    active_max_index: u32,
    mut following_arena_cost: F,
) -> Result<PresentationDeferredBatchSelection, PresentationDeferredBatchSelectionError<E>>
where
    F: FnMut(u32) -> Result<u32, E>,
{
    let remaining = active_max_index.checked_sub(active_index).ok_or(
        PresentationDeferredBatchSelectionError::Progress(DeferredCollectionProgressError {
            active_index,
            active_max_index,
        }),
    )?;
    if signed_word(quota) <= 0 {
        return Ok(PresentationDeferredBatchSelection {
            requested_arena_count: quota,
            selected_arena_count: 0,
            next_index: active_index,
            completed_this_update: false,
        });
    }
    if quota > remaining {
        return Err(
            PresentationDeferredBatchSelectionError::QuotaExceedsRemaining { quota, remaining },
        );
    }

    let mut selected_arena_count = u32::from(quota != 0);
    while selected_arena_count < quota {
        let ordinal = active_index + selected_arena_count;
        let cost = following_arena_cost(ordinal).map_err(|source| {
            PresentationDeferredBatchSelectionError::FollowingArenaCost { ordinal, source }
        })?;
        if cost != 0 {
            break;
        }
        selected_arena_count += 1;
    }
    let next_index = active_index + selected_arena_count;
    Ok(PresentationDeferredBatchSelection {
        requested_arena_count: quota,
        selected_arena_count,
        next_index,
        completed_this_update: selected_arena_count != 0 && next_index == active_max_index,
    })
}

fn validate_control(
    control: DeferredCollectionBatchControl,
    value: u32,
    range: &std::ops::RangeInclusive<u32>,
) -> Result<(), DeferredCollectionBatchControlError> {
    if range.contains(&value) {
        return Ok(());
    }
    Err(DeferredCollectionBatchControlError {
        control,
        value,
        minimum: *range.start(),
        maximum: *range.end(),
    })
}

fn signed_word(value: u32) -> i32 {
    i32::from_be_bytes(value.to_be_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_defaults_and_stream_selection_are_exact() {
        let controls = DeferredCollectionBatchControls::default();
        assert_eq!(controls.min_arenas_per_tick(), 10);
        assert_eq!(controls.max_collection_ticks(StreamKind::Simulation), 20);
        assert_eq!(controls.max_collection_ticks(StreamKind::Presentation), 10);
        assert_eq!(controls.max_collection_ticks(StreamKind::Texture), 5);
        for raw in [2, 3, u32::MAX] {
            assert_eq!(controls.max_collection_ticks_for_raw_stream_type(raw), 5);
        }

        assert_eq!(
            controls.arenas_for_update(StreamKind::Simulation, 0, 284),
            Ok(14)
        );
        assert_eq!(
            controls.arenas_for_update(StreamKind::Presentation, 0, 284),
            Ok(28)
        );
        assert_eq!(
            controls.arenas_for_update(StreamKind::Texture, 0, 284),
            Ok(56)
        );
    }

    #[test]
    fn minimum_clamp_uses_total_count_before_remaining_clamp() {
        let controls = DeferredCollectionBatchControls::default();
        assert_eq!(
            controls.arenas_for_update(StreamKind::Simulation, 0, 9),
            Ok(9)
        );
        assert_eq!(
            controls.arenas_for_update(StreamKind::Simulation, 5, 100),
            Ok(10)
        );
        assert_eq!(
            controls.arenas_for_update(StreamKind::Simulation, 96, 100),
            Ok(4)
        );
    }

    #[test]
    fn quotient_is_based_on_original_total_on_every_update() {
        let controls = DeferredCollectionBatchControls::default();
        for active_index in [0, 14, 140, 266] {
            assert_eq!(
                controls.arenas_for_update(StreamKind::Simulation, active_index, 284),
                Ok(14)
            );
        }
        assert_eq!(
            controls.arenas_for_update(StreamKind::Simulation, 280, 284),
            Ok(4)
        );
    }

    #[test]
    fn complete_collection_returns_zero_and_past_end_is_rejected() {
        let controls = DeferredCollectionBatchControls::default();
        assert_eq!(
            controls.arenas_for_update(StreamKind::Simulation, 31, 31),
            Ok(0)
        );
        assert_eq!(
            controls.arenas_for_update(StreamKind::Simulation, 32, 31),
            Err(DeferredCollectionProgressError {
                active_index: 32,
                active_max_index: 31,
            })
        );
    }

    #[test]
    fn exact_registered_cvar_ranges_are_accepted() {
        for min_arenas in [1, 100] {
            for ticks in [1, 600] {
                let controls =
                    DeferredCollectionBatchControls::new(min_arenas, ticks, ticks, ticks).unwrap();
                assert_eq!(controls.min_arenas_per_tick(), min_arenas);
                assert_eq!(controls.max_collection_ticks(StreamKind::Simulation), ticks);
            }
        }
    }

    #[test]
    fn every_out_of_range_cvar_is_rejected_with_identity() {
        for (values, expected_control, expected_value, minimum, maximum) in [
            (
                [0, 20, 10, 5],
                DeferredCollectionBatchControl::MinArenasPerTick,
                0,
                1,
                100,
            ),
            (
                [101, 20, 10, 5],
                DeferredCollectionBatchControl::MinArenasPerTick,
                101,
                1,
                100,
            ),
            (
                [10, 0, 10, 5],
                DeferredCollectionBatchControl::MaxSimulationCollectionTicks,
                0,
                1,
                600,
            ),
            (
                [10, 20, 601, 5],
                DeferredCollectionBatchControl::MaxPresentationCollectionTicks,
                601,
                1,
                600,
            ),
            (
                [10, 20, 0, 5],
                DeferredCollectionBatchControl::MaxPresentationCollectionTicks,
                0,
                1,
                600,
            ),
            (
                [10, 20, 10, 0],
                DeferredCollectionBatchControl::MaxTextureCollectionTicks,
                0,
                1,
                600,
            ),
            (
                [10, 20, 10, u32::MAX],
                DeferredCollectionBatchControl::MaxTextureCollectionTicks,
                u32::MAX,
                1,
                600,
            ),
        ] {
            assert_eq!(
                DeferredCollectionBatchControls::new(values[0], values[1], values[2], values[3]),
                Err(DeferredCollectionBatchControlError {
                    control: expected_control,
                    value: expected_value,
                    minimum,
                    maximum,
                })
            );
        }
    }

    #[test]
    fn sampled_controls_change_only_the_proven_clamps() {
        let controls = DeferredCollectionBatchControls::new(3, 8, 4, 2).unwrap();
        assert_eq!(
            controls.arenas_for_update(StreamKind::Simulation, 0, 80),
            Ok(10)
        );
        assert_eq!(
            controls.arenas_for_update(StreamKind::Presentation, 0, 80),
            Ok(20)
        );
        assert_eq!(
            controls.arenas_for_update(StreamKind::Texture, 0, 80),
            Ok(40)
        );
    }

    #[test]
    fn division_is_unsigned_but_native_clamps_are_signed() {
        let one_tick = DeferredCollectionBatchControls::new(10, 1, 1, 1).unwrap();
        assert_eq!(
            one_tick.arenas_for_update(StreamKind::Simulation, 0x8000_0000, 0x8000_0020),
            Ok(10)
        );

        let defaults = DeferredCollectionBatchControls::default();
        assert_eq!(
            defaults.arenas_for_update(StreamKind::Simulation, 0, 0x8000_0000),
            Ok(0x8000_0000)
        );
    }

    #[test]
    fn presentation_quota_one_never_reads_the_unconditional_first_ordinal() {
        let controls = DeferredCollectionBatchControls::new(1, 600, 600, 600).unwrap();
        let selection = select_presentation_deferred_batch(
            controls,
            StreamKind::Presentation as u32,
            8,
            9,
            |_| -> Result<u32, &'static str> {
                panic!("quota-one selection must not read the first ordinal")
            },
        )
        .unwrap();
        assert_eq!(
            selection,
            PresentationDeferredBatchSelection {
                requested_arena_count: 1,
                selected_arena_count: 1,
                next_index: 9,
                completed_this_update: true,
            }
        );
    }

    #[test]
    fn presentation_all_zero_following_costs_select_the_complete_quota_lazily() {
        let mut ordinals = Vec::new();
        let selection = select_presentation_deferred_batch(
            DeferredCollectionBatchControls::default(),
            StreamKind::Presentation as u32,
            0,
            125,
            |ordinal| -> Result<u32, &'static str> {
                ordinals.push(ordinal);
                Ok(0)
            },
        )
        .unwrap();
        assert_eq!(selection.requested_arena_count, 12);
        assert_eq!(selection.selected_arena_count, 12);
        assert_eq!(selection.next_index, 12);
        assert!(!selection.completed_this_update);
        assert_eq!(ordinals, (1..12).collect::<Vec<_>>());
    }

    #[test]
    fn presentation_nonzero_cost_stops_before_candidate_then_candidate_is_next_first() {
        let controls = DeferredCollectionBatchControls::new(1, 1, 1, 1).unwrap();
        let mut first_reads = Vec::new();
        let first = select_presentation_deferred_batch(controls, 0, 0, 6, |ordinal| {
            first_reads.push(ordinal);
            Ok::<u32, &'static str>(if ordinal == 2 { 0x8000_0000 } else { 0 })
        })
        .unwrap();
        assert_eq!(first.requested_arena_count, 6);
        assert_eq!(first.selected_arena_count, 2);
        assert_eq!(first.next_index, 2);
        assert!(!first.completed_this_update);
        assert_eq!(first_reads, [1, 2]);

        let mut second_reads = Vec::new();
        let second =
            select_presentation_deferred_batch(controls, 0, first.next_index, 6, |ordinal| {
                second_reads.push(ordinal);
                Ok::<u32, &'static str>(0)
            })
            .unwrap();
        assert_eq!(second.selected_arena_count, 4);
        assert_eq!(second.next_index, 6);
        assert!(second.completed_this_update);
        assert_eq!(second_reads, [3, 4, 5]);
    }

    #[test]
    fn presentation_provider_refusal_identifies_its_exact_lazy_ordinal() {
        let controls = DeferredCollectionBatchControls::new(1, 1, 1, 1).unwrap();
        let mut ordinals = Vec::new();
        let error = select_presentation_deferred_batch(controls, 0, 0, 6, |ordinal| {
            ordinals.push(ordinal);
            if ordinal == 3 {
                Err("missing asset data")
            } else {
                Ok(0)
            }
        })
        .unwrap_err();
        assert_eq!(ordinals, [1, 2, 3]);
        assert_eq!(
            error,
            PresentationDeferredBatchSelectionError::FollowingArenaCost {
                ordinal: 3,
                source: "missing asset data",
            }
        );
    }

    #[test]
    fn presentation_selector_preserves_raw_batch_control_routing() {
        let controls = DeferredCollectionBatchControls::new(1, 4, 5, 10).unwrap();
        for (stream_type_raw, selected_arena_count) in [(0, 4), (1, 5), (2, 2), (u32::MAX, 2)] {
            let selection =
                select_presentation_deferred_batch(controls, stream_type_raw, 0, 20, |_| {
                    Ok::<u32, &'static str>(0)
                })
                .unwrap();
            assert_eq!(selection.requested_arena_count, selected_arena_count);
            assert_eq!(selection.selected_arena_count, selected_arena_count);
        }
    }

    #[test]
    fn presentation_completion_uses_actual_stopped_index_not_requested_quota() {
        let controls = DeferredCollectionBatchControls::new(1, 1, 1, 1).unwrap();
        let stopped = select_presentation_deferred_batch(controls, 0, 0, 4, |ordinal| {
            Ok::<u32, &'static str>(u32::from(ordinal == 3))
        })
        .unwrap();
        assert_eq!(stopped.requested_arena_count, 4);
        assert_eq!(stopped.selected_arena_count, 3);
        assert_eq!(stopped.next_index, 3);
        assert!(!stopped.completed_this_update);

        let completed = select_presentation_deferred_batch(
            controls,
            0,
            stopped.next_index,
            4,
            |_| -> Result<u32, &'static str> { panic!("the final candidate is unconditional") },
        )
        .unwrap();
        assert_eq!(completed.next_index, 4);
        assert!(completed.completed_this_update);

        let already_complete = select_presentation_deferred_batch(
            controls,
            0,
            4,
            4,
            |_| -> Result<u32, &'static str> { panic!("complete progress has no candidate") },
        )
        .unwrap();
        assert_eq!(already_complete.selected_arena_count, 0);
        assert_eq!(already_complete.next_index, 4);
        assert!(!already_complete.completed_this_update);
    }

    #[test]
    fn presentation_selector_rejects_quota_beyond_unsigned_remaining() {
        let error =
            select_presentation_deferred_batch_with_quota(3, 8, 10, |_| Ok::<u32, &'static str>(0))
                .unwrap_err();
        assert_eq!(
            error,
            PresentationDeferredBatchSelectionError::QuotaExceedsRemaining {
                quota: 3,
                remaining: 2,
            }
        );
    }

    #[test]
    fn presentation_signed_nonpositive_quota_selects_nothing_before_unsigned_bound() {
        let mut read_count = 0;
        let selection = select_presentation_deferred_batch(
            DeferredCollectionBatchControls::default(),
            StreamKind::Presentation as u32,
            0,
            0x8000_0000,
            |_| -> Result<u32, &'static str> {
                read_count += 1;
                Ok(0)
            },
        )
        .unwrap();
        assert_eq!(
            selection,
            PresentationDeferredBatchSelection {
                requested_arena_count: 0x8000_0000,
                selected_arena_count: 0,
                next_index: 0,
                completed_this_update: false,
            }
        );
        assert_eq!(read_count, 0);

        let direct = select_presentation_deferred_batch_with_quota(
            0x8000_0001,
            8,
            10,
            |_| -> Result<u32, &'static str> {
                panic!("signed-nonpositive quota must precede unsigned rejection")
            },
        )
        .unwrap();
        assert_eq!(direct.selected_arena_count, 0);
        assert_eq!(direct.next_index, 8);
        assert!(!direct.completed_this_update);
    }
}
