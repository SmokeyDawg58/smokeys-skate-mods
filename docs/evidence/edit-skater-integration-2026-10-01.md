# Edit Skater integration — source ready for compilation

Scope: the original Edit Skater frontend, room, clothing customization,
temporary clothing unlock override, and native SKATER.P save/startup restore.
No build or runtime test has been performed for this work.

Reuse the existing APT interpreter, character recipe/scene/palette owners,
mounted asset registry, actor/player attribute services, and frontend pause.
Do not add a second save format, UI implementation, or character loader.

## Source checks

- LoadInputMap826dde58 maps AptLeft148, AptRight149, ManualLStickX205 and
  DPadLR35. These extend the same retained PRES map used by StartHub.
- UpdatePresEntity827e61e8 replaces the skater presentation entity on a recipe
  identity change. The host replacement retains its existing allocation policy,
  restores the original base/face pose and invalidates GPU bindings by palette
  identity. Simulation and city owners remain paused during editor ownership.
- PlayerUI::ReloadAttributeData828aa470 writes IsPlayerStanceRegular to its
  FootForward field. Completed registration now updates the existing retained
  SessionMarkerTeleportQueues field; no second PlayerUI owner is introduced.
- CreateCopyFromComponents82d13ad0 selects original model/material IDs; raw
  82d13f48..3f5c compares both complete64-bit IDs for stamp assignment. The
  first matching stamp key provides its material and user-generated flag.
- GetStampableTextureFromMaterialVariant82d093d0 searches component `Misc`,
  model asset order, first LOD, then material instances for the stamp material.
  Raw82d09490..949c compares the complete64-bit material ID. The channel name
  at8302481c points to `decal`. ApplyTextureCustomizations82d09568 changes
  only an existing channel. Host composition performs that before asset loading.
- ClassRefSpec_cac_stamp type761c03280f994647 has16-byte stride; original
  Construct82167a48 is empty. CASStampHelper82595724 reads only the first
  collection qword. The existing typed Vault projection admits only that field.

## Connected flow

- StartHub Edit Skater acquires the existing frontend pause. Entry waits for
  actual player registration; an acknowledgement is never fabricated.
- Original createskater APT assets, controls, localization, thumbnails, colour
  controls, gender confirmation, room, body/camera attributes and animations
  are connected. Main menu9 uses the authored standing pose; Slappy's menu1
  uses the couch. Do not replace these with one forced pose.
- Original clothing, wear styles, colours, body/face morphs, tattoos/graphics,
  stance/style, truck/wheel tuning and equipped gesture selection update the
  CAC record. A host policy temporarily unlocks mounted base-game clothing;
  it does not fabricate installed DLC. Gender changes preserve both outfits.
- Male/female hierarchy names are cached. The room remains the animation
  owner; appearance changes no longer reload a second full animation owner.
- Exit composes native gameplay clone3, stages and finalizes its asset scene
  before releasing the previous registered recipe, restores existing actor/
  presentation owners and validates material residency before saving.
- Startup reads the same native SKATER.P/CAC record and loader path. Unknown
  profile chunks are preserved. The store validates encoded read-back, writes
  a temporary file and retains an old-save backup when replacing a save.
- GPU bindings are replaced by actual palette identity; city/NPC ownership is
  retained. Editor input preserves the partial frontend timer interval even
  when rendering much faster than60Hz. Preview delay retains fractional ms.

## Cloth admission correction and CPU publication

Raw827d7b20..7b40 plus PDB pegasus::tROptiMeshData prove the first mesh+0x24
is its MATERIAL pointer; material+4 is flags. InitCloth tests flags&1. The
decompiler incorrectly typed this optimized object as an ordinary mesh, making
the field appear to be MeshHelper.vertex_buffer_count. That initial reading
was rejected; neither an existing .r2b nor a single vertex buffer proves cloth
admission. Default male hoodie's flags are zero despite its .r2b existing.

The original430-model CAS LOD0 audit found68 matching .r2b files,40 flagged
candidates and38 successful mappings. The two flagged windbreakers with bad
mappings use ordinary shaders and follow native per-effect deletion; all38
successful mappings use _ropa shaders. No shader suffix is invented.

The existing mounted decoder now retains actual ROPA particles/constraints,
position-W particle IDs, driver IDs and concatenated palette offsets. Shipped
vector constraint/integration equations and live original cvars are retained.
StartJobs827dbb90 supplies the saved hips matrix, with axis W lanes cleared;
it is distinct from root*hips used for the render entity transform. Selected
presentation advances cloth using completed GAME ticks, with native restart
window and wipeout scale. CPU skinning reads the same gathered local palettes
before GPU packing, and native WriteDestination position quantization is kept.

The host GPU adapter represents completed CPU vertices with rigid weights and
the entity matrix in the existing shader/palette binding. This is explicitly
a host representation of the native _ropa world-only branch, not a claim that
native skinning rewrites bone indices. Prepared CPU rows are required; no
unimplemented-cloth guard is blindly suppressed. Vertex uploads occur only
for a new pose generation. CAS shadow passes use the actual60-byte layout.

## Validation and practical limits

