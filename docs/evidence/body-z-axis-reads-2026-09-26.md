# Board Z axis reads: mBodyZAxis (+0x160) vs the flattened mBodyZDirection (+0x170) (2026-09-26)

**Status:** rustfmt parse PASS on every touched hunk. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`, with VMX operands decoded by hand.

## Native fields (PDB `Sk8::Physics::ProcessedPhysIn`)

- **+0x160 `mBodyZAxis`:** deck Z times the flipped-controls scalar. The port's `toolkit.body_z()`.
- **+0x170 `mBodyZDirection`:** the same axis with Y zeroed, normalized when long enough. The port's `toolkit.body_z_direction()`.
- **Where they differ:** on flat ground they point the same way. On slopes, transitions and vert they do not. On a vert wall the flattened vector is nearly parallel to the wall normal and the up vector.

## Native consumers

| Native consumer | Native load | Port before | Port now |
|---|---|---|---|
| `Toolkit_CalcPushAcceleration` 82b272f8 | +0x160 (274dc/274ec) | +0x170 | +0x160 |
| `Skateboard::UpdateSpeedModel` manual/spin sign | +0x160 (a558/a56c) | +0x170 | +0x160 |
| `ToolKit_CalcStraightenOutTorque` | +0x160 (286a8). Dotted with `mVelocity` +0x190, negated when behind, crossed with velocity against ground normal +0x1D0. | +0x170 | +0x160 |
| `PhysState_SlideGround::UpdateSkateboard` | state+0x10 `mProcessedPhysIn` +0x160 (82d8e50c). Dotted and crossed with the slide-rotated velocity; ±normal fallback on turn.y. | +0x170 | +0x160 |
| `Skeleton::FillPhysOut` shoulder twist | Skeleton+0 `mProcessedPhysIn` +0x160 (82b59644), fed to `AngleBetweenVectorsOnPlane` about Reckoning up (Skeleton+0x3DF4 `mReckoning`, +0x1B0) | +0x170 | +0x160 |
| `Toolkit_CalcBraking` 82b26df0 | +0x170 (27024/27058) | +0x170 | unchanged; label corrected to 170 |

**Already correct, unchanged:**
- `Toolkit_CalcGroundJump`: +0x160.
- `ToolKit_CalcHeadingTorque` and `Toolkit_CalcPumpForce`: +0x1B0 `mZAxisVelocity`.
- `ToolkitHelper_CalcFootForce`: +0x180 `mBodyYDirection`.
- `Toolkit_CalcCollisionForce`: +0x1C0 `mZAxisVelocityDir`.

## Effect

- **Push.** The force now follows the board's real forward axis on slopes; it was horizontal.
- **Straighten-out torque and slide-ground turn.** Both now use the board's real axis. On inclined surfaces the flattened vector had changed both the forward test and the cross product.
- **Shoulder twist** (`PhysOut_Skeleton.mShoulderTwist`). On vert the flattened vector is almost parallel to the plane normal, so the projected angle was degenerate.
- **Speed model.** Fixes the sign of the manual/spin term during manual pivots.

## Files

- `physical_player_push_source.rs`
- `skateboard_speed_model_update.rs` (variant renamed `BodyZAxis`) and `skeleton_processed_speed_model_source.rs`
- `skeleton_processed_ground_force_input.rs` (new `body_z_axis`, brake label corrected)
- `physical_player_straighten_source.rs` and `skateboard_straighten_torque.rs` (doc only)
- `physical_player_slide_ground.rs`
- `skeleton_fill_live_runtime.rs`
