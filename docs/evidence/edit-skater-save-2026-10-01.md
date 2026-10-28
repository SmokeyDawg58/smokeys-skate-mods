# Edit Skater native profile serialization

Source-only implementation. No
build, cargo check/test, or game launch was performed for this work. Rustfmt
parsed the two new Rust modules successfully. No original SKATER.P was found
under the configured external original-game root, so real-save interoperability
is not yet verified.

## Native container evidence (high confidence)

IDA PDB types and decompilation/disassembly recovered:

- `MemcardProfile::GetDataBufferSize` 8255fdb8 returns 0x400004 for type 'P'.
- `MemcardProfile::GenerateSaveData` 82569f48 clears bytes4 onward, writes root
  at20 with ID hash `MEMCARD_ROOT`, nested flag at32, children at36, and CRC at0.
  A version wrapper precedes each registered savable's wrapper. The root Size
  is the child byte count. `LoadData` 8256ece8 requires save version1025.
- `MemcardChunk` is16 bytes: ID u32 at0, Version u32 at4, Size u32 at8,
  nested bool at12. Values are native Xbox big endian.
- `SaveMemcardSaveVersion` 8255f6a8 emits a nested wrapper and leaf
  `IMemcardSavable::SaveMemcardSaveVersion` v1 with WriteInt1025.
- `SaveToBuffer` 8255f7a0 writes a module wrapper. CAC registers as `CAC`
  in constructor82580b60.
- `StartWriteChunk`8255f9b8, `WriteData`8255fba0, `WriteBool`8255fb10,
  `EndWriteChunk`8255f9e0: tuple is byte length u32 then raw bytes without
  alignment. EndWriteChunk adds five zero bytes, records Size INCLUDING the
  leaf header, then advances align4(Size)+16. The resulting extra16-byte
  gap is intentionally reproduced, not corrected to a conventional format.
- `ICELITE::StringHash`8297e018 is seed0xffffffff, multiply33, add unsigned
  char, wrapping u32.
- `GetBufferCRC`82cde5b8 hashes all0x400000 bytes after the CRC word.
  `GetCRC`829b30e8 initializes from complement of first big-endian word,
  then shifts left8, ORs next byte, and XORs the old top-byte table entry;
  final complement maps zero to0xffffffff. kCrcTab821f16c0 begins
  00000000,04c11db7,09823b6e,0d4326d9, confirming non-reflected04c11db7.

`formats::skater_profile` preserves unknown module/child payloads. A newly
created host profile contains only version and CAC module wrappers. This is
an explicit host scope, not a reconstruction of all native career backends or
a claim that the original console will load a CAC-only host profile.

## CAC record evidence (high confidence)

`CACBackEnd::SaveData`8256c6d0 writes leaf `CACBackEnd::Data` v7, a bool tuple,
and (when valid) a CACRecord tuple; `SaveUserGenData`82569af8 then writes int2
and four UGC slots, each WriteData86016, WriteUInt actual size, WriteData256
URL. Constructor82580b60 zero-initializes those UGC buffers. Load82573c90
requires leaf v7 and deserializes the record.

`CACRecord::Serialize`82b69768 / `Deserialize`82b6bca0:

1. u32 record version3, u32 recipe byte size, RecipeData::Pack.
2. Thumbnail15000 bytes and u32 actual size.
3. Installed26 pairs of u64 model/material IDs.
4. For each slot0..25, u32 owned count and that many ID pairs. Pad total
   owned pairs to6000 with the invalid ID. Serializer asserts at >=6000.
5. Twelve u64 colour IDs, skin-normal buffer64 bytes, Closet, Customizations.

`RecipeData::Pack`82b657b0 / `Unpack`82b658f0: u32 version7, u32 length,
recipe bytes (capacity8192), two f32 tuning values, u32 hat, stance, style,
four u32 gestures, bool gender byte. Float bits are preserved in the codec.
`CACRecord` PDB offsets: recipe0, thumbnail2030, thumbnail size5ac8,
installed5ad0, owned5c70, skin5c90, closet5cd0, customization5cd4,
colour IDs5cd8, dirty5d38. These are object offsets, not file offsets.