Three agents and root checked IDA/PDB/raw instructions and original
assets. Rustfmt syntax parsing succeeded for the changed integration sources.
No Cargo build/check/tests or game were run.
This is readiness for the first compile/playtest, not evidence that the
build passes or that the whole editor is pixel-perfect. CAS shader formulas
were traced from original sources with targeted final-program checks; complete
instruction-by-instruction GPU and Xenon numerical parity is not established.

Later face/actor restoration errors can still leave the live session partially
replaced, although the previous disk profile is preserved until validation
and save succeed. Do not hide such errors or call runtime validation complete.

Suggested check: build-release.bat, play-quiet.bat, Start -> Edit Skater;
change a clothing item/colour, choose Finished, skate, then exit/relaunch and
confirm the outfit was restored. Also check a clothing submenu's couch pose,
gender switching, morphs and a cloth-enabled hair/top. A confirms; B backs;
Start returns to the editor root or finishes from that root.

See sibling room/save evidence documents and `artifacts/edit-skater-*` source
captures. New assets are decoded from the original mounted archives;
no original assets are modified. Saves use `SKATE2_SAVE_ROOT/SKATER.P`, or
`saves/SKATER.P` under the launch working directory, with an old-save backup.

## First-playtest cloth header correction

Boot failed at ControlledPlayer with `RopaError("unsupported ROPA magic")`.
The parser compared `magic[..13]` against the 14-byte `ROPA2FILE_0.5\0`
literal, which can never match. IDA recheck of Effect::ParseLoadedFile82860100
confirms the native loader copies16 bytes, forces byte15 to zero, and compares
the C string against `ROPA2FILE_0.5` (82860140..82860170). The corrected parser
requires the complete signature and its terminator, ignoring trailing padding.
This retains version validation and does not disable cloth or substitute assets.

Read-only inspection decompressed all126 entries of the original ropa2.big
using the existing tools/extract_big.py RefPack reader, validating their lengths
against ropa2.bh. All126 have the same valid16-byte header; the old predicate
rejects all126 and the corrected predicate accepts all126. Added a Rust parser
regression for valid trailing padding, missing terminator, and wrong version.
Rustfmt syntax parsing passed. Cargo compilation, the Rust regression and live
playtesting remain pending; this is not full cloth-runtime validation.

## First-frame cloth attribute correction

The following run reached Materials and rejected array30407bb86613ecc7.
This is material_charattributes/default.g_charattributes. The runtime called
try_vector_array_data, whose explicit stored-pointer-array restriction rejects
this valid class-layout array. Do not weaken that reader's storage guards:
InitSkinParams827d2f5c..827d3038 uses mCharMaterialModel.mLayoutPtr directly.
Private::GetLength829766b8 reads the second halfword (+2); the requested
float4 is element1 at layout+32. Constructor8272bb48 substitutes
DefaultDataArea(64) for an absent layout, and InitSkinParams substitutes
DefaultDataArea(16) when count<=1.

The cloth runtime now reuses try_cached_layout(64), reads count and element1,
and retains those native zero-default branches. A read-only inspection of
original skater.vlt/bin in db.big found definition offset0/size16/max3/flags7,
type1ae26d4c37271167; layout BIN+614240 starts0003000300108000 and element1 is
[0,0.8,0,2]. No authored value is replaced with a guessed constant.

Follow-up review checked all eleven ROPA cvar accessors against their typed
registrations and selected-runtime lists, the first-frame ClothUpdate-before-
palette-upload ordering, and the earlier original-asset metadata audit:
40 flagged candidates share compatible buffers/descriptors; only the two
already documented windbreakers have invalid particle/driver mappings.
Added a regression covering the actual cached layout, count vs capacity,
second-element selection, and missing layout. Rustfmt parsed the edited file.
No compilation, Rust test execution, or game run was performed. These checks
do not establish that the full live cloth/editor path is now error-free.

## Edit Skater entry: typed default-colour reference

Entry failed with `CAC default colour reference absent`. fresh_record
incorrectly called the general RefSpec reader at cac_colour_zone layout+24.
GetDefaultColourForZone8258e368 instead constructs cac_colour from a typed
ClassRefSpec_cac_colour at that offset. Raw8258e1e8 reads its cache at+8;
8258e1f4..e208 selects class6d934d0e38c95e19, and8258e214 loads the complete
64-bit collection key from+0. The general RefSpec reader wrongly read+8 as
the key, finding zero. A local typed binding now reads layout+24's qword and
uses the existing immutable Vault instance owner; no new reference subsystem
or substitute colour is added.

Original skater.vlt/bin inspection confirms all nine selectable colour zones
have valid typed default keys and zero cached words. All12 initialization slots
(skin slots3/4/5 map to2) resolve; all palette roots exist and authored base,
highlight and preview RGB values are finite. Reviewed the colour-menu palette
lookup separately: its zone+8 seed belongs to a general RefSpec at layout+0,
so that existing access is correct and remains unchanged. Added a regression
for the typed default key with its zero cache. Rustfmt syntax and whitespace
checks passed; compilation, Rust tests and game validation remain pending.

## Edit Skater entry: native blend-shape objects and target stride

