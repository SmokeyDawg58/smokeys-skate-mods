# Original character GPU skinning contract

2026-09-12, bounded source and original-asset audit. Confidence is **high** for
the shader formulas and raw CPU packing below, **medium** for the complete
selected material route until the retained recipe material overrides and Vault
effect lookup are run together. No skinning implementation, Cargo run, synthetic
trick pose or original asset publication accompanies this packet.

## Actual assets and selection boundary

Read original `data/content/marquee.big` named entries directly and decompressed
them with a read-only Python transcription of the existing BIG/RefPack readers.
The binary `data/content/recipe/marquee/tutorial_male.recipe` is 1,956 bytes,
SHA256 `fe8078e1f89efd248c63fb64909e8ec2d80cf2d2677559495082d17173fd919b`.
The XML companion is version 6 and is corroboration only; the runtime uses the
version-7 binary decoder. Twenty original RX2 model files were extracted outside
the repository into `%TEMP%/skate2-original-character-shaders` for inspection.

The RX2 string census finds these serialized material names:

| Parts | Original material name in both LOD arenas unless noted |
| --- | --- |
| Arm, Rostral, SkateBoard | character.default_skin |
| Feet, Hat, InnerTorso, Organ, Pants | character.default_cloth |
| Hair | character.default_hair |
| OuterTorso | LOD0 character.default_cloth_ropa; LOD2 character.default_cloth |

This census locates names, not a substitute for structured channel selection.
The recipe also carries material overrides: marquee_skin_light for Arm/Rostral,
marquee_skin_skateboard for board, marquee_eyes for Organ, marquee_hair for Hair,
marquee_cloth for ordinary clothing and marquee_cloth_ropa for OuterTorso LOD0.
The recipe's component/LOD arena IDs match the twenty named RX2 entries.

Fresh `CreateEffectListFromPegasusMaterialSkinned82e20828` reads metadata channel
1. Only a null name invokes character.default_cloth or character.default_cloth_ropa
according to material flags bit 0. It resolves an actual Vault collection before
building the effect list. Therefore the names above alone do not prove that every
part selects the same vertex shader. `own_character_scene` currently retains the
override records but does not execute full native material-scope mutation.
Root was asked to run the actual selected material/vertex descriptor census when
mounting the retained character loader. That is still a required admission gate.

## Original shader evidence

Read final shader UPDB files from original `data/big/render.big`. They contain
preprocessed HLSL, constant reflection and source maps (compiler 7645). These are
reference assets, not an emulator translation. Originals remain outside the repo.

| File | Bytes | SHA256 |
| --- | ---: | --- |
| defaultcharacter_defaultVS.updb | 95108 | ece4f1298a331cbd0d9109aa4754c0c1fe151851a83a4a5e44492225e9fc3ed6 |
| defaultcharacter_ropa_defaultVS.updb | 94626 | 46f721f4345c8c9fa0398b9d12d104773451b12e58f8066faf7b23a6c6ba5a73 |
| cacstamp_skin_defaultVS.updb | 95252 | 751b16709669eeb43a23da870a8635387a42cc3e999655023512f08c3e103bf6 |

The corresponding XML hashes are defaultcharacter
`63ce26daf125f77f3f3d9bd7aa2f808ad4a3db69fe0abd2b44c2e3ad55d2b03d`,
defaultcharacter_ropa
`c02efaf454e74177d4a5f1d885d1b21ca8e3203d5f92111c2f072a20abcbb447`,
and cacstamp_skin
`262e08ec3f3361ecc57c2bb63ed6136ad854cfd0455d405b15da7f36487e2081`.

`character_skinning.fxh:23` GetSkinMatrix_4 is shared by all three shaders:

1. Bone indices are multiplied by 3 to address three float4 rows per bone.
2. Nonzero offset selects division of all supplied weights by 255. All three
   selected candidate defaultVS calls pass offset 0.8, so this branch is active.
3. Weight W is overwritten with `1 - dot((1,1,1), weights.xyz)`. Never normalize
   all four stored weights or use the stored fourth weight as the final weight.
4. Each output row starts with X-weighted bone data, then adds Y, Z and W terms
   in that order. No clamp is present for negative residual weights.

Standard defaultcharacter and cacstamp_skin defaultVS expand input position with
`OffsetPos((0,0.8,0,2), position)`, i.e. position*2+(0,0.8,0), then append W=1.
They dot this vector against the three blended rows, set output W=1 and multiply
the resulting world position by g_matVP. There is no subsequent i_matWorld
multiply in these entrypoints. Existing SHORT4N decode only normalizes components;
it does not already perform this position expansion.

