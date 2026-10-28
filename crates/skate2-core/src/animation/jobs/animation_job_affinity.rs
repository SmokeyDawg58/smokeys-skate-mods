//! Selected ACS category affinity and ready ordering from the native scheduler.
use super::AnimationExecutionError;

/// `SetCategoryForJob824d4f44..58` maps zero to1 and nonzero to4. The current
/// host path admits categories0/2 only: `WaitForJob824d53e0..e8` returns
/// immediately for other categories, which is not a completed-job guarantee.
pub(crate) fn selected_category_affinity(category: u32) -> Result<u32, AnimationExecutionError> {
    match category {
        0 => Ok(1),
        2 => Ok(4),
        _ => Err(AnimationExecutionError::UnsupportedCategory(category)),
    }
}

/// AddNotReady829afb18..44 packs complemented affinity, unsigned byte priority
/// and submission low47. RunSingleJob829b00bc..d8 ORs in complemented caller
/// affinity before unsigned64 minimum selection at012c..013c. Selected entry
/// validation fixes priority128, so it is a constant component of this key.
pub(super) fn ready_order(job_affinity: u32, caller_affinity: u32, submission: u64) -> (u32, u64) {
    (
        !(job_affinity & caller_affinity) & 63,
        submission & ((1_u64 << 47) - 1),
    )
}