`Init`82b6a9e0 uses invalid part IDs and all-ones colour IDs; clears thumbnail.
Invalid IDs come from dynamic initializer82f7e278 hashing the exact string
`Null Asset ID` at82268e74 with existing rw_hash64_string, then82f7e2c8
copies the ID to model and material. Static BSS zeros are NOT that default.
`InitPlayer`82573bb0 installs male morph defaults from CASMorphParams, tuning
0.3 and0.7, and tattoo8 in all slots. Those customization defaults are caller
supplied; the codec does not invent them.
`GestureSet::Reset`82561d70 disassembly selects gestures0,1,2,3. Decompiler
omits some stack-byte stores, so disassembly was used. Stance Goofy0,
Regular1; style Normal0, Loose1, Aggressive2; hat None0.

Closet serializer82b65030 is version1, last-worn male/female indices and
outfit count, then outfits. Outfit serializer82b64b68 version3 writes17060
bytes per outfit; Vector3 helper82a6e330 writes12 bytes. The codec retains
the closet verbatim and locates/replaces only the following customization
bytes, preserving saved outfits. Customization interpretation belongs to the
existing character recipe work, not a duplicate owner here.

## Host storage and validation boundary

`core/session/edit_skater_profile.rs` takes an explicit caller-owned path.
Missing file is first use; invalid existing bytes return an error. Encoding
is read back through the parser before disk mutation. It writes a unique
same-directory temporary file, syncs it, renames any existing save to a
unique backup, then promotes the temporary file. Replacement failure attempts
rollback. Successful backup files remain available. Windows replacement has
a brief rename gap; this is not claimed crash-atomic.

Still required: compilation, original-SKATER.P corpus load/edit/save,
fresh-profile persistence across process restarts, and the complete connected
Edit Skater UI/application playtest. Source review and self-roundtrip are not
substitutes for reference runtime validation.

## Native defaults and assembly continuation

`session/edit_skater_record.rs` reads original defaults through the retained
AttributeRuntimeVault, not hardcoded corpus tuning. CASMorphParams Init825916c8
and GetDefaultSetting8258dd70 scan class7f47ca62f4100f41; all index>20 rows
update the fallback, then indices0..20 update individual zones. Male default
is layout+20, female+32. Missing fallback starts0.5 as native. Colour zone
ae1f950ca0d1adf2 is selected by layout+44; skin slots3/4/5 use zone2. Its
RefSpec+24 resolves colour6d934d0e38c95e19; baseRGB+32, highlightRGB+0, name+52.
Full class IDs were checked in raw PPC `insrdi`, not truncated decompiler pairs.

CACRecord ctor82b6ba60 allocates a closet with TWO initialized outfits and
Customizations capacities12 colours/3 stamps/6 embedded/21 morphs. Outfit
Init82b63b98 raw disassembly confirms zero morphs, tattoo8, male, regular stance1,
normal style0, invalid part IDs, and all-ones colour IDs. Unused thumbnail and
path tails are deterministically zeroed by the host; this is not a claim about
uninitialized native memory. Fresh active record uses InitializePlayer8258fb90
for default parts, colour IDs/targets and `average` skin normal, retaining
InitPlayer82573bb0 tattoos8 and truck/wheel0.3/0.7.

Default part selection82b68f78 is first matching gender/default, followed by
first default-wear-style in the same gender/group, with original-part fallback.
Group82b62cc8 is style-subcategory, or hat material variant when absent. Native
boolean fields consider both0 and255 false. PDB RawPartDataV6 field offsets
determine dependency indices; no guessed text attribute mapping is involved.

Wear82598858 uses SetInstalledPartID82b695d0: RemovePartID82b68690 removes a
chosen owned item (or clears an identical installed item), then old installed
choice goes to owned inventory before replacement. Removing nullable slots
uses ExchangePart82b6bfd8 and SetOwnedPartID82b6bbd0. Inventory padding is
rebuilt with the actual invalid ID. Source capacity assertions become checked
errors rather than host memory corruption.