The three row XYZ values also transform tangent/binormal; the standard shader
derives its normal from these transformed directions. UV0 is passed through;
cacstamp_skin additionally passes UV1. Full material lighting remains outside
this geometry contract. HLSL half/float declarations and compiler optimization
require compiled shader/reference validation before numerical parity claims.

The ropa entrypoint has an additional **live** `i_vtx_params.x > 0` branch. True
uses the same skinning calculation and transforms an explicit input normal.
False expands position identically but uses i_matWorld and no bone weights.
This branch corresponds to real CPU skin/cloth integration; it cannot be hardcoded
true for the local player's clothing. The helper GetWeightsAndIndices exists in
shared source but these entrypoints consume separate BLENDINDICES/BLENDWEIGHT;
do not select the packed helper merely because its name looks relevant.

Reflection independently declares i_boneArray as 183 float4 constants (61 bones).
Standard/cacstamp use c7..c189; ropa uses c8..c190, i_matWorld c191/count4 and
i_vtx_params c7. Standard/cacstamp have no active i_matWorld constant.

## Actual CPU matrix ownership and packing

`SkaterPresEntity::InitSkinning827e4b48` uses the current LOD's actual model array,
calls RemapBones with retained mHir and fallback Hips for each part, then invokes
FillInvBPBuffer at4d4c. Existing `CharacterScene::build_skinning_state` represents
these operations; the hierarchy must be the actual presentation hierarchy.

`UpdateBoneTransforms827efb78` copies full 64-byte matrices from the actual
hierarchy result, using mesh palette u16 -> part remap u32 -> hierarchy index.
Raw fc64/fc7c and four lvx/stvx pairs fc80..fc9c prove this indirection. It walks
meshes forward, appending palettes. It gates model total bones at <=80 and asserts
mesh palette count <=61. Its MatLtoW parameter is not used; world placement occurs
later, not by adding a transform during palette gathering.

`cModelInstance::PackAndMultiplyMatricesForUpload828221e0` reads each full matrix,
composes it with actual localToWorld, transposes the composed XYZ components into
three float4 rows, and writes 48 bytes per bone in place. Raw source stride is
64 at229c; destination increments are 16+16+16 at227c/2294/22ac. Corrected VMX128
operands: 22240 word1577e8b1 is VD11,VA55,VB61 (not IDA VA87);
222e8 word1a56ab25 is VD50,VA54,VB53;
222ec word1a36ab65 is VD49,VA54,VB53;
222f0 word1a149b25 is VD48,VA52,VB51.

For each source axis vector V, the arithmetic is L.xAxis*V.x, followed by fused
L.yAxis*V.y+prior, L.zAxis*V.z+prior, L.wAxis*V.w+prior. Standard VMADD instruction
operands at22a0..22d4 confirm this ordered graph. Merge-high/low words then form
the three rows consumed by the shader. Both animated matrix W components and
localToWorld W components participate before the last output row is discarded.

`cModelInstance::Resolve82853de8` selects i_boneArray for skinned models, with
three float4 constants per palette entry. Raw540bc reads mesh+48 (palette count),
540c0 multiplies by three, and540f8 binds that count and the retained matrix pointer.
Raw5421c/5422c/54234/54238 advances the pointer by palette_count*3*16 for each mesh.
The decompiler incorrectly calls mesh+48 a blend-shape pointer; the raw offset
and existing optimized mesh layout resolve the type.

`SkaterPresEntity::EndJobs` calls the packing function at827dbe24 with actual
m_MatLtoW, after any pending local-player CPU skin jobs. EndSkinJob and cloth
owners are therefore a real selected ropa dependency, not a generic GPU-only
replacement opportunity.

## Minimal next interfaces and blockers

Keep a character submission owner distinct from static world scene indices.
Use existing `character.rs` hierarchy remapping, inverse binds and per-mesh palette
ranges, plus `animation::apply_inverse_bind` after a checked actual pose snapshot.
The new GPU consumer needs original position format, BLENDINDICES/BLENDWEIGHT
formats, per-mesh palette rows and actual placement; no named-trick pose enters it.

The body animation's synchronized buffers and AnimOutPhysIn publication are real
owners, but they are not automatically the complete 148-bone presentation pose.
Validate names/parts and required hand/face/hair/cloth producers against the
selected presentation hierarchy. Missing matrices cannot be filled with a fresh
bind/default pose and called live output. Actual SkaterPresentation data/root
publication and presentation scheduling remain an independent source boundary.

