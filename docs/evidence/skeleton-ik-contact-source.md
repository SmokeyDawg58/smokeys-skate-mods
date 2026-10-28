# Skeleton IK contact update and direct helpers

2026-09-08, read-only source recovery. Raw original IDA bytes,
VMX128 operand extensions, scalar BO/BI fields, and cvar initializers were
inspected. No Rust changes, Cargo run, runtime trace, or completed world line
query implementation is claimed. Independent review remains a separate gate.

## Source identities

Spans are half-open and SHA256 covers the actual original bytes, including
the final padding word in ForceOffsetForPenetration's assigned span.

| Function | Span | SHA256 |
|---|---|---|
| UpdateContact | 82b55560..82b55bec | 8b738e8c3ba3e27f130b56814114c33a5588ac87fd79acfec9c9892674914f75 |
| AddInContactOffset | 82b49538..82b49698 | 41def365cdc7e27b3ef50d2ad486421ea909cd04dcbc0569004aba7ad9042d2c |
| ForceOffsetForPenetration | 82b49698..82b49760 | a96b14f93b6a00b1ff9988a141b0b939a710a51360fb5866b9fb78ed80611399 |
| GetDistToContact | 82b53220..82b53398 | 748cd539f0f45790cb342594269eb111ce7409495af71b8463bf5de95da84d8a |
| ContactLineTest | 82b49760..82b4994c | 94b2b2f4e4c6ce5f67477e72d9254903bf292be49064f10ef855286406ec0ecd |

All are `Sk8::Physics::SkeletonIK` members. Decorated suffixes are respectively
`?UpdateContact@SkeletonIK@Physics@Sk8@@AAAXXZ`,
`?AddInContactOffset@SkeletonIK@Physics@Sk8@@AAAXH@Z`,
`?ForceOffsetForPenetration@SkeletonIK@Physics@Sk8@@AAAXHM@Z`,
`?GetDistToContact@SkeletonIK@Physics@Sk8@@AAAMH@Z`, and
`?ContactLineTest@SkeletonIK@Physics@Sk8@@AAAXH@Z`.

The top Update caller samples enable-contact at 82b59c94/98, after actual
InitialTransforms and BlendTransforms; when enabled it calls UpdateContact at
59ca8, then DriveFrames at 59cb0. The enable gate remains the caller's duty.
This helper has no own DoIK/enable early return.

## Same-owner inputs and distinct index namespaces

Offsets below are relative to the actual retained IK object. DataIn starts at
+20; processed data is the retained pointer at +14.

| Member | Offset | Existing authority |
|---|---|---|
| processed EffectiveTransform | processed+80 | actual processed matrix |
| processed flags | processed+8c4 | source flags word, reads described below |
| processed ContactBoneID | processed+910 | signed hierarchy bone ID |
| inverse skateboard matrix | +e0 = DataIn+c0 | retained typed DataIn matrix |
| skateboard matrix | +160 = DataIn+140 | retained typed DataIn matrix |
| most vertical toe contact normal | +1a0 = DataIn+180 | retained actual collision-derived vector |
| actual island capability | +1c8 = DataIn+1a8 | same owner's retained island identity |
| mode[i] | +1f4+4*i | SkeletonIkState |
| actual bone ID[i] | +204+4*i | SkeletonIkTargets.bone_ids(), hierarchy namespace |
| target Part index[i] | +460+4*i | SkeletonIkTargets.parts(), distinct namespace |
| primary world matrix/P | +480/+4b0+64*i | SkeletonIkState.part_to_world |
| parent world matrix/P | +8c0/+8f0+64*i | SkeletonIkState.parent_to_world |
| target/current contact offset | +bc0/+bd0+4*i | existing contact scalar histories |
| contact position/state | +be0+16*i / +c20+4*i | existing contact histories |
| SkateboardDontPush | +c70 | existing retained flag |
| TellAnimDontPushOrBrake | +c71 | existing retained flag |

