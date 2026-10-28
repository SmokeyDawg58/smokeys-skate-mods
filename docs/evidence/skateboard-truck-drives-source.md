# Fresh skateboard truck-drive source

Source recovery and implementation checkpoint, 2026-09-07. The original
investigation was source-only; the fresh suffix now has a Rust implementation
and mounted-data test, with final code review recorded below. Scope: fresh
mNumDrives=0, mForceTurn=false, original
physicsdeck knob true, and the two mandatory truck drives. Confidence **high**
for the inspected PPC/PDB behavior; no Xenon per-tick trace is claimed.

## Functions and callers

All four methods belong to `Sk8::Physics::SkateboardBody`:

| Function | Reference range | SHA-256 |
| --- | --- | --- |
| CreateDrivesWithOffset(float) | 82b37b88..82b37c68 | 37bd7ec1b01ff02b3a9f0a92e3898a32d12ac041f7760e9417278b95b29a79a9 |
| CreateDrives() | 82b37ea8..82b380bc | 1bccdb6ad7a5dc5d81294b85072d81154ef301478189786b6a97e62e807e61a2 |
| SetTruckDriveFrames() | 82b34fa8..82b35270 | e94bd7528124e308c8f02a869ddc828d210826b54fcd8cd8de7235c7ac52a05f |
| SetTruckDriveDynamics() | 82b2cb08..82b2d0e8 | 8146f9dc1f0e3007db282fc44c2c18b4d4ed25817cc947c4f72746d4ddb8533e |
| SetDriveFrames2(DriveFrames&, Matrix44Affine const&, Matrix44Affine const&) | 82b2c998..82b2cb04 | 83d8bad209bf12aae2ca036d16b146dbefa15d493933eb2d5b95a5019839c50a |

Decorated names follow the PDB methods, notably
`?SetDriveFrames2@SkateboardBody@Physics@Sk8@@AAAXAAVDriveFrames@physics@rw@@ABVMatrix44Affine@vpu@math@6@1@Z`.
The constructor at 82b38270 calls CalcTruckTransforms at 82b38828, then
CreateDrives at 82b38830. CalcTruckTransforms is separately recovered in
`skateboard-joints-source.md`; none of the four methods above calls it.

Constructor range 82b38270..82b38914 has SHA-256
`572d9a001bca3cc986eb0f1f234b44f8deb0026ab61639ac070e5a8c8597f123`.
Its fresh stores include six truck dynamics slots with both Params entirely
zero, type NO_DRIVE=0. Slot 0 stores are 82b38518..82b3853c; slot 1 stores
82b38540..82b3855c. Front/back turn angles, mForceTurn and damping-factor
initialization are at 82b386c0..82b386d0. mNumDrives starts zero.
Independent live PPC review also verifies fresh frame contents: gLVector at
8227e920 is [0,0,0,3f800000], bound through r10 at38300/310; v63 is zeroed
at38304. Frame0 qa/pa/qb/pb stores are38430/38438/38494/38498, frame1 stores
384a0/384a4/384ac/384b0. All six frame slots initialize identity quaternions
and positive-zero positions through38514. The Rust fresh values are therefore
source-backed, not assumed inactive/uninitialized memory.

## Relevant PDB ownership

| Member | Offset | Shape |
| --- | --- | --- |
| mTruckdriveframe | 1a60 | DriveFrames[6], stride40 |
| mTruckdrivedynamics | 1be0 | DriveDynamics[6], stride20 |
| mTruckDrives | 1dc0 | Drive*[6] |
| mNumDrives | 1dec | int |
| mForceTurn | 1df0 | bool |
| mTurnAngleFront | 1df4 | float |
| mTurnAngleBack | 1df8 | float |
| mSpeedDampingFactor | 1dfc | float |
| mFrontTruckTransformLevel | 1e10 | Matrix44Affine |
| mBackTruckTransformLevel | 1e50 | Matrix44Affine |
| mFrontTruckTransform | 1e90 | Matrix44Affine |
| mBackTruckTransform | 1ed0 | Matrix44Affine |

Existing `SimulationDriveFrames` is the exact four-vector value: child
quaternion_a +00, position_a +10, parent quaternion_b +20, position_b +30.
`SimulationDriveDynamics` has linear Params +00 and angular Params +10;
each Params contains spring, damping, strength, raw_type in four words.
PDB DriveType values: NO_DRIVE=0, SOFT_DRIVE=1, HARD_DRIVE=2.

