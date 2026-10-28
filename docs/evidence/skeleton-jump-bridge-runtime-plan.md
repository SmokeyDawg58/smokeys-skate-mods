# Skeleton jump bridge: actual attributes, retained adjustment and input

2026-09-08, read-only original XEX/PDB and repository audit.
No code/Cargo. This closes concrete binding questions for the new
skeleton_processed_jump_events.rs consumer; it does not invent Actor input.

## Two global instances, one non-default collection

| Singleton slot | Generated type/class | Selected collection | Source |
|---|---|---|---|
|+124|anim_motion /90c3decf0105631e|jumping /ac260b44fa3ca0a4|GlobalAttributeCollection ctor828b2cb0..2cc8; store2cd4|
|+d0|physics_jump /c2e0a8b6c13990fd|default /d7edbd362d7d2152|Global ctor828b2748..2750; store275c|

The first is NOT a class named anim_motion_jumping and NOT its default
collection. PDB field anim_motion_jumping names the singleton's selected
anim_motion instance. Raw2cb0..2cc4 constructs collectionac260b44fa3ca0a4;
anim_motion generated ctor828b1590 constructs class90c3decf0105631e, FindCollection
15c0 then Instance15d0, port0. Null cached layout fallback size5c0 at15e0/15e4.
physics_jump generated ctor828b0f78 constructs classc2e0a8b6c13990fd,
FindCollection0fa8/Instance0fb8, fallback layout1a00fcc. Globalctor'sr28 is
defaultd7edbd362d7d2152 assembled1630..1644 and passed274c. Both allocate real
0x10-byte generated instances; immutable host instance retention projects the
successful construction, not mutable singleton replacement/editor behavior.

New SkeletonJumpVaultAttributes::from_retained_instances already accepts the
two exact retained AttributeRuntimeInstances with capture/lookup/value tokens.
Root should bind these exact two selections once in the actual retained
physical/attribute owner and retain that provider. No existing concrete
physics_jump or anim_motion runtime provider was found to share; physics.rs
contains semantic jump tuning values, not this singleton lifetime. Do not
rebind per event or pass a collection chosen by equal scalar values.

All are ordinary FETCH/index0, separate source capture/lookup/dereference:

| Key | Selected instance | Read | Original corpus payload |
|---|---|---|---|
|b52e6ee4f322a9f4|anim_motion.jumping|byte enable endpoints|01 (inline word01000000)|
|982f499ec97735a9|same|float lower endpoint|3efd70a4|
|a1f65bb8c4d1ba8b|same|float upper endpoint|3f666666|
|34bb37c315bc14b0|physics_jump.default|byte retain prepared adjustment|01 (inline word01000000)|

Original external skater.vlt was read with the existing inspect_vault executable,
not rebuilt. Both selected collections have parent0; jumping has5 explicit
entries and physics_jump.default14. The four nodes have flags40; byte definitions
size1 and floats size4. These payloads belong in corpus assertions only, never
runtime constants. Missing FETCH pointer uses DefaultDataArea1 or4 as source;
a malformed present scalar does not become a default. Existing consumer retains
late endpoint reads and post-strength hippy overwrite ordering.

## Actual Skeleton adjustment storage

Skeleton constructor82b56d70 loads f31 from resolved original address8220a90c
word00000000 at56e54. It stores mJumpAdjustX+3e34 at57478 and mJumpAdjustZ+3e38
at57480. These are source-backed +0 constructor writes, not processed+954/+958.
ProcessAnimAttributes PrepareJump writes them at5aeac and5aeb8, after its
ordered two map calls. The retain-adjustment branch later loads+3e34 at5b95c,
stores processed+954, then loads+3e385b964 and stores processed+9585b968.

An aligned immediate-load/store scan of original82400000..82fc0000 for3e34/3e38
found only these ctor stores, PrepareJump stores and post-loop reads. Fresh
TeleportReset82b54228..542bc and ResetSkeleton82b45bf0..45dd4 do not store these
fields. Thus retain them across those resets; do not reset them when resetting
selected processed output adjustment fields. Indirect/bulk future writers must
be audited if another caller is introduced; this scan is not a universal proof
against all aliasing memory operations.

Current actual SkeletonNormalAssembly in dynamic_assembly_skeleton_normal.rs
owns the retained Skeleton state/session, current animation, roots/general state,
history and hips offset. It has no retained jump-adjustment slot yet. Add one
small selected Skeleton adjustment substate HERE (or its actual pre-normal
construction owner transferred unchanged on admission), initialized only at
selected constructor57478/57480. Do not add another pair of producer fields to
SkeletonProcessedContact. That owner already correctly stores the separately
published processed+954/+958 results in jump_events; those are consumers.