The matching gate uses **hierarchy bone IDs**, not Part ordinals and not
reparented target bone IDs. Raw Init82b5ebc0..ebdc reads SkeletonData+12ac,
+12bc,+127c,+128c and stores +204,+208,+20c,+210. These are data bone-ID table
entries for Parts15/19/3/7. Original retained hierarchy IDs are24/20/16/12;
the Part IDs at +460 are15/19/3/7. Root's existing `bone_ids()` accessor is the
appropriate authority. Reusing an integer from the wrong namespace can pass
an identity-mapped synthetic fixture and still fail the original hierarchy.

Only entries0 then1 execute. Loop index/offset pointers advance at55bc4..d8;
hands2/3 and their contact histories are retained. Other IK matrices, modes,
blends, PartPos, original target transforms, mapping tables, geometry, previous
animation, and actual mapped animation volumes are not written here.

## Common arithmetic and constants

All scalar intermediate arithmetic is f32. No timestep multiplies these
per-call slew steps. Define the exact native fsel graph:

```text
select(s, positive, negative) = positive if ordered s >= 0 else negative
bound(v,L,U):
    a = select(L-v, L, v)
    return select(U-a, a, U)
slew(current, target):
    difference = target-current
    step = bound(difference, -step_limit, +step_limit)
    return step+current
```

The shown subtracts and additions remain separately rounded. `bound(NaN,L,U)`
selects U for finite endpoints; replacing this graph with host min/max or
clamp changes unordered behavior. Signed zero selectors take the positive arm.
For the source's target-zero decay branch the difference is **fneg current**,
not a rewritten `0-current`; preserve that distinction for zero/NaN bits.

| Purpose | Actual literal address | f32 bits |
|---|---|---|
| target lower bound -0.1 | 820103ec | bdcccccd |
| target upper bound +0.1 | 82010114 | 3dcccccd |
| negative slew limit -0.015 | 82268af4 | bc75c28f |
| positive slew limit +0.015 | 820229c4 | 3c75c28f |
| valid contact height addition +0.0825 | 82268b80 | 3da8f5c3 |
| push distance test +0.03 | 8206978c | 3cf5c28f |
| push current-offset test -0.01 | 8202a104 | bc23d70a |
| foot surface adjustment +0.025 | 820103f8 | 3ccccccd |
| query start fraction +0.3 | 820103b8 | 3e99999a |
| positive zero | 8220a90c | 00000000 |

The diagnostic double at82268b88 is3f9eb851e0000000, the exact f64 widening of
the f32 distance threshold. It is only formatted debug text, not a separate
comparison threshold. Signed-low-address resolution matters: the negative
offset literal is **8202a104**, not8222a104.

For matrix M and point p, use the accepted full-four-lane affine graph:

```text
t = FMA(M.X, splat(p.x), M.P)
t = FMA(M.Y, splat(p.y), t)
t = FMA(M.Z, splat(p.z), t)
```

Its input W is not a coefficient. Direction transformation in the contact
helpers is a distinct graph: X multiply, then Y FMA, then Z FMA, with no M.P.
Do not drop zero-coefficient products or assume a canonical output W.

## UpdateContact: complete ordered branch graph

At55580..a8, read processed flags8c4 once and retain
`request = (flags & (0x02000000 | 0x40000000)) != 0` for BOTH feet. Diagnostic
strings refer to aborting a push; the literal mask is the source authority.
Store TellAnimDontPushOrBrake=false at555b4. **SkateboardDontPush is not cleared**
here. No attribute or cvar has been read yet.

For each i=0,1:

1. Load mode[i] at55654. If it is not exactly1, take the nonmatching route.
2. Only for mode1, read processed ContactBoneID and retained actual hierarchy
   bone_ids[i] at55668..74. Unequal values take the nonmatching route.