Native pool slots retain mutable pointers into the board's arrays, not
copies. Existing `Arc<RwLock<SimulationDriveFrames>>` and
`Arc<RwLock<SimulationDriveDynamics>>` already preserve this lifetime and
later mutation visibility. Do not replace them with immutable final-value
snapshots or share one slot owner between both truck ordinals.

## Fresh construction sequence and null behavior

CreateDrives starts at 82b37ecc by calling CreateDrivesWithOffset(+0).
Within WithOffset:

1. Load old mNumDrives at 82b37b98 and assert old_count<6 at 82b37b9c..7bb8.
2. Fetch island GetRWSimulation through virtual offset +3c, call 82b37bcc.
3. Call SetTruckDriveFrames at 82b37bd8.
4. Call SetTruckDriveDynamics at 82b37be0. It still fetches the deck byte;
   fresh count zero skips its coefficient loop and all dynamics writes.
5. AddDrive at 82b37c10: parent deck part 6, child truck part 4, frame and
   dynamics owners at old_count. Store returned pointer at 82b37c2c.
6. AddDrive at 82b37c44: parent deck part 6, child truck part 5, owners at
   old_count+1. Store returned pointer at 82b37c50.
7. Read current mNumDrives, add two, store at 82b37c54..7c5c.

There is no null-result branch between or after the two AddDrive calls.
Both attempts happen; count advances even if one or both returned null.
There is no rollback of an accepted first drive when the second is null.
The offset parameter is unused by the inspected WithOffset instructions;
do not invent a placement adjustment from its name.

After return, CreateDrives calls SetTruckDriveDynamics at 82b37ed4 (now two
slots), then SetTruckDriveFrames at 82b37edc, then checks the optional linear
override. Thus successful AddDrive initially retains zero dynamics; later
angular writes must reach those same owners. Precomputing final dynamics
before insertion would change the source lifecycle.

The count assertion is not a safe general append API: old_count=5 passes
the native comparison despite two writes, and SetTruckDriveFrames always
updates slots 0/1 rather than old_count/old_count+1. This packet supports
fresh old_count=0, not generalized repeated construction. Safe-host refusal
for unsupported states must be explicit policy, not claimed native checks.

## Angular attribute reads and arithmetic

`GlobalAttributeCollection::physicstrucks_drives_default` is at +a0, type
`Attrib::Gen::physicstrucks_drives*`. Class key is `6599909c97c57a89`, default
collection `d7edbd362d7d2152`.

SetTruckDriveDynamics first fetches physicsdeck key `103fdc19514d4246` at
82b2cb70, index zero. Null selects DefaultDataArea(1); `lbz` at 82b2cb84
tests the byte. It reads mNumDrives before the branch. Shipped byte is 1.
For the true branch, positive count enters an ordinal loop. It rereads
mForceTurn at +1df0 each iteration; fresh false selects the normal branch.

For each slot the normal branch performs these four index-zero lookups,
retaining every returned pointer until all four lookups finish:

| Lookup address | Full key | Semantic role | Inspected original scalar bits |
| --- | --- | --- | --- |
| 82b2cd28 | a9f972c55a528cb7 | strength input | 3f800000 (1) |
| 82b2cd80 | 71be814cd51a993f | base damping | 42200000 (40) |
| 82b2cdd8 | aa32fea00e7d0921 | speed damping multiplier | 3f800000 (1) |
| 82b2ce30 | 22f543369220c8c7 | spring | 450ae000 (2222) |

Null scalar pointers select DefaultDataArea(4), not a guessed tuning value.
The semantic role labels do not claim original plaintext key spellings.
The precise subsequent loads/calculations/stores are:

- ce44 load current mSpeedDampingFactor;
- ce48 load multiplier, ce4c load base damping, ce50 load strength input;
- ce54 `fmadds`: damping = speed.mul_add(multiplier, base);
- ce58 load spring;
- ce5c store angular type SOFT_DRIVE=1;
- ce60 store spring, ce64 store damping;
- ce68 `fmuls`: strength = input * f32::from_bits(0x4560fffe);
- ce70 store strength, then reread mNumDrives and continue in ordinal order.

