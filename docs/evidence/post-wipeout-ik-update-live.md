# Mandatory SkeletonIK postphysics update

2026-09-13; confidence **high** for ordinary recovered semantics. New unregistered
draft; independent source review, Cargo and live corpus integration still required.

## Actual call and registrations

Skeleton.UpdatePostPhysics_PostWipeoutCheck82b578b8 ALWAYS calls
SkeletonIK.UpdatePostPhysics82b55398 at82b57bfc, after conditionally selected
foot/hand helpers and before HeadTracking. Teleport/offboard may skip those
earlier helpers but do not skip this call. Success here does not complete
Update4, HeadTracking, pose publication or a playable frame.

- Register physical_player_post_wipeout_ik_update.rs as child
  `post_wipeout_update` of skeleton_ik_state.rs.
- Register physical_player_post_wipeout_ik_update_runtime.rs as a child of
  the existing proposed `post_wipeout_foot_live` runtime module.
- Parent authorized the only prior-draft edits: foot core `two_bone` and
  `commit` are now pub(super), so this sibling reuses the independently
  reviewed postphysics solver and exact Part-before-record commits.

The runtime child reuses the SAME concrete Source, actual retained field split,
Part setter, typed PhysRecord getter/setter and live debug services supplied by
the foot draft. It allocates no alternate state, filters, rows or providers.
It borrows FootIkLiveInput for source reuse; the extra deck/global attributes
are not sampled by UpdatePostPhysics. The common solver reads actual rows and
its existing kDrawPostFeetIK; it does not consume foot angle/height attributes.

## Exact loop and mutation

First sample actual kDoNewPostPhysicsIK at553b4/b8. Its default is TRUE.
If enabled, visit entries0 and1 only. The deciding float is actual
`external_blends[entry]` at1e4+4*i: raw r27=470, lfs -28c(r27) at553f8.
It is neither on-deck blend1d4 nor target blend32c. Root raw-byte correction:55400=4099010c uses BO4/BI25 (!GT), skipping
nonpositive and unordered weights. NaN does not enter the loop. No clamping or weight multiplication occurs.

Require actual mapped parent470+4*i at55404/0c. Read retained
parent_to_world[entry].P at8f0+40*i first, then the current actual parent
PhysRecord translation at55444. Subtract full vectors and invoke the already
reviewed DoTwoBoneIk_PostPhysics at55450. Reuse its exact source semantics,
fixed limits0/180, one attempt, source debug timing and typed W propagation.

On successful solver result, sample primary mapping460+4*i and write actual
Part primary transform from retained part_to_world480+40*i at55478, then its
PhysRecord at55488. Write actual parent Part from retained parent_to_world
8c0+40*i at554a8, then parent PhysRecord554b8. If solver returnsfalse these
caller commits are skipped; solver prefix remains committed.

Regardless of solver true/false, the active entry samples actual
kDrawNewPostPhysicsIK at554bc/c0. Enabled checks DebugOverlay singleton atcc/d4.
With missing singleton native asserts/DebugBreak then checks again; the Rust
error stops at that first assertion boundary. With a valid singleton there
is no further drawing call in this function. Disabled or skipped-weight entries
do not sample this debug cvar. The next entry reads its own current weight.

After successful loop completion, or when enablefalse, clear all twelve flags
in three successive source groups:

1. external_target_normal_set[0..3], SkeletonIK320..323, stores55520..2c;
2. external_target_set[0..3],324..327, stores55530..3c;
3. local_target_set[0..3],328..32b, stores55540..4c.

Do not reset weights, modes, target positions or transforms. Errors before
this suffix retain every flag and earlier legitimate Part/record commits.
Exclusive Rust owner borrowing excludes per-store concurrent native observers;
the stores still preserve source group order in the implementation.

## New live controls

Reserved213/214 by root. Both category Physics_Skeleton_IK, callbacknull:

- 213 kDoNewPostPhysicsIK, title `Do New PP IK`, TRUE;
  initializer82f7dc18, constructor82f7dc44.
- 214 kDrawNewPostPhysicsIK, title `Draw New PP IK`, FALSE;
  initializer82f7dbc8, constructor82f7dbf4.

IkPostUpdateLiveDebug extends the existing live foot debug contract. Root must
bind these callbacks to those real cvars, not snapshots or injected booleans.
The inherited kDrawPostFeetIK reads remain inside the reused solver.

## Validation and remaining work

Three focused tests exercise disabled-but-mandatory twelve-flag consumption,
NaN external blend skipping the solver, a later positive blend reaching the
real mapping requirement while retaining flags on error, and exactly-two-entry
iteration regardless of weights2/3.
Existing foot math and commit code is reused rather than duplicated.
Standalone rustfmt passed; no Cargo or real-corpus execution by this author.
Root owns cvar registration, typed PhysRecord amendment, mounting actual
PostWipeout caller continuation and independent validation.

HandIK remains separately required when its actual source branch is selected;
HeadTracking and UpdatePoseBuffer remain mandatory following this function.
