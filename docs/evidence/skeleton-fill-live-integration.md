# Full Skeleton.Fill live adapter draft

2026-09-12. New files only, unregistered; no Cargo or tests run. Root owns
shared amendments, source review and integration. Source kernels retain the
existing numerical-policy limitations; this is not a hardware parity claim.

Concrete new pool/world mounts are now saved:
`physical_player_skeleton_fill_suffix_pool.rs` is a skeleton_members child
(reexport as skater_pool::skeleton_fill_suffix) and
`physical_player_update5_skeleton_suffix.rs` is a constraint-session child.
World consumes the existing Update5BeforeSkeletonAngularVelocity token including
its proposed captured_com_velocity field, delegates the same-owner pool call,
and returns Update5BeforePumping only on success. Both are unregistered/uncompiled.
The pool first validates the pair/expected owner, copies only the actual
Processed Arc under a short Reckoning guard, validates constructor bindings,
and borrows the SAME mutable Normal and existing PhysOut allocation. No
Reckoning guard spans the live source's later up read, and no source data
snapshot, fake token, replacement output, or skipped mandatory child is used.

`skeleton_fill_live_runtime.rs` is a proposed Normal child, invoking the existing
complete suffix `82b589b8..59848`, mandatory `SkeletonIK.Fill56590` and mandatory
`AudioCollisionInfo82b3efc0`. It contains no alternate math implementation.
The same output borrow passes directly through all three kernels. No services
callback locks PhysOut again. The only mutable source fields are the actual
IK histories, actual Collision accumulators, and actual General.want_respawn.

## Exact captured COM continuation

Fresh raw confirms `585ac` samples the latch, `585b0` loads v124 from retained
Skeleton COM velocity, `585d8/dc` publishes COM, and `58614` publishes that same
v124. Later `58b3c` subtracts v124 from the first loaded limb velocity. It is not
a second call or a reread of PhysOut. The registered prefix currently discards
this local by returning unit; both intermediate tokens currently retain owner
only. `skeleton-fill-live-integration.patch` proposes a single capture-return
implementation, preserving old unit-return wrappers, and carries the typed
value through both actual tokens. A later failure still retains earlier stores
and returns no successful continuation.

`physical_player_phys_out_com_capture_tests.rs` changes the collaborator's
current velocity during the later COM read. It checks the returned and published
values are both the earlier capture, including signed zero and unknown W, with
only one velocity read. Register it as a second test child in
`physical_player_phys_out_com.rs`. This test is saved, not run. Existing suffix
tests additionally distinguish the two fresh effective-transform calls, captured
COM limb speed, mandatory audio failure and late respawn publication.

## Actual inputs and mutable ownership

The parent validates the actual pair, constructor owner and Processed Arc. Use
`owner.inputs().vault`; use that same owner's PhysOut allocation and the actual
world DebugOverlaySlot. A new local immutable attribute view follows the
existing Collision.Update bind policy; no replacement Vault or local-body
profile stands in for the global collection. No tuning payload is fetched at
adapter construction. Immutable-Vault admission does not model editor/global
singleton replacement races.

The source adapter performs angular reads of actual assembly Parts23/20/16,
each through a fresh session observation and existing body angular accessor.
Packed body W stays unknown. HookToHips comes from same post_physics_records;
proxy/contact bytes come from same state.collision. Physics velocities come
from same physics_history.record, not the output or animation record. Effective
animation transform delegates the existing same-root/late Processed8cc getter
twice, and Part transforms delegate the existing live attribute-selected getter
at each source site. Each Processed mutex guard ends within that method.

Root review correction: shoulder plane normal is actual Reckoning.up+1b0,
not Processed.prior_output.up220. Fresh raw5963c/44 forms the Processed bodyZ160
pointer;59658 separately loads Skeleton+3df4 Reckoning,5965c loads its+1b0 and
59660 stores that plane normal before59664 AngleBetweenVectorsOnPlane. The bodyZ
value is consumed inside the latter call, after the Reckoning normal capture.
Source now receives the same actual PhysicalPlayerAssemblyOwner and takes a short
Reckoning lock only in reckoning_up; no outer guard spans this callback. Missing
actual Reckoning and a poisoned lock remain distinct late errors.

The shoulder NormalizeSafe59638 uses the shared normalizer with explicit
824b1988/1998 final-negated fused residual refinement, not the legacy drive policy.
Source596d8 samples the Collision output latch before596e4 reads its compliant
byte;596e8 then checks the sampled latch. The extracted publication helper preserves
that exact sequence. Two additional suffix tests discriminate sample-before-read
versus check-after-read and negative-zero VNMSUB residual. All remain unrun.

IK is resolved only at its mandatory native call, using the existing mutable
current_animation.ik_runtime. Its geometry, own attributes, part_to_deck matrices
and foot histories are never replaced. Services borrow physics_history disjointly:
deck P/J/K/I and feet positions/velocities are fetched as directed by the existing
kernel. Processed93c is loaded only at its source timestep request. Hand margin
uses captured global physics_skeleton.default, generated70 layout20; it is
distinct from IK's own keyed vector77dd00c18946c6cf. Live cvar129 and each enabled
overlay requirement execute at the existing kernel sites.

