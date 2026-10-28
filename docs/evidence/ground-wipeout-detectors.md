# Ground bail detector draft

2026-09-13, author draft. New unregistered physical_player_ground_wipeout.rs,
_attributes.rs, _leaves.rs and _tests.rs. Fresh IDA source/raw reviewed for
Ground.UpdatePostPhysics82d8e998 and Wipeout.CheckForGroundWipeout82de09c0,
Squashed82b400f8, Displacement82b3ff08, ContactForce82b3fd90, Vehicle82dd9340,
DeckFlipped82dd45a8, DeckAcceleration82dd91a0, OffBalance82ddf028 and
Leaning82ddf4c0. Root independent review, concrete adapter and Cargo gates
remain required. This author has not edited registered files or run Cargo.

## Scheduling and request effects

Ground.Post calls detector at82d8e9b4 before its separate AI path following
branch. State!=1 clears actual offBalanceTotal+B8 then stores state1+BC.
Timer+AC is sampled next. **BLE0a04** selects the detector only for timer<=0:
positive and NaN timers subtract actual Processed+93c dt and return. No clamp,
reset or repeated spawn invincibility is introduced. Actual spawn timer0.04
is already an enclosing HiLOD constructor effect. Crossing zero still returns
for that call; the next call enters detectors. Full detector never decrements
timer and never clears the requests; actual Update2 Clear remains required.

The first group is exclusive: Squashed=>type18, else Displacement=>type1,
else ContactForce=>type0. All later stages still run: Vehicle7, DeckFlipped3,
DeckAcceleration2, OffBalance11, Opposing16, Leaning19. Each source request
stores requested=true, value=+0, then increments wrapping event count, including
duplicate types. The existing WipeoutRequestLedger can implement this; owner
currently needs a source mutation entry point. Do not gate detection by enable
cvars or stop after first successful request. DoesPlayerRequest later handles
live enable filtering and Runout depends on total event count exactly1.

## Existing retained producers to bind

* SkeletonCollisionData: contact_force[8], skater_contact, vehicle_contact,
  hit_vehicle, compliant_just_collided, opposing_contacts, and
  most_vertical_contact_normal. These have real collision-update producers.
  Skeleton.IsSkeletonInCollision82b3b840 reads compliant_just_collided directly;
  do not substitute the Processed collision bit.
* SkeletonPostPhysicsRecords: max_displacement and hook_to_hips. Real post-record
  Update computes both; max_displacement is unknown before that source update.
* SkeletonRootTransforms: world_to_animation; actual board Part transform; and
  Processed+0/+10/+20 skate basis for DeckFlipped.
* **SkateboardBody+290 is CollisionInfo.maxContactVel**, not acceleration[6].
  Reuse board post_prefix.max_velocity, produced by actual contact prefix.
  numPartsInContact is body+360, existing normal tail writes board post fields.
  CollisionInfo flags body+364 masks40000000/20000000 have contact-prefix
  producers. Require their actual known bits when reached.
* Actual Processed up220/Y224, deckSpeed968, timeDelta93c and named flags8c8,
  8cc,8c4. Word8c8&1 chooses initial squash threshold; native **lhz+8c8&1**
  is big-endian word00010000 and independently chooses trick multipliers.
  8c8&40000000 selects AI/skater modifiers;8cc&10 selects skitch modifiers.

No missing contact, body, pose or phase may become a fabricated zero/false.
This draft supplies domain callbacks; binding SAME solved/postphysics owners
and phase validation is necessary before it can be called live.

## Live Vault and cvars

The attributes enum contains24 full64-bit keys independently extracted from
raw lis/ori/insrdi sequences, not truncated decompiler signed integers. Reuse
SkateboardAttributes::Wipeout, already bound to physics_wipeout/default in the
retained Vault. The new lookup helper preserves the existing runtime inheritance
and DefaultDataArea(4) only for a genuinely missing pointer. Keep lookup and
value reads distinct: GroundArms lookup0d48, GroundContact lookup0d5c, arm read
0d64 then body read0d70; trick Y lookup0e5c, XZ lookup0eb4, Y read0ed0 then
XZ read0ed4. Optional skater/vehicle/skitch/board-contact modifiers remain lazy.
Every reached attribute call requires actual GlobalAttributeCollection binding.

Two cvars are still required for the new detector adapter:

| Label | Section | Title | Seed | Initializer | Table entry |
|---|---|---|---|---|---|
|kDrawSkeletonCollisionWipeout|Physics_Skeleton|Draw Skeleton Collision Wipeout|false|82f7c418|82fd453c|
|kWipeoutOnCollision|Physics_ATEST|Wipeout On COllision|false|82f7c6e8|82fd4560|

