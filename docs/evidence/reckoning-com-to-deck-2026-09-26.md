# Reckoning COM-to-deck input: ProcessData copied the animation-space COM (2026-09-26)

**Status:** rustfmt parse PASS on every touched hunk. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

## Native data flow (IDA)

- **State base.** `Skeleton::ProcessData` 82b5f1cc passes `this+0x1940` (the `SkeletonState`) to `UpdateAnimRecord`.
- **`CalculateCentreOfMassAnim` 82b48550** writes the animation-space centre of mass to State+0x10C0 and its delta to State+0x10D0 (Skeleton+0x2A00 and +0x2A10).
- **`SkeletonState::UpdateAnimRecord` 82b50e40:**
  1. Transforms that COM and the deck translation (pose 0, State+0x40) by animToBoard (Skeleton+0x2D40).
  2. Stores world COM minus world deck at State+0x10F0.
  3. Stores the difference from the previous value, scaled by ResetScalar (+0x1358), at State+0x10E0.
  - These are Skeleton+0x2A30 and +0x2A20. `GetCOMToDeckWorld` 82b3b468 returns +0x2A30; `GetCOMToDeckWorldDelta` 82b3b478 returns +0x2A20.
- **`ProcessData` 82b5f2c4..f2e0** copies Skeleton+0x2A30 to Processed+0x2F0, then Skeleton+0x2A20 to Processed+0x300.
  - The PDB type `Sk8::Physics::ProcessedPhysIn` names these fields `mComToDeck` and `mComToDeckDelta`.
  - These are the only ProcessedPhysIn stores in ProcessData besides +0x9CC, +0x8C4, +0x8C8 and the conditional +0x2E0 hook.
- **Readers.** A scan of every physics-range load, store and `li`/`addi` of offsets +0x2F0..+0x308 finds:
  - `Reckoning::UpdateGroundStatesFixedHeading` 82ddd4f4: NormalizeSafe(mComToDeck), then candidate = J + dynamicUp - comDir.
  - `Reckoning::UpdateAirStates` 82ddff38: |mComToDeck| feeds `Air_MaxUpVectAngleDelta` (layout +0x150).
  - Only `ProcessedPhysIn::Reset` and `GetSyncLog` touch +0x300. The other hits are unrelated `this` fields (Skitching, OffBoard_Pushing, OffboardTrajectorySelector, HiLOD pointers).

## Port bug

`skeleton_process_data_output_runtime.rs` wrote `record().centre_of_mass()` and `centre_of_mass_delta()` (State+0x10C0/+0x10D0, the animation-space COM) into those two slots. The Reckoning reader (`skeleton_processed_reckoning_input.rs`, "COM to deck+2f0") consumed it.

## Why it matters on ramps and vert

- **Flat ground: no visible difference.** The world COM-to-deck and the animation-space COM both point roughly up.
- **Native on slopes.** The COM-to-deck direction follows the skater's actual lean in the world, so the grounded candidate `J + dynamicUp - comDir` pulls the Reckoning up vector back toward world up.
- **Port on slopes.** The animation-space COM direction is the board-local up whatever the terrain, so the candidate collapsed to `dynamicUp` and the up vector went straight to the ground normal.
- **Vault graphs (`physics_reckoning` default):**
  - `DynamicUpVsGroundY` (global +0x420) weights the candidate 1.0 for ground y >= 0.65, falling to 0 at vertical.
  - `GroundVectorBlendGraph` (+0x3E0) targets ground-normal blend 0.09 at 30 deg, 0.35 at 45 deg and 0.87 at vertical (input = ground angle / pi).
- **Simplified model** (a local script, `reck_sim.py`: the grounded selection logic above with the real graphs, the ground normal assumed converged, native comDir taken as the previous up):
  - *Steady state.* The two differ only on gentle slopes. On a 15 deg slope native settles its up vector at about 8 deg from vertical, the old port at 15 deg. From about 30 deg upward both converge near the normal, because the ground blend takes over.
  - *Transient.* Riding a 0.5 s curve from flat to vertical, native's up vector lags the surface by about 12-16 deg through most of the curve (44.8 deg at a 60 deg slope, 67.6 deg at 84 deg). The old port lagged only 1-4 deg (57.3 and 80.2).
  - So native keeps the body more world-upright while the board curves up the transition, then converges.
