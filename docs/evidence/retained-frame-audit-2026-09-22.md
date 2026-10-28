# Retained-frame resumption audit, September 22

Goal remains the first rendered skating test. DMO/livingworld and grinds remain
parked. This is an in-progress evidence record, not a strict acceptance checkpoint.
The inherited working tree contains 44 modified and 1,493 untracked paths; none
were reset or cleaned. Existing untracked source is treated as prior work.

## Spawn and first reached failure

The repeated-frame mode in `physical_player_ground_startup_corpus.rs` now reads
the original BAM_Sim.loc through the existing mounted corpus service and copies
the entire A_RobsPark transform before Actor and physical construction. The five
older fixture modes retain their original spawn. This is explicit host selection
of authored content, not native spawn-manager/AdjustSpawnPosition parity.
Independent review verified full-matrix ownership and startup order;
the capsule/line constructor assertions are local initialization, not origin-only
world assumptions. Recognizer setup still primes retained input with diagnostic
Ollie samples, so these frames are not a pristine neutral-input boot trace.

`artifacts/resume-2026-09-22-authored-spawn1.log` compiles and fails frame0 at
BoardPost InitializeTransformsPending(BadData). Independent fresh IDA review of
IsBadData82b2b398 confirms that this means NaN XYZ in the actual Part6 transform;
it is not an unknown-W or large-coordinate rejection. Source Board.Post82b37d30
sets WantsRespawn before InitializeTransforms, TeleportReset and return2. The
existing guard correctly stops before those absent recovery operations.

`artifacts/resume-2026-09-22-numeric-origin1.log` isolates the corruption to the
solver: all seven board transforms/velocities pass finite checks before Actor,
before pre-solve and before solver. After solver, Part6 has NaN in all transform
XYZ columns, with finite velocity approximately [0,-10,0]. Root is tracing the
actual RF/force/quaternion producer; no guard relaxation or tuning repair is made.

## Air collision collaborator

New `skeleton_controller_air_update.rs` and
`physical_player_skeleton_air_collision.rs` reuse the existing controller and
validated same-pair collision lease. Independent author/reviewer IDA checks:

- UpdateAirCollision82dd9fc8 reads `lhz +8c8`, low bit: full big-endian word
  mask00010000. True skips velocity; otherwise real Processed DeckVelocity+190 Y
  must be strictly greater than +0 (NaN and either zero are false).
- Changed-only latch store82dda048 precedes SetSkeletonCollisionState82dda05c,
  requesting7 or6, including failure prefixes.
- Existing SkeletonNormalAssembly performs actual collision mutation; the pair
  lease invalidates publication before possible writes.

Confidence high; registration/compilation and tests are pending. The force-air
flag's selected-path producer is not yet found; unit tests of the flag are not
live producer coverage. Full Air.Update, trajectory and post/fill remain open.

## Duplication corrections to older handoffs

- GPU skinning already exists in `apps/skate2-pc/src/gpu_renderer/character.rs`,
  including create/update_character_object and four-weight WGSL. Missing work is
  actual Actor presentation/draw mounting. Do not implement another skinner.
- Existing unregistered `skater_presentation_*` drafts cover initialization,
  restore, compression, root move, inverse, twist and expressions. Their required
  SQT range/category2 amendments also exist. Review and extend them, not recreate.
- Presentation still needs actual constructor capacities, scheduler category2,
  SetSQT/SetMatrix/PoseWithMatMult commands, synchronized in-place replay mutation,
  WriteSkater/ReadSkater/UpdateBodies, and same-Actor publication. Existing generic
  format decoders/inverse-bind arithmetic are not native-order numeric proof.
- `physical_player_skeleton_launch_info.rs` already exists unregistered. Its
  intended typed launch-record migration is unfinished. Keep actual retained
  OnboardTrajectoryOwner; do not install a parallel persistent selector.
- Existing World skeleton trajectory queries are linear with zero gravity;
  Air requires ballistic Predictor-owned futures, not those jobs unchanged.
- Legacy physics.rs selector wrappers differ from native retained store order
  and omit completed COM production. Reuse verified math after addressing these
  differences; their existence does not establish a live Air path.

Strict accepted baseline remains September20 channel lifecycle, documented in
the September20 handoff. Neither three-frame success nor playable parity is
established by this audit.

## Solver defect recovery

The next diagnostic isolates NaN to Drive angular Build; incoming RF, quaternion,
force and torque were finite. Independent raw reviews found and corrected:

1. Drive Horus angular projection must consume B.RF2-A.RF2, not attachment-point
   relative motion. Direct25c4 versus2678 and indirect2f60 versus3000 prove the
   distinct inputs. Pure translation/spin regression covers both entrypoints.
2. Shared Jacobian_RQD::Create rowK X/Y incorrectly reused stale rowI
   intermediates. Raw register overwrites3840/3848 feed inserts3898/38cc.
   See [complete raw proof](rqd-create-raw-register-audit-2026-09-22.md).
   Identity and same-quarter-turn cases independently expose the wrong axis.

`artifacts/resume-2026-09-22-rqd-fix-original1.log` now completes authored frame0
through post-solve, with all board transforms finite. Frame1 still legitimately
selects Air and hits Ground-only dispatch. The NaN cause is repaired, not hidden
by guard removal, physical tuning, or synthesized contacts.

Corrected Drive Horus golden values come from the separate reproducible scalar
fmaf evaluator `artifacts/review-drive-horus-fmaf-2026-09-22.py` and its .txt
output. Shared-row mutation coverage now compares actual final shared RF because
both ordering variants saturate the same second accumulator. Downstream Build
goldens were regenerated from independently amended source oracles; workspace
checks pass with these values.


## Air integration batch validation

Workspace-air-launch-2 passes4,375 unit+6doc tests. Original-air-launch-1 passes
16/17 (4.72s) with actual Air selected at frame1. Strict clippy-air-launch-6
and formatting pass.
Private Launch preparation and normal continuation, typed ballistic World jobs,
same90-slot predictor queue, max-time, AirPost/Fill and Air wipeout all have
independent raw source approval. The source GetLaunchInfo and AdjustStart bridges
are registered. Actual normal submission remains separate from scheduled harvest.
No complete Air tick or runnable skating claim.

Air.CalculateBodySpinSpeed82d841a8 is now in physical_player_air_spin_target.rs:
FETCH c84eb5b5a3d7dd4a/default4 before Processed960, multiply scale then literal
3c8efa35, cached Air layout graph8 atf0/110 without fallback, graph(time*2),
final graph*radians. Raw4250 bytes177ee0b5 decode VA62/VB60 despite IDA phantomv94.
Independent source review approved the same-owner adapter; no per-tick trace yet.

Fan legacy expectation changes use the independent scalar/FMA oracle
artifacts/review-fan-scalar-oracle-2026-09-22.py and .txt. Source Sin(pi) and
Cos(pi) polynomial residuals are intentionally retained; nonfinite W is never
forced zero. No expected value was copied from the implementation under test.
