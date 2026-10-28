# Challenge travel, race gates, map header and security capture (2026-10-11)

Implemented and final validation completed, solo. Live Jan17 rich-symbol IDA is read-only. Preserve all existing
work and the real save. Scratch: `.scratch/challenge-security-20261011`.

## Confirmed mismatches

- Map challenge_info SetMode3 skipped UpdatePositions3f6d..40db's header
  smalltext/smalltextcenter selection. It now uses the existing translated
  header routine after the original body-offset calculation3ee4..3efc.
  Original APT: `challenge_info.txt` (extracted; not in the repository).
- ChaseMan.Update8270f880/8270fa20 reads PhysOut+0x40 -> PhysOut_Filtered+0
  (mFilteredPhysicsState). The port read PhysOut_State.mPhysicsState instead,
  making the wipeout4/offboard6 comparisons wrong. Corrected to the filtered
  output. Confirmed PDB layouts through live IDA; capture gates are detailed below.
- Race gates were absent in visual_director.prepare, and
  vi_presentation.update_gate_splines disabled all32 slots. Native writer
  AddRaceGate827fda90 and UpdateGateSplines828037e0 do produce/render them.
  The producer/reader/presentation path is implemented below.
- ForeignActor is a selected-session restriction on SkaterManager queries.
  Challenge teleport can change the camera's local index independently of
  PlayerUI input. The separate native actor queries are now restored, as described below.

## Implemented source paths

- SessionMarker.Update828acc90 / VerifyPosition828ab328 use GetLocal;
  PlayerUI.UpdateSessionMarker828af3f0 uses GetLocalActor(mPlayerIndex).
  SelectedSessionMarkerSources now resolves those independently from the real
  roster, including local breadcrumbs and the input player's stance/transform.
  A legal AI camera target no longer fails a selected-actor pointer comparison.
- VDWriter.Prepare827ff4a8 takes 80 visible triggers in existing group order;
  AddRaceGate827fda90 packs up to32 enabled types2/3/4. Full volume transform,
  half lengths (+44/48/4c), next/finish flags and original ribbon/style keys now
  cross VisualIndicatorFrame, which the replay recorder already retains.
  WriteData827ff928 zeroes both style indices before lookup (missing keys use
  index0). BuildIndexMap82800178 uses sorted GetCollections order.
- VIPresentation.UpdateGateSplines828037e0 now uses its existing32 slots.
  GetTransformMatrix82800c70 scales axes by reciprocal(bounds)*halfLength*2;
  planar bounds.z==0 uses X's ratio for Z. Original colours, textures, render
  settings, culling, refresh clocks and NIS hiding are retained. Decoded raw
  VX128 source operands using tools/ida/vmx128_emu.py; bytes/listings are in
  scratch gates-raw.json. Shared host numeric boundaries still apply.
- Challenge character spawns now retain Pedestrian.mChallengeOwner (+1a08).
  ChaseMan receives the actual mutable challenge World during queued frames.
  JackSkater8270fd08 selects the overlay only for GM_SinglePlayer with no main
  challenge; the guard owner's SetCappingOnJacked8289ca48 is now connected.
  SetJackedRespawnPosition826f3c28 reads challenges attribute015d0811243856b2,
  nearest valid named location with strict distance comparison/first tie.
  Missing/empty lists preserve the old override; nonempty invalid lists clear
  its flag. Read-only Vault inspection confirms nine activities have lists,
  including Mongocorp HQ's three MongocorpHQ_Jack01/02/03 locations.
- Update8270fad4/e8 recognizes Photo6/Pro28; at timer expiry a local captured
  actor goes through ChallengeTeleportManager to StartLocation
  (e9b699e31f7c45ca), player/posID both local, reposition false, time0,
  setCamera false. Other cases use the normal breadcrumb respawn, including
  authored offboard/right-foot override (SetRespawnPosition82ce17f8).
- Native JackSkater/Update/SetCappingOnJacked and FE42.OnEnter82642450 do not
  deduct cash. Original jacked_overlay has Continue and no cash setter. No
  SecurityCaught/Fine/Jacked references were found in shipped challenge Lua
  blobs or progression handler source. No fine was invented.

## Build and live inspection

- check1 found five Arc coercion errors in the new marker query adapter;
  corrected with explicit as_ref(). No unit tests added/run.
- Release build1 failed during LLVM with out-of-memory (exit101). check2 passed
  (1m20s). With memory freed, build2 and subsequent live checks are recorded below.
- Env-gated trace adds gate counts/transforms and
  capture begin/cancel/respawn. MAP_PATH diagnostic drives ordinary menu pad
  input to a requested HAL/category row and stops on its info component.
- The first explicit hash printed during this task is
  348F24FA8B40BC80EC0299A78EFAF1161EDB5846EC98A13D7D59645295EB7837.
  The older completed-task handoff's C2B071DA hash is historical; the save has
  changed since. All planned runtime profiles are copies under this task's
  scratch directory. No command here has written the real save.

