# Actual board outer post-physics admission

October11 continuation: both exceptional reset branches described below are
now implemented through the existing board reset, with native return codes,
output requests and retained post-solve contacts. See
[board recovery evidence](board-post-recovery-2026-10-11.md). The remainder
records the original draft's historical boundary.

New-only draft files:
`physical_player_board_post_outer_live.rs` (constraint-session child),
`skateboard_post_outer_predicates.rs` (its child),
`input_cvar_board_post_outer.rs` (input-cvar child), and
`skateboard_entry_postphysics.rs` (entry-state child).
No registered/shared file edits or Cargo commands were performed.

## Source and predicate observations

`Skateboard.UpdatePostPhysics`82b37d20..37e8c first runs real EndWheelLineTest.
IsBadData82b2b398 calls Body.GetPosition(part6)82b2b3b0, which resolves the actual
Part transform, not merely the raw rigid-body transform. Draft uses existing
`try_live_part_transform_typed` to preserve this composition. Raw VMX128 decoded
compare operands are self/self: b3c0 A62/B62, b3d8 A60/B60, b3f0 A58/B58,
b418 A56/B56, b430 A54/B54, b448 A52/B52, b470 A50/B50, b488 A48/B48,
b4a0 A46/B46, b4c8 A44/B44; each follows a scalar XYZ splat. All four columns
are checked, W is ignored. This rejects NaN but accepts infinity. Host scratch W
must not be promoted. Local predicate tests cover all12 XYZ positions, infinity,
and unrelated NaN W. The source checks rows separately after a failed row; the
already-captured local matrix has no intervening callback or mutable owner read.

HasAcceleratedStrangely82b26070 calls actual GetDeckVelocity82b25ff8, which reads
attached Part6 body's velocity+20. Draft reads that current body observation via
existing backend getter, not last-frame collision velocity. Native W is intrusive
link storage; only Y participates. Source order: current velocity call2608c,
capture Processed owner26098, min target260b0/value260bc, actual Processed
velocity+190 vector260d0, first test260dc, optional max target26104/value26108,
second test26124. Raw decoded260dc A62/B60 means currentY>min;26124 A56/B58
means max>oldY. Both are ordered strict comparisons. Old velocity is loaded even
if first predicate fails; max cvar is not loaded in that case. Tests cover equal,
both unordered positions, and propagation of the late cvar failure.

Outer37d9c byte102 is **existing `SkateboardEntryState.animated`**, native name
mSkateboardAnimated, constructor false82b38b14. Do not add a duplicate byte.
It is read only after HasAcceleratedStrangely returns true. False selects the
exceptional reset branch; true admits ordinary post-physics.

## Cvars157/158, parent integration

```rust
//157 ref8308db58 init82f7b028 table82fd4408; GUI0..100; null callback
Definition {
    section: "Physics_ATEST",
    label: "kMinBadVelY",
    title: "Min y vel bad",
    initial: Value::Float(f32::from_bits(0x41c8_0000)),
},
//158 ref8308db8c init82f7b088 table82fd440c; GUI0..100; null callback
Definition {
    section: "Physics_ATEST",
    label: "kMaxGoodVelY",
    title: "Min y vel GOOD",
    initial: Value::Float(f32::from_bits(0x40a0_0000)),
},
```

Fresh raw b064/b0c4 `li r10,0` defeats bogus Hex-Rays trailing arguments.
Insert registration157,158 after85(table4404) and before142(table4410).
Increase count to159 after154..156. Register cvar child `board_post_outer`.
The existing source-case-sensitive `(section,title)` sort puts both entries
before `kUseXYZCentreOfMass` (Physics_Animation), in order
`kMaxGoodVelY`, `kMinBadVelY`: uppercase GOOD precedes lowercase bad. Preserve
the exact labels/titles above, not lowercased ordering.

Register entry-state child `postphysics` by pathskateboard_entry_postphysics.rs.
It adds only a narrow setter on actual existing wants_respawn, initialized false
at native82b38b0c. The bad-data path calls that setter at37d68 before returning
its explicit InitializeTransformsPending error at37d6c. No duplicate field.

Register constraint-session child `board_post_outer_live`. Public-to-crate entry:
`try_update_board_postphysics_live(board,skeleton,ground,&mut BoardPostNumerics
{line,vector,rounding,truck})`. All four numeric collaborators are the existing
caller-supplied capabilities; no new numeric policy is installed. Reuse existing
154..156 live adapter. It returns native0 only after actual ordinary Body normal
stages, truck suffix, UpdateUpVector and existing force-queue clear succeed.

Caller replacement in dedicated solved Actor check: after EndSkeletonLineTests,
replace standalone EndBoardWheelLines and ChoosePhysicsSurface with this entire
outer call. Do not invoke either again. Initial up captured before the solver is
not reused: ordinary native37e3c reads actual Reckoning+1b0, then37e40 reads
actual Processed93c dt. Both short locks are released before Body services.
The current Processed guard used for UpdateUpVector is acquired only after Body
finishes; that existing adapter does not recursively lock Processed or Reckoning.

## Exceptional boundaries, no fake completion

Bad data native37d68 first sets actual wants_respawn; InitializeTransforms37d6c
then TeleportReset37d7c from actual Processed deck transform+40 follow. Strange
acceleration with animated false calls InitializeTransforms37dac then
TeleportReset37dbc from Processed effective transform+80, then SetVelocity zero,
then physical **angular** velocity+30 XYZ zero with retained W. The prior packet's
description of this final store as linear velocity was incorrect; fresh typed
source identifies mOmega and raw+30 agrees. Return codes2/1 are set after the
respective operations and all completed branches then clear the force queue.

Those live InitializeTransforms/TeleportReset consumers are not reconstructed
by this bounded draft. Both branches stop at the first absent operation and
preserve earlier native stores; they neither return1/2 nor clear queues as if
the reset completed. Existing authored transform initialization kernel is a
possible next dependency, but reset must also update actual body/Part attachments,
effective transform, turn fields, hook dynamics, target speed, and up histories.
No ordinary branch is hardcoded and no caller capability silently skips reset.

Compilation/original execution is pending parent-owned registration and gates.