3. On a match, inspect raw unsigned contact_state[i] at5567c. State0 is waiting,
   state1 is valid, state2 is not valid, and values>=3 use the unknown route.
   Do not reject or reinterpret unnamed raw values before their actual branch.

The routes below all finish at the common per-foot debug stage unless a
collaborator refuses. `refresh` means store contact_state[i]=0 THEN call the
actual ContactLineTest for that entry. It does not overwrite contact_position
before the query. Native writes occur at55770,558fc,55a6c,55b08 depending on
the path; all have this same store-before-query ordering.

### Nonmatching mode or bone: 55b1c..55b50

1. Load current offset, then store contact_state=0 at55b20.
2. Store target_offset=+0 at55b28.
3. Compute difference by fneg current, apply the slew bounds and store the
   resulting current_offset at55b48.
4. Call AddInContactOffset at55b4c.
5. Proceed to common debug. **No line query runs on this route.**

An earlier mode mismatch does not read ContactBoneID. Existing contact_position
is retained. AddInContactOffset's own debug read can fail after all three stores.

### Waiting state0: 55a80..55b18

Clamp the OLD target_offset to[-0.1,+0.1], store it at55aa0; slew current toward
that target and store at55abc; call AddInContactOffset at55ac0. Then read the
contact-debug cvar at55ac4/c8. If true, acquire the debug singleton and execute
the folded no-op "Waiting For Contact" draw. Finally refresh. Waiting retains
the old target's direction; it does not set target0 or immediately chase a new
query result in this call.

### Not-valid state2: 55694..55780

Clamp OLD target and store at556b4; slew current and store at556d0; call
AddInContactOffset at556d4. If captured request is true, store TellAnim=true
then SkateboardDontPush=true at556e4/e8. Read contact-debug at556ec/f0. If true,
acquire singleton for "No Contact"; if request is true, acquire singleton again
for the separate "Aborting Push..." draw. There is no second cvar read between
those two singleton calls. Refresh after all this work.

### Unnamed state>=3: 55770

Refresh directly. There is no offset clamp/slew, AddInContactOffset, push-flag
write, or branch-specific debug read before this refresh. The common debug
stage still executes afterward.

### Valid state1: 55784..55a7c

Transform the retained contact_position through the retained inverse
skateboard matrix; `height = transformed.y + f32(0.0825)`, separately rounded.
The contact-debug value is loaded at557d4, before the final two matrix FMAs
and height addition, then tested at557fc. If true, acquire singleton and execute
the folded "Valid Contact" draw. Only afterward read the max-contact-offset
cvar at55844/48. That cvar is not an eager per-frame tuning input.

If ordered `height > max_contact_offset`:

1. Clamp OLD target and store at55874, slew current and store at55890, then
   AddInContactOffset at55894.
2. If captured request is true, skip the toe-threshold read and store TellAnim
   true then SkateboardDontPush true at558ec/f4.
3. Otherwise read toe-contact-threshold at558a4/b4, then load the retained most
   vertical toe normal at558c4. Set both flags in the same order only if ordered
   `normal.y > threshold` (VMX splat/all-lanes comparison at558d8).
4. Refresh whether or not those flags were set.

If `height > max_contact_offset` is false, including unordered:

1. REREAD processed flags8c4 at55908/0c. Test masks0x01000000 and0x00800000;
   this is a distinct fresh read from the captured request above.
2. If neither flag is set, clamp HEIGHT into target and store at55950, slew
   current and store at5596c.
3. If either flag is set, load current, fneg it, store target=+0 at5597c,
   then bound that negated difference and store new current at55994.
4. Call GetDistToContact at559a0 and retain its scalar distance.
5. Call ForceOffsetForPenetration at559b0 with that distance. Its possible
   current_offset correction and late debug failure happen here.
6. Call AddInContactOffset at559bc with the now-current offset.
7. Compute ordered `distance > 0.03` and ordered `current_offset < -0.01`
   (559c0/d8). Only when BOTH are true AND the initially captured request was
   true, store TellAnim=true then SkateboardDontPush=true at55a08/0c.
