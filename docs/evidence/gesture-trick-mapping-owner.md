# Actual gesture mapping member

2026-09-12. NEW unregistered files gesture_trick_mapping.rs,
gesture_trick_mapping_records.rs, gesture_trick_mapping_debug.rs and tests.
No Cargo. The earlier gesture callback draft now reuses this same TrickIntents
type rather than maintaining a parallel pair type.

## Records and construction

Fresh Init82ccca08 contains270 calls to XCore_EASTL_Category_Alloc(size0x30,
Anim category, tTrickIntents, flags1), each followed by nullable pair
construction and unique insertion82cc0648. The derived table contains all270
key/ordinary/mirrored triples in original order, with the native insertion
call address beside each row. This is behavioural data, not original source
or an executable/disassembly copy.

All270 names/pairs were independently cross-checked within this work against
raw constant arguments at the actual FastString and tTrickIntents calls;
all270 target-map offsets were separately checked against raw register
constant dataflow, preserving nonvolatile owner r26. No mismatches. The
decompiler's implicit `(map*)this` group is Nose, confirmed raw offsetzero;
it must not be inferred as the preceding Tail group. Parent independent
review remains required.

Init group order/counts: Square30, Tail30, Nose30,90Tail45,N90Tail45,
90Nose45,N90Nose45. Four final held mappings in order are Kickflip->
KickflipHold, Heelflip->HeelflipHold, N_Kickflip->N_KickflipHold,
N_Heelflip->N_HeelflipHold. Group-specific fallback pairs are intentionally
different: e.g Tail N_Kickflip maps to Ollie, Nose Kickflip maps to Nollie.
No generic prefix stripping or generated trick-name guess is used.

Ctor82cd4bd8 constructs seven inline maps in PDB member order:
Nose+0,Tail+764,90Nose+ec8,90Tail+162c,N90Nose+1d90,N90Tail+24f4,
Square+2c58; held+33bc follows. Each map's ctor82cd4720 asks native
GetPrevBucketCountOnly82bd2278 for46. Raw constant prime table822852f0
contains41,43,47 around this bound, so actual bucket count is43.
Held ctor82cd47e8 asks5 and receives5. Max load factor10000 prevents any
rehash during these45/4-entry domains. The draft uses fixed inline arrays.

Allocation callback errors retain prior inserts in the same incomplete owner;
construction cannot restart. Actual NULL allocation is different: a null pair
is inserted and construction continues, just like native Init. The normal
consumer must not replace a missing pair with a guessed trick. No fake
SkaterManager publication or whole-manager ctor claim is attached to this
selected mapping member.

## Native lookup order

Fresh DoInsertValue82cc0648 hashes the six packed words by wrapping addition,
unsigned remainder by bucket count, compares all six words in the chain, and
keeps an existing duplicate in place. A new node points to the old head and
becomes the head (raw0734..0748); count increments afterwards0750..075c.
GetGestureIntents82cc11b0 selects the actual group then iterates ascending
buckets, head-to-next. The first mapping key present in the real Action map
wins; values including0 and NaN are still present. It does not iterate Action
order. Concrete collision example: InwardHeelflip,360PopShuvit and
N_InwardHeelflip all hash to bucket0, whose head is N_InwardHeelflip.
Ollie hashes to11 and is therefore later, despite first Init insertion.

Existing ActorGraphIntents uses the same proven name packing/hash algorithm,
but hardcodes61 buckets/64 nodes and float payloads. Reusing its storage would
be wrong. The new private fixed-map implementation preserves this template's
43/45 and5/4 parameters and pointer-valued pair ownership; it reuses the
existing ActorGraphIntents::native_name and actual Action get(name) lookup.
There is no competing general-purpose map framework or shared refactor.

GetGestureIntents output is optional, matching native null output used by
conditions. A selected NULL pair with no output can still return true if debug
is disabled; with output it fails exactly at the actual dereference. The
output pair is copied BEFORE the live debug read. A miss reads no debug cvar.
GetHeldTrickIntent82cc1688 writes output only on an exact held-map hit.

## Actual late debug path and reserved cvar

Reserved cvar index124 (no shared edit): kGestureMappingDebug initializer
82f815a8/table82fd493c/ref830a30d4, section Animation_Skater, label
kGestureMappingDebug, title Gesture Mapping Debug, defaultfalse/nullcallback.
Suggested getter try_gesture_mapping_debug. Parent owns registration/window.
The LiveGestureMappingDebug switch closure must invoke this real getter each
time enabled() is reached, not capture an earlier value.

Raw1474/1478 samples the reference AFTER any requested output copy. If enabled,
the source prepares uppercase strings on discarded stack storage, requiring
the selected pair even with null output. It then requires the actual shared
DebugOverlay holder15e4..1608. Raw165c's target is825187c0 word4e800020,
so display is a proven BLR; no renderer substitute is introduced. String
preparation has no externally observable result on these valid authored names.

Global yPos830207d0 is .data-initialized400. Source reads it162c AFTER the
overlay requirement, invokes BLR, writes wrapping+20 at1664, unsigned-compares
to500 at1668, and stores400 when greater. The new
GestureMappingDebugPosition is an explicit application-global Arc<Mutex<u32>>;
clones retain the same global. It is passed separately from Actor/mapping,
not constructed per query or per Actor. Tests verify missing overlay preserves
position, aliases observe the same updates, and500 advances to400.

## Integration API and remaining work

Root module registration: gesture_trick_mapping (owns records/debug/tests).
GestureTrickMapping::new then try_initialize(allocator) retain the actual
member and successful prefix; callback receives original allocation ordinal,
size48 and flags1. try_gesture_intents takes &ActorGraphIntents, original
GestureGroup, optional mutable pair, and late debug collaborator.
try_held_intent takes exact retained gesture name and mutable destination.

Mount via selected SkaterManager member lifecycle825fb468, returned by
GetGestureTrickMapping825f7fd8. The actual callback's captured mapping must be
the same owner, not a newly built query table. Actor callback adapter still
needs to connect its exact Action/Motion/Animation handles and this selected
manager-member getter; no fake global readiness should be inferred from a
plain local mapping value. Existing callback kernel explicitly exposes this
late mapping boundary.

Seven focused tests drafted: native priority over Action order, all group
counts and real fallbacks, output-before-debug failure/miss laziness, NULL
pair conditional dereference, allocation-prefix retention/no replay, exact
held lookup, and actual shared debug-position/overlay ordering. No tests run.

## Root integration review, 2026-09-12

Registered mapping, actual Actor callback adapter, lifecycle and Trick52..54
dispatch. A fresh independent Hex-Rays extraction of Init82ccca08 compared all
270 source insertion blocks with the Rust group/key/ordinary/mirrored records:
270 compared, zero mismatches. Raw constructor82cd4720 confirms hint46,
43-bucket selection, 45-node pool and max-load10000; raw DoInsert confirms
six-word sum/modulo, equality, head insertion and count increment. Display165c
targets native BLR825187c0; original global yPos is400.

Root also reviewed original Begin/Update/End and generated Trick Update828ed790;
independent agent raw review found no defects in the registered bounded path.
The current corpus captures one initialized mapping and shared live cvars/debug
holder for its selected Actor session. Full multi-Actor SkaterManager scheduling
is still pending. Clippy71 passes; runtime/corpus validation of this batch is
pending and supersedes the earlier unregistered status above.
