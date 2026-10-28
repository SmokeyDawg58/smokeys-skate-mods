# Gesture-to-trick lifecycle draft

2026-09-12. NEW unregistered actor_action_gesture_trick.rs and its six focused
tests. No Cargo. The actual retained mapping owner is still missing, so this
is not live trick execution or a claim that a released Ollie reaches Motion.

## Native instance and scheduling

CreateInstance82cba2b8 requests0x40/alignment16, invokes Data82cba1d8 only
after successful allocation, and preserves NULL otherwise. Fresh PDB/raw
identify firstUpdate+8=true, found+9=false, gesture+c (six packed words) empty,
heldIntent+24 empty, isHeldTrick+3c=false. The new typed instance retains one
issuance token and these exact fields. Padding is not invented as an input.

Begin82cd4da8 resolves Action, Motion, SkaterAnim in order, including their
actual cached/UCOM identity assertions. The combined nonnull assertion occurs
only after all resolutions. Motion map4f94 is acquired before Action map4fac.
The local pair is zeroed before the required gpSkaterManager read/assert;
manager virtual+30 at5024 returns its retained GestureTrickMapping. Lookup
GetGestureIntents82cc11b0 executes at5038 even if an override was authored.

No match returns without touching the instance, including a NULL instance.
On success raw5050 stores found=true BEFORE override/mirror/map operations.
The override branch inserts the authored name at value+0, then copies the name
to instance+c. Otherwise the late SkaterAnim virtual+1c at50dc is
GetIsMirrored82cb7598, existing animation flags mask40000000. Only when true
does50f8 query Action DontMirrorTrick presence (actual global initializer
82f81be0 assigns literal DontMirrorTrick). Selected ordinary/mirrored name
is inserted with value+0 before its six-word copy to the instance.
GetHeldTrickIntent82cc1688 writes the held destination only on a match;
the returned boolean is stored at+3c by51a0. Failures preserve all prior
found/name/map/output writes. Begin does not reset firstUpdate.

Update82cd51b0 dereferences byte8 first, then byte9 only if not firstUpdate.
FirstUpdate or not-found goes straight to byte8=false, without resolving
Motion. Otherwise Motion is resolved/required EVEN IF isHeldTrick is false.
If held, one GetMotionGraphIntents supplies the map for erase(original), then
insert(held,+0). There is no one-shot clearing of isHeldTrick/found: subsequent
updates repeat this operation. Byte8=false is stored last after success.

End82cc1d58 dereferences found first. False returns without interface access.
True resolves/requires Motion. Held uses one fresh map call and erases held;
then ALL found instances make another fresh map call and erase original.
No instance flags or names are cleared. The source trait preserves these
distinct map calls rather than eagerly acquiring one map at callback entry.

## Existing owners and actual missing producer

ActorGraphIntents already owns the actual Action and Motion maps. Existing
Actor context/interface checks and retained SkaterAnim flags supply those
dependencies. A future concrete source adapter must capture their identities
at the calls above and retain them through each callback; it must not replace
maps or read a detached trick-intent summary. The draft source contract exposes
the distinct handles, resolutions, map acquisitions, and late mapping calls.
There is deliberately no fake implementation reporting no gesture match.

There is currently no Rust GestureTrickMapping/SkaterManager owner. Fresh
SkaterManager constructor825fb300 calls GestureTrickMapping ctor at825fb468;
GetGestureTrickMapping825f7fd8 simply returns that same member. Mapping ctor
82cd4bd8 constructs seven fixed maps (capacity45, requested buckets46) and one
held map (capacity4, requested buckets5), then calls Init82ccca08. The maps
are Nose, Tail,90Nose,90Tail,N90Nose,N90Tail,Square in construction order.
Actual bucket construction/prime policy must be recovered, not inferred from
the requested bucket counts. Init authors the real gesture keys, ordinary/
mirrored pairs, and held entries; it is substantially larger than the callback.

GetGestureIntents82cc11b0 selects a group map, iterates its actual bucket/chain
order, and finds the FIRST key present in the actual Action map. It does not
iterate Action insertion order and does not compare intent float values.
On success it copies the two packed names before sampling the live
kGestureMappingDebug cvar. The enabled debug branch requires the actual
DebugOverlay, invokes a display callback, and changes retained yPos. This
must be preserved at the mapping boundary; not preflighted on a miss.
GetHeldTrickIntent82cc1688 performs an exact-name lookup in the held map;
failure leaves its destination unchanged.

The next minimal live producer is a source-initialized retained mapping
member slice with original Init records and verified native iteration order,
plus its late debug collaborator. That can be mounted with an explicit
selected manager-member scope without claiming the entire manager ctor has
returned. It cannot use a hand-authored Ollie-only lookup or arbitrary Rust
HashMap order. No shared registration or manager implementation is in this
batch.

## Focused draft tests

Tests distinguish no-match/null-instance success, match/null failure at the
found store, override skipping mirror while still requiring lookup, found
publication surviving later insertion failure, first-update interface skip,
held swap and two End map acquisitions, and erase surviving a failed held
insertion. They use controlled source collaborators and do not stand in for
an original-data producer or successful actual graph scheduling.
