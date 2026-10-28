# Native channel dictionary order for ordinary expiry

Fresh raw source review 2026-09-20. Confidence high within the actual selected
InitChannelAnimatable(maxChannels=6), default load1/growth2 domain. New draft
`actor_animation_channel_dictionary.rs`; standalone rustfmt passed, no Cargo.

## Reuse inventory

Repository search found no generic dynamic source-backed EASTL hash container.
`actor_graph_intents.rs` already implements the same six-word wrapping hash,
full-name equality, bucket-head prepend and ascending-bucket iteration, but its
source constructor fixes 61 buckets and load10000 (64-node allocation). Its
policy cannot be reused for this dictionary. World pointer-culling buckets
have unrelated hashing/ownership. The new helper is narrowly channel-specific;
it reuses FastString36 and does not add another string encoding implementation.

## Exact source policy

InitChannelAnimatable82d7cc68 constructs dictionary bucketcount1 pointing to the
empty array, elementcount0, maxLoadFactor1.0, growthFactor2.0, nextResize0.
No reserve/rehash is performed for maxChannels at dictionary construction.

GetRehashRequired82bd2438 raw:

1. Wrapping u32 total=elementCount+elementAdd (2448), compare against nextResize
   (2458/245c), return false immediately when total<=nextResize.
2. Treat bucketcount1 as0 (2460..2468). Convert unsigned effective bucketcount
   and total through fcfid then frsp (2488..2494). fdivs total/maxLoad (2498).
3. 24a0409900a8 is !GT(minBuckets,effectiveBuckets), including unordered. In
   this branch nextResize=integer(ceil(effectiveBuckets*load)), no rehash.
4. Otherwise multiply growth*effectiveBuckets (24ac), subtract from minimum
   (24b8), FSEL chooses max (24bc), truncate via fctidz and take low word
   (24c0..24c8). Binary lower_bound in the actual 256-entry prime table starts
   at822852f0 (24b4); 24dc/24e0 uses unsigned >=.
5. Return selected prime and publish nextResize=integer(ceil(prime*load))
   (2518..253c). Policy updates BEFORE node allocation/rehash in insertion.

Selected <=6-entry domain makes all these conversions/arithmetic exact integer
operations: no wrapping total, fractional value, NaN or overflow can occur;
load/growth are source constructor values, not mutable tuning. Prime table
prefix read fresh is [2,3,5,7,11,13,17,19,23,29,31,37]. Reachable counts are
initial1, then2 on first insert,5 on third,11 on sixth. Deletions retain both
bucketcount and nextResize, including after the dictionary becomes empty.
The helper implements this bounded mathematical specialization, not a generic
replacement for arbitrary EASTL policies. It explicitly refuses a seventh
unique entry; actual six-channel owners cannot reach it after free-slot checks.
This refusal is a host scope guard, not the native post-insertion count assert.

## Hash, insertion, rehash, erase, iteration

Insert82d7c198 rawc1b8..c204 sums all six FastString36 encoded u32 words with
wrapping adds, uses unsigned divwu/mullw/subf for modulo; find824d03e0 uses
the same hash. Equality is the shared full-six-word DoFindNode824cf258.
A hash collision alone is not a match.

New unique insertion runs GetRehashRequired, allocates the node, optionally
rehashes, then prepends to the selected bucket and increments size. A duplicate
does not rehash or move. `try_insert` includes the channel pointer assignment
after native dictionary operator[]; it therefore replaces a duplicate value
in place, returning the previous value, without moving it. NewChannel already
rejects duplicate names before this point.

DoRehash82d7b520 rawb588..b61c visits old buckets ascending. For each, pop old
head (b5dc), calculate its new bucket, set node.next to NEW bucket head (b5f4),
publish new head (b5f8), repeat old bucket. Thus collisions reverse on rehash;
neither globally sorting names nor just reverse insertion order is correct.
Host node storage indexes are private and do not stand in for native addresses.

Erase82d71d40 computes the next iterator, then unlinks only the current node,
deletes it and decrements elementcount. It does not shrink or reset policy.
Helper remove returns the retained channel index; a missing key is a no-op.
RemoveChannel must call it at the existing source erase point AFTER free-list
and tree publications and BEFORE parent/base/root relinks.

UpdateTransitions82d74818 and RemoveExpiredChannels82d7e678 iterate ascending
native buckets, following each head chain. Expiry saves the next iterator
before removal; callers can snapshot ordered names once (no insertion occurs
during this pass), then inspect each channel at its source turn. Do not snapshot
all expiration booleans ahead of execution or reorder resulting removals.

## Integration contract

Root registers `actor_animation_channel_dictionary.rs` as
`actor_animation_construction::channel_runtime::dictionary`. Change the one
InitializedAnimationChannels dictionary field from BTreeMap to ChannelDictionary,
initialize with new(), remove the now-unused BTreeMap import if no other use.
At the existing insertion point use try_insert(name,index), mapping Capacity
to GestureChannelError::ChannelCapacity. Keep native caller count assertion.

Lookup APIs remain get/contains_key/Index; len/is_empty/keys/values/iter are
provided. iter returns (&FastString36,&usize) in NATIVE order. remove(&name)
returns Option<usize>, matching the requested removal producer contract.
There is no clear/reset helper: preserve existing scoped owner lifecycle and
recover native Reset separately when required rather than constructing a fresh
dictionary mid-session and losing growth history.

Four authored production-helper tests cover both growth reversals with six
equal-hash distinct names; head/interior/tail removals and retained grown-empty
policy; wraparound hash/full-key equality and ascending buckets; duplicate
assignment at capacity plus explicit unsupported seventh insertion. Expected
six-collision traversal is [6,2,1,3,4,5], not [6,5,4,3,2,1]. Tests await root.
Host-success fixed storage avoids introducing another fallible allocator;
native heap-address/allocation-failure emulation is not claimed.