Proposed exclusive work: new `render_character.rs` admission/submission adapter,
GPU skinning child under `gpu_renderer.rs`, and root-owned narrow checked
actor/presentation snapshot API. First gates must include actual binary recipe
override->Vault effect selection, original vertex format/index ranges, residual
fourth-weight cases, position expansion once, palette subset offsets, world
transform once, and actual published pose generation. The formulas above enable
that work but do not yet prove every first-skater part admitted.

## Follow-up: selected Vault effects and live ropa branch

2026-09-12, read-only follow-up using the existing `inspect_vault` executable,
original VLT/BIN relocation reader and fresh IDA. No new probe or Cargo run.
This narrows the material-selection uncertainty above; it does not establish
an executed character submission or the complete presentation pose.

Original `skater.vlt` (851568 bytes, FNV64 f7db0ed527db57b4) and `skater.bin`
(812520 bytes, FNV64 6b4fc086777fe4df) resolve class `material_character`
(`b6b8997286f19549`), field `effect` (`2dd8d97eff11667e`), layout offset 0,
size 4, flags 6. There are no explicit effect-entry overrides in the inspected
class collections. The cached collection layout therefore supplies the field,
as in the app's existing `resolve_attribute_location` fallback after inheritance.
The layout relocation points to these actual BIN strings:

| Collection | Collection ID | Parent | BIN layout offset | Effect string offset / value |
| --- | --- | --- | ---: | --- |
| default | d7edbd362d7d2152 | none | 611680 | 46794 / defaultcharacter |
| default_cloth | 54b5e91f721e18ec | default | 613456 | 46794 / defaultcharacter |
| default_skin | 51ae5b5a26244642 | default | 613616 | 46794 / defaultcharacter |
| default_hair | 7fa4983afcdda114 | default | 613776 | 46937 / defaulthair |
| default_cloth_ropa | 4207a9291ab4cd31 | default_cloth | 614176 | 47014 / defaultcharacter_ropa |

The recipe's `marquee_*` material type strings are not collections in this
class. `CharacterEntity::RebindComponentToMaterial82d08fc0` uses the component's
selected material shader-parameter map to find each original channel by its
shader-input name. The retained asset ID at parameter+8 is resolved through
created textures (92b0), swapped IDs (92c4), then the actual registry and missing
texture ID when needed (9314..9358). Raw 9364..936c writes the resulting texture
pointer to channel+1c. This path does not replace the channel-1 Attribulator
material-name string. Thus a recipe material type must not be used as an effect
name, nor should `cacstamp_skin` be assumed merely because the recipe is editable.
Other customization/stamp operations remain separate admission requirements.
The app currently retains these material instances in `own_character_scene`;
retention alone does not execute the native texture-rebinding operation.

Consequently, the selected ordinary skin/cloth parts use `defaultcharacter`,
Hair uses `defaulthair`, and OuterTorso LOD0 uses `defaultcharacter_ropa`.
OuterTorso LOD2 uses ordinary `defaultcharacter`. This combines the earlier RX2
name census with actual Vault data; the final structured per-draw channel and
resident texture binding still needs to run through the retained character scene.

The newly checked original final `defaulthair_defaultVS.updb` is 90336 bytes,
SHA256 `1acf6000adf342c6631f85beb902c2fb4044aad072325d4306e7b13987e228e2`;
final `defaulthair.xml` is 10199 bytes,
SHA256 `c02fb780500f79a8629d5e9571c872f16bf5d6cc29048c4c1309de6e3cdf07c1`.
Its defaultVS uses the same four-weight, three-row skinning, 0.8 offset selector,
position expansion and world-space output described above. It additionally
passes UV0 and UV1, and performs `GetZBias(z, 0)` before output. The pixel shader
reads `i_alpha` using UV1 and multiplies resulting alpha by `i_params[0].x`.
An opaque diffuse-only hair draw therefore does not implement this material.
Some meshes on this effect (glasses) carry no TEXCOORD1. Their UV1 reads zero XY:
`D3D::PatchVertexShaderToMatchVertexDeclaration` 0x82c43db8, for a shader input with
no declaration element (0x82c43eb0..bc), sets swizzle 0x9250, constants (0,0,0,1).

PDB `SkaterPresEntity` places `i_vtx_params` at +910. Raw constructor
827e5960..59d0 loads its existing full vector and inserts X=1 using vrlimi mask 8;
it does not initialize YZW in this write. Constructor cloth admission first
uses actual local-player, schema comparison, cloth-disabled state and live
`cv_EnableRopa` (827e58b4..5944). Do not infer that ropa is ready from recipe type.

