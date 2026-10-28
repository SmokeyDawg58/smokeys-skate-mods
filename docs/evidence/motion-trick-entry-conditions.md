# Motion Trick entry: actual retained conditions

New draft `actor_motion_runtime_trick_conditions.rs` is deliberately unregistered.
It adds no fields, publications, playback implementation or assumed state.

Fresh original raw review, 2026-09-12:

- Pre434 starts8291c770. Calls HasIntent139 at c794, AllowedToTrick140
  at c7ec, OkToDoTrickOnStairs141 at c844, Ground89 at c89c,
  optionally Grind13 at c8f4, then IsGrabbingObject138 at c948.
  Actual lazy expression remains `139 && 140 && 141 && (89 || 13) && !138`.
- AllowedToTrick82c9d0e8 validates the actual ISkaterMotionGraph cache/UCOM
  identity, asserts nonnull, then virtual+74 at d1b8/d1c0. Concrete
  AllowTrick825fdff0 is `lwz r11,ca4(r3); extrwi r3,r11,1,6; blr`.
  Interface subobject+a78 makes the destination graph+171c. Existing
  ActorSkaterGraphState.flags is that word; mask02000000 is consumed only
  when its known_mask establishes it. The native constructor establishes
  high byte03000000, already retained by actor_graph_construction.
- OkToDoTrickOnStairs82c9bfd0 validates the actual IPhysical cache/UCOM
  identity and asserts nonnull. At c0a0/c0a8 virtual+28 returns PhysOut;
  c0ac loads Animation pointer+38, c0b0 reads byte+a2; cntlzw/extrwi returns
  true exactly for zero. Existing Animation.should_disallow_tricks_due_to_stairs
  is this actual destination. There is no output write-latch sample.
- IsGrabbingObject82c9aef0 has the same physical resolution. afc0/afc8
  calls virtual+28; afcc loads OffBoard pointer+48, afd0 reads byte+38.
  Existing OffBoard.is_grabbing_object is this actual destination. Again,
  no output write-latch read. Both methods use cached_physical_allocation,
  not constructor-only listener authority or a detached output summary.

Each adapter checks the existing generated Motion context, then reads the
actual owner at the condition call. No combined physical snapshot or lock
across further graph evaluation is introduced. The existing condition cache
and parent write remain in generated_motion_runtime.rs.

## Registration and dispatch patch for root

In actor_motion_runtime_session.rs add the child path
`actor_motion_runtime_trick_conditions.rs`, module `trick_conditions`.
In generated_motion_runtime.rs condition dispatch:

- Shared `(ConditionClass::IsGrabbingObject, SharedPayload::Plain)` calls
  `actor.motion_runtime_is_grabbing_object(context)`.
- Motion `ConditionPayload::Plain(PlainCondition::AllowedToTrick)` calls
  `actor.motion_runtime_allowed_to_trick(context)`.
- Motion `ConditionPayload::Plain(PlainCondition::OkToDoTrickOnStairs)` calls
  `actor.motion_runtime_ok_to_trick_on_stairs(context)`.

IsGrabbingObject is SHARED: the loader first checks ConditionClass::from_name,
so adding only its otherwise existing Motion Plain variant would not close138.
No changes to expression434 are necessary. Two focused inline tests distinguish
unknown AllowTrick from a known false bit and verify the constructor's known
true bit plus a later clear. No Cargo run for this unregistered draft.

## Next audited dependencies, not implemented

After entry gate, TailTrick72 first examines MikeCarroll186 expression86.
Raw8290d1fc loads condition142, d200 calls IsProSkater; predicate's payload
is MikeCarroll. Activation82caac80 resolves actual ISkaterAnim, calls
virtual+30 at ad50/ad58, then compares FastString30 at ad60. Existing
Actor animation player.pro_skater retains the actual character name and
selected initialization writes; do not substitute fixture identity.

AttachIntent is the first OnBoard Enter occurrence17. Its existing binding
CreateInstance82bb3da8 returns a null instance; Begin/End825187c0 are native
BLR. Update82cc9f30 resolves IAnimatable, IMotionGraph, ISkaterMotionGraph
in that order, then asserts all three, obtains the actual Motion intent map,
and queries intent presence/value. Missing intent causes no destination write.
Present intent writes the actual MG attribute first, then optionally the
actual animation settable attribute. This requires existing attribute owners
and a narrow runtime dispatch, not a new PlayAnimation implementation.
OnBoard occurrences17..27 are AttachIntent; subsequent28..32 still require
their own source callbacks. Current runtime only dispatches Default's six
slots, so passing the three new conditions alone does not claim playable tricks.

## Subsequent lazy selection audit and pointer correction

The earlier Ground label for outer+38 was incorrect. Fresh PDB `Sk8::Physics::PhysOut`
has Animation+38, Filtered+40, Air+8, Ground+20. The draft getter now reads
Animation, and the old interpretation must not guide registration.

Actual authored `step_loaded` presently stops after Action.Update. The only
mounted Motion.Update corpus covers two Default6 ticks. A released Kickflip
requires actual subsequent Motion.Update, following ClearMgAttributes; passing
Action52 does not imply playback.

Following Pre434, fresh Pre cases are:86 (8290d1dc) condition142 MikeCarroll;
91 (8290d3bc) condition165 MarkGonzales;92 (8290d41c) condition167 Ollie;
87 (8290d23c) condition143 Kickflip. All consume actual condition payloads.
Expression446 at8291ddcc..8291e1fc is a lazy OR of IsGrinding144..155.
IsGrinding82caa310 resolves ISkaterAnim then IPhysical, reads actual
Filtered.is_grinding (outer+40, field+50); false skips the name read. True
performs a fresh GetPhysOut and compares Filtered.skating_grind_name (+8).
An earlier verbal claim that these were Grind output fields was wrong.
The existing real Physics conditioner publishes these exact Filtered fields.

Expression294 (82912f58..82913018) is LastState161 OR ManualOutTimer162;
expression90 (8290d35c) is LastState163. LastState82c94a00 resolves the
condition's actual parent, calls GetSubState on the authored path, then
tests StateHasDescendent against actual last state. It is not string equality.
ManualOutTimer82c9deb8 resolves actual SkaterMotionGraph and compares its
live timer strictly >0; unordered is false. The existing timer starts at0.
Keep generated INDICES transition indirection; never substitute direct indices.

For an actually non-Mike/non-Mark character, absent Ollie and real false
grind/manual predicates, the path is0/3/19/72/189/517/791/1078. Its29 slots
are0..3,17..32,67..69,455,1126..1128,2019..2020. Slot2019 reuses the existing
PlayAnimation implementation for B_KICKFLIP_IN_G. Other constructors/callbacks
remain necessary; current Update dispatch admits only Default's six slots.

Before this path, complete actual Update5 output producers and invoke the
existing Physics conditioner at its selected source boundary. Its first three
children consume System COM velocity, empirical deck position/DT, then actual
State.type/category, Collision and Ground.wall_ride_exit. Actual type100 with
false wall exit produces filtered Ground; no forced output store is needed.
This audit does not declare the intervening Update5 callbacks complete.
