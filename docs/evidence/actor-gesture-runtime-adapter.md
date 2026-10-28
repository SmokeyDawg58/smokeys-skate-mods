# Same-owner gesture callback adapter

2026-09-12 NEW unregistered actor_action_gesture_runtime.rs and
generated_action_runtime_trick_dispatch.rs. No Cargo/shared registration.
These consume the earlier callback and mapping drafts rather than duplicate
their lifecycle or lookup algorithms.

## Captured real owners

Proposed Actor intent-bridge children: gesture (actor_action_gesture_trick.rs)
and gesture_runtime (actor_action_gesture_runtime.rs). The latter's
action_gesture_begin/update/end validate the real Action context and instance
issuance, including a genuinely null instance, then call the ordered kernel.
End/Update do not require a mapping service because native does not.

The adapter captures the actual Action graph occurrence, nullable Motion graph
occurrence, and Arc<Mutex<MountedActorAnimation>> at the original separate
interface resolutions. Existing Action/UCOM checks are reused. Motion null
is retained until the combined Begin nonnull assertion after animation has
resolved; foreign cached/UCOM identity errors remain at resolution. Update
and End require Motion at their kernel's original point.

Actual Action and Motion map acquisitions each verify the captured graph
occurrence and existing intents owner, then retain that graph-bound handle.
Every subsequent access requires the same captured occurrence. The maps are
never taken from their Actor, replaced, cloned into a query snapshot, or
reconstituted from a summary. Each primitive obtains a short prefix guard.
No prefix guard spans the lifecycle kernel or another Actor callback.
The mapping lookup holds its read guard over the concrete mapping scan and
its cvar/overlay side effects; none calls back into Actor.

SkaterAnim is captured once during Begin resolution. Its late mirrored read
uses that retained allocation directly, reads known flags mask40000000, and
preserves the unknown-flags error. It does not reacquire a replacement Actor
animation. DontMirrorTrick reads the actual captured Action map only when
the kernel's mirror short circuit reaches it.

## Required shared manager-member capability

GestureTrickRuntimeServices::try_capture_mapping_member takes the SAME
Arc<GestureTrickMapping> produced by the selected manager member constructor,
cloned live InputCvarReferences, shared DebugOverlaySlot and shared global
GestureMappingDebugPosition. It refuses an incomplete mapping and performs
no query or debug preflight. Construct/initialize the mapping in its retained
owner first; a failed initialize therefore preserves the partial owner.
This adapter does not recreate the mapping inside each Actor or callback.

Root's ActionRuntimeServices should retain an optional Arc to this capability.
Pass its borrowed Option into action_gesture_begin; Source.mapping() clones
it only at the original late manager getter, after both map acquisitions.
None reports MissingManagerMapping at that exact stage. Cvar124 getter
try_gesture_mapping_debug remains a required proposed shared registration;
the captured references share actual live targets, and the getter runs only
when the mapping lookup reaches its post-copy debug read.

The actual mapping member follows SkaterManager ctor call825fb468 and is
returned by GetGestureTrickMapping825f7fd8. This selected capability is not
proof of full SkaterManager constructor return, list membership, or local
player readiness. The parent integration must label that scope explicitly.

## Exact selected Trick update

Fresh raw828ed790 calls shared parent thunk828ed318 atd7ac. That thunk
calls OnBoard828ecfe0 and then updates packed slots19..25, all actual
CreateMGIntentFromAGIntent occurrences80..86. The outer thunk then reads
slot26 instance+d4, stores Context+8 atd7b8, reads behaviour+d0 and calls
StateBehaviourDirect_CreateTrickIntentFromGesture_Update atd7c0.

The new dispatcher admits only loaded leaves52/53/54, exact update address
828ed790 and exact active occurrences0..10,42..49,80..86, then131/132/133.
It reuses existing dispatch_callback for positions0..26 in source order.
Ground occurrences68..79 are not part of this selected state. Three focused
draft tests cover distinct leaf lists, captured mapping identity/no debug
preflight, and incomplete mapping rejection.

Root still needs the typed GeneratedActionInstance gesture variant with
native allocation0x40/16, and Begin/Update/End dispatch passing Option rather
than prematurely rejecting null. The callback kernel retains exact late
null-instance errors and partial mutations. First released-Trick selection
also needs original expression91/HasGestureIntent108(groupSquare), then
HoldPattern109/InParentStateForTime110 only if its native short circuit
reaches them. This adapter does not assert state selection or skip these
remaining predicates.