Both use null callbacks. All three skeleton helpers sample draw live, even
when there is no collision. ContactForce captures draw target before force
loads, reads forces0,1,5,4, samples draw value atfdd4, then forces7,6,2,3.
Displacement samples wipeout-on-collision before the lazy compliant byte.
When drawing is enabled, the source callbacks must execute the actual debug
branch prerequisites/read graph or explicitly return pending. The visible draw
callees resolve to shared empty825187c0, but singleton and payload/numeric reads
must not disappear. These debug-true branches are deliberately adapter contracts
in this bounded draft; ordinary false controls run the complete detector path.

## Numerical and branch details

ContactForce computes the exact fsel pair-max order: max(0,1), max(5,4),
max(7,6), max(2,3), then combines the first three for body and fourth for arms.
NaN in a-b chooses the second fsel operand. Final body BGT then arm BLE means
body>threshold OR !(arms<=threshold), including unordered arm comparisons.
Squashed and displacement instead use ordered BGT (NaN false); displacement
compares dot(hook,hook) against separately rounded threshold squared.

DeckFlipped computes three corresponding basis dots, min(x,y), min(result,z),
max(-1,result), min(1,result), then the source XMVectorACos and ordered greater
comparison. Reuse ReckoningNumeric/try_acos and retain VMX min/max exceptional
selection as explicit numeric service boundaries. DeckAcceleration transforms
actual maxContactVel by three world-to-animation basis columns with first
rounded product then two FMADDs; absolute Y and XZ length are independent.
Its branch is XZ>threshold OR !(absY<=threshold), because92ec is BLE.
Use existing try_length/two reciprocal-sqrt refinements, not generic sqrt.
Only test fixtures use host sqrt/acos as simple dependency probes.

OffBalance looks up speed, decay, limit unconditionally, then native BGT
requires speed>0 (NaN also asserts). It reads8cc&02000000,8c4&20, deck speed,
upY, computes ((speed-deckSpeed)/speed)*(1-upY)*gateB, then **FMSUBS f1f8**
scaled*gateA-decay. Contribution>0 adds to retained total; otherwise total0.
No dt factor exists here. Store total before final **BLE f21c** comparison:
!(total<=limit) requests, including unordered limit. Vehicle uses the same
BLE unordered fallthrough. Leaning uses **BGE f548** skip, so !(upY>=limit)
reaches the actual compliant query; final normalY comparison is ordered VMX>
constant bits3f34fdf4 (0.707). No generic finite sanitization is permitted.

The numeric primitives retain existing host reduction/estimate/NaN limitations;
this is not new Xenon bit-parity evidence. Five authored tests cover timer
crossing/NaN order, all seven simultaneous requests and duplicates, delayed
Vault pointer loads, late failure preserving requests and exceptional branches.
Standalone rustfmt passed. Concrete runtime integration and independent review
are pending; no complete playable frame is claimed by these leaf tests.

## Concrete adapter handoff

Further author draft adds physical_player_ground_wipeout_runtime.rs using
PhysicalAssemblyWorld's actual same-pair validation and existing Skeleton,
board, Processed and numeric getters. It takes the retained SkateboardAttributes
binding by mutable reference: caller must supply its actual GlobalAttribute
binding and schedule after SAME Skeleton.Post. This method validates pair/owner
identity, not the completion of earlier tick callbacks. No contact values are
prefetched before timer/branch gates. Board contact state is obtained lazily via
the existing actual force runtime; Skeleton collision reads use state().collision.
Processed+0 was checked against PDB as mSkateboardOrientation, and the adapter
uses skateboard_orientation(), not deck/effective transform substitutes.

Exact shared registration additions for root:

* lib.rs: modules physical_player_ground_wipeout and
  physical_player_ground_wipeout_runtime. Existing children of the first module
  register its leaves/attributes/tests.
* physical_player_wipeout.rs: child `ground_access`, path
  physical_player_ground_wipeout_owner.rs. This new child directly accesses the
  actual private state, timer, balance and ledger. It exposes narrow getters/
  writes and request mutation, and synchronizes legacy request_counter with the
  ledger event count after every request. Remove the now-consumed timer field's
  old dead_code expectation when the production path is registered.
* input_cvar_runtime.rs: child `ground_wipeout`, path
  physical_player_ground_wipeout_cvars.rs. Append193 DrawSkeletonCollisionWipeout
  and194 WipeoutOnCollision with the exact definitions/table positions above.
  The helper captures Arc target193 separately from scalar loads and reads194
  live. This is required even for empty collision data.

No new Skeleton getter or alternate contact/pose owner is required. The
adapter's debug-true branches explicitly return DebugBranchPending (squashed
and force first require the actual overlay); they do not claim successful
debug execution. Ordinary false live controls run all recovered detector logic.
Host min/max selections remain the stated numerical boundary. Root must run
independent review and Cargo gates; author only performed standalone rustfmt.