Entry then failed with `CAS blend-shape object type unsupported`.
The morph consumer incorrectly treated target references as ordinary/optimized
meshes. PDB tROptiMeshData::tBlendShapeInfo+8 is tRBlendShapeData*, whose own
24-byte header holds a vertex descriptor at+0 and mesh helper at+4. Native
Fixup829c6c50 resolves both as arena IDs; CreateBlendJob827e89d0 uses these
links to obtain the target streams. Added a dedicated RenderBlendShapeData
stream-link parser and consumed it through the existing arena/helper/buffer
readers. The original type ID in the asset dictionaries is00eb000e.

The follow-on audit found target descriptors also include VERTEXFORMAT_FLOAT1
(002c83a4), confirmed by the IDA renderengine::VertexFormat enum. Added its
4-byte width to the existing format-width table. This is necessary for the
target's16-byte stride; treating only position+normal as12 bytes would corrupt
vertex stepping. Morph_VMX827d4720 still consumes SHORT4N position and HEND3N
normal using the recovered offsets; this change does not invent weight usage.

A read-only original CAS recipe/geometry audit covered860 distinct arena IDs:
324 morph meshes and364 target references, all targets type00eb000e. Source
strides were32 or40; all target strides16, position SHORT4N at0, normal HEND3N
at8, stream0. All inspected buffers contain their declared bytes, lengths are
whole vertices, and every target covers its source vertex count. Added checked
source/target resource bounds and target stride/remainder admission, plus a
regression fixture for the separate header and FLOAT1 target descriptor.
Rustfmt syntax and whitespace checks passed. No build, Rust tests, or game run;
full morph numerical/visual parity remains unverified.

## Loose-room material registration correction

The Edit Skater room decoded RX2 geometry but omitted the existing
`bind_runtime_material_attributes` step used by the streamed world. Parsed
`PresentationMaterialEffect` values deliberately start with `binding: None`,
so queue admission failed first on `environment.transparent`. The room now
uses that same binder before publication. No new shader, fallback material,
draw-order override, or relaxed queue validation was introduced.

IDA rechecked: LoadLooseWorld827e3510 loads DIST_petes textures then geometry;
cLooseWorldAssetsLoaded::OnComplete827d6568 registers the arenas through
SendPresRegArenaMsg before marking the room ready. The shared material manager's
CreateEffectListFromPegasusMaterialUnSkinned82e20948 resolves the serialized
name to an Attribulator collection, creates the effect list from that key,
then attaches Pegasus material channels. The host's existing binder supplies
that material resolution, which the new loose-room loader had skipped.

Read-only inspection of original DIST_petes.rx2 in miscboot.big found four
distinct mesh material names. Original skater Vault lookup (including parent
attributes) resolves all four without a fallback:

| Material | Manifest | Draw order |
|---|---|---|
| environment.default | baseenvironment | 3 |
| environment.transparent | transparentenvironment | 11 |
| environmentsimple.default | defaultenvironment | 3 |
| incandescent.default | baseincandescent | 3 |

All four manifest effects already have renderer implementations. Confidence:
high for the omitted registration step and original attribute resolution;
room visual/runtime parity remains unverified. Rustfmt syntax and diff
whitespace checks only; compilation and playtesting pending.

## RAW editor animation decoding and entry verification

Compilation followed, with repeated Edit Skater entries.
The reported empty pose [0,0] selectors exposed a missing RAW family, not a
missing editor animation. Rechecked Vector_ExtractPackedN82d6f838: selector
header+28 chooses float4 quaternion (zero,16 bytes) or packed quaternion
(nonzero,4 bytes); header+27 controls translation packing. The existing zero
translation selector uses float3. Masks select scale/rotation/translation in
bone order. No selected channels means the initialized identity survives;
zero-stride clip parts therefore consume no payload for any selected frame.
Scale and quaternion initialization references remain82d6f86c/884/8dc.
The native selector/width code was checked again at82d6fbd0..fc74.

The shared RAW decoder now accepts authored [0,0] and [0,1] selectors, reads
unpacked XYZW directly without normalization, and handles genuinely empty
zero-stride parts. Mask, payload and nonempty stride validation remain.
The full-clip sampler bounds frames against the owning clip and skips only
the inapplicable payload/stride division for empty parts.

Read-only original archive audit: cac_edit_male.abin146 RAW parts (81 empty),
cac_edit_female.abin146 (77 empty), SkaterFacePres.abin360 (none empty).
All inspected masks/selectors/strides/payload sizes match these layouts.
Five release-mode RAW regressions PASS, including packed vehicle/face cases,
unpacked XYZW/frame stride, empty identity, and malformed payload rejection:
artifacts/edit-skater-raw-tests-20261001.log.

Added opt-in --live-controls-demo edit-skater to the existing raw-pad driver.
It navigates StartHub using Start/D-pad/A, with an input counter that continues
while gameplay is paused. It requires a visible editor and completed original
intro at600 input ticks; it uses the existing render capture path. No direct
editor-state jump or normal-play input change. Release/runtime verification
results are recorded below when completed.

First release build succeeded (7m39s). The scripted entry then reached the
room, resolving all201 sampled texture channels and submitting33 meshes
(10 Default,1 BaseIncandescent,18 Base,4 Transparent) with no diagnostic
fallback or unbound effect. It exposed `CAS animation part size mismatch`.