`SkaterPresEntity::Update827e8568`, after the actual effect-loading and
successful `ClothInit` path, marks `mbROPAInitDone[index]` and inserts X=0 at
827e87b4..87f0, preserving the other lanes. It then runs `SkinModel` (87f4),
`WaitForJobToFinish` (8800), and `DoubleBuffer` (880c). `DoubleBuffer827d2840`
copies the retained CPU output into the current real vertex buffer, publishes
that buffer to the cloth mesh at2908, then advances its three-buffer index.
Keeping X=1 after this transition or setting X=0 without that vertex producer
would both be incorrect.

`SkaterPresEntity::BindConstants827dcf50` binds the same +910 vector by pointer
to every current-LOD part (827dd0e4), count 1, instance scope. Raw cf9c and d0c8
identify the named `gPixelParam` block even though the compiled ropa vertex
shader consumes this parameter; preserve the native binding mechanism instead
of inferring a different owner from the shader stage. The separate
`CACPresEntity::BindConstants827df6f8` sets static `fDoGPUSkin.x=1` and binds it
under `i_doGPUSkin` (name proven by initializer82f6cb18). That is not
`i_vtx_params` and cannot substitute for the ropa branch producer.

Remaining first-skater requirements are concrete: actual full hierarchy pose and
per-mesh packed palettes; original recipe texture rebinds/residency; retained
Skater presentation constants and cloth-loading/CPU-vertex transition for ropa;
hair UV1/alpha state; actual render effect technique and draw-state selection.
The shader family and branch producer now have high-confidence source/data
evidence. Full shader/material numerical parity and complete live admission remain
unproven. Original shader files remain outside the repository.

## Bounded GPU consumer implementation

`apps/skate2-pc/src/gpu_renderer/character.rs` now implements the authorized
diagnostic unlit diffuse character path. It accepts original SHORT4N positions,
UBYTE4 weights/indices and decoded original UV0 (FLOAT16_2, FLOAT2 or SHORT2N).
Other forms fail admission instead of assuming equivalent weight normalization.
Every original index must fit the actual per-mesh palette, including zero-weight
and fourth entries. Existing indexed draw/base-vertex validation is reused.

The caller supplies exactly three already-composed float4 rows per palette bone,
the actual world-to-clip matrix and source generation. The upload preserves all
row bits. Constant-buffer padding beyond the supplied palette cannot be addressed
by admitted indices and is not a substitute pose. Subsequent writes reject an
older generation. The GPU applies the source weight/residual and position graph;
hair retains its additional zero-bias division/add. Ropa admission requires the
actual retained X value to compare ordered greater than zero; missing X and the
CPU branch produce distinct typed errors before GPU mutation.

These objects enter the existing map render/capture pass with their own vertex
pipeline and palette binding. Fragment output is deliberately opaque unlit
diffuse, with the same diagnostic display transfer as map inspection. Creation
logs that hair alpha and original material shading are pending. The child does
not load an Actor, create a pose, select a recipe texture or replace CPU cloth.
Three CPU tests cover original byte retention/index admission, exact live rows
and all non-GPU ropa predicates. They do not validate shader arithmetic on GPU.
No Cargo or GPU run was performed by the implementing agent; root integration,
independent formula review and actual device validation remain required.

Follow-up integration caches three family pipelines and the shared palette layout
once in `GpuRenderer::new`. Existing map-window startup now compiles/validates
these programs on the selected GPU without constructing any character object,
palette or pose. This tests shader admission, not a live skater draw.
The ignored app test `original_tutorial_male_lod0_character_vertex_admission`
loads the actual default Vault recipe and retained character scene, traverses
LOD0 instance/model/mesh references, checks original palette bounds and exercises
the same vertex admission. Unsupported formats are reported with their actual
descriptor census; the test does not guess replacement formats. Root owns its
original-data execution and the subsequent map-window shader validation.

Parent review/additional validation September12: original character LOD0
admission passed 10 actual meshes and 131 palette entries. All original
position/weight/index/UV descriptors passed the strict GPU upload admission.
Renderer-textured-run7 exited zero on Vulkan after startup created
all three cached pipelines. This validates shader compilation/pipeline layout,
not a character draw or live pose. Strict Clippy36 passes. Full workspace29
and original27 precede only the subsequent pipeline cache/admission test change.
