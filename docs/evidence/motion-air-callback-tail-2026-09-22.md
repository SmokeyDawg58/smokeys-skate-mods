# Immediate Air callback tail and next transition reuse map

## Connected callback packet

Actual selected Air252 declares occurrence650 ControlAirLegExtension,651 ClearTrickAttr,652 ApplyingBodyTilt. After the already integrated650 constructor,651 is necessarily visited. Its registered binding is vtable8230087c: Create82bb3da8 returnsNULL (store0 at3dac); Begin/Update825187c0 are shared no-ops; End82cb3020 is real work. No fake instance or blanket no-op is introduced.

End resolves cached IAnimatable against UCOM, asserts presence, encodes Trick, invokes slot26 at82cb3100. Same actual SkaterAnim Animatablevtable82302d28+68 points82593058 RemoveConstructionAttr. This finds the packed key in mConstructionMap, erases only whenfound, and leaves a missing key unchanged. Existing ActorAnimationConstruction.base.construction_map is the same fixedcapacity6 producer used by motion_set_construction_attribute. Removal shifts host occupied slots without constructing another map; clearing the final Option records host occupancy, not native clearing of unused memory.

652 ApplyingBodyTilt already has its actual Begin/Update/End behavior. Then Sequence690/Default691 selects1359/1360 PlayAnimation; both use parameter section1, attributes2752..2754: attribute=disttocog/from=lastAnim/SetBlend=true. Existing PlayAnimation lifecycle controls when FromLastAnim samples, rename/default rules, accepted raw types0/2, ignored types1/3, and invalid-type errors. The previously pending adapter.tree_attribute now calls the SAME retained cached GetTreeAttr query recovered in the selection packet and projects the actual raw type/firstword; it does not replace lifecycle control flow. Float/Vector/Bytes24 preserve firstword bits; retained unsupportedtype2 bytes are read as the original word. Short host payload data is an explicit checked error, not fabricated data. Absent attribute remains None.

Tests reuse existing actual base-owner test constructor, verifying absent/duplicate removal and neighboring authored keys; typed cached-sample tests preserve signedzero/NaN bits and original type bytes. Author ran standalone rustfmt and apply-check --ignore-space-change. Root owns Cargo and originaldata execution.

## Read-only next-frame map (no transition implementation here)

Existing Graph.search performs each priority3?0, walking current leaf691?252?79?21?3?0. Leaf691/252 have no transitions. Priority3 first reaches Air21 transition1456 expr39 (existing c87 PhysicsWantsRunout), then root transition0 expr273 (existing teleport predicates). Priority2 reaches Air21 transition1455 expr338 BEFORE OnBoard3 transition326 expr420. Therefore the first missing expression after ordinary false priority3 gates is338, not a later landing leaf.

Fresh Pre33882915358 = c160 IsBodyFlipping then lazyc87 PhysicsWantsRunout. IsBodyFlipping82c9cb40 verifies same IPhysical and reads actual PhysOut+8 Air+1a9 at82c9cc24; existing Air.is_flipping and try_air_output are the concrete producer/consumer seam. No new flip owner is needed. OnBoard420 already reuses c2..5 physical wipeout gates.

Priority1 AirStatic79 checks1423 expr292 (already c9&&c85),1424 targetLateflip80 gate335 (already c223&&c139),1425 targetOneFoot81. The latter descends259/260 expr191/192 requiringc218 IsRidingGoofy; fresh82c9b308 samples Animation+9a then+99 from two original PhysOut getters and returns their boolean equality. Use actual existing animation fields after exact field mapping, not backend stance inference. OneFoot children336/337 additionally requirec240/244 then lazyc159 TimeToLand; those are outside this callback packet.

Air21 priority1 then1047 targetLand22 gate339: fresh82915424 reads c89 PhysFilteredState then lazy NEGATEDc282 HasTiltToLargeForPreland. Condition82c9c1c0 resolves same ISkaterMotionGraph, missing returnsfalse, otherwise virtual+1c already recovered as825f48d0. Existing motion_prepare_land_query::has_tilt_too_large and same actor PrepareLand Source can be exposed directly; do not duplicate its pointgraphs/tilt math. Transition1048 targets existing Grinding18 and remains subject to explicit parked-grind policy.

Priority0 AirStatic79 transitions1426/1427 reuse recovered grab/board-adjust sibling trees, then1428/1429 manual targets1176/1334 require242/243. No speculative manual/grind/offboard code is added. Follow original next reached gate before extending these callbacks or selected-branch state update thunks.
