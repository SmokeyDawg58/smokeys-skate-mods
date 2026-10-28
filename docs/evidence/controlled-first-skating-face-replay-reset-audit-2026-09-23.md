# Controlled first-skating face replay reset audit (2026-09-23)

Read-only review of the matched `sk82_na_m.xex` IDA database, registered Rust
owners, and release captures at frames 30, 60 and 120. The missing native copy
below is **exact** for the inspected source branch; whether it alone causes the
visible face spikes is a **hypothesis** until the live pose is measured. No
registered source, asset, or test was changed for this audit.

## Source order and ownership

`Sk8::SkaterPresentation::DoSimRestore` (`0x827e6438`) calls
`Andale::PoseBufferExt<AnimCmdSys::SQTExt>::OverrideWith` at
`0x827e6fe0..0x827e6ffc` for every current entry: destination is
`mReplaySQTBuffer[2*i]` (even), source is `[2*i+1]` (odd). Its next call is
`UpdateBodies` at `0x827e7014`. The same per-restore copy follows authored
face pose setup on the initial path. The registered constructor performs that
initial copy; it does not cover later restores.

`UpdateBodies` (`0x827de988`) calls `UpdateBody` at `0x827deae0` and extracts
SQT into even replay at `0x827deaec`, then synchronizes at `0x827deb1c`.
`UpdateBody` (`0x827de868`) passes odd replay as `ReadSkater`'s source and even
replay as its destination at `0x827de914` for the ordinary selected branch.
`ReadSkater` (`0x827ddb60`) reads the odd/base face quaternion and the even
destination face translation, rotates each decoded face delta, adds it to that
destination translation, and stores the result in the even pose at
`0x827de1cc..0x827de288` (Face0; corresponding Face1/2 loops follow at
`0x827de314..0x827de520`). Thus the preceding override supplies the base
translation for every restore; the face delta is not meant to accumulate from
the prior completed frame.

The registered `OwnedAnimationSqtBuffer::try_override_from(&self, source: &Self)
-> Result<(), ActorFaceError>` at `actor_animation_sqt_output_ranges.rs:50`
implements native part-wise `OverrideWith825920b0`: it iterates source parts,
finds the first matching destination part, and copies source bone counts in
order. IDA `0x825920bc..0x82592198` confirms the source hierarchy/pose comes
from the method argument, the destination from `this`, and each matched part
uses `memcpy` of `48 * source_bone_count` bytes at `0x82592184`. The Rust method
requires synchronized source and idle destination and retains partial writes
on error. The selected runtime stores the actual odd base in `base` and
even working pose in `replay` (`skater_presentation_selected_runtime.rs:171-172`),
so those identities are already correct.

## Registered mismatch and observed scope

`skater_presentation_restore.rs:139` invokes `previous.try_override_from(&next)`
only during selected base construction. In repeated
`SelectedPresentationRuntime::try_update`, the path goes from frame/header
admission directly to `animation.try_decode_and_extract(&self.base,
&self.replay, ...)` at `skater_presentation_selected_runtime.rs:287-294`.
There is no per-frame odd-to-even override. The face loop in
`skater_presentation_read.rs:200-202` computes
`destination.previous.translation + rotated_delta`, so a nonzero repeated
delta can drift each tick. Existing finite-matrix and palette-generation
assertions do not detect finite but incorrect drift.

The captured body is already reclined at frames 30 and 60, with a board offset;
at frame 120 the head/face has pronounced triangle spikes. The copy omission
is localized to replay state before matrix generation and can explain
progressive face distortion. It does not establish why the whole body reclines
or why the board is displaced. The GPU path retains original raw UBYTE4
weights/indices (`gpu_renderer/character.rs:303-305`), uses residual fourth
weight and original position expansion (`:416-422`), and consumes checked
per-mesh rows from `CharacterPalette`; no renderer-only alteration is supported
by the present evidence.

## Smallest faithful fix and validation

After current frame/header/material admission and before
`try_decode_and_extract`, invoke the existing
`self.replay.try_override_from(&self.base)`. Keep it inside the selected
runtime's failed-publication boundary. `SelectedPresentationError` has no
`ActorFaceError` carrier today; add a narrow `ReplayOverride(ActorFaceError)`
variant rather than misclassifying this ownership copy as a decode or GPU error.
Preserve the source's part-wise copy, ordering, and partial-error behavior.

The existing mounted repeated-frame driver at
`physical_player_three_frame_corpus.rs:395` calls
`SelectedFrameProbe::check_with` with the same selected runtime across ticks;
its observer is in `skater_presentation_frame_corpus.rs:105`. Extend that
observer to record a representative `Face0/1/2` bone's odd/base and even
translation before read and after `Decoded` on two identical submitted face
payloads. Before read, even must equal odd after the override; after read, the
result must equal base plus the same decoded delta on both ticks, with no
linear growth. Then run the same 30/60/120 capture and compare face geometry.
If spikes persist, compare decoded face channels, final ACS head/face matrices,
per-mesh palette rows, and CPU-recreated skinned vertices for the affected mesh
before changing shader weights or remaps. A native per-tick pose trace is still
needed to claim full presentation parity.

## Integration result

The selected runtime now calls `self.replay.try_override_from(&self.base)`
before each `try_decode_and_extract`, with a dedicated `ReplayOverride` error.
The mounted original-data suite passed 29/29 after this change. A release
120-completion run exited zero and its capture
(`artifacts/controlled-skating-release120-face-reset-2026-09-23.png`) shows the
face spikes are gone. The body remains reclined near the ground and the board
remains displaced; that separate pose/physics problem is unresolved. The
source copy is independently IDA-reviewed. The renderer result is a runtime
observation, not native pose parity proof.