The constant `4560fffe` is 3599.99951171875, loaded at 82b2cbb8. It is not
exact 3600. The two slots independently perform all four lookups and loads;
do not reuse sampled coefficients from the first slot. No finite-value,
positive-value or damping clamp is present in this branch.

For the fresh inspected inputs the resulting angular Params are spring
2222, damping40, strength3599.99951171875, type SOFT_DRIVE. Linear Params
remain entirely positive zero, type NO_DRIVE. These are corpus expectations,
not permitted runtime constants.

The mForceTurn=true and physicsdeck-byte-false branches exist but are not
the bounded fresh profile covered by these equations. The true-force
branch uses additional keys and HARD_DRIVE behavior. Do not silently route
unsupported live steering states through the fresh false-force branch.

## Optional linear override is off in the original corpus

CreateDrives fetches physicstrucks key `7210da40dfb18272` at 82b37f2c,
index zero, null -> DefaultDataArea(1), byte load 82b37f40. Original byte is
zero, so 82b37f48 skips the entire linear override. This is distinct from
the true physicsdeck knob above. The fresh full method has eleven attribute
lookups: the first count-zero dynamics byte, the second dynamics byte plus
eight per-slot scalar lookups, then this linear-override byte.

For boundary identification only, the skipped branch evaluates a vector
rsqrt estimate at 82b37f88 for constant100000 (47c35000), two source
refinements, zero-selection and multiplication by two. Its per-slot loop
fetches physicsdeck103fdc19514d4246 again, stores linear strength48afc7ff
(359999.96875), and for true uses spring45bb7fff (5999.99951171875), damping0,
type HARD_DRIVE=2. The false alternative uses spring100000, the computed
twice-square-root damping, and SOFT_DRIVE. This skipped branch is not needed
to produce the inspected fresh profile and is not claimed implemented.

## SetTruckDriveFrames

This function performs no Vault lookups. It reads existing level matrices
from CalcTruckTransforms and live turn-angle fields:

1. SinCos(mTurnAngleBack) at 82b34fe4.
2. Construct a turn X matrix with raw columns `[1,0,0,1]`, `[0,c,s,0]`,
   `[0,-s,c,0]`, and zero translation.
3. Compose BackLevel * turn matrix. For each rotation column and each of
   four lanes: initial Lx*column.x, then Ly.mul_add(column.y,partial), then
   Lz.mul_add(column.z,partial). Translation starts with
   Lx.mul_add(+0,Ltranslation), then Y and Z zero FMAs. Do not replace the
   zero-FMA graph with a copy, or discard raw W lanes.
4. Store mBackTruckTransform at 82b350cc..82b350d8.
5. Load mTurnAngleFront at 82b350dc, negate at 82b350e0, call SinCos at
   82b350f0. Repeat composition with FrontLevel, storing Front transform.
6. Call SetDriveFrames2 at 82b3520c for frame slot0: identity/zero-origin
   parent matrix, child matrix mBackTruckTransform.
7. Reconstruct identity/zero-origin parent and call SetDriveFrames2 at
   82b3524c for frame slot1 with mFrontTruckTransform.

Fresh SinCos arguments are +0 for Back and -0 for Front. Do not merge them
or substitute a hardcoded identity turn. Both transformed matrices are
constructed before the first frame-setter call. The function runs twice
during CreateDrives, on opposite sides of the insertion/dynamics updates.

## Required SetDriveFrames2 dependency

The generic helper computes inverse-parent times child, then sets the child
frame to identity/zero-origin and parent frame to the computed relative
transform. It does not merely copy the child matrix, even when parent is
the identity supplied by SetTruckDriveFrames.

The parent inverse rotation transposes its XYZ axes into columns with W
zero. `negative_origin = [0;4] - parent.translation` at 82b2c9d8. Inverse
translation starts with inverse-Z * negative_origin.z, then inverse-Y FMA,
then inverse-X FMA (82b2ca58, 82b2cab0, 82b2cac0).

Relative rotation columns start with inverse-X * child_column.x, then
inverse-Y FMA and inverse-Z FMA. Relative translation starts with
inverse-X.mul_add(child_position.x,inverse_translation), then Y FMA and
Z FMA (82b2cad0..82b2cad8). The inverse axes have W zero, so raw W values
in the input child matrix must not be copied into the relative rotation.
No guessed generic matrix inversion or orthonormalization replaces this.

