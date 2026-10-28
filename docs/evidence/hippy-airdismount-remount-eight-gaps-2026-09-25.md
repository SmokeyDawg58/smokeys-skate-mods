# Hippy antic, air dismount, remount and Motion gaps ? 2026-09-25

Implemented for the eight reported failures. SOLO. Compilation and runtime testing remain pending; no Cargo or game launch was performed.

## Source checks and implementation

- Motion Pre119: 8290de38 reads cached condition199 (HasIntent); Sustain119 is native default true at829221c8.
- Motion Pre158: 8290eda4 negates condition231 (HasIntent), preserving the unnegated condition cache. Sustain158 is default true.
- Motion Pre172: 8290f35c reads condition267 (HasIntent). Its independent Sustain at8291fbd8 also reads267.
- Update828f3d80 (reported2190425472) is Cycle1248. Its native parent is Grabbing995 at828f25f0, which calls Down251 before its own slots. Both authored aliases1237/1248 have exact ordered ancestry arrays, with no duplicate callbacks. Alias1237 ScoringGrabs2244 now uses the already implemented scoring callback alongside2260.
- Update828f4f60 (reported2190430048) is FromFakie1339. Native Push1028 at828f2e60 calls Riding277 then SetTurning, SetScoreAugmentation and a native empty callback. Leaf calls SetPushCoefs, ComputeTargetCoefsFromSpeedAndStrength and PlayAnimation. Existing implementations reused, both authored aliases1339/1341 admitted.
- Callback180 at82712148 is HippyJumpAntic, identified from the authored descriptor rather than the shared factory symbol. Constructor82cb6c78/vtable82302b0c has callbacks82712148/825187c0/82ca80f8/825187c0. The 0xc instance retains time initialized to zero. Update requires IAnimatable, increments by source fixed f32 0x3c888889, reads full attribute key5bdf9f0a81a276b0 from the existing retained Attrib instance, evaluates its eight-point graph (+16 X, +48 Y), and writes TrickHeight using existing AddSettableAtt. Missing attribute uses the source zero default area. Begin/End are native empty; existing instance allocation/removal owns lifetime.
- Callback1319 at82ca5ef0 is AirDismounting. Its 0xc instance retains integer frames. Begin82ca5db8 obtains base animation time then length, computes (length-time)*f32(426fffff), uses native fsel against1 and fctiwz through existing numeric conversion. Update82cb4248 requires ISkaterAnim/IAnimatable, queries presence of airdismountrevert, and calls the equivalent of DoAirDismountRevert82cb7738: write frames, OR player flag0x400. Existing GetPhysUpdateData consumes this request and clears the flag. End is native empty.
- MountValidationPending replaced with Update5_ProcessOutput82e04ea8 span82e058cc..5a10. Only when processed8d0 masks0x200000 and0x40000 are both set: compose Skeleton animation-to-world with current animation part0 (GetAnimPartTransformWorld82b3b878); obtain actual Skateboard.GetTransform from live deck part6. Compare squared translation distance first, then absolute Z-axis angle, then X-axis angle. Reuse existing matrix composition/dot3/AngleBetweenVectors helpers. Original degree factor verified as3c8efa35. Only an invalid result writes true; valid does not clear an existing invalid latch.
- Mount cvars registered with original section/label/title/defaults: ground linear0.2/angular25, air linear0.3/angular40. Initializers82f9ec48/eca8/ed08/ed68, table82fd62d8..62e4. Existing standard refs read before air override for current physical state502. Appended definition indices535..538 preserve all prior indices; registration order follows existing FeetLineTestRadius before subsequent initializers.

## Static validation

Rustfmt parse completed on changed Rust files. Script checked all four state ancestry arrays against authored parent/behavior declarations, exact order and uniqueness. Checked all539 cvars appear exactly once in registration. Updated the existing post-category test to cover valid mount, invalid mount and retained invalid latch; not executed. Existing no-mount test verifies the validation callback is not queried outside its flags gate.

This is source-checked implementation, not a claim of runtime validation or completion of unrelated states. Prior Boneless physical state602 remains unfinished. Quiet play and rendering were not changed.

## Compile correction

Compilation caught E0609 in the new air-dismount setter: frame storage belongs to existing `before_sprint`, while the request bit belongs to `player`. Corrected the setter to write the same `before_sprint.air_dismount_revert_frames` consumed by GetPhysUpdateData, then set player flag0x400. No duplicate field added. Rustfmt parse passed; rebuilding remains pending.