CASAssembler::Correct825932f8 ordering is preserved: hair, rostral, torso,
wrist, legs, feet display/in-game selection, skateboard display/in-game selection,
component removals, colour target correction, stamp cleanup. Detailed sources:

- Hair82589808/825898d0, GetHairModelByStyle82b68460: full/none/hat requirements;
  matching hair style and gender; native first-hair fallback on failed search.
- Rostral82589a28/GetRostralID82b65538: last matching gender/skin/shadow
  style/shadow colour/facial style/eyebrow style. Bald shadow string is `0`
  (82b67cb8). Caller supplies the existing runtime CorrectRostral cvar.
- Torso82589b40, outer82585db0/82b65328, inner82585e40/82b653e0:
  required outer model/material, default outer fallback, inner-cut matching by
  full material ID, then arms82b65698 match gender/type/skin/normal. Inner arms
  precede outer arms. Alternate skin82b66d40 retains last match; the native
  switch handles skinned outer torso, not inner torso.
- Wrist82589770 requires arm SupportsWristItem=`yes` when an arm exists.
- Legs82589df0/82b67b58 match pants-required leg type, gender, skin, and sock
  style. The native leg lookup does NOT compare its normalIndex argument.
- Display/game82b650d8/82b65258 use same model-name and alternate material-name.
- Removals82591a50 operate in slot order with exact comma-separated names.
- CorrectColours8258ded0 replaces saved arms/legs/rostral and skinned torso,
  while hair/garment display selections remain separate. Stamp825900d0 and
  cleanup82585cd0 retain only logoable target slots.

No mutable alternate CharacterRecipe/CACPartDb owner was introduced. Helpers
operate on the existing parsed database, record, customization and Vault owners.

## Backend and existing actor reload

PlayerAttributeServices::apply_cac_record follows ApplyPlayerAttributes828158f0:
gestures, male, truck then wheel, character ID(all ones for CAC), regular iff
stance==1, hat. Native Apply does NOT set style; the explicit style UI/startup
caller must invoke the source SetPlayerStyle8255b640 separately.
New setters live on the existing PlayersBackendSettings selection fields;
source setters8255b120/8255b720/8255b640 do not clamp values.

Actor::ReloadCharacterData825f4690 reads the actual local Actor player index,
uses the existing GetCACSettings pro-name mapping and current backend fields,
then writes that Actor's retained SkaterAnim. SetCharacterData82cae1d0 preserves
relative stance/mirror-bit behavior, gesture order and null-pro-name no-write
semantics. SetProSkater82cb7ba0 is a direct FastString assignment. This does
not recreate animation owners or change physics pose/reset state.

`session/edit_skater_player_attributes.rs` is registered as a child of the
existing animation-construction module to access that same private player
field owner. Loader CAC attribute preparation retains primary/ghost/network
ordering. GameModule::ReloadLocalPlayerAttributeData82729d38 remains a caller
continuation for the actual matching UI owner; apply_cac_record returns whether
a local Actor existed after completing its reload.

All continuation changes remain source-only and rustfmt-parsed. Compilation,
original-data defaults/assembly runtime and persistence playtests remain
pending. No claims of rendered customization or full runtime parity are made.

## Mounted CAC replacement and registration

`player_mounted_recipe_load.rs` now reuses the existing load request/recipe
registry/face-completion path for CAC replacement. The old registered Arc must
match exactly before releasing its one registry reference. Presentation users
retain their own Arcs until detachment. CAC gameplay uses clone type3/category3;
the host reconstructs resources from the original arenas, so customized clone3
requires the same selected-RModel material rebind as its resident native master.
Rebinding is restricted to the selected instance's meshes/material indices.

The original PresManager Update82818f2c completes registration only after the
actual published presentation recipe has the matching low32 peer identity.
`complete_presentation_registration` preserves that pending condition and only
then calls the existing loader completion and frees the shared registration
request. No second physical Actor registration is introduced. The CAC loader's
SetAsSkater branch82819cb8 applies physics, hat, character, stance, **style**,
gestures and sex before the outer load callback; this is separate from the
outer ApplyPlayerAttributes operation described above.