Root cause: the private CAS library is parsed without native database-global
part registration, but its fetch loop matched serialized global-name fields,
which are all zero in both shipped libraries. UpdatePartNames82d6e728
(especially82d6e828..858) obtains the original local part from header bits1..5
and replaces it with a registered global ID. The editor now matches those
authored local indices directly within its same library/hierarchy, preserving
strict bone-count and destination-bounds checks. It does not create a second
global registry or modify the existing registered gameplay fetch owner.
Read-only audit of all clips in both CAC base libraries found zero bone-count
mismatches using their local indices. Rebuild/retest follows this correction.

Second release build succeeded (4m58s); entry progressed through animation to
GPU admission, exposing missing TEXCOORD1 on an original CAS clothing mesh.
The original cacstamp_cloth VS declares UV1, but the native D3D runtime handles
an absent declaration element. PatchVertexShaderToMatchVertexDeclaration
82c43db8, missing-element branch82c43eb0..bc, sets swizzle0x9250. Applying
kPatchSwizzleMask822c96b0 produces component selectors4,4,4,5; PDB GPUSWIZZLE
identifies4 as constant0 and5 as constant1. Thus the supplied value is0001,
with zero XY for the decal UV. This is a recovered native default, not a
guessed texture mapping or copying UV0. The CAS vertex uploader now supplies
zero XY only when TEXCOORD1 is absent, and preserves existing format/duplicate
checks if it exists. The original shader's UV positivity test disables decals
for the absent stream. Added a regression for absent, authored and duplicate
UV1; runtime rebuild follows.

Third release build PASS (2m18s), recorded in
artifacts/edit-skater-entry-build3-20261001.log. Scripted raw-pad entry PASS:
artifacts/edit-skater-entry-run3-20261001.log ends with EditSkaterMain,
five rows, intro complete; process exited0. Screenshot inspected at
artifacts/edit-skater-entry3-20261001.png: room, animated character and original
menu visibly render. Test uses an isolated SKATE2_SAVE_ROOT and does not invoke
save/exit or modify the normal profile. Missing/authored/duplicate UV regression
also PASS in artifacts/edit-skater-cas-uv-test-20261001.log (one test).

Visible limitations: the captured head/hair/neck appearance is wrong and menu
description/button text layout needs further work. These are NOT claimed fixed
by entry success. No speculative morph or shader correction was applied during
follow-up inspection. CAS Init82597140 face overlay, GetTargetOrder82588e68,
GetNextPriorityBlendshapeIndex827e0f98, CreateBlendJob827e89d0 and Morph_VMX
827d4720 were re-read; they did not establish the visible issue's cause.
The entry test does not verify clothing changes, all submenus, female/gesture
paths, save round-trip or return to gameplay. Those remain separate coverage.

## Follow-up: incorrect depth and original text layout (October 1)

Two playtest captures showed hair through the face, exposed neck geometry over
clothing, broken couch occlusion, overflowing description text and a generic
Wear Styles label. These were real integration defects, not evidence that the
native assets needed changed winding or replacement geometry.

### Projection owner (root cause of inside-out appearance)

A diagnostic captured every Z lane of the CAS clip matrix as zero. The code
mistook cac_camera's reference 241cc15ba939c1ba for a rendering collection.
CasCamera::SetTargetState82587634..78 actually writes this reference to
RenderMan.PostFX.m_FEAttribsCSM. It does not change CameraPresentation.mRenderAttribs.
CameraPresentation ctor827f217c..2194 initializes rendering/default
(d7edbd362d7d2152); GetFarPlane827f2630 reads that instance at+0x64.
GetFOVY827f64b0 selects mICEFOV directly while ICE is active, bypassing the
ordinary rendering FOV override. AssumeControl82587690 and SetTargetState
provide the authored CAS transform, FOV, and kCasCameraNearPlane (0.05).

The editor now keeps those separate owners: the reference is named
postfx_csm_collection; far comes from rendering/default; the active ICE FOV is
the authored CAS FOV. Invalid/collapsed near/far ranges fail before publication.
No winding, hair pass, mesh, morph weight or bone transform was changed as a fix.
Temporary no-hair and reverse-culling experiments were removed. An opt-in
SKATE2_DIAGNOSTIC_CAS_DUMP captures projection and character streams if needed.
The original asset regression checks that post-FX selection does not alter the
projection and that increasing distance produces increasing, nonzero depth.

### UI source corrections

- SkateAptString82a70b28 wraps multiline fields using native font advances at
  spaces and recognizes '^' as a forced line break. Text is now wrapped to its
  authored width before glyph submission. The original line-height spacing is
  retained. The default Body Mods description fits its panel in three lines.
- createskater.UpdateItemValue010030..046 gives category controls the result of
  CAC_GetStringValue, not the option caption. WearStyles uses the existing
  ID_CAC_WEAR_STYLE_OPTION_n strings. highlight_category.SetTitle001538..155d
  measures the label and places its right arrow after it.
- Highlight SetState controls arrow visibility separately from the selected /
  unselected animation. Slider subtitle colours use the script's values.
- createskater ctor00e637..64a sets Start help to HelveticaPresto,28,12047872;
  general button help retains Marker_streak,24,16776940. button_menu.SetInfo
  centers text vertically relative to the authored button with its -4 offset.
  RefreshItems always starts from mOriginalStartX; repeated selection refreshes
  no longer drift the help labels horizontally.
