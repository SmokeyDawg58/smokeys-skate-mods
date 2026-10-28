# Runout, contact, fastplant and grab continuations

Seven distinct missing paths were reported (RunoutFront callback128 repeated).
Work solo. Source edits are not runtime
verified. Renderer reuse helped somewhat; new-section hitches persist.
This batch makes no additional performance claim/change.

## Checked IDA paths

- RunoutFront128: Begin82ca57e8 calls ISkaterAnim+98 with true;
  End82ca58e0 calls the same slot with false. SetRunningOutFront82cb76a8
  changes only retained player flag0x4000. Reused existing lifecycle_animation(2)
  interface validation and motion_locomotion_flag setter. Existing getters and
  locomotion selection already consume this flag. Create82bb3da8 returns null
  instance; Update825187c0 is BLR. Wired all four lifecycle phases.
- TrucksOrDeckInContact227: Activation82c99aa0 gets IPhysical PhysOut,
  reads PhysOut+18 -> Collision+d70 mIsATruckInContact, then only if false
  Collision+d73 mIsDeckInContact. IDA type verified full outer/inner offsets.
  Added checked direct component getter without a write-latch check, matching
  this Motion reader. Existing board ground fill already publishes both fields.
  No extra physics state or guessed contact is introduced.
- Motion Pre1148290dc58: HasIntent194 (sConditions+308). Sustain114 uses
  default829221c8 true without reads.
- Motion Pre30482913884: c227 || !c85; Sustain default829221c8.
- Action Pre8382903b88 ->shared82901514: c58 &&
  ((c39 && !c42) || (c38 && c42)). Sustain8382908bdc: c39 || c38.
- Motion update828f0e70: FSFastPlant220 parent828efbd8, then SetPushCoefs30,
  ScoringTrick31, PlayAnimation32. Parent: Trick19 then native empty23,
  CreateAttribute24/25, empty26/27, CreateAttribute28/29.
  Added all15 authored aliases, reusing existing instance/callback implementations.
- Motion update828f3ea8: Grabbing995 parent828f25f0, empty27/28,
  PlayAnimation29. Parent chains checked via Down251828efe38,
  Grab686828f1848, Grabbing995828f25f0, CrailGrab1153828f3618.
  Added four authored grab-out aliases with their distinct behavior indices.

## Static verification

All19 new (state,address) arms are unique and exactly match their authored
ancestor-expanded behavior sequences. No duplicate parent execution.
Added regression assertions for collision field selection (wheel/skeleton
contact must not trigger this predicate), Motion304 short-circuiting/default
Sustain, and Action83 Sustain's omission of entry/mirror gates.
Rustfmt parse-only passed ten changed Rust files. No Cargo or game launch.
Check with build-release.bat, then play-quiet.bat; errors remain visible.