Audio operates on the actual mutable state.collision with same-Vault global
physics_collision.default. Each requested delta/position/mass uses
physics_history.record.velocity_deltas/positions and physics_history.part_masses.
These are the existing State.PhysRecord projections (native relative f10/c10),
not post-record errors, current animation, or body angular velocity. The existing
audio kernel captures its one output borrow only after all four attribute pointer
reads and the fourth float load/guard order. Live cvar130 and enabled overlay
requirements remain late. Native byte fields currently enter bool output fields
through !=0, the existing explicit noncanonical-byte limitation.

Foot compression reads the same IK contact_offsets at index0 then1, does the
existing source FSEL selection, then the plain global Skeleton threshold lookup
2f1f013c99079afb (index0/default4). Only the selected branch acquires the fresh
generated70 layout graph X50/Y60. Nothing preloads that graph. WantRespawn reads
same General flag; source59844 first stores output request and59848 then clears
the flag. The narrow proposed clear accessor has no additional reset semantics.

## Prerequisites and completed-frame corpus placement

The new adapter consumes real completed postphysics output. The actual
UpdateSpecificBodyParts in Collision.Update must run; it establishes both current
and recent specific-contact bytes, including the face value read late by audio.
Constructor and Reset do not establish that producer. Do not seed bytes to avoid
the missing producer. `skeleton-postphysics-live-caller.md` supplies the complete
selected preparation/history/collision/volume/post-record sequence and its actual
solved-frame checks. That sequence must precede this Fill occurrence.

Use a dedicated completed-frame invocation of the existing original collision
fixture, replacing its controlled postphysics calls with the live sequence while
preserving the old controlled fixture invocation. Retain the same pair, Actor,
Processed, PhysOut, Vault, cvars, overlay and actual solved-frame identity.
Then execute the existing actual Update5 opening/Board.Fill producer chain and
its prefix/parts tokens, followed by this suffix. Do not construct a fake token,
unlock output directly, or append the postphysics chain to the existing same-frame
controlled run (one-attempt guards correctly reject replay). If an earlier real
Update5 collaborator remains unavailable, report that boundary rather than
calling this a full frame.

The original test should check:

- Physics history/collision completion identities still equal that solved frame;
  the same output and source Arcs remain installed.
- Prefix-captured COM velocity is the value forwarded through both tokens;
  output COMvelocity has the same bits. Suffix performs no new COM read.
- All three mandatory child kernels return; inspect original-produced limb,
  foot interaction and audio outputs for expected source-derived relations.
  Do not use a named-trick or bind-pose replacement to make them available.
- IK last-foot positions and Collision accumulators mutate on their existing
  owners. For actual want_respawn=true, guarded output request precedes clear.
  A false initial flag must remain false without inventing the true branch.

The live method stops after Skeleton.Fill returns. HiLOD.Update5's subsequent
collaborators and final output locking are separate caller work. No completed
Update5/frame or rendered skating claim follows merely from this adapter.

Root additional review: raw HandGrab a270..a4e8, Feet3bc0..3f60 and Fill56590..56618 match the draft's guards, same histories, strict ordered comparisons, full-W transpose and final-negated reciprocal graph. Independent graph_execution review confirms remaining debug gate/four overlay calls. Global pointer capture precedes infallible private hand translations, validation follows; draft comments corrected. Root read SMALL_FLOAT_132 at82261ed0 as37800000 and checked the complete plane-angle dot/projection/gate source. Audio's four attribute captures/loads, f154 guard before fourth value, and final four-plus24 overlay queries checked against raw. Remaining host numeric/global lifetime limits still apply.

## Root acceptance checkpoint58

Latest accepted connected Skeleton output checkpoint: **workspace58 passes 4,085 unit tests and six doc tests; original50 passes all 17 core corpus tests (9.07s); Clippy94, format43 and diff44 pass.** A fresh real Actor-backed pair now executes actual ProcessData, original-terrain solve/feedback, complete Skeleton post-physics and full Skeleton.Fill through mandatory IK/audio collision-data publication. COM velocity is captured once at585b0 and carried through the same output tokens. Ground.UpdateSkateboard also reaches its a998 return through retained constructor hang state and complete pinning. Active hang geometry/pulse services remain explicit pending branches. This is a component schedule with controlled solver iteration/freezing settings; full Board.PostPhysics, outer Ground/controller/force scheduling, output conditioners and animated trick consumption remain pending.
Root independent raw review confirmed same-owner bindings, all mandatory IK/audio source calls, COM capture, guard ordering, actual controller non-held predicate and Hang/Pin defaults. Narrow records reciprocal uses final-negated VNMSUB at541a0/a8; generic drive policy remains unchanged. Original50 includes successful freshActor solve/fullFill and all prior fixtures.