It invokes these actual setters:

- DriveFrames::SetChildFrame at 82b2cae0, target82b25248..82b25404, SHA-256
  `b0b91fd2e1a3675d15463e1a1af2950a202f53bbc4b25aaf76ddbe3a827d0ec3`;
- DriveFrames::SetParentFrame at 82b2caec, target82b23470..82b2362c, SHA-256
  `f79fb51cfa470cf86a22197da96b9d23b0988c0fd1bb6b2ff25a3271145527be`.

Both use the same unnormalized quaternion candidate graph recovered for
JointFrames, with one vector estimate and two refinement rounds, strict
trace/axis selection, and no final normalization. Child writes quaternion_a
and copies translation to position_a; parent writes quaternion_b/position_b.
The existing `world::try_simulation_quaternion_candidate` is the appropriate
numeric reuse seam. The normalized public wrapper is not interchangeable.

One SetTruckDriveFrames invocation performs two SinCos calls followed by
four candidate evaluations: child0, parent0, child1, parent1. CreateDrives
invokes it twice: four SinCos and eight candidate evaluations total. These
are additional to the separately called CalcTruckTransforms calculations.

## Actual pool integration and retained state

The concrete `SimulationDrivePool::try_add_drive` already takes the body
pool, parent/child live handles, and separate Arc<RwLock> frame/dynamics
owners. Its native source AddDrive is 82ae43f0..82ae4484: Free-head to
Active-tail insertion, parent into body_b, child into body_a, spy reset,
tag untouched. Native capacity null means no pool mutation. Existing host
body/authority checks and lock poison errors are explicit safe-host policy.

`SimulationAssemblyBackend` already implements the corresponding add port.
Reuse this pool and existing Params/Frames, rather than introducing a second
drive representation. The owner must retain both payload write capabilities
after insertion so SetTruckDriveDynamics and SetTruckDriveFrames update the
same data later observed by the drive Jacobian/Horus consumer.

Smallest useful next implementation: a fresh two-slot mutable truck-drive
owner with exact pre-insertion/post-insertion sequence, supplied existing
level transforms and actual seven-part body handles. Extend the retained
attribute provider with the new class binding. Do not manufacture an island,
simulation or clustered-mesh capability to bridge missing authored admission.

## Validation and remaining unknowns

Read-only inspection used the existing inspect_vault executable with external
`data/db/unpcaked#/data/db/skater.vlt` and its skater.bin dependency. The
physicstrucks_drives/default collection has eight explicit entries and no
parent. The four relevant values above are inline scalar nodes40 with no
PtrN in their scalar words. The deck knob was byte01, truck linear override
byte00. This was the previously documented relocated-pointer-word extraction,
not a new raw-mounted corpus test. Original assets remained external.

Unknowns/limits: no captured Xenon estimate/trigonometric outputs or native
live drive trace; original plaintext names for newly recovered keys were not
established; repeated WithOffset/nonfresh counter cases and later forced-turn
updates are not generalized here. Optional wheel drives and the mandatory
separate hook body/drive remain outside this bounded packet.

Focused discriminators for implementation: count-zero byte lookup without
coefficient reads; pool insertion sees zero initial dynamics; later writes
visible through retained owners; two independent coefficient samples; fused
damping arithmetic and load/store chronology; first-null/second-null does not
change the two-attempt/count-increment sequence; repeated frame passes and
their combined call order; signed-zero turn arguments; raw W versus inverse
W-zero behavior; unchanged fresh linear NO_DRIVE; actual mounted retained
Vault reads into both live drive payloads and the existing solver consumer.

## Independent review and next integration constraints

2026-09-07: independent review checked all eight packet hashes against
live IDA and passed the bounded fresh profile, lookup/load/FMA order and both
frame passes. The four scalar corpus observations were not independently rerun
by that reviewer; the next mounted test must read the raw retained Vault.
Global binding sites are FindCollection828b23dc, Instance828b23ec and +a0
store828b23f8. Selected-class binding order is deck, world, wheels,
trucks_drives, trucks, not appending the new class after trucks.