- FrontEndManager::SetScreenSize82614300 supplies Screen_EdgeOffset136 only
  when not widescreen. For the retained native 1280x720 PC UI canvas the value
  is zero: createskater ctor00e5c0..627 shifts the screen -136 and its bottom
  button groups +272. Those previously omitted transforms are restored.
- UpdateInfoPanel00f6b4..f847 chooses big/small/wrap based on native maxscroll
  (SkateAptString's wrapped line count). Blurb text is no longer also written
  into the subtitle; tattoo/graphic thumbnail descriptions use blurb as in
  GetInfoPanelType826691c0.

### Exit/save flow

CharacterRecipeLoader::OnComplete82d0a140, raw branch82d0a234..274, uses
CAC_male/CAC_female for a category3 recipe, including male for null custom data.
The mounted gameplay face loader now uses that branch instead of constructing
an unavailable faces/cas_db.abin path from the editor schema name.
An isolated SKATE2_SAVE_ROOT roundtrip saved SKATER.P, reloaded the gameplay
character and returned to active gameplay: edit-skater-roundtrip-run-20261001.log
(exit0), screenshot edit-skater-roundtrip-20261001.png. The normal save
was not touched. Additional final-build verification is recorded below.

Final verification for this follow-up:

- Release build PASS: artifacts/edit-skater-depth-fix-build2-20261001.log.
  The earlier depth-fix build hit an executable lock from a running playtest, not
  a Rust compiler error; once the game closed the rebuild succeeded.
- Original asset regressions PASS2 in
  artifacts/edit-skater-original-regressions2-20261001.log: projection depth /
  post-FX independence, original category labels, arrow state and repeated
  button layout. The English original localization says "Option 1" / "Option 2",
  not "Style 1" / "Style 2"; no replacement labels were invented.
- Multiline regression PASS1: artifacts/edit-skater-wrap-regression-20261001.log.
- Scripted Start -> Edit Skater PASS: edit-skater-depth-fixed-run-20261001.log,
  screenshot edit-skater-depth-fixed-20261001.png. Captured depth matrix now has
  nonzero Z coefficients and -0.050000273 camera-relative translation. Head,
  neck/hoodie, hair and room couch occlusion visibly corrected.
- Scripted Merchandise/couch PASS: edit-skater-couch-fixed-run-20261001.log,
  screenshot edit-skater-couch-fixed-20261001.png; seated pose and couch checked.
- Scripted hat selection reaches Wear Styles with colour and graphic controls:
  edit-skater-styles-fixed-run-20261001.log, screenshot
  edit-skater-styles-fixed-20261001.png. This chosen hat has no category style
  row; the Option1/2 category path is covered by the original-asset test, not
  claimed covered by that live screenshot.
- Each new process used the earlier isolated roundtrip's SKATER.P at startup.
  The real profile remained untouched. Final roundtrip result follows.

Coverage remains scoped: these fixes and captures do not prove every gender,
gesture, garment, menu and lighting/post-processing path pixel-identical. The
CAS camera's per-view PostFX CSM reference is retained separately; this change
corrects its erroneous use as projection data, not a complete new CSM pipeline.
- Final-build save/return PASS: artifacts/edit-skater-roundtrip-fixed-run-20261001.log
  exits0 after631 gameplay ticks and explicit Finish/save -> active gameplay.
  Captured frame: artifacts/edit-skater-roundtrip-fixed-20261001.png.
  Isolated SKATER.P is4,194,308 bytes, updated during the run. Subsequent editor
  entry processes above already verified startup reading of this same profile.
# October 1 follow-up: editor layout against the reference comparison

The reference image is PS3 retail; use the Xbox 360 debug executable and its
original APT as the implementation authority, retaining Xbox button artwork.
The confirmed differences are host translation errors, not a reason to scale
the whole card or invent camera offsets.

- `createskater` constructor `00e590..00e596` calls
  `LetterBox.EnableLetterBox(false)`. The host incorrectly played `niceintro`.
  Keep the editor letterbox disabled on entry/exit/re-entry. It covered the
  card's lower margin and Select/Back row; the authored card dimensions remain.
- `mDone.mBack` is character 144, containing three animated chevrons. The host
  incorrectly hid it in configure. Retain the authored visibility/timeline.
- `button_menu` prototype `000ad1..000ad5` defaults `mJustify` to `right`.
  Translate `RefreshItems0009c4..000aa8`: subtract auto-sized text width, place
  text, add 10, subtract button width, place button, subtract spacing 25.
  Start/Finished is anchored to the existing right edge. `createskater`
  `010110..01014d` adds B then A, so the rendered order is A/Select then B/Back.
  Keep the original start coordinate across refreshes.
- Original `highlight_slider.SetState` separately shows/hides its arrows.
  The existing fix is retained; the regression now exercises selected,
  unselected, and deselected states for 120 timeline ticks each.
- IDA `GetButtonHelpTextA82653000`, `B82649608`, `Start82649658` confirm
  Select/Back/Finished on Main9 with Body Mods selected.
- Rechecked raw `SetScreenState826850d0..82685104`: both Main9 body and camera
  use AD454A0ECD0E2EDF, matching the current frontend. `SetTargetState825874f0`
  and `GetFOVY827f64b0` retain authored ICE position/look/up/FOV. No unsupported
  camera repositioning or FOV tuning was introduced to match a different pose
  or platform capture.
- The host-only FPS counter now shares the gameplay font upload with the
  editor scene, avoiding its old pixel-font fallback.

Targeted original-asset regression PASS:
`artifacts/edit-skater-layout-regression-20261001.log` (button direction and
refresh stability, slider timeline visibility, intro/outro/re-entry without
filmstrip, chevron visibility).

Release build PASS: `artifacts/edit-skater-layout-build-20261001.log` (2m02s).
Scripted StartHub -> Edit Skater entry PASS, intro complete, exit0:
`artifacts/edit-skater-layout-run-20261001.log`. Visually inspected
`artifacts/edit-skater-layout-fixed-20261001.png`: full card/bottom margin,
no filmstrip, Finished plus original chevrons, A/Select followed by B/Back,
no arrows on unselected sliders, consistent trick-font FPS counter.
Used isolated `artifacts/edit-skater-diagnostic-saves`; normal save untouched.
This verifies the reported UI fixes, not complete PS3/Xbox lighting or pose
identity. Main camera's authored selection and projection were retained.

# October 1: editor controls, popup registration and frontend shadows

Source audit and changes (solo; reuse existing controller, APT playback,
localizer, shared scrollbar and GPU shadow owners):

- `sk8popup` character29 InitAction `0x1b00` registers the popup class. Admit
  that exact fingerprint to the already translated gender popup constructor,
  Refresh, selection and intro/outro paths; unknown scripts still fail.
- `HALUtils::HALifyString82669738` uppercases then replaces each literal space
  with underscore. `GetStringValue826775f0` uses it for hair/facial hair/brows.
  The catalog spelling `Moosed Up` therefore resolves
  `ID_CAC_HAIRSTYLE_MOOSED_UP`; original English displays **Moussed Up**.
  Do not substitute generic text for missing localization. Separately checked
  `WearStyleHandler::GetCASPartDescriptionHALID82678798`: clothing descriptions
  only uppercase, so do not apply this space conversion indiscriminately.
- `highlight_selector.SetSubtitle0016a6..0016d6` autosizes the subtitle left,
  then positions the right arrow at subtitle.x + text width + margin (0).
  Reuse the existing category text metrics; no guessed fixed arrow position.
- `basic_slider.SetValue001840..0018a1`, SetNumberOfItems0018c8..00194e and
  UpdateDisplay001bfc..001cb2 retain the window start and update the thumb's
  size/position. `createskater` uses length320 and list window11. Its thumbnail
  initializer00f242..00f255 first computes floor((itemCount-1)/3)+1 into
  register3, then00f2a1..00f2be passes that ROW count, window3 and
  floor(selection/3). Correction from the October1 follow-up: the earlier
  audit missed the register3 producer and incorrectly called it item count.
  Reuse the map's same
  basic_slider geometry with caller-specific window/length, preserving its
  prior9-row/275 behavior. Moving within a list window no longer shifts rows.
- `GetFilterText82642c88` returns "#" unconditionally. APT UpdateFilterText
  00f044..00f097 empties the text and only animates off when previously on;
  initial visibility is already off. Stop replaying a fade containing the
  authoring placeholder on every value change. Refresh layout only on menu
  changes, while selected values continue to update normally.
- Exhaustively advanced every nonempty timeline of editor assets15..25,
  including all highlight controls and sk8popup, through every authored frame.
  Linkage-only class symbols have no display frames and are not instantiated
  independently. Popup intro/selection/outro, slider selection/deselection,
  filter stability and scrollbar window updates receive behavioral checks.

The editor previously set `shadow_inputs=None`. This was a missing caller of
the existing renderer, not a reason to create another shadow pipeline.
`CasCamera::SetTargetState82587634..82587678` supplies `m_FEAttribsCSM`.
`RenderShadows_CSM82855888` disables the gameplay local-skater shortcut for
frontend/loose worlds and reads that collection's scale, offset and layout+24
depth precision. `ShadowView::Get_BBOX_Height82829de8` pads the caster bounds
15%. `SkaterPresEntity` constructor827e5a30..827e5b2c lowers model bounds Y by1;
`SceneRenderView::Cull828501c0` adds entity translation. The existing CAS pose
publisher now returns the same root passed to the presentation palette for
these bounds. Cache local bounds by recipe and shadow attributes by camera
collection. Shared light basis, cascade rendering, filters, receivers and
character material pipelines remain in use.

Original close-up collection09d4a1e32ed3c262 has depth precision0. Raw
82856434..82856530 explicitly handles this: replace the depth column with
(0,0,1,0), avoiding a reciprocal of zero. Retain that original branch, rather
than rejecting the asset or inventing a nonzero precision. The camera corpus
check covers both shops and27 distinct part/face selections, including this
case. Gameplay's existing nonzero depth/+2 top shortcut is unchanged.

Original-asset regressions PASS:
`artifacts/edit-skater-controls-test5-20261001.log` (2 tests: full UI audit and
camera projection/CSM corpus). Release/live validation is recorded below.

Lighting scope: the room already uses original CAS render location
664b0d048078fe31. Its scene has no final_blit, so the gameplay-only brightness
gain is not the reason for a brighter editor. No subjective darkening was
added. Full native tone mapping/auto-exposure remains outside this control and
shadow repair; these changes do not establish pixel identity with PS3 retail.

Release build PASS: `artifacts/edit-skater-controls-build-20261001.log`
(2m21s). Shared map scrollbar/animation regression PASS and seven existing
shadow regressions PASS (`edit-skater-map-regression-20261001.log` and
`edit-skater-shadows-regression-20261001.log`). No runtime error was suppressed.

Scripted native input StartHub -> Edit Skater standing and Merchandise/couch
runs both exited0. Captures visually checked:
`artifacts/edit-skater-controls-shadows-20261001.png` and
`artifacts/edit-skater-couch-shadows-20261001.png`; the latter shows the skater's
shadow on the couch. Logs: `edit-skater-controls-live-20261001.log` and
`edit-skater-couch-shadows-live-20261001.log`. Used only isolated diagnostic
save root. These live runs verify entry/room/camera/shadows; the popup and
selector/scroll interactions were verified by the original-asset regressions,
not a claim that every customization was exercised through the controller.

# October 1: first gender initialization, modal placement, clothing UI cost

Playtesting found the first gender switch failing `CAC slot cannot be empty`, popup
shifted left with incomplete screen dimming, and lower FPS in part thumbnails.

- IDA `DoSwitchGender82590430` stores the old outfit, then either restores the
  other closet entry or runs `CASPlayerInitializer::InitializePlayer8258fb90`.
  Raw/default loop8258fc48..8258fcb4 uses SetInstalledPartID for a valid default
  and ExchangePart for an invalid default in **every** slot. The host wrongly
  used the user-facing Remove command's nullable-slot gate. Extracted the
  already implemented exchange-to-invalid inventory operation into one helper,
  used by initialization and by Remove after its unchanged UI policy check.
  `ExchangePart82b6bfd8` and `SetOwnedPartID82b6bbd0` retain old items in owned
  inventory; no forced replacement, error suppression or widened Remove menu.
- Original screenmanager CreateScreen0008a4..0008ce mounts popup screens below
  `gPopups`, independently of createskater. Moved the retained sk8popup clip to
  its own screen owner, sharing the movie clock and callbacks but rendering
  with identity screen transform. It no longer inherits createskater's -136
  card translation. Authored panel position and Refresh001f2e..001f4b vertical
  core-centering stay intact. Extend only backdrop shape25:2 to wide windows,
  following the existing map/dimmer adaptation; foreground art stays unstretched.
  Refresh popup body/options on changes instead of rebuilding them per redraw.
- The clothing view formerly called description_candidates for every catalog
  item every render frame. Each call scanned the entire parts database to
  reproduce WearStyleHandler::GetCASPartDescriptionHALID82678798. Cache the
  immutable row metadata per part index/gender inside that frontend's lifetime;
  selection, preview and installed record remain live. Original lookup order,
  fallback keys, temporary unlock policy and thumbnail names are unchanged.
- Host UI now localizes a changed view once and retains it. Texture admission
  uses existing GPU texture membership instead of cloning all CPU texture
  keys each redraw. APT timelines and drawing still advance every frame;
  resizing still rebuilds projected geometry with current window dimensions.
  These remove confirmed repeated CPU work. They do not establish a measured
  FPS target or imply that all close-up GPU costs have disappeared.

Added raw-controller diagnostic routes edit-skater-popup and edit-skater-gender;
the latter enters Body Mods, initializes female, and restores the male outfit.
Always run with an isolated SKATE2_SAVE_ROOT for these diagnostics.

Validation PASS:
- Original APT regression, including separate popup lifecycle and full dimmer
  coverage at1280x720 and2560x720: edit-skater-gender-popup-test-20261001.log.
- Release executable: edit-skater-gender-popup-build-20261001.log (2m15s).
- Live gender cycle: edit-skater-gender-cycle-live-20261001.log, exit0. Explicit
  checks require female after first confirmation and male after restoration.
- Popup: edit-skater-popup-position-live-20261001.log, exit0; visually checked
  edit-skater-popup-position-20261001.png. Full-screen dimming, no right-edge gap,
  original panel placement independent of the editor card.
- Clothing preview: edit-skater-cached-clothing-live-20261001.log, exit0;
  edit-skater-cached-clothing-20261001.png confirms hat/preview/menu updates.
All artifacts above are under artifacts/. Normal save untouched. The captures
were refresh-limited; no uncapped FPS gain is claimed from those runs.

## Palette arrows, outer-shirt removal, and scene fades

Source checked again on October 1:

- `highlight_color.SetNumSwatches` in the original APT at002552..2566
  positions the right arrow at each admitted item's `_x + _width + margin`.
  The final admitted swatch wins; `mArrowMargin=-8` at00276f..2775.
  Shared color controls now perform that operation instead of retaining the
  ten-swatch placeholder at316. This covers skin, eye, hair, clothing and other
  color rows. Selector/category text-width placement was already present and
  is reused. Slider/morph arrows remain at their authored track ends; their
  original classes do not place them after the title text.
- `CanRemoveItem82642e30` calls assembler `IsNullablePartSlot82585ee8` but also
  explicitly permits OUTER_TORSO23. The frontend offered Remove while the
  record mutation rejected23. A shared `can_remove_part` now governs both;
  the existing ExchangePart inventory operation is reused, including retaining
  the removed shirt. `CASSimManager::RemovePart82598ae8` additionally replaces
  required OUTER_TORSO with the FIRST skinned part of the arms' gender (undo
  torso if no arm). It reuses GetAlternateSkinColourPartID82b66d40 with both
  skin and normal from that reference; raw82598dd8..8dfc confirms these inputs.
  Failed selection restores the undo torso. The existing alternate_skin and
  inventory operations implement this, preventing a second missing-torso
  error after merely allowing23. InitializePlayer's invalid defaults unchanged.