- The ForeignActor challenge is Alphabet Yard against Shingo
  (`t_1b_01_skate_alphabet_shingo`), not Snakestyle. Live IDA
  Online.SwitchCamera8288a4e0 confirms SetLocalIndex of the supplied player.
  Original Shingo handler listing is scratch/shingo-script.txt.

- Release build2 passed12m28s. Shingo1 on a copied real profile starts the
  requested Alphabet Yard variant, switches camera player0 -> player1 and
  continues into the leader/copy states without ForeignActor. Completed successfully as recorded below.
- Follow-up source audit: VerifyChallengeOkToUse828ab6d0 calls
  ChaseIsOn826f3b48 (mNumChasers != 0). Removed the stale always-false
  selected-session adapter. Original security-service constructor default
  remains for the unported Big Black service producer. SetCappingOnJacked
  now reads live locations first and uses shared host_dot3 for native
  vsubfp/vmsum3fp1288289cc14..20; host numeric limits still apply.
  These two follow-ups were applied after build2 and included in successful build3.

### Build2 runtime checks

- shingo1: copied current real profile, Alphabet Yard Shingo variant accepted
  and loaded; camera0 ->1 ->0 while native leader/copy handlers execute.
  Exit0 after8908 sim ticks/148.466667s. No ForeignActor. Screenshot015.
- map1 diagnostic selected the wrong contest category (SCONTESTS), stopped.
  map2 selected native TCONTESTS, Snakestyle at1620. Screenshot023 confirms
  smalltextcenter's one-line header and clear Teleport spacing. Stopped after
  capture: the diagnostic's equal start/end pad range emitted no button.
  Corrected the range for map3 (to run after final rebuild). No product defect
  was inferred from that empty diagnostic input interval.
- race1: original Fabio Memorial Race accepted, run_heat at2317,11 gates in
  the frame and actual ribbon draws. Gate count changes11 ->10 ->9 ->8 as
  the diagnostic teleports through normal trigger volumes. Captures022/026
  show the authored gates in-world; results at3072, A at3377, main cleared
  at3385 and gate count0. Exit0 after6096 simulation ticks/101.6s. This uses
  diagnostic traversal, not a manually skated race or a reference trace.
- Security1 used a copied real profile at original F_DT_Mongocorp
  coordinates read from BAM_Sim.loc in worldbam.big; capture results follow.

- security1: natural guard capture owner505, overlay=true, native4.5s timer;
  selected MongocorpHQ_Jack01's exact matrix, then breadcrumb respawn.
  Original JACKED/Continue screen visible (014); A restores focus and skater
  stands offboard at the authored exit (032). Exit0 after6182 simulation
  ticks/103.033333s. No capture cancellation or controlled-tick failure.
- Final release build3 included the two follow-up source corrections.
  All games were closed before compiling; final results follow.

## Confidence and remaining validation limits

- High confidence in recovered control flow, original authored data and APT
  layout selection. The live runs establish the reported flows in the port;
  they do not establish instruction-level or frame-for-frame native parity.
- Race transform reciprocal/reduction still uses the existing host VMX numeric
  adapter. No Xenon estimate/rounding identity is claimed.
- Photo6/Pro28 security StartLocation exceptions and non-career overlay gates
  were source-audited against native Update/JackSkater; those combinations
  have not been exercised in-game in this task. Original invalid/empty spawn
  list and closest-location tie semantics are retained by source inspection.
- The existing unported Big Black service producer remains a separate feature
  boundary. No fake active bodyguard state was introduced.
- No unit tests or broad test suite were added or run. Original-data live
  diagnostics use separate profiles and binaries, one process at a time.

### Final executable

- Release build3 passed11m20s (existing warnings), with both follow-ups.
  target/release/skate2-pc.exe timestamp2026-10-11 06:11:46 local.
- map3:1720x720, TCONTESTS -> Snakestyle info -> A Teleport. Capture026
  confirms smaller one-line title and separate Teleport row;063 confirms
  gameplay at the destination. Focus restored and exit0 after2586 sim ticks.
  Wall261.3s includes map pause and substantial streaming/frame backlog;
  this is functional coverage, not a performance/parity measurement.

- security2: final build repeats natural owner505 capture,4.5s timer,
  MongocorpHQ_Jack01 respawn and JACKED screen. A restores focus, Y remounts;
  subsequent pad input does not restore the stuck bail (captures021/025).
  Exit0 after5141 simulation ticks/85.683333s (116.97s wall).
- Final real save SHA256 remains
  348F24FA8B40BC80EC0299A78EFAF1161EDB5846EC98A13D7D59645295EB7837.
  No compiler/game process left running; no original game assets or IDB modified.
  No source changes after successful build3. All four requested fixes are in
  the ready release executable, subject to the explicit validation limits above.