8. Only on that all-true branch, read contact-debug at55a10/14; if true acquire
   singleton for its "foot not close enough" draw. All paths then refresh.

The small-distance flag decision uses distance returned BEFORE penetration
correction but current_offset read AFTER correction and AddInContactOffset.
No later query result revisits this decision during the same call. NaN height
enters the second route; NaN distance or offset fails the corresponding strict
flag predicate. Do not use nonordered <=/>= complements for these tests.

### Common per-foot suffix: 55b50..55bd8

Read contact-debug anew at55b50/54. If true, require the debug singleton before
loading current and target offset values for the diagnostic at55b94/ba8. The
draw is the proven no-op described below. Advance to the next foot only after
this stage completes. Thus a common-debug failure retains the whole first foot
and prevents the second foot from starting.

## AddInContactOffset, 49538

First read contact-debug at49554/58. On true, source checks debug singleton at
49570 before any offset/matrix read. The folded debug code has repeated null
assert/reload sites at49588/90,495a8/b0,495c8/d0 and a valid-contact conditional
assert at495fc, but the available-singleton path jumps directly to495d4. It is
one actual singleton availability read on that path, not three unconditional
service callbacks. A checked host refusal stops at the first native assertion;
it must not emulate continuing through DebugBreak to manufacture a singleton.
No renderer call survives in this helper's successful debug block.

After debug, read the CURRENT contact_offset[i] and construct the full vector
`local=(+0,offset,+0,+0)`. Read retained skateboard matrix axes X/Y/Z (+160).
Calculate full4, preserving the zero terms:

```text
world_offset = Skateboard.X * +0
world_offset = FMA(Skateboard.Y, offset, world_offset)
world_offset = FMA(Skateboard.Z, +0, world_offset)
```

Load primary P at4965c; add this world_offset and store full P at49680.
Only then load parent P at49684; add the same offset and store at4968c.
The parent store is unconditional: there is no optional-parent-ID check. The
ordinary caller visits the two feet, whose real parent targets exist anyway.
Do not combine the two additions atomically or compute a parent replacement
from another target authority. Neither rotation nor the retained local contact
position is written by this helper.

Unknown W follows the full axis multiply/FMA and P-add graph. An unknown axis
W multiplied by zero remains unknown; a known NaN times zero still executes.
There is no W=0/W=1 repair and no translation in the direction transform.

## ForceOffsetForPenetration, 49698

This helper takes an actual scalar f32 distance in f1. At496b4/b8 it proceeds
only for ordered distance<0; raw4098008c branches on LT-not-set, so zero,
positive and NaN distances all skip every cvar and store.

Only for negative distance, read max-penetration-offset at496c4/c8, negate it
with fneg, and require ordered `distance > -maximum`. Raw40990070 at496d4
branches on GT-not-set: equality and unordered skip. Therefore the correction
occurs only inside the strict interval(-maximum,0), not for deep penetration.

Read current_offset at496e8; store `current_offset-distance` at496f0. Target
offset is untouched. Then read contact-debug at496f4/f8; if true, acquire the
singleton at49728 and execute the folded "Fixing for penetration" draw. A late
debug refusal preserves the corrected scalar, before the caller applies it to
the two world positions. No clamp, absolute value, timestep, or normal is used.

## GetDistToContact, 53220

Assert contact_state[i] is exactly1 at53244..64 before reading contact offset
or matrices. Nonvalid raw values are host assertion failures, not a zero
distance fallback. This function does not modify retained state.

Use the same full direction graph as AddInContactOffset to transform
`(+0,current_offset,+0,+0)` by retained Skateboard.X/Y/Z. Let offset_world be
that result, old_primary be the current primary world P at+4b0, and C the
retained contact_position. Compute the two independent point transforms:

