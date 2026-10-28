//! Proposed child of processed Contact. Constructor projection, not query output.
use super::SkeletonProcessedContact;
#[derive(Default)]
pub(crate) struct ProcessedHangFlags(pub(super) Option<u32>);
impl ProcessedHangFlags {
    /// Processed ctor ea4 sets member+420, ef4 resets all190 bytes.
    /// Invoke only in actual constructor, BEFORE primary `GrabSpline` and Reset.
    pub(super) fn initialize_constructor(&mut self) {
        self.0 = Some(0);
    }
}
impl SkeletonProcessedContact {
    /// No ordinary Reset mutation: later full manager publication must replace
    /// this SAME word when the parked investigation pipeline resumes.
    pub(crate) fn retained_hang_flags(&self) -> Option<u32> {
        self.hang_flags.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constructor_initializes_once_and_ordinary_reset_preserves_embedded_flags() {
        let mut processed = SkeletonProcessedContact::new();
        assert_eq!(processed.retained_hang_flags(), None);
        processed.try_initialize_constructor_fields().unwrap();
        assert_eq!(processed.retained_hang_flags(), Some(0));
        processed.hang_flags.0 = Some(0x0c00_0042);
        processed.reset();
        assert_eq!(processed.retained_hang_flags(), Some(0x0c00_0042));
        assert!(processed.try_initialize_constructor_fields().is_err());
        assert_eq!(processed.retained_hang_flags(), Some(0x0c00_0042));
    }
}