- `EnableBackground826066c8`, `UpdateFade82614f10`, and
  `RenderBackground82630dc0`: black RGBA fill, alpha incremented by signed
  kFadeSpeed, clamped at0/1; fade-in decrements the delay before advancing.
  `kFadeSpeed` initializer82f58320 is FE/"Fade Speed", default float0.028,
  native slider range .005..1. Registered in the existing live CVar owner.
  GameModule::Update8273edc8 calls FE Update with0x682 (1666), consistent with
  the fixed frontend clock. No render-FPS-based fade acceleration.
- StartHub::FadeToBlack826718f0 blocks input and fades before ProcessSelection
  via FadeComplete8266b238. CAS startup completion8268d638 disables the black
  background with a10-update reveal delay. CAS OnUpdate82670828..85c covers
  exit before completion/pop. ChallengeMap DismissAndTeleport826750f0 and
  OnMenuSelect826828f0 likewise cover first, then FadeComplete8266b838 calls
  DoTeleport8265f280. OnOutroDone8265ec48 reveals only if no load remains.

The PC adaptation holds black across synchronous asset work and existing
terrain/presentation/teleport readiness. A successful swapchain presentation
acknowledges black before loading starts; surface timeouts/minimized windows
cannot release it. The new destination must be prepared before the native
  ten-update reveal delay begins. The same retained fade owner is used for