- **What the up vector drives:**
  - the Reckoning transform (skater orientation);
  - `CalculateTilt` 82dd7e60, which becomes `PhysOut_Animation.mAnimTilt` (+0x0). `SettingBodyTilt` 82c9e288 reads it as `tilt_x`, and `BTREE_RIDING` blends it.
  - `CalculateDynamicLean` 82dd82d0;
  - the board contact-normal cone (Body PostPhysics compares contacts against mUpVector).
- **Air.** The air rotation limit used |animation COM| instead of |COM-to-deck|.

## Fix

- **`skeleton_process_data_output_runtime.rs`.** ProcessData now stores `record().com_to_deck()` to +0x2F0, then `record().com_to_deck_delta()` to +0x300, in native store order.
- **`skeleton_processed_process_data_output.rs`.** The fields are renamed to `com_to_deck` and `com_to_deck_delta` (the PDB names) and stored as `SkeletonRootVector` with W retained. The writers and the reset test are updated, and a slot test is added.
- **`skeleton_processed_reckoning_input.rs`.** `GroundVector::ComToDeck` reads the stored vector directly.
- **W lane.** The Reckoning transform comes from `compose` with the known flip transform, so animToBoard's W lanes are known. The record's COM-to-deck W is therefore the same all-lane graph as native, which is 0 for a rigid transform. Unknown W would only propagate as `None`: Reckoning's lane helpers never reject it, and dot products ignore W.

## Other animation-COM consumers checked (correct, unchanged)

- `UpdateRootTransformsKnownAir` 82b3b1c8 loads Skeleton+0x2A00 (82b3b308), the animation-space COM. The port's `update_known_air` correctly uses `record().centre_of_mass()`.
- `LandingOnDeckManager::CalculateAccurateIKOffset` 82dceab0 calls `GetAnimCentreOfMassAnimSpace`, which the port matches.
- Air state, controller, grip, hippy-jump and landing-on-deck sites read `prior_output().centre_of_mass()`, which is Processed+0x250 (world COM). That is a different source and is correct.

## Also verified identical in this pass (no change)

- **`Skateboard::UpdateSpeedModel` 82b2a240**, prefix plus suffix: every key, register role and fsel/NaN branch, the graph offsets (surface +0x60/+0x80, idle +0x90/+0xB0, manual +0xE0/+0x100), force axis and magnitude. VMX operands were decoded by hand.
- **`Reckoning::UpdateGroundStatesFixedHeading` 82ddd468**, line by line: cross products (vnmsubfp operand order vD,vA,vB,vC = vB - vA*vC), angle and endpoint selection, ground blend, deck-up usage, slow/fast PID, clamped up acceleration, damping, reversal, coffin, and the transform/lean/tilt tail. `CalculateTransform` leaves `this` in r3 for `CalculateDynamicLean`.
- **`SettingBodyTilt::Update` 82c9e288**: mirror sign, spin graph, air/ground clamps, the two-stage fsel limiter and the kUseTilt/kSetTilt overrides.
- **Simulation step.** `Simulation::OnRequestSetSimRate` 8289e350 normal rate is 0.016666668 (60 Hz), which matches the port's fixed 1/60.
- **Cvar defaults.** `kUseAutoPump` defaults to true (82f86950) and `kTimeToConsiderPumpingMode` to 0.5 (82f869a0), matching the port.
- **Motion active set.** Riding Idle1025/Default1170 (`(1170, 0x828f_39b8)`) includes Crouching 723 and Pumping 1925. AggressiveOrLoose1169 is `IsProSkater` only.
- **Camera shake.** The shake result goes into the frame matrix and the offset reaches the positioner with collision attenuation (`try_publish_frame`).

## Trace

The `SKATE2_TRACE_VERT` build (`play-vert-trace.bat`) now also prints `vert reckoning:` every grounded Reckoning update. It shows the COM-to-deck input direction, candidate, ground normal, up, up target, ground blend, tilt_x and dynamic lean.
