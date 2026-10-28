# CreateAttribute1927 shared attribute lifecycle

Author draft2026-09-19, fresh IDA raw/decompile; not registered or Cargo-tested
by author. Two unique files: actor_motion_create_attribute.rs and nested
motion_create_attribute.rs, four inline focused kernel tests. Rustfmt passed.

## Source and implementation

Existing generated factory binding CreateAttribute: helper82cac480, object40,
vtable82301d40. InstanceCreate82bb3da8 is exactly li0/store0/BLR, so retain
null instance; no synthetic allocation or per-instance animation owner.
Begin82caba88 reads byte+30, Update82ca92d8 byte+31, End82cabaa0 byte+32.
False returns before context/interface access. True loads f32 from+34/+38/+3c
respectively then tailcalls common Create82ca1568. No cvars required.

Create resolves cached ISkaterMotionGraph, checks actual UCOM occurrence,
requires non-null; vtable+10 call1658 is AddMgAttribute825fe7a0. This APPENDS
name+1c/value to actual retained Motion attribute buffer; duplicate names are
not replaced and NaN is not rejected by this operation. Only afterward sample
mIsSettable+33 at165c. If false there is no animation interface lookup.
If true resolve cached IAnimatable and check registered UCOM identity. Both
absent is allowed (16d8 skips); disagreement is failure after Motion append.
When present,16e0 loads sequence=-1,16e8 normalized=false and16fc calls+48
Animatable::AddSettableAtt82d7f5f0. Existing try_add_gesture_attribute preserves
first matching name+sequence replacement, capacity, native non-NaN assertion,
and normalized/sequence semantics. An animation failure keeps earlier Motion
append; no rollback or pre-resolution of optional animation is introduced.

Adapter is intended child motion_runtime_session::create_attribute. Method
motion_runtime_create_attribute(context,&existing CreateAttribute,Phase) routes
all three phases with their original enabled flags. It uses actual Actor Motion
occurrence/state and optional mounted Animation occurrence/entity. Retain the
existing generated loader's constructor semantics: `always` sets all phases but
stores UPDATE+38 twice, leaving END+3c zero, and early-returns before `set`.
Neither this adapter nor lifecycle dispatcher should reinterpret authored XML.

## Lifetime and reset ownership

End is an attribute emission using configured end value, NOT removal. There is
no behavior-owned reset token or sequence allocation. Shared owners provide the
native lifetime: Actor phase1 ClearMgAttributes825f43a0 immediately precedes Motion
Update; existing try_clear_motion_attributes empties actual Motion state buffer.
Animation settable attributes use fixed sequence=-1 and replace matching entries
within the pending batch. Existing try_animation_phase2 applies that batch to
the actual current tree, then resets settable_attributes.len=0 after successful
application. Do not bypass this with a permanent custom dictionary or duplicate
append-only animation attributes; do not clear either buffer from this behavior.

Read-only caller search during authoring found try_clear_motion_attributes calls
in actor_action_runtime_corpus.rs and the method implementation, but no additional
live phase caller. Root's native tick integration must preserve that phase1 call
before each subsequent Motion update; this behavior draft does not claim the
whole recurring tick schedule is mounted. Existing phase2 performs animation
batch consumption/reset; root must continue invoking it on the mounted owner.

## Validation boundary

Four tests cover inactive phase no interface access; Begin/Update/End emissions
including end negativezero; optional missing animation and late mismatch preserving
Motion append; unset avoiding animation lookup and preserving Motion NaN bits.
Existing actor_graph_motion_attributes tests cover duplicate append/NaN and
actor_animation_gesture_writes tests cover actual name+sequence replacement and
settable NaN rejection. No duplicated owner implementation introduced.

Next runnable validation after root mounts: scoped CreateAttribute kernel tests,
existing actual owner tests, then original selected Actor neutral1170 lifecycle.
Independent source review assigned to source_review. Author performed no Cargo.