Provide late methods on the actual Skeleton occurrence for X store, Z store,
X read, Z read. No combined transactional setter: source failures preserve its
first store. Null Actor input after PrepareJump preserves previous Skeleton
adjustments. Constructor-known values can legitimately be copied by the retain
branch; unknown/unconstructed fields must refuse rather than silently zero.

## IDs and actual input source

controller_input.rs already defines ControllerControl::LeftStickX=74,
LeftStickY=75,RightStickY=78 and MappedControllerInput::mapped_value. It is an
adapter for COMPLETED cInputMap expression results, explicitly excluding hardware
polling, pad normalization, map expression execution and gestures. Its
MappedControllerSnapshot is a diagnostic precomputed array, not native InputSK8.
RawControllerInput.update825ee088 samples those mapped controls and applies
bumper arbitration; its previous argument is supplied by the caller (native
previous-copy825f1c6c..1c88). No actual retained Actor input/history service or
cInputExec evaluator was found connected to this paired physical runtime.
SkeletonPhysicsHistory is BODY motion history, not player controller history.

Native PrepareJump queries Skeleton.mIActorQuery+3e70, virtual GetInput at5ae40.
The actual Actor::GetInput825f11a8 returns null when mAIController exists;
otherwise it gets the Simulation singleton, requires Workspace InputMan, and
returns InputMan.inputs[playerID].mInput. This explains a legitimate null path,
but does not authorize returning null for a missing human-player input service.
The abstract caller uses GetInput again later for the alternate adjustment branch.

Native cInputMap::GetValue8298c718..c7f8 validates its lookup conditions and calls
cInputExec::Execute8298c7e4 with the actual pad/program. Each mapped sample must
use the captured InputSK8's current pad+8, map+c, globalsg0..g3+10/+14/+18/+1c.
At PrepareJump ID74: readg3ae54,g2ae58,padae5c,g1ae60,mapae64,g0ae68,callae6c.
ID75 independently reloadsg3ae78,padae7c,g2ae80,mapae84,g1ae88,g0ae8c; lower-X
fsubs/fsel happensae90/94 BEFORE second GetValueae98. Do not reuse one combined
axis snapshot or call RawControllerInput.update (which samples extra controls).
Post-loop nonretained-adjustment branch samples74/78 at5b9ac/5b9d8; using75 for
both stages would alter source behavior. Reuse the ControllerControl ID enum
at the boundary, but an actual map evaluator/Actor-owned InputSK8 binding is
still a producer prerequisite, not present just because the trait exists.

Hippy input is current processed+8d0 bit0x8000 after strength publication.
The newly supplied SkeletonJumpInput::hippy_jumping must read a known producer
on SAME processed owner. A supplied false fixture or inferring false from missing
animation events does not establish that bit. Complete existing raw flag producer
or keep the bridge explicitly refused at that source point. No second flags word
or unrelated Boolean state should substitute for shared ProcessedPhysIn ownership.

## Minimal honest closure

First bind both actual Vault selections once and add actual Skeleton adjustment
storage/constructor provenance and late methods. Then adapt the selected jump
consumer using SAME pair processed lock plus actual Skeleton occurrence, with
explicit ActorInput capability for captured InputSK8 and per-ID map calls.
Its absence is a typed missing producer; controlled maps remain test-only.
Connect actual raw animation event traversal and hippy-bit producer to this
same owner, preserving flag/store prefixes. Reuse existing jump consumer, not
physics.rs generic strength/clamp or a second event dispatcher.

Tests should verify original four payloads and selected hashes; null input
preserves real retained adjustment; repeated PrepareJump calls reload74/75;
second sample failure leaves both oldSkeletonvalues; X/Z store failures retain
source prefix; reset preserves Skeleton fields while processed reset clears its
own fields; false-retain branch uses74/78; hippy unknown refuses after earlier
strength store; foreign map/Actor/Skeleton/processed occurrences refuse without
invented values. Original retained human-input sampling remains unfinished until
actual InputMan/map/history/Actor scheduling is connected.

Fresh function hashes: generated physics_jump828b0f78..0ff0
64255457cf757e1055751e1f6e4f83f5d0365f370ebb2cdedf732436afd123d3;
anim_motion828b1590..1608
b36cfa730ffa80cbbb0724dd410ea54c097f144e4215e885ea96930f69148252;
GetValue8298c718..c7f8
5fe0c0595e5d7a1efd049816d2420d2b9930513d4c923cccc09b351bcaa0def3;
TeleportReset82b54228..542bc
08d3b8f82a3276ecdd112e66f4276db80b18d5ef9f45d98dd76b26494b8daa45.
Confidence exact for listed raw class/collection/field/call evidence; high for
bounded integration plan. No live Actor, input-history or full jump claim.