Create the persistent owner before side effects and retain accepted drive
handles even if a later host attribute/numeric/lock operation fails. Native
capacity null is not the same as a host validation/poison error. The existing
SimulationAssemblyBackend add trait records non-capacity failures separately;
the post-admission route must not silently treat those as native nulls.
Do not use the ordinary Assembly attachment drive loop: it rolls back on null,
unlike this constructor suffix. Keep body and drive handles private and tied
to the attached authority, with explicit cleanup before body teardown.

Root destructor inspection was subsequently independently verified in raw PPC:
`??1SkateboardBody@Physics@Sk8@@UAA@XZ`,82b2b040..82b2b2f0, removes truck drives
in ascending ordinal order at82b2b0dc, rereading count at82b2b0ec; it does not
test each pointer for null. Optional wheel drives follow at82b2b110, then hook
drive at82b2b128, RemoveHookAssembly at82b2b1a4 and RemoveSkaterBody at82b2b1bc.
Simulation::RemoveDrive82ae3360..82ae33b4 starts with lwz drive+8: null is not
safe. It unlinks the slot, appends Free-tail, decrements Active/increments Free,
does not dereference bodies, and retains frames/dynamics/spy/tag and occurrence.
Destructor does not clear pointers or reset count. Truck count is signed >0;
each iteration rereads count at82b2b0e0. Full destructor hash:
1f8a77fe6e5ddda3d98f8667984e533aaa1e94baa29b82e4c12be61ffcfa3c38.
RemoveDrive hash:
3fb5cc54dfb49406ea9042f68347ab1068439ab1b0c2d38228019ded51779966.
Skipping absent handles and clearing the private host list is explicitly host
recovery from a native-fault path, not equivalent native null behavior.

## Rust implementation and validation

skateboard_truck_drives.rs owns two distinct mutable frame/dynamics slots before
any constructor effects. A failed host callback preserves completed stores and
admission observations, and phase tracking prevents a second construction
attempt. Count increments only after both native-shaped attempts, even if both
are null. Unsupported deck/linear byte profiles return at their original read
site rather than prefetching those choices ahead of effects. This fresh owner
does not expose nonzero turn/speed fields or claim later forced-turn updates.

dynamic_assembly_post_drives.rs extends the existing persistent session with
private supplemental tokens. Every Add validates attached authority and part
indices and reserves token storage before its pool call. Native null is false;
backend validation errors are returned separately. Earlier accepted handles
are not rolled back or lost on later errors. A body detach refuses until
supplemental drives are explicitly removed in ascending ordinal order. No body
capabilities or fake serialized object IDs are exported. Pool payload owners
survive removal until slot reuse/pool destruction, matching the existing pool.

Twelve focused constructor tests pass: combined callback chronology, zero
dynamics observed at insertion, same-owner later updates, all four null
combinations, delayed independent coefficients, fused arithmetic, signed-zero
trig and raw matrix lanes, partial callback/poison failures and retry/profile
refusal. Root added a noncommuting Rz/Rx inverse-parent fixture and analytic
last-Z FMA discriminator: (1+2^-23)*(1-2^-23)-1=-2^-46 (a8800000), while the
split product cancels to zero. These are controlled arithmetic fixtures, not
captured native output. Four extension tests cover real pool capacities0/1/2, forward cleanup,
authority/endpoint refusals, a deliberately injected foreign second body with
the first drive retained, and live pool reads/lifetime after payload mutation.

The expanded raw-mounted Vault corpus now loads the truck-drive class and
asserts the four scalar inputs and two profile bytes independently. Existing
source-produced7bodies/6joints are attached first; CalcTruckTransforms then feeds
fresh drive construction into actual deck6->truck4/5 pool records. Pool reads
observe final spring2222/damping40/strength4560fffe/angularSOFT and untouched
linearNO_DRIVE. Those values are assertions only, never tuning constants.
This proves local lifecycle and live payload visibility, not Jacobian/Horus
execution or a complete skateboard tick. Host trig/estimate policies remain.

Final strict root command4281 completed exit0:2310 unit tests
(2082core+187formats+38app+3symbols),6doctests, formatting and all-targets
warnings-denied Clippy, followed by all four core original-data tests passing.
The app ignored corpus was not rerun. Separate final independent implementation
review passed all constructor/numeric/ownership/corpus boundaries above with
no blocking discrepancies. The next connection is documented in
authored-constraint-solver-path.md; Drive normalization still precedes Build.
