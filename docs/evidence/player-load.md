# Offline local-player load

Confidence is `high` for the selection, request, default-marquee asset, and
version-7 recipe structure documented here. Asset-ID resolution, the real
presentation hierarchy, and character skin-palette mapping are also validated;
compressed animation sampling and skater simulation construction remain open.

## Selection before `GameLoad`

`GameModule::GameLoad` (`0x8273be50`) does not construct a skater. It obtains
`Simulation::mLocalPlayerLoader`, starts it when unloaded, and advances only
after `PlayerLoader::IsLoaded` succeeds.

The front end configures slot zero before that wait:

- `FrontEndState_FreeSkateOption::UpdateGameSettings` (`0x8264b268`) selects
  either the chosen marquee collection or the local CAS record, then calls
  `Load`.
- `FrontEndState_MainMenu::EnterGame` (`0x826788b0`) normally requests CAS and
  uses a fixed marquee for the marketing demo.
- `StateMainMenu::FilterImpl` (`0x82770510`) requests
  `MarqueeDefs::GetDefaultMarquee` when profile/CAS data is unavailable.
- `Simulation::Init` (`0x828a6a70`) also configures the loader from
  `CommonBackEnd::mPlayerCharacterName` through the filename overload.

`PlayerLoader` contains four `PlayerLoaderImpl` pointers and constructs slots
0 through 3. The implementation is 120 bytes in the PDB. Its relevant fields
are requested selection at offsets `0x10`/`0x18`/`0x20`, current selection at
`0x28`/`0x30`/`0x40`, player index at `0x38`, current request at `0x48`, five
pending unload pairs at `0x4c`, and callback at `0x74`.

The constructor (`0x82816dd8`) starts with requested type `INITIAL_STATE` (2),
an out-of-enum current-type sentinel (3), invalid marquee/peer IDs, no current
request, and five unused pending-unload slots. The named selection values are
`MARQUEE = 0`, `CAS = 1`, and `INITIAL_STATE = 2`.

## Asynchronous protocol

`PlayerLoaderImpl::Load` (`0x8281aa80`) first short-circuits an already loaded,
unchanged selection. Otherwise it copies requested selection to current and
uses `SimCharacterManager`:

1. A marquee selection resolves its collection ID with
   `MarqueeDefs::GetSkaterName` and sends a load request in
   `Category_Marquee`.
2. A valid CAS selection attaches the serialized recipe and sends a
   `Category_CreateACharacter` load. Invalid CAS data switches to the default
   marquee.
3. `SimCharacterManager::Load` (`0x8281a1f0`) emits the useful fields of a
   92-byte `cMsgPresCharacterLoadRequest`: request ID, category, 64-byte model
   name, and local-player flag.
4. `PresCharacterManager::ReceiveSimCharacterLoadRequest` (`0x8281b6f0`)
   allocates a managed request. A marquee name is mapped back to its collection
   ID and passed to `RecipeFactory::CreateMarqueeLoader`.
5. `PlayerLoaderImpl::OnSimCharacterLoaded` (`0x82817f28`) stores the returned
   character asset, applies player attributes, and replaces the load request
   with a simulation-registration request. A stale completed load is scheduled
   for immediate unload.
6. `OnSimCharacterRegistered` (`0x82818098`) completes only the matching
   request/asset pair and clears the current request.

`crates/skate2-core/src/session/player.rs` expresses this boundary as a typed native
state machine. It owns selection, asset-loading, registration, stale-result,
and loaded phases; it contains no emulated message queue or CPU state.

## Exact default marquee and recipe

`MarqueeDefs::GetDefaultMarquee` (`0x82cfdbe8`) returns:

```text
0x2275b90050166969
```

The generated `characters_marquee` constructor (`0x824b8eb0`) uses class ID
`0x91bca6693efc9aa7`, which matches the native lookup8 hash of
`characters_marquee`.

`GetSkaterName` (`0x82cfe920`) is an important accessor special case: it reads
the first pointer from the collection's compiled `mLayoutPtr`; it does not call
`Collection::GetNode`. Following the real collection export's `mLayout`
fixup and its layout-offset-zero `EA::Reflection::Text` pointer resolves the
default ID to:

