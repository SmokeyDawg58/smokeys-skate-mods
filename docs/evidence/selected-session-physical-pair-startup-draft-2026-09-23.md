# Selected physical-pair startup extraction draft — 2026-09-23

Unregistered source: `crates/skate2-core/src/session/startup/selected_session_physical_pair_startup_draft_20260923.rs`.
Review patch: `artifacts/selected-session-physical-pair-startup-2026-09-23-draft.patch`.
The exact paths were absent before creation. No registered module was edited and
no Cargo command was run by this author. `rustfmt --edition 2024 --config
skip_children=true --check` on the draft and `git apply --check` on the patch
both passed. The root integrator must obtain an independent IDA/source review
before applying the patch or making an integration claim.

## Native source and ownership

IDA debug IDB `sk82_na_m.xex.i64` identifies `Sk8::Actor::Actor` at
`0x825fe850` and `Sk8::Physics::PhysicalPlayerHiLOD::PhysicalPlayerHiLOD`
at `0x82e03c50`. In the Actor constructor, the live animation's bind pose is
obtained at `0x825ff2b4`; the selected PlayersBackEnd hat is read at
`0x825ff2c4`; the Sim-category physical object is allocated at `0x825ff2d8`;
`HiLOD` is called at `0x825ff2f4`; only after its return is `mPhysical`
published at `0x825ff300`. The Actor fills `Init_IPhysical` earlier from its
actual hierarchy/physics data, World, query, flags and player type. This
justifies preserving the already selected world, player and Actor instances.

IDA disassembly of `HiLOD` gives the constructor order: `ProcessedPhysIn`
`0x82e03ef0`, `PhysOut` `0x82e03f20`, conditioners `0x82e03f58`, surface
physics `0x82e03f90`, `GrindAirAdjust` `0x82e03fc4`, Reckoning
`0x82e03ff8`, trajectory selectors `0x82e04030`/`0x82e04068`, AnimOut
`0x82e040f8`, Skateboard `0x82e0413c`, Skeleton `0x82e041c8`, state
changer `0x82e04204`, Skeleton Init `0x82e04228`, skateboard controller
`0x82e0437c`, physics-state construction beginning `0x82e0456c`, capsule
creation `0x82e04c60`, and initial state selection `0x82e04d10`. These
addresses establish source order and ownership, not a new host physics
implementation. The existing `try_complete_selected_constructor` retains
the reviewed Rust breakdown and final Actor publication.

## Bounded extraction

The selected corpus first replaces the controlled RAW rig input carrier with
`SelectedPlayerStartupOwners::try_physical_inputs`, which obtains constructor
hierarchy, physics parameters and evaluated bind pose from the *same* Actor
animation, and checks the Actor player index against the selected loader.
The draft exposes that exact operation. The caller then binds its existing
Action services and gesture mapping before allocation, as the current corpus
does. The draft's second function takes the caller's world, Actor and resulting
inputs, calls the existing `ActorConstruction::try_allocate_physical`, retains
and rebinds the in-progress factory against that same world, then calls the
existing `try_complete_selected_constructor`. On success it returns the
original board and skeleton handles, a clone of the factory's owner `Arc`,
and the unique retained construction state. That state stays alive through the
corpus's downstream camera/tick work. On bind or constructor failure it returns
the in-progress state alongside a typed error; on allocation failure the state
slot is `None` because `try_allocate_physical` does not hand it back. Existing
partial Actor/world writes remain and no retry or transaction is claimed.
The wrapper creates no other World, player, Actor, physics owner or scheduler.
Actor allocation and retained rebind check manager/Actor authority.

The review patch replaces only the selected repeated-frame constructor path.
Legacy component fixture branches still allocate through their own prior
path. The corpus asserts registered/cached Actor allocation identity and
independently compares the cached owner with the `Arc` returned from the
construction factory. It also checks initial Ground state, camera/roster
identity and the same subsequent three-frame or queued run. The `true`
capsule answer and both panic callbacks
stay visibly selected by the corpus; the draft accepts those as host-supplied
policy instead of making them defaults. The selected A_RobsPark spawn remains
the corpus's authored choice. Bouncy/human flags and hat type come from the
existing selected Actor/player input chain, with runtime lookup timing left
to that chain. No VLT class inventory or asset loader is copied.

Confidence: **high** for the native Actor-to-HiLOD ordering and the unchanged
Rust constructor composition; **medium** for treating the present mounted
host path as a full native startup, because player registration and whole
Simulation scheduling are still absent. No Xenon per-tick parity is inferred.

## Review and integration gate

An independent reviewer should compare the draft and patch against the IDA
addresses above, verify the input projection precedes Action service binding
and allocation, confirm board/skeleton/controller/state/tail order is still
owned by `try_complete_selected_constructor`, and inspect state ownership on
both success and each failure edge. The cached/factory owner check should be
reviewed independently of the registered/cached allocation check. The root
should then apply the patch,
run focused original-data selected-roster and scripted Kickflip coverage,
formatting, the workspace suite, and record the result. The author did not
compile this unregistered source.

This extraction alone does not bind a production app session. The remaining
boot gates include original spawn selection, host pad/platform and remap
ownership, real Action/gesture manager mapping placement, graph/roster and
player registration sequence, camera/presentation continuation, post-tick
character palette delivery, and map/skater GPU presentation. First-test
host choices such as capsule allocation, assertion and hat-enable answers
must stay explicit until their source owners are connected.