## Original ROPA assets and parsing

Default male outer torso is the baggy pullover hoodie, whose LOD0 instance is
`da4611b991216196`. Original `data/big/ropa2.big/.bh` contains its genuine
`data\\ropa2\\0xda4611b991216196.r2b`, 795628 bytes. The magic is
`ROPA2FILE_0.5`, followed by ten little-endian header words:
`1297,1297,0,5223,3926,0,1297,1,0,1297`. File existence alone does not admit
this model to cloth: InitCloth first checks the selected mesh's material flags.
The default hoodie may therefore take the no-candidate path despite having
this original effect asset. Read-only inventory finds 1297 authored masses of
1.0 and static flags0; zero-range driver constraints subsequently pin particles.

`formats::ropa::RopaEffect::parse` follows ParseLoadedFile82860100,
LoadHeader8285df58, LoadParticles8285af58 and LoadDistanceConstraints8285ea68.
Header order is particles, plane volumes, sphere volumes, total constraints,
distance constraints, sphere constraints, plane constraints, static forces,
dynamic forces, drivers. Native loaders consume these record strides:
global52, plane116, sphere88, particle88, driver240, distance48, plane/sphere
constraint24, static force76, dynamic force120. Authored global, collision,
plane/sphere constraint and force contents are deliberately unused by these
native loaders; they are not invented as extra solver inputs. Constructor
8285fd48 creates two active constraint sets: distances then drivers.

Particle XYZ are at record+0; static flag+80; mass+84. Inverse mass is1/mass
when mass>=1e-5, else1. Driver i initially references particle i. Distance rows
use particle+4, driver flag+8, reference+12, min+16, max+20. Driver rows update
the matching driver range; zero/zero marks that particle static. Non-driver
rows are packed in the original conflict-free eight-lane order, including the
rotating exchange with preceding full octets (8285ef90..8285f5b4). Counts and
indices are bounded by the native signed16 particle and16-bit octet limits and
by the supplied file size; no unrelated capacity constants were introduced.

InitCloth827d7a38 identifies candidates through the first optimized mesh's
material flags low bit and caps live effects at4. Raw code loads mesh+0x24,
which the optimized-mesh PDB layout identifies as the material pointer, then
tests flags at material+4. The decompiler's non-optimized mesh type misleadingly
labels that pointer as a MeshHelper; vertex-buffer count is not this predicate.
Missing r2b
files produce no effect. ClothInit827d7d88 uses decoded position.W as particle
index; last duplicate vertex wins and every particle must map to a vertex
below0x8000. Its explicit failure causes native effect deletion, rather than
disabling all cloth. Actual loaded effects require the solver path.

## Shipped vector ROPA graph

`rendering/skater/skater_ropa_solver.rs` owns mutable particle state only.
Construction takes the retained parsed effect and the original W mapping;
`step` takes skinned world-space driver positions/normals, native dt/reset,
effect scale and live cvar tuning. Returned positions remain in world space.
The runtime caller owns skinning, inverse-hips conversion and vertex publication.

Dynamic initializers82f716e8 and82f71788 select vector solver1 and vector
integration. This matters: vector TCVIntegrate8285ac38 does **not** read the
vertical-speed-dampening field used by scalar8285c8d0. It preserves previous
position flags and integrates movable mapped particles using one refined
reciprocal of lastDT, fused velocity plus current+gravity*dtSquared.

DistanceSolve8285b190 is transcribed by corrected raw VMX128 register fields,
not IDA's misleading extended source-A labels. Decoder:
`VD=((w>>21)&31)|(((w>>2)&3)<<5)`,
`VA=((w>>16)&31)|(((w>>5)&1)<<5)|(((w>>10)&1)<<6)`,
`VB=((w>>11)&31)|((w&3)<<5)`; ordinary VMX operations retain their own format.
Two unusual native vector details are preserved:

- Final position transposes inject v0=[1;4] as W at8285b4a8/b508 and store
  atb4c0..b534, so inverse masses become1 after participating in a pass.