```text
tutorial_male
```

`rw::RwHash64String` (`0x82bb5800`) is 64-bit FNV-1 with initial value
`0xcbf29ce484222325` and prime `0x100000001b3`. The reconstructed operation
gives `tutorial_male` schema asset ID `0x4d12b500299d209d`.

`CharacterAssetManager::UpdateRequest` (`0x82d14ac0`) checks the presentation
asset registry first. If absent and binary recipe loading is enabled,
`Character::GetRecipeFileName` (`0x82cfdfc0`) builds:

```text
data\content\marquee.big|data\content\recipe\marquee\tutorial_male.recipe
```

The mounted archive contains that exact inner path. The native VFS reads the
1,956-byte recipe successfully. XML mode would instead select the same name
under `data\content\recipe\marquee\tutorial_male.xml`, with a graveyard XML
fallback after a failed load.

## Binary `CharacterEntity` recipe

The binary path does not use `CharacterDescriptionReader`; that class is the
XML reader. `CharacterAssetManager::BackgroundThread` attaches the recipe to a
`Sk8::ByteStream` and calls `Character::CharacterEntity::Unpack`
(`0x82d11d90`). Its inverse `Pack` (`0x82d01338`) provides an independent write
contract.

IDA's imported PDB layouts confirm the decoded container semantics:

- `tComponent::models` maps `assman::cAssetID` to `tModel`;
- `tModel::lods` maps an unsigned LOD byte to `tLod`;
- `tLod` owns serialized `assetid` and `arenaassetid`, a runtime-only
  `loadedassetid`, and a vector of material instances;
- `tMaterialInstance::materialVariants` maps asset IDs to `tMaterial`;
- `tMaterial::shaderparams` maps channel strings to `tShaderParam`;
- `tShaderParam` owns serialized `assetid` and runtime-only `loadedassetid`.

The exact version-7 stream is big-endian. A string is `u32 byte_length`
followed immediately by that many bytes, without per-string alignment. The
body contains version, schema, category, memory category, clone type, then the
nested component/model/LOD/material graph. A one-byte customization flag
follows it. Version 7 pads with zero bytes to a four-byte boundary and ends in
a `u32` body-size footer. The default recipe's footer is 1,952 and its file is
1,956 bytes, exactly matching `Unpack`'s consumed-byte assertion.

`skate2-formats::recipe` now parses this as domain types with bounded counts,
UTF-8 checks, zero-padding validation, and an exact footer/trailing-data check.
The real default recipe decodes to:

| Field | Value |
| --- | ---: |
| Components | 10 |
| Model selections | 10 |
| LODs | 20 |
| Material instances | 20 |
| Material variants | 20 |
| Shader parameters | 48 |
| Distinct serialized asset references | 83 |

The component names are `Arm`, `Hair`, `Rostral`, `Feet`, `Organ`,
`SkateBoard`, `Pants`, `Hat`, `InnerTorso`, and `OuterTorso`. The recipe has no
customization payload. Customized CAS serialization is kept as an explicit
unsupported branch until `Customizations::{Serialize,Deserialize}` is lifted.

## Asset extraction and standalone RX2 arenas

`CharacterEntity::Load` (`0x82d0fad0`) calls `ExtractAssetList`
(`0x82d0c348`) and `RequestPendingLoads` (`0x82d0f738`). This resolves the
apparent ID mismatch without inventing an arithmetic conversion:

- each LOD geometry request uses its serialized `arenaassetid` in
  `data\content\marquee\model\tutorial_male\<component>\0x<ID>.rx2`;
- its runtime `loadedassetid` is FNV-1 over the full compound BIG path;
- each shader parameter requests
  `data\content\marquee\texture\0x<ID>.rx2`;
- a texture's runtime `loadedassetid` is FNV-1 over the `0x<ID>.rx2` basename.

The Rust `asset_requests` implementation reproduces those named rules. The
default recipe emits 20 geometry and 48 texture requests. Texture reuse reduces
the latter to 22 unique files, so the complete default character set is 42
unique RX2 arenas. Every path exists in `marquee.big`, and all 48 texture
loaded IDs match `marquee_texture.toc`.