editor entry/exit and map travel. It reuses the APT solid-fill pipeline after
scene/UI drawing; the diagnostic FPS counter remains above it. Loading-tag
art/animation is not introduced or claimed by this change.

Validation so far:

- Original-asset palette/selector/category/slider/popup regression PASS:
  artifacts/edit-skater-arrows-fade-test2-20261001.log. Palette sizes1/2/3/6/10
  checked through selected/unselected timelines and back, with60 ticks each.
- Initial fade executable build PASS: edit-skater-arrows-fade-build-20261001.log.
  This build precedes the final required-torso replacement fix.
- Editor entry/Finish/save/gameplay roundtrip PASS, exit0:
  edit-skater-fade-roundtrip-20261001.log.42 fully black presented frames across
  entry/exit, then alpha returns to zero; diagnostic isolated SKATER.P only.
- Map raw-controller category/location/Teleport path PASS, exit0:
  map-teleport-fade-20261001.log.280 fully black frames cover destination work;
  final gameplay reveal verified. map-teleport-fade-20261001.png visually checked.
  These two runtime checks use the initial build; map fade logic is unchanged.
  The final editor exit sequencing refinement is noted below.

Additional checks:

- Live clothing palette run exit0 and screenshot
  edit-skater-palette-arrows-20261001.png visually checked: right arrow follows
  the six visible swatches. Reuses the unchanged palette implementation above.
