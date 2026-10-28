# Retained channel removal and transition completion

Read-only source recovery, 2026-09-19. Confidence high for ordered removal and
completion; dictionary growth is identified below but not yet raw-reviewed to
an implementation verdict. No Rust changes or Cargo. New evidence path checked
absent before creation.

## Caller and expiration

ChannelAnimatable::AddTime82c8a558 calls virtual UpdateTransitions82d74818,
RemoveExpiredChannels82d7e678, then Animatable::AddTime82592b58. The channel
passes occur BEFORE this frame advances animation time. Existing body transition
pruning remains in its original Animatable position, after these channel passes.

Both channel passes return immediately if mRootChannel is NULL. They traverse
the native dictionary by ascending bucket and each bucket's linked chain.
UpdateTransitions examines each channel's +84 mIsTransitioning and only then
calls its +44 transition's virtual TransitionComplete; true calls CompleteTransition.
Do not fail merely because the flag is set, and do not complete early at equality.
For TransitionBlend, completion82d6e070 is ordered time>length; Sequence needs
its own retained type/method rather than treating every transition as a blend.

RemoveExpiredChannels saves/advances the dictionary iterator BEFORE examining
or deleting its current element. The channel's ChannelBlend flags are at+3c:

* If blending_out08000000: remove when !(blend_out_time < blend_out_length).
  Raw82d7e708=40980028 is BO4/BI24, includes unordered. Fields are+2c/+30.
* Otherwise remove only when cycle10000000 is clear AND expired04000000 set.

The existing channel_time predicate is correct, but its RemoveChannel error
must be replaced by the actual release. Fetch the root AFTER removals, not the
root captured before the pass. All comparisons use prior-frame clocks.

## RemoveChannel82d7d8a8

Name lookup via dictionary find824d03e0; missing name returns without mutation.
For a found retained channel index, preserve this sequence:

1. Append channel to free_channels (d914); THEN assert size<=max_channels
   (d94c). Source publication survives assertion/failure.
2. Clear the channel_tree pool (d968, PoolStack::Clear82c8ef98). Append that
   tree index to free_trees (d994), THEN assert size<=2*max_channels (d9c4).
3. If transitioning is true, Clear transition_tree (d9f0), append it to
   free_trees (da1c), THEN assert the same capacity (da4c). Tree order matters:
   transition tree is appended last and is the next allocation's LIFO choice.
4. Erase dictionary entry (da7c, erase82d71d40). No erase-before-release reorder.
5. Read channel base(+8c) and parent(+88), relink:
   - parent+base: parent.base=base, base.parent=parent, parent runtime.base
     references that base channel's ChannelBlend node (dadc/dae0/dae4).
   - parent, no base: parent.base=None (dab8), THEN call actual GetBaseAnim
     virtual and write parent runtime.base to the retained body reference or
     true Empty if NULL (dad0). Do not preserve removed node's stale base.
   - no parent, base: base.parent=None (daa4), root=base (daa8).
   - neither: root=None (da9c).
6. Null channel_tree and transition_tree (dae8/daec); transitioning=false
   (daf0); parent/base=None (daf4/daf8); priority=0 (dafc).

NO source zeroing of ChannelBlend node payload, runtime clocks, coefficient,
flags, evaluated, mTransition pointer, Sequence or TransitionBlend payloads.
Keep these stale payloads until the next source Init overwrites them; do not
replace the whole channel with a fresh default. Runtime must be unreachable
through dictionary/root after removal. Clear the actual tree pool using its
existing runtime.clear() (allocation-order destruction and cursor reset), not
drop/reallocate the OwnedAnimationPool, nor clear unrelated body trees.

Update ActorChannelAnimatable.root_channel pointer mirror when root changes;
the native equivalent is the single actual root pointer.

## CompleteTransition82d7d7d8

1. Read channel transition(+44)->mToAnim(+c), publish channel ChannelBlend's
   mChannelAnim at+18 FIRST (d7f4..d800).
2. Clear old channel_tree (d804), append it to free_trees (d830), THEN assert
   count<=2*max_channels (d860).
3. Load transition_tree(+4), set transitioning=false (d884), replace
   channel_tree with transition_tree (d888).

Do NOT null transition_tree or mTransition afterward: source leaves both.
Do NOT free the transition destination tree; it becomes the active tree.
Channel links, dictionary membership and free_channels do not change.

## Current owner representation and minimal implementation

InitializedAnimationChannels in actor_animation_initialization.rs retains
channels, dictionary:BTreeMap<FastString36,usize>, free_channels, trees and
free_trees; six channels/twelve trees are source-constructed. Each channel
already retains channel_tree, transition_tree, transitioning, parent/base,
priority and ChannelBlendRuntime. The ordinary NewChannel producer initializes
transitioning=false. Body TransitionBlend storage is DISTINCT from a channel's
inline Sequence/TransitionBlend. Do not use a body-store slot index for +44.

Recommended methods on ActorAnimationConstruction<K,W>:

```rust
pub(crate) fn try_remove_channel(&mut self, name: &FastString36)
    -> Result<(), GestureChannelError>;
pub(crate) fn try_remove_expired_channels(&mut self)
    -> Result<(), GestureChannelError>;
pub(crate) fn try_update_channel_transitions(&mut self)
    -> Result<(), GestureChannelError>;
fn try_complete_channel_transition(&mut self, index: usize)
    -> Result<(), GestureChannelError>;
```

Use retained owner handles, source-point validations and existing clear().
Do not preflight all trees/links/capacity before the first source publication,
and do not roll back completed releases after a later typed error.

Smallest active batch is ordinary channel removal + exact dictionary traversal.
Channel transitions additionally require a typed channel transition selector
(Sequence vs inline TransitionBlend) with TO animation handle/pool provenance.
Current channel.transition is only AnimationPointerState and the channel's
runtime.animation is a tree handle, so CompleteTransition cannot infer its TO
from pointer truthiness. Recover/mount the real StartTransition82d7ee68 producer
when that path is required; retain explicit missing-service errors meanwhile.

## Dictionary order is a concrete prerequisite

BTreeMap iteration is NOT native hash iteration. Multiple expirations append
released slots in visit order; later channel/tree allocations pop from those
lists. This is observable even if the final set of channels is identical.

Fresh InitChannelAnimatable82d7cc68 decompile establishes dictionary bucket
count1 using gpEmptyBucketArray, elementcount0, maxLoad1.0, growth2.0,
nextResize0. Insert82d7c198 and find824d03e0 use wrapping SUM of all six encoded
FastString36 words, modulo bucket count. Unique insertion is bucket-head prepend.
Rehash82d7b520 traverses old buckets ascending, pops each old head, and prepends
it into the new hash bucket; chains can reverse. Do not derive collision order
by sorting names or sorting insertion serials without retaining rehash history.

GetRehashRequired82bd2438 is the ordinary EASTL policy (not plasma8244e990).
Its exact numeric/prime table path needs a small raw review before authoring
the dictionary. No fixed bucket count is asserted here. A source-ordered side
structure alongside the lookup map is sufficient; a broader generic container
rewrite is unnecessary. Keep the six-word hash and chain semantics local or
reuse existing intent-map hash helpers only after verifying compatible policy.

Validation should cover missing name no-op; root/middle/bottom/single removal;
free-list append order and immediate reuse; two simultaneous expirations in
different/same buckets; equality and NaN expiry; partial capacity failure;
transition completion preserving destination and stale transition fields.
