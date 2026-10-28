# Live PrepareToolkitInput adapter

2026-09-12. Source `PhysicalPlayerHiLOD::Update1_ProcessInput82e01f98` calls
`Skateboard::PrepareToolkitInput82b26910` at `82e036cc`, then copies OLD skeleton
line results at36d4, conditionally writes left/right surface IDs, and calls
`Skeleton::ProcessData82b5ecc8` at3718. This packet covers the toolkit adapter,
not the separate ProcessData prefix or a completed physical frame. Confidence
**high** for source read order/owner mapping; numerical GPU/PPC parity remains
bounded by the explicit existing numeric policy and backend XYZ representation.

The arithmetic/source packet remains `skateboard-processed-toolkit-source.md`.
Previously, the only full toolkit callers were controlled corpus/unit providers:
`physical_player_ik_complete_corpus` made a separate processed object, fixture
flags0, zSpeed3, fixture Reckoning normal and literal cvar defaults. None of
those providers is reused by the new runtime adapter.

## Actual owner and read sequence

`PhysicalAssemblyWorld::try_prepare_player_toolkit` validates the actual board/
skeleton pair using `try_with_jump_pair`, obtains the already-retained
`owner.reckoning().processed()` Arc, releases the Reckoning lock, and holds that
same processed allocation while preparing input. This allocation is populated
by the preceding Update1 stores through36c4. No new processed owner, reset, or
new PhysOut sample supplies toolkit motion or ground input.

1. Actual `skateboard_body_mass::try_skateboard_body_mass` reads all attached
   Part inverse masses in source order (GetMass82b243e8), then commits processed
   mass. The actual mapped Part6 transform is read for deck, committed, read
   independently again for effective, then the retained processed flip bit is
   sampled at the source late point. Existing mapped Part.GetTransform backend
   supplies XYZ with a host canonical-W boundary; native W stays unknown. Do
   not switch to attached Part.GetBodyTransform, which omits Part-local inverse
   composition.
2. The existing directions algorithm owns its destination fields separately
   from a borrowed view of the same processed `jump_events` and `prior_output`.
   It preserves inverse Y/X/Z and late flag order, then reads the retained prior
   zSpeed twice at its two source sites. Neither read queries current deck
   velocity. A missing/unknown source fails at its first required site after
   retaining earlier writes.
3. Jump-normal continuation reads processed prior ground normal+1d0, not a new
   line hit or fresh Reckoning output. `try_with_board_force_runtime` supplies
   the same constructed SkateboardForceRuntime's retained jump normal+c0.
   The existing algorithm commits that board normal before the debug gate,
   then rereads it for the final processed normal only after the gate succeeds.

The helper split lets old tests/callers and live code share the same algorithms.
It avoids recursively locking processed from its own trait callbacks and avoids
copying its source fields into a fixture snapshot. The mutex excludes alias
mutation while each actual field read still occurs at its original code site.
The numeric service remains supplied explicitly.

## Different flip operations are intentional

Fresh raw `Skateboard::GetEffectiveTransform82b23778`: Part6 GetPosition call
2379c, flags+8c4 mask00100000 read237a8/ac. 237b8 splats integer-1;237c4/c8
shift to construct80000000 per lane;237cc/237d8 VXOR X/Z before full-column
stores237d0/dc. The existing prefix's bitwise sign flip is correct, including
NaN sign/payload behavior. In contrast, Skeleton effective animation getter
82b3ba48 multiplies X/Z by -1 atbae4/bae8 using processed+8cc mask8. Toolkit
directions also independently multiply physical X/Z by its +/-1 scalar. These
operations must not be collapsed into one generic flip helper.

## Live cvar targets

| Reference | Initializer / table entry | Section / label / title | Initial value |
|---|---|---|---|
|8309d970|82f7b228 / 82fd4420|Physics_Skateboard / kDrawJump / Draw Jump Info|false|
|8309d994|82f7b318 / 82fd442c|Physics_Skateboard / kGroundMaxCosAngleDelta / Cos of max angle change|3f6f5c29|
|8309d920|82f7b378 / 82fd4430|Physics_Skateboard / kGroundProportionalBlend / Blend to extreme ground normal|3dcccccd|

Both float constructors use GUI range[0,1]. Rawb354/b3b4 explicitly set r10=0
for the update callback; Hex-Rays' `buffer[153]` annotation is incorrect. The
runtime startup INI path does not clamp these values. Registration order is
Draw, Max, Blend before existing SoftDrive table4480/4484.

`ToolkitCvars` clones the actual Arc target at capture, then locks that captured
target only at the later value read. Maximum target capture26ccc precedes ground
and board reads/dot; its value is read26ce0. Blend is captured/read only in the
blend branch26d0c/26d24. Draw target is captured26d80/read26d84. Relinking references
between capture and value access must preserve the old captured target, while
mutation of that old target remains visible. A focused test exercises both.

Fresh independent raw review of26d80..26de4 corrects the former overlay-pending
boundary: when Draw is enabled,26d94 loads the current DebugOverlay singleton;
26d9c branches directly to26ddc if present. No debug draw call exists on this
path. Missing invokes FailAssert/DebugBreak, then a second singleton check and
assertion before the final copy. The native assertion boundary is represented
by an error at the first missing lookup, preserving the earlier board normal.

`ToolkitCvars` now retains a clone of the actual application `DebugOverlaySlot`
holder, not a captured instance. It performs `require()` only at this source
point. A later singleton publication remains visible through the shared holder.
Source-default false skips the check; enabled with the real empty allocation
also reaches the final processed-normal copy. This checks source existence,
not renderer/GPU completion. Tests cover a missing singleton, publication after
adapter construction and success from the same retained holder.

## Registration and validation handoff

New files: `physical_player_toolkit.rs` (child of constraint session),
`skeleton_processed_toolkit_runtime.rs` (child of processed toolkit), and
`input_cvar_toolkit.rs` (child of input_cvar_runtime). Shared changes are the
narrow directions/jump destination split and prefix helper extraction, plus
the three cvar definitions and target-array expansion. Existing Skeleton
trajectory cvars requested by its owner share that expansion, with separate
source ownership. Root owns outer Update1/corpus mounting and gates.

Required gate: original same-pair Update1 processed writes -> toolkit -> existing
old line Fill, verifying actual mass/Part6 reads, retained prior zSpeed and
ground-normal inputs, force-owner jump-normal mutation and no new query Sync.
Focused failures should preserve partial mass/deck/direction/board-normal writes.
The three modules and coordinated five cvar definitions are now registered;
the old/new callers share the refactored helpers. Selected-file rustfmt and
diff whitespace checks pass. Root outer mounting, source review and test gates
are pending; no Cargo or GPU test was run by this agent.