Standalone RX2 files begin directly with an RW4 arena, unlike streamed XSF
payloads with an 80-byte compression envelope. Their five resource buffers are
stored back-to-back at exact serialized sizes; descriptor alignment belongs to
runtime destination allocation and does not pad the file. The native splitter
and existing RW4 decoders validate all 42 files: 5,249,932 bytes and 328 typed
objects with no unknown object type.

The twenty geometry arenas contain 20 render-model/mesh/material graphs and decode to
14,026 native positions, 103 non-position attribute arrays, 58,755 indices,
and 20 draw packets. Their model records declare 426 named bone slots and
inverse-bind matrices, 260 packed model matrices, and 260 u16 mesh-palette
entries. This
path added the PDB-named `SHORT4N` position and `FLOAT2` attribute formats that
the world corpus did not exercise. The twenty-two texture arenas decode all
244 mip/face surfaces to 3,594,184 bytes of native-order block data.

`CharacterEntity::BuildTInstanceArray` groups loaded `pegasus::tInstance`
records by their serialized LOD byte. `PresentationEntity::InitFromRecipe`
then creates one model instance per selected part through the instance's
`m_pRModel`. The native `CharacterScene` now preserves that same chain: 10
components/model selections, 20 LOD arenas, 20 Pegasus instances, 20 render
models/meshes/materials, and 22 texture assets. Of 88 raster-shaped material
channels, 48 resolve to those textures and 40 are the two named metadata
channels per material; no sampled channel remains unresolved.

`SkaterPresEntity::InitSkinning` (`0x827e4b48`) calls `RemapBones`
(`0x827f8138`) and `FillInvBPBuffer` (`0x827f8338`). Model bone names map by
case-insensitive hierarchy name with `Hips` as the missing-name fallback, which
matches Andale's recovered `FastString` assignment semantics. Inverse-bind
matrices are written into hierarchy order. `UpdateBoneTransforms`
(`0x827efb78`) separately walks each mesh's u16 remap table to gather animated
hierarchy matrices into the packed model palette. The readable Rust
implementation covers all three rules and now instantiates them against
`data\anim\SkaterPresentationDB.abin`: its `rig_tpose` hierarchy contains 148
bones in 18 parts. The two real character LOD levels map all 426 named model
slots with zero fallback bones and gather 131/129 packed matrices.

The matching type-3 `rig_tpose` pose is structurally decoded into its 18
animation parts, including bounded compression header/data regions. Its exact
packed spherical-quaternion and float-translation selector combination now
materializes all 148 authored bind SQT transforms. The body animation databases
provide 3,004 named OnBoard/OffBoard clips. Their timing, looping, part layouts,
ordinary VBR frame blocks, inverse DCT, SQT assembly, and normalized
interpolation are decoded in readable Rust.

The real `r_antic_360shuvit_l_0_cyc` frame produces 36 body transforms across
eight named hierarchy parts. Those ranges overlay the full bind pose, evaluate
through the reference hierarchy rule (including trajectory-parent resets), and
multiply by the real inverse-bind buffer before `UpdateBoneTransforms` gathers
the mesh palettes. The focused probe produces 148 finite skin matrices, all 131
LOD-0 and 129 LOD-2 packed matrices, and zero fallback bones. Bind globals times
inverse bind stay within `0.030320402` of identity across the 106 model-used
hierarchy bones.

## Open boundary

- Resolve the isolated VBR 256-significance-flag edge and unused raw selector
  combinations, then validate native pose and skin matrices against a captured
  reference frame.
- Reconstruct the specialized hand/face overlays and hips, truck, and board
  corrections visible later in `SkaterPresentation::ReadSkater`.
- Recover animation-controller construction and Andale record types 6 through
  10 before selecting/blending a complete animated skater state.
- Lift customized CAS serialization after the stock marquee path is working.
- Reconstruct `ApplyPlayerAttributes` values, character registration, skater
  entity construction, and the later physics-controller ownership graph.
- Add CAS recipe loading after the marquee proof-of-concept path is end-to-end.