- v41 atb43c is deltaX*inverseMassB*minimumFactor. Atb470 it is negated and
  selected only for a static reference atb4a4; b4e0/b4e4/b4e8 subtract that
  same value from all three A correction coordinates. This differs from the
  scalar variant and is not replaced with a textbook distance correction.

DriverSolve8285b848 keeps the original min>0 reversed-normal selection,
vertical and tangential distance scaling, ellipsoid correction, normal
halfspace projection and effect-scale blend. It leaves static-particle previous
XYZ unchanged, unlike the scalar variant. Distance passes use NbIterations;
the driver constraint set retains its constructor iteration count1.

DoUpdate8285fa50 default FixedDT uses the zero-initialized30-element dt ring
and its mean (first normal dt/30). Native reset initializes lastDT to that
mean. The optional variable branch uses maxDT82ff0278=1.0 and
maxIter82ff0274=50; raw8285fc4c stores the whole input frame dt into lastDT
after each substep. Native reset in ClothUpdate827d24f8 copies skinned drivers
to current/previous without invoking integration and leaves the ring intact.
Host estimate seeds and fused arithmetic are the same explicit numerical
boundary as other host presentation graphs, not claimed Xenon bit parity.

Native skinning evidence for the runtime adapter: DoSkinMesh_vector82841488
handles strides32/40 by decoding all four stored weight bytes and normalizing
their sum with one Newton reciprocal refinement. The scalar function's
`1-sumXYZ` policy must not be used for that shipped vector path. InitSkinParams
827d2938 reads material_charattributes/default array index1 for fOffset.
Original skater.vlt/.bin confirms class38225df1cffaffaa, collection
d7edbd362d7d2152, array30407bb86613ecc7, index1=[0,0.8,0,2]. Decode is
position*fOffset.w+fOffset.xyz with position.W set1. WriteDestination8285cae0
applies inverse hips/system transform before packing positions; it does not
recompute the already-skinned normals.

Parser/solver sources were rustfmt-parsed only. Read-only original asset
inventory and IDA/raw instruction inspection were performed. No compilation,
unit tests, game launch or runtime/rendering validation was performed here.

Runtime adapter review: DoSkinMesh_vector82841488 expands each skin-weight
byte across a word, shifts right one and converts unsigned fixed-point31
before normalizing all four weights (82841760..828418e4). The adapter now
preserves that conversion. vec_put_short4_clamped8285a060 clamps [-1,1],
converts signed fixed-point31 and retains the high halfword; negative
fractions floor at that halfword boundary. HEND output converts/truncates
then masks 11/11/10 bits, without an added component clamp. The full vector
function contains no reciprocal-square-root/square-root or Normalize call;
normal/tangent/binormal vectors are transformed then packed directly.

The entity constructor827e5884, Update827e8568, InitSkinParams827d2938 and
BindConstants827dcf50 do not rename material effects. InitSkinParams copies
the original first mesh material pointer into its CPU-skinned model copy.
BindConstants publishes the same i_vtx_params pointer on every current-LOD
model; successful ClothInit changes its X to zero globally. Effect-family
selection therefore belongs to the earlier material/recipe owner. Source
review and rustfmt parsing only; no execution parity claim.

Destination attribute follow-up: DoSkinMesh_scalar828367f8 writes only
position, normal, tangent and binormal through SetUncompressedVector; it
does not replace skin weights/indices. The vector32-byte branch beginning
82842418 retains packed input words3,5,7. Raw VMX128 transpose at828424fc retains
word3 in v38,82842528 retains word5 in v37,828424c4 retains word7 in v62;
these registers remain unchanged until the output interleave at82842f84,
82842f90 and82842fc0 and bulk destination stores82842fdc onward. That branch's
descriptor correspondence has not been established: the default hoodie's
serialized words3/5/6/7 are UV/binormal/weights/indices respectively. Thus
this vector trace alone is not proof of weight preservation for that descriptor.
The scalar path establishes no rigid rewrite there; no native rigid255/bone0
rewrite has been identified. Host rigid attributes in the CPU
view are an adapter for its already-skinned stream, not native buffer bytes.
