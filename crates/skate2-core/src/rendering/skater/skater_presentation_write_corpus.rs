//! Original-data call after the actual selected post-solve completes.
use super::*;
use crate::{actor_construction::ActorConstruction, reckoning_numeric::HostReckoningNumeric};
#[derive(Default)]
struct Submission {
    blocks: Vec<Vec<u8>>,
    fail_second: bool,
}
impl PresentationDataSubmission for Submission {
    type Error = &'static str;
    fn submit_data(&mut self, bytes: &[u8], alignment: usize) -> Result<(), Self::Error> {
        assert_eq!(alignment, 2);
        if self.fail_second && self.blocks.len() == 1 {
            return Err("second submission");
        }
        self.blocks.push(bytes.to_vec());
        Ok(())
    }
}
pub(crate) fn check(actor: &ActorConstruction, offsets: &PresentationOffsets) {
    let source = actor.try_capture_presentation_pose().unwrap();
    let mut matrix_rotations = vec![[0xabcd; 4]; offsets.matrix_rotations.len()];
    let mut rotations = vec![[0xabcd; 3]; offsets.quaternion3_count];
    let mut translations = vec![[0xabcd; 3]; offsets.translation_count];
    let mut scratch = PresentationWriteScratch {
        matrix_rotations: &mut matrix_rotations,
        rotations: &mut rotations,
        translations: &mut translations,
    };
    let mut failed = Submission {
        fail_second: true,
        ..Submission::default()
    };
    assert!(matches!(
        source.try_write(
            offsets,
            &mut scratch,
            &mut HostReckoningNumeric,
            &mut failed
        ),
        Err(PresentationWriteError::Publication("second submission"))
    ));
    assert_eq!(
        failed.blocks.len(),
        1,
        "second failure retains the submitted Q prefix"
    );
    let mut complete = Submission::default();
    source
        .try_write(
            offsets,
            &mut scratch,
            &mut HostReckoningNumeric,
            &mut complete,
        )
        .unwrap();
    assert_eq!(complete.blocks.len(), 2);
    assert_eq!(complete.blocks[0], failed.blocks[0]);
    assert_eq!(
        complete.blocks[0].len(),
        offsets.matrix_rotations.len() * 8 + offsets.quaternion3_count * 6
    );
    assert_eq!(complete.blocks[1].len(), offsets.translation_count * 6);
    let first = scratch.matrix_rotations[0];
    assert_eq!(
        &complete.blocks[0][..8],
        &first
            .into_iter()
            .flat_map(u16::to_be_bytes)
            .collect::<Vec<_>>()
    );
    eprintln!(
        "same completed Actor pose -> native Q4/Q3/T compression -> ordered two-block submission ({} bytes); provider is an explicit test recorder, not live renderer publication",
        complete.blocks.iter().map(Vec::len).sum::<usize>()
    );
}