```text
contact_local = affine_point(InverseSkateboard, C)
prospective_world = old_primary + offset_world
prospective_local = affine_point(InverseSkateboard, prospective_world)
return (prospective_local.y - f32(0.025)) - contact_local.y
```

The two final scalar subtracts are separate. Native contact transform is
completed/stored locally at53340 before the prospective transform at53360..68;
it reads contact-local Y at53344. Constant0.025 is loaded before that local
scratch is reused. This is a prospective offset computation before the caller
adds offset to world P, not a second application to an already-corrected input.
There are no dot/estimate, cvar, query, or external numeric calls here. Affine
XYZ ignores input W; full4 intermediate W follows the actual graph but is not
consumed by the final scalar. Unknown W alone must not block this function.

## ContactLineTest, 49760

This method itself does not clear contact state before querying; UpdateContact
does so at the source call sites. It uses processed EffectiveTransform, not
DataIn Skateboard or current roots, and the CURRENT primary world P after any
preceding AddInContactOffset.

1. Read Effective.Y, Effective.X, Effective.P at497bc/c0/c4. Construct the full
   down vector by multiplying Effective.Y by literal-1, including W. The foot
   side sign is+1 for entry0 and-1 for every other entry; do not use -Y via
   subtraction from zero or canonicalize down.W.
2. Load primary world P at497fc, then form `relative=P-Effective.P` full4.
3. Read sideways-offset scalar at4980c/10; multiply scalar by the side sign.
4. Compute dot3(relative,Effective.X) at4982c. This is the sole numeric service
   boundary in this helper. Source has already read the draw-cvar target
   pointer at49814, but loads its BOOL VALUE only at49830, after the dot.
   A retained immutable cvar binding can read that value lazily after dot;
   do not move the value read ahead of a fallible numeric call.
5. Form the projected point using separate multiply, subtract, FMA, add:

```text
projection = Effective.X * dot
in_plane = relative - projection
shifted = FMA(Effective.X, signed_sideways_offset, in_plane)
point = shifted + Effective.P
```

6. If the sampled sideways-draw bool was true, require the debug singleton at
   49858..78 before constructing the segment or querying. No renderer call
   survives in this block.
7. Load the actual island identity at49880. Form the exact full4 segment:

```text
down_fraction = down * f32(0.3)
start = point - down_fraction
end = start + down
```

Do not rewrite end as `point + down*0.7`: native performs the rounded start
subtraction first and then adds down. This is a meaningful finite and signed-W
discriminator, besides exceptional zero-product propagation.

8. Invoke the actual island vtable slot+44 at498e4/ec with r3=island,
   r4=literal label `SkeletonIK::ContactLineTest` (82266768), r5=&start,
   r6=&end, r7=&actual local LineCollisionResult, f1=positive-zero fatness.
   No skeleton-assembly exclusion pointer is passed here. DataIn's assembly
   capability is not read by this helper.
9. On hit, load full result position at49914, store contact_position at49918,
   then store state1 VALID at4991c. No normal, lineParam, surface or owner is
   copied into IK state. On miss, store state2 NOT_VALID at49930 and retain
   the old contact_position. An unavailable query must return a typed error,
   preserving the caller's prior state0 and every earlier offset/flag write.

All direction/projection/segment operations are full4; dot3 ignores W. Unknown
W therefore reaches typed query endpoints rather than being filled with a
homogeneous constant. The actual query result's W must likewise follow the
recovered query producer. The lower traversal/result-position provenance is
not established merely by this caller packet.

## Live cvar ownership, defaults, and debug services

These are actual cVar references, not Vault keys or fields of the IK Instance.
Float values load from ref.target+0xd8 (target pointer at ref+28); bool values
load ref.target+0xcd (pointer at ref+20). Preserve each late read above. The
initializer defaults are evidence about initial values, not permission to
hardcode them in place of runtime providers.

