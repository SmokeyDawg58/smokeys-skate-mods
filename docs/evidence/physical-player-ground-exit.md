# Retained Ground Exit and original-terrain Update2

Date: 2026-09-12. Implementation author: ground_path.
Independent root review and new Cargo/corpus validation pending at authorship.

## Source and ownership

Fresh IDA queries against the connected rich-symbol debug image recovered:

- `?Exit@PhysState_PhysicsGround@Physics@Sk8@@UAAXXZ`, `82d84a80`.
- `?Toolkit_ClampJumpVelocity@Skateboard@Physics@Sk8@@QAA?AVVector3@vpu@math@rw@@ABV4567@@Z`, `82b23990`.
- `Skateboard::RestoreWheelMaterial`, `82b29ff8`; `SetWipeout`, `82b30830`.
- `ProcessedPhysIn::Reset`, `82b39098`, with PDB member layouts.
- `SkeletonDrives::HookSetAngularSoftHips`, `82b47e70`, and linear `82b48000`.

The new `physical_player_ground_exit.rs` uses the same physical-pair owner,
Ground fields/bindings, Pumping, board session, retained Processed and seven-Part
velocity setter. It does not construct another state or publish its successor.
Source `SetPhysicsState` must invoke old Exit between its transition-prefix
stores and later state publication; completing this method does not implement
that outer transition or Air/WipeoutGround Enter.

## Ground Exit ordering

`82d84aac` writes damped turn zero, `ab4` writes push scalar one, `abc` resets
the actual Pumping owner, `ac8` sets actual board linear drag zero, `ad0`
restores only Parts 0,1,2,3 wheel materials. The `do_jump` test at `ad4`
follows all cleanup. It does not clear `do_jump` or alter current state.
Failure of later host operations retains this prefix.

The jump branch loads Processed gravity (`b08`) and timestep (`b14`), then
forms all four candidate lanes as `gravity_vector.mul_add(dt,new_velocity)`
(`b28..b40`). The gravity vector is `[0,gravity,0,0]`; Ground+`d0` is PDB
`SkateboardJumpInfo.mNewVelocity`, retained under the existing Rust field name
`jump_direction`. Toolkit clamp runs at `b50`, followed by all-Part SetVelocity
at `b68`. No parameter is replaced with a generic jump tuning value.

Processed PDB `mTimeDelta` is `+93c`; Reset store `82b3943c` loads the original
`0x3c888889` constant. Its new retained optional field preserves unknown-before-
Reset semantics. This Reset value does not establish current-frame Update1.
PDB `mVelocity` is `+190`; actual Reset `82b3921c` stores vector zero, now
represented in the existing prior-output field with `PhysOutOrigin::Reset`.

## Clamp dataflow discriminator

The name is misleading if implemented as a generic candidate-vector clamp:

1. `23990` loads `Skateboard+110`, PDB `mProcessedPhysIn`.
2. `23998` loads the supplied candidate into v63; `239a0` computes its dot3.
3. `239b0` **overwrites v63** with Processed+190; `239b8` computes its dot3.
4. Both squared lengths receive an rsqrt estimate (`239d0`, `239e0`) and two
   refinement rounds. Zero squared length selects positive zero after the
   refinements, rather than bypassing estimate execution.
5. `23a44` captures candidate length in f12; `23a4c` captures stored length
   in f13. `23a50/54` selects stored length when `stored_length-1 >= 0`,
   otherwise one (including unordered).
6. `23a58/5c`, raw40990008 (BO4/BI25), skips division unless the
   denominator is ordered greater than candidate length. Unordered therefore
   preserves scalar1; only ordered GT executes FDIVS23a60.
7. `23a88` multiplies the **stored** vector v63 by the selected scalar and
   stores all four lanes. Unknown native W remains unknown in the host type.

Raw words `239a0=17dff9b5`, `239b8=177ff9b5`, `23a88=16ffc0b5` preserve the
evidence needed to reject IDA's erroneous high-VA `v95` display. The actual
VA source is v63, following the previously documented VMX128 decoder defect.

The leaf preserves final negation of fused VNMSUB residuals:
`-squared.mul_add(square,-1)`, separately rounded half/product and FMADD.
Host dot reduction, rsqrt estimates, VMX denormals and NaN payloads remain
explicit numerical boundaries; no Xenon bit-exact result is claimed.

## Validation and confidence

Confidence: **high** source control/dataflow and retained-owner routing;
not per-tick runtime trace parity. Root independent review remains required.
Two focused tests discriminate stored direction/W from candidate direction,
zero, and unordered division. An actual-pair corpus helper runs the genuine
nonjump Ground Exit after the existing wipeout-material diagnostic, proves
restored wheel ownership and untouched body snapshots/current Ground state.
The jump-state producer is not injected into this original-data helper.

The grounded Update2 helper now runs immediately after the genuine initial
Air-prefix refusal and before hip-soft/ragdoll/wipeout diagnostics. It verifies
actual Ground/controller6/IK state, explicitly relocates the same board using
original Rob's Park surface queries, then invokes actual Update2 and its sole
source `ClearWipeoutRequests` suffix. No forced contact or state is introduced.
The initial all-ray miss and pending-Air result remain intact.

Fresh hip-soft review agrees with existing stores: linear estimate `8144`
precedes spring/type/strength (`8180/8188/819c`), final damping `81e0`;
subtraction constant `0x3c83126f` and strength `0x4560fffe`. Wipeout board
sets freewheel, deck angular drag zero, then material Parts6,0,1,2,3,4,5 and
actual collision-volume enable. These leaves are prerequisites, not a complete
native WipeoutGround entry or scheduler.

## September12 raw-branch correction

Root fresh read of23a5c found40990008. The prior ordered-LE interpretation
was incorrect: PPC BO4/BI25 branches when GT is false, including unordered.
Corrected the production selection and existing NaN discriminator; the retained
finite velocity now survives a NaN candidate with its W knowledge unchanged.
