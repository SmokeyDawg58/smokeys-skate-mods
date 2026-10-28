# Edit Skater loose room and authored presentation

Source inspection and implementation only. No compilation, tests, or game run.
Rustfmt parsed the new presentation and GPU room modules.

## Native owners and lifetime

- FrontEndState_CaC::OnEnter82659a08 starts CAS then loads the eCAS loose world.
- LooseWorldPresentation::LoadLooseWorld827e3510 adds
  `data/content/world/models/DIST_petes_Textures.rx2` before
  `data/content/world/models/DIST_petes.rx2` and loads those assets. No freeroam
  teleport is involved. GetWorldRenderLocation827d0cc8 (raw827d0d6c..7c) selects
  render location `664b0d048078fe31`; the original skater Vault contains it.
- StateCAC::DoCACEditor8275f848 pauses SIM; WaitForSimPause82758028 waits for
  that pause before FlowStateReady. Exit82749d70 releases the CAC pause.
- CASPresManager::Startup825a05e0 assigns the camera and assumes ICE control.
  Update8259f990 progresses base library load before active presentation;
  shutdown waits for animation shutdown before relinquishing camera control.
- The host retains the whole world GpuScene and its LiveSkatingView owners while
  the loose room is visible, then restores the retained scene. This is the host
  ownership adapter for native loose-world/paused-SIM lifetime, not a native
  world-stream collection or alternate map load.

## Camera, body, and animation

- CasCamera::SetTargetState825874f0 reads cac_camera layout position+0,
  look-position+16, FOV+32, up attribute513070d9cbbe801a, and rendering RefSpec
  241cc15ba939c1ba. It publishes ICE transform/FOV/near directly.
- ComputeTransform82587350 normalizes look-position minus position, normalizes
  cross(up,forward), and computes cross(forward,right). Both normalizations
  use reciprocal-square-root estimate and two refinement steps. The shared
  numeric helper preserves this graph; its host estimate is not Xenon bit parity.
- RenderMan uses the existing linear angular perspective implementation in
  ViewCamera::SetPerspectiveMatrixRadians82840770, including negative X and
  fixed16:9. This is deliberately shared with existing camera projection.
- Near-plane CVar initialization82f54320 default=.05, range .001.. .1; the
  room API accepts the caller's near value. Rendering layout far+0x64 and
  FOV override flag+0x5a/value+0x60 remain original attributes.
- CASCharacterEntity::SetBodyState8259e8c8 reads position+0, animation name+16,
  yaw degrees+20 (conversion bits3c8efa35), gesture flag+24. Base switches
  preserve animation time. UpdateTransform82587c88 supplies Y rotation.
