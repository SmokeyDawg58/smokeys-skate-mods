# Actual Motion character and AttachIntent consumers

New-only unregistered batch, 2026-09-12. No Cargo run or source data copied.

`actor_motion_attach_intent.rs` is a child of actor_motion_runtime_session.
It contains motion_runtime_is_pro_skater and motion_runtime_attach_intent.
The latter owns child motion_attach_intent.rs, with four focused tests in
motion_attach_intent_tests.rs. Register actor_graph_motion_attributes.rs as a
child of actor_graph_construction.rs to access the existing private vector;
this is a method on the actual ActorSkaterGraphState, not another attribute store.

## Character condition

IsProSkater Activation82caac80 resolves actual ISkaterAnim/cache/UCOM identity,
asserts nonnull, then at82caad50/ad58 calls virtual+30. Atad60 it compares the
returned FastString30 with its actual constructor payload. The adapter captures
ONE existing animation Arc through the existing PlayAnimationActor owner helper,
checks interface[2] and reads the SAME retained player.pro_skater. It does not
derive character from Actor fixture names or change animation state.

Root dispatch patch: Motion ConditionPayload::IsProSkater {skater} calls
actor.motion_runtime_is_pro_skater(context,skater). Expression86/condition142
is the first MikeCarroll branch; MarkGonzales expression91 follows in native
order. Those missing selection expressions are still a separate integration
task and must not be bypassed to select generic Kickflip.

## Lifecycle and exact Update order

Binding: AttachIntent CreateInstance82bb3da8, Begin825187c0,
Update82cc9f30, End825187c0. CreateInstance writes NULL to the result intrusive
pointer; it neither allocates nor reports failure. Begin and End are actual BLR.
Thus root lifecycle patches should admit this payload with no Instance token
and no allocation request; Begin/End perform no owner lookups.

Update resolves IAnimatable, IMotionGraph, ISkaterMotionGraph in that order,
including each source identity assertion. The ISkaterMotionGraph cached pointer
has its own immediate null assertion. Only after all three resolutions does
the combined required-interface assertion occur. The adapter retains separate
captured graph occurrences plus the animation Arc and releases Actor guards
before subsequent callback calls. No existing animation/controller callback is
invoked under a prefix mutex guard.

Raw82cca0f0/f8 obtains IMotionGraph's map via virtual+10; a100 uses behavior
intent+1c; a104 calls HasIntent. a110 skips all writes when absent. Present
zero, negative zero and NaN remain values, not absence.

The present branch a118 addresses attr+34 and captures the returned scalar.
a12c/a134 calls ISkaterMotionGraph virtual+10 AddMgAttribute. Only afterward
a138 loads behavior isSettable+48. If true, a148 sets sequence=-1, a150 sets
normalize=false, and a15c/a164 calls IAnimatable virtual+48 AddSettableAtt.
The runtime calls the already recovered try_add_gesture_attribute; no new
animation storage, tree writer or playback implementation is introduced.

AddMgAttribute825fe7a0 raw e7b0 captures f32 and e7c0..d0 loads all five packed
name words. e7ec calls push_back825fe708 on interface+a64 = actual graph+14dc.
There is no lookup/update-by-name or NaN rejection. Inline push increments end
and copies the name and f32; duplicate names remain separate ordered records.
The actual vector has20 inline entries. Capacity exhaustion enters native
DoInsertValue825fe510, whose disabled overflow allocator supplies NULL; the
host adapter returns checked Capacity at this unsafe overflow boundary instead
of inventing a successful growing allocation. Existing records remain intact.

Tests distinguish: no-intent required resolution without writes; MG capacity
failure before animation; NaN MG append followed by animation's existing NaN
failure with no rollback; non-settable negative-zero preservation. The actual
state append test separately checks duplicate names and NaN preservation.

Root runtime patch: admit AttachIntent in Enter allocation match as None,
Begin/End no-op; Update calls motion_runtime_attach_intent with the actual
payload intent,attribute,settable. The generated non-Default Update thunk still
requires exact selected-slot dispatch reconstruction; this batch does not
claim that all OnBoard callbacks28..32 or Trick callbacks are implemented.