| Reference | Name | Raw initializer span | Default / range |
|---|---|---|---|
| 8308e610 | kDrawIKContactDebug | 82f7dd88..82f7ddd4 | false atddb0 |
| 8308e1d4 | kMaxIKContactPenetrationOffset | 82f7df48..82f7dfa8 | f32 0.3, [0,1], load df80 |
| 8308e314 | kSidewaysOffset | 82f7dfa8..82f7e008 | bits3e8f5c29 (0.28), [0.05,0.5], load dfe0 from8203b714 |
| 8308e2f0 | kSidewaysOffsetDraw | 82f7e008..82f7e054 | false ate030 |
| 8308ec74 | kMaxIKContactOffset | 82f7ddd8..82f7de38 | f32 0.3, [0,1], load de10 |
| 8308ea98 | kToeContactThreshold | 82f7da08..82f7da68 | bits3f333333 (0.7), [0,1], load da40 |

The common formatted draw target825187c0 is the already raw-verified single
word4e800020 BLR. Do not invent missing rendering work or discard preceding
singleton assertions. Direct singleton checks and Get82581e40 call sites both
require the actual singleton to be available. Thread-owner transfer argument
at every shown Get call is0. Mutable native cvar bindings and debug-continuation
after assertions are separate host fidelity limits, not successful fallback
paths. No new global physics_animation or physics_skeletonik read occurs in
this contact stage.

## Actual query connection and coherent integration proposal

Use the existing same-owner SkeletonIkState contact fields and primary/parent
world matrices; borrow SkeletonIkTargets solely for its actual bone IDs.
Borrow the actual processed data and retained DataIn. An implementation should
place the methods on that state or use a narrow borrow exposing individual
existing fields; a new contact snapshot or replacement pose authority is not
needed. The returned status remains a plain successful update unless an actual
late collaborator refuses; geometric misses are successful queries with
state2, distinct from unimplemented/capability errors.

A bounded service split can expose the six named cvar reads above, actual
debug availability, dot3 through ReckoningNumeric, and a typed actual-owner
line-query call carrying label/start/end/fatness. Keep query capability lazy:
DataIn already retained island identity without null assertions and must not
fail earlier because contact geometry is unavailable. Actual world-query
dispatch and remaining traversal work are in
[skeleton-ik-line-query-source.md](skeleton-ik-line-query-source.md).

That packet proves slot+44 and+48 both dispatch to Island827b2538, forwarding
to its actual World. World LineCollisionTest827ab338 acquires its retained
simulation-island batch, then Begin/AddTest/End/Sync/GetResult0/Release. Default
flags7 select ground, DMO and conditional vehicle children. Aggregate children
retain source order and strict-less nearest-result tie handling. This helper
does **not** authorize a generic host raycast, static-only success claim,
always-miss substitute, or invented empty dynamic collections. The lower
clustered-mesh traversal, triangle intersection and actual result W require
their own recovery before that query can claim completion.

Focused implementation tests should discriminate nonidentity hierarchy IDs;
state0/1/2/unnamed routes; initial request versus late flags reads; preserved
SkateboardDontPush; strict and unordered height/distance/normal gates;
target-before-current stores and fneg decay; no/double debug-singleton paths;
penetration open interval and post-store debug refusal; primary-before-parent
P writes; dot-before-sideways-draw value; exact start-then-end segment; query
failure after caller's state clear; hit position-before-valid and miss retaining
position; first-foot common-debug refusal before second-foot work. Original
world/terrain corpus tests must exercise the eventual real query and continue
into actual DriveFrames/mapped-row consumers, independently of the synthetic
callback-prefix checks.

Confidence high for these five routines' control, field ownership, constants,
full4 graphs and service/store ordering. Xenon dot reduction, exceptional
floating behavior/zero signs, native mutable-binding concurrency, and the
lower world query remain explicit boundaries. This is an implementation-ready
contact kernel packet, not a completed active IK pipeline or game-loop claim.