- Inventory exchange regression PASS in the compiled core test target:
  edit-skater-removal-core-test-20261001.log (first test-target compile16m18s).
- Native fade boundary/presentation/delay regression PASS using that same
  already-built test executable: edit-skater-fade-direct-test-20261001.log.
  A redundant second Cargo test rebuild was intentionally stopped; the direct
  test runs the identical fade implementation. Do not mistake its interrupted
  compile log for a test failure.
- Final exit refinement: Shutdown82659de8..9dfc sets FADE_CAS_EDITOR and
  ShowScreen(false) before OnOutroDone82649348. Hide the menu immediately and
  start room cover on Finish, keeping the retained outro only for cleanup.
  The first roundtrip capture predates this refinement; repeat on final build.

Final verification:

- Release build PASS: artifacts/edit-skater-arrows-fade-build2-20261001.log,
  7m22s. Existing warnings remain; no compile errors.
- Raw Start -> Edit Skater -> Merchandise -> T-Shirts -> Remove PASS, exit0.
  artifacts/edit-skater-remove-shirt-20261001.log and matching.png. Screenshot
  visually checked: bare torso on couch, original Remove Item selected,
  required torso component retained; no empty-slot/assembly failure.
- Final raw editor entry -> Finish/save -> gameplay roundtrip PASS, exit0:
  artifacts/edit-skater-fade-final-roundtrip-20261001.log and matching.png.
  Covers the final immediate menu-hide/room-fade exit sequencing.
- Real save untouched. All live runs used the isolated
  artifacts/edit-skater-diagnostic-saves root. No game/compiler left running.
  The release executable is ready for play-quiet.bat.