- CASAnimationManager::LoadBaseAnims8259be98 globals point to cac_edit_male
  and cac_edit_female; CASAnimationLibrary::Load82596fa0 reads data/anim/*.abin.
  Init82597140 uses hierarchy header name for bind pose, one-character RAW ACS,
  truncation offset .5, trajectory enabled. Eval requests USEDEFAULT2; newly
  created ACS default FETCH0. Existing buffered timing, frame selection, RAW
  decode and trajectory kernels are reused, not a second clip decoder.
- Face pose selection tries character name, CAC_male/CAC_female, rig_Tpose.
  GetAnimTree8258e618 wraps animation in AddSQT. EvalCurrAnim8258f178 saves hips,
  resets hips, applies expressions, computes globals/inverse binds, and combines
  entity transform with trajectory and saved hips before palette publication.
  ComputeExpressions8258d188 loads original ratio attributes before each of
  eight twist calls. Class BABB3EC110849E08 and collection2DB7709645A19CE8 are
  exact native keys; no inferred class spelling is used.

Read-only original archive inspection established both base ABINs contain eight
RAW direct clips: cac_bind_pose, cac_female, cac_male, rig_tpose,
cas_gesturestand_cycle, cas_ontable, cas_sitting, cas_stand; their sole type3
bind poses are cac_male and cac_female respectively. Both have a hierarchy
with that same name. Thus ordinary seated and standing selection uses real
authored clips. Gesture libraries/controller trees remain explicit unsupported
paths rather than fabricated poses.

Original Vault values corroborate UI raw-key recovery:

| State | body/camera key | Body | Camera position | FOV |
|---|---|---|---|---|
| Main9 | AD454A0ECD0E2EDF | CAS_Stand, (.5,0,.3) | (-.9,1.2,-1.1) | 40 |
| Slappys1 | B5C3CAC4B3DC0165 | CAS_Sitting, (.391,0,.929) | (-.8,.9,-.2) | 60 |

The couch therefore belongs to the Slappys state; main editor submenus must
follow native standing/body transitions instead of forcing a seated pose.

## GPU and material reuse

Inspection::from_assets reuses existing room material admission, texture upload,
queues, environment, and GPU scene creation. CAS uses original render-location
lighting and the existing CharacterPalette/selected-character material path.
Changing the owned recipe rebuilds bindings; unchanged recipes update palette
rows. Loose room frames have no gameplay shadow inputs or moving world owners.
Whole-world replacement retains those owners for exit.

MarkerUi owns the original createskater movie at slot15 and its letterbox;
LiveSkatingView::update_editor_ui advances the authored timelines and returns
intro/outro completion to the frontend owner. It renders the flat original
1280x720 stage without gameplay HUDs. Imported texture owners16..24 follow
their original movie names. Clothing thumbnail resources extend the existing
GPU texture map; image geometry uses the authored component bounds. Swatch
RGB transforms retain inherited alpha and compose parent colour transforms.
The movie/controller/asset translation belongs to the UI agent's evidence.

CACPresEntity constructor827ea528 invokes the SkaterPresEntity constructor
with entityType0xa2/index0, isClothDisabled=true (r9=1), next wide argument=1
(r10), and final bool=false (stack+0x57). BindConstants827df6f8 calls base then
overwrites world and doGPUSkin.x=1 on all parts. These facts were handed to
the parent for the shared material-admission CAS policy.

Remaining parity limits: host numeric estimate rounding, asynchronous native
loading/ICE ownership timing represented by synchronous retained host owners,
gesture/controller animation paths, and runtime rendering validation. No
claim of visual/bit-exact parity is made without a build and playtest.


## Customization geometry and materials

`CACPresEntity::CreateBlendJob827e89d0` deduplicates destination vertex buffers,
orders target buffers with `GetNextPriorityBlendshapeIndex827e0f98`, and supplies
the contiguous 21 morph multipliers. `GetTargetOrder82588e68` uses original
`CASMorphParams::Init825916c8` keyword/order records (class7f47ca62f4100f41),
including global_fat/fat alias. `tROptiMeshData::Fixup829c74f0` at829c768c adds
the mesh base to each blend entry name offset; at829c76a8 resolves target object
IDs. The host module mutates private resource copies before shared geometry
loading, retaining original destination W and SHORT4N/HEND3N quantization.
`MorphJob_vec827d4dd8` / `Morph_VMX827d4720` provide the blend/normalization kernel.
This has only source/syntax review, not a build or original target corpus run.

The GPU CAS programs are distinct material paths for cacstamp_cloth,
cacstamp_cloth_ropa, cacstamp_skin, cacstamp_face, cacstamp_shift, cac_hair and
cac_hair_ropa. They bind actual normal/decal/decal2/alpha channels, base/highlight
custom colors and the six native tattoo indices. `SetTattoo827d8868` maps right
hand,left hand,front,back to vector0 xyzw and right/left leg to vector1 xy.
`GetCustomColour82cff8c8` supplies default white and matches original model plus
first LOD material key. `SkaterPresEntity::BindConstants827dcf50` supplies custom
colors; constructor827e5740 starts dirt and bruise enables at zero. Default
irradiance comes from the active render_locations nine-float3 probe attribute
f45dfcab6ec195cd (Attrib count+2, stride+4, data+8).

Material formulas derive from original render.big final shader UPDB source.
CAS-specific final Xenos instruction parity has not been audited; common
lighting reuses the existing final defaultcharacter shader implementation.
This is a remaining validation boundary, not a claim of exact shader parity.
CPU cloth requires its native simulation/vertex source; a CAS ropa effect name
does not establish permission to substitute GPU skinning. The parallel cloth
owner supplies that path. No compilation, tests, shader validation, or game
execution was performed.


## Retained ROPA source metadata

CharacterLodAsset now retains CharacterRopaBinding for each admitted first-mesh
model instance: original model GUID/index, first mesh index, Arc<RopaEffect>
parsed from the actual ropa2/0xGUID.r2b, and raw per-vertex particle IDs. Existing
ropa_effects admission metadata remains available. The loader performs this
before releasing the arena buffers and preserves the customized geometry.

GetUncompressedVector8282ec38 SHORT4N disassembly distinguishes XYZ from W:
8282f32c loads normalization38000100 for XYZ; 8282f3ec loads the packed qword,
8282f3f0 clrlwi16 extracts the unsigned low halfword, then fcfid/frsp stores W
without normalization at8282f404 and inserts it at8282f41c. ClothInit827d7d88
converts that W to an integer, validates particle range, and writes both
vertex-to-particle and particle-to-last-vertex maps. New formats accessor
cloth_particle_indices supports this verified SHORT4N path and explicitly
rejects another particle-position format rather than guessing its mapping.

CPU driver coord-index caution: InitSkinParams827d2938 initializes a u16 table
for every vertex, then each later mesh's full index buffer assigns that vertex
the sum of preceding meshes' remap_index_count. Raw827d2cc8 loads optimized mesh
+0x48 (remap count), and827d2cd0 stores low16. The final mesh referencing a shared
vertex wins. A full shared vertex stream cannot safely assume mesh0's palette
covers every vertex. The CPU runtime owner was notified of this source fact.

Validation: static source/API review and rustfmt syntax parsing only; no builds,
tests, shader validation, or game execution.


ROPA binding metadata additionally retains matrix_palette_offsets and the
concatenated original matrix_palette. These offsets iterate each later mesh's
full decoded index buffer (not draw ranges) and preserve native u16 storage.
Driver mapping is separate: InitSkinParams recognizes native descriptor element
TYPE26 (descriptor byte11), and DoSkinMesh_scalar82836964 decodes its X if
present. Otherwise82836988 copies source position W. 828369bc truncates it and
82836f28 addresses the driver buffer. Retained driver_particle_indices represents
this mapping; it must not be confused with ClothInit's vertex_particle_indices.

The selected GPU adapter consumes CharacterMaterialView.cpu_rows only when the
retained native vertex parameter takes the CPU branch. It validates original
palette row count and explicit rigid weights255,0,0,0 plus indices0. Only its
internal prepared GPU pose changes x to1; native material state is preserved.
Thus the shared shader applies the supplied root/hips rows to already CPU
skinned normalized OffsetPos geometry, without a second skeletal skinning.
The existing vertex buffer now has COPY_DST and each update checks identical
layout/size before queue.write_buffer. CAS streams retain their60-byte tangent,
binormal and decalUV layout; ordinary surface streams52/hair60 and plain
streams28/hair36 are preserved. No GPU vertex buffer is recreated per frame.


## Original cloth corpus inspection and shader follow-up

Read-only Python inspection of430 distinct original cas_db LOD0 arenas found68
matching .r2b models (51 OuterTorso,17 Hair), all single-mesh, descriptor/buffer
consistent, no explicit CoordIndex, and driver indices equal position W. All
female matching assets cover their particle sets. Default male hoodie arena
000009ac03e387df / modelda4611b991216196 has1488 vertices,1297 particles with full
coverage,22 palette remaps, and offsets0. Five matching assets have invalid ClothInit mappings (only the two windbreakers
actually pass the material flag candidate gate): c30bd7280f4944db covers1009/1269 particles;
a1f6076ccf3b13da covers943/968;97834ce28c5acabc,efebf9c05c4b438c,and
cba7b98a3ba74689 contain out-of-range IDs. Native failure is a return0 and must
not become a fabricated successful solver setup. Reader produced no errors.
Scratch Python audit is .scratch/ropa_metadata_audit.py, JSON report same stem.
This is data inspection, not execution of the host implementation.

Final CAS shader targeted inspection: cac_hair_defaultPS.fpo instruction47
multiplies specular by world-shadow, in addition to diffuse shadow at52. Fixed
that omitted specular shadow multiplication in the CAS hair path. Cloth final
instructions122..135 and skin157..182 retain the common SH floor, lighting and
tone-map structure; this limited inspection does not establish whole-program
instruction parity. Shadow draws now select the actual60-byte CAS vertex layout
and share the rigid root palette/updated buffer. CPU vertex writes occur only
when the published palette generation changes; camera rows update every render.


## Cloth material flag admission and compiled shader audit

All68 matching ROPA models were inspected for actual serialized
AttribulatorMaterialName channels:26 character.cloth_ropa,25 character.cloth,
12 character.hair_ropa,and5 character.hair. Vault material_character resolves
cloth to cacstamp_cloth and hair to cac_hair; default male hoodie
000009ac03e387df/da4611b991216196 is explicitly character.cloth, not cloth_ropa.
The original recipe XML material type classification is not itself the runtime
shader selection. Raw82e2084c reads channel-table+0x38 (second channel StreamName
at+24). RebindComponentToMaterial82d0936c writes channel+0x1c (raster at+28) only.
CloneTInstanceMaterial82cff370 copies channels, MaterialFixup829c5b38 only
relocates, InitCloth and InitSkinParams retain the material pointer, and
cModelInstance::Resolve82853f30 selects the named skinned effect unchanged.
The initial audit mistakenly interpreted optimized mesh+0x24 as the nonoptimized
RMesh helper pointer. PDB tROptiMeshData and its serialized parser identify
+0x24 as m_pMaterial (+0x2c is helper). Raw InitCloth827d7b30 follows +0x24 and
then tests material.flags at+4, bit0. The loader now uses the resolved original
PresentationMaterial.flags and removes the incorrect helper-count predicate.

The corrected corpus has40 candidates:26 cloth_ropa,12 hair_ropa, and2 ordinary
cloth windbreakers. Both ordinary cloth candidates are exactly the invalid
windbreaker mappings listed above and native ClothInit returns0. All38 successful
candidates use ropa effects, cover their particles and have valid driver maps.
Default male hoodie has material.flags0 and is not a cloth candidate even though
an R2B with its GUID exists. The original asset corpus therefore has no successful
cloth binding with an ordinary shader; the earlier apparent inconsistency was
caused by the wrong structure interpretation.

Final compiled cacstamp_cloth_defaultVS.vpo binds boneArray c7[183], no
 i_vtx_params or i_matWorld, and unconditionally skins at instructions14..45.
The ropa variant binds i_vtx_params c7 and world c191..194; instruction18 tests
c7.x and branches between bone skin20..50 and world-only51..64. This confirms
that retaining the actual effect distinction is necessary. The selected GPU
CPU-row guard remains strict. Texture rebinding now also retains serialized
StreamName, changing only the host raster identity/residency projection, as
native RebindComponentToMaterial does.
