# Pedestrian stamp presentation, 2026-09-30

Confidence: **high**, source reconstruction only. No build, tests, shader
validation, runtime or visual parity check was run; those remain pending.
Scoped Rust files passed rustfmt syntax parsing (`--emit stdout`,
`--config skip_children=true`), which does not type-check Rust or validate WGSL.

## Existing owners and connected path

The selected pedestrian presentation retains its one completed CharacterPalette.
The renderer consumes its original mesh remaps, positions, UV0, byte blend
weights/indices and skin rows. `pedestrian_render.rs` now admits the original
`livingworld_stamp` material alongside `defaultlivingworld`; other effects still
fail explicitly. Stamp adds original TANGENT and BINORMAL (D3D usage7) float3
attributes and a52-byte stream. Its shadow caster uses the same skinning with
that stream stride, not an unskinned proxy. GPU cache membership now includes
the owning palette pointer, avoiding stale generation reuse after destruction
and a new pedestrian occupying the same entity/recipe slot.

`cPresPedestrian::BindConstants`8280b5d0 copies nine coefficients from
SHLightingMan's default probe. The snapshot reuses the existing vehicle default
probe reader: render_locations/ec763034105b866b, attributef45dfcab6ec195cd,
exactly nine float3 entries (missing element uses DefaultDataArea). The renderer
retains seven original material parameter rows, original diffuse and normal
rasters, three g_charattributes rows and per-frame light/view/fog. Existing
ShadowMaps owns both the actual dynamic CSM atlas and WorldShadow map/row buffer;
stamp binds those resources directly.

## Final compiled shader, not embedded UPDB text

Original assets remain outside the repository in
`%TEMP%/skate2-pedestrian-shaders-20260930`. Evidence is the original
livingworld_stamp.xml, defaultVS.vpo and defaultPS.fpo, plus embedded
livingworld_stamp.a2f2e54a9729.hlsl for names/structure. Read-only
`tools/xenos_disasm.py` decoded the shipped programs. Final PS constants:
CSM c0..3; WorldShadow c4..6; light c7; view c8; charattributes c9..11;
material c12..18; opacity c19; red/blue c20..21; SH c22..30;
materialLOD c31; textures CSM0,World1,diffuse2,normal3.

VS skins tangent and binormal independently and emits their cross product as
normal, unnormalized. Position uses the existing original offset/scale skinning.
PS decodes normal RGB*2-1, transforms through that TBN and normalizes. The decoded
normal green lane is the specular factor; it is not the raster alpha lane.
Diffuse is squared and recoloured when its squared green is below0.001225.

The final PS differs materially from UPDB:

- CSM uses nine taps with g_charattributes[0].x depth bias and .z radius;
  horizontal radius/1024, vertical radius/512. First cascade wins. Final world
  shadow samples the separate World texture, UV=(.5x+.5,-.5y+.5), and returns
  inside(|xy|<.99)*saturate(coverage*(stored_depth>=depth)).
- SH uses the original nine ordered basis terms, then floors RGB at
  (.036,.040,.042), final pc103.
- Rim diffuse is saturate(N.R*rim_fresnel)*material[6].rgb, without the old
  source's additional specular-factor multiplication.
- Key/rim specular sum multiplied by decoded normal green is saturated, pc121.
- pc112..124 constructs far diffuse as diffuse*.3*saturate(world_shadow+.1)
  and linearly blends toward it with i_material_lod.x.
- pc123..134 applies material[0].y gain, fog, g_charattributes[2].z exposure,
  original tone curve and sqrt; alpha is the entity's opacity.

The manifest default pass uses CULL NONE. Ghost retains two FRONT passes:
depth-only then source-alpha/inverse-source-alpha RGB with alpha>=1/255.
Both use the full stamp shader. The baseline LivingWorld ghost now likewise
uses its own colour/fog/tone shader rather than diagnostic skater shading.

## Distance inputs and opacity

`cLivingWorldPresEntity::Update`82811bd8 fetches material distance from
rendering/**51416546b54f421d** (not default), full attributef412fc7b191f16c8.
The core snapshot uses live AttributeRuntimeVault and registers rendering.
Material blend computes source vector length, f32 division by that parameter,
f64 subtraction of0.7, f64 multiplication by3.333333333333333, FSEL lower/upper
bounds, then f32 narrowing. The attribute is a FLOAT ARRAY (type
e22228fbb3c209d8), and material LOD reads element0. Missing elements retain native
zero DefaultDataArea;
no substitute distance is supplied.

Opacity reuses LivingWorldOpacityState once per entity per presentation update,
with original model distance parameters and cvars. Bounds center comes from the
LOD0 RModel union midpoint (`CalculateBoundsInfo`827f2708), then original
X/Y/Z/W transform multiply order (`SetLocalToWorldMatrix`827efd08). Velocity
comes from the existing Manager-equivalent local_skater_census_centre actor
provider; the selected runtime has no separate replay category-velocity buffer.
The regular base constructor's spawn opacity starts1. The separate full
hand/face constructor828126f0 resets it to0 (828127e4); that category is not yet
correctly presented by the current regular-only owner. Recipe destruction drops
the associated renderer state.

## Geometry LOD

`SceneRenderView::UpdateEntityLOD`82826a10 selects geometry from squared distance
between actual entity position and render camera. `GetLODDistancesFromAttrib`
82821a48 reads/squares all three elements of the same character FLOAT ARRAY;
only thresholds0 and1 select levels0/1/2. Original skater.vlt/bin collection
51416546b54f421d has array header0003000300040000 and values15,20,25 (research
only; runtime retains live Vault reads). The first comparison is BGE; the second
is BLT. This matters for unordered operands and is retained explicitly.

`PresentationEntity::UpdateLOD`828219a8 decrements the requested level until the
recipe has parts, asserting if level0 is absent. Both regular/full pedestrian
vtables use SwitchLOD82806528: base sets level/BindConstants, then InitSkinning
828063b8 rebuilds remaps while retaining the original inverse-bind allocation.
The host shares the distance/fallback helper with the existing vehicle renderer,
switches the same CharacterPalette, and repacks the already synchronized pose.
It does not reevaluate animation. GPU cache identity includes selected LOD.
Native UpdateLODs82829828's forced LOD0 for NIS character/skater types does not
apply to ambient pedestrian entity type222. Host dot4 follows the existing
shared numeric policy; native VMX reduction bit parity is not claimed.

## Hands/face audit

Regular pedestrians also require HandFaceManager::Update828115c0, followed by
SyncHands82807690/SyncFaces828077e8. It partitions model types, selects at most10
eligible pedestrians per group in retained iteration order, validates original
replay pose token offsets, and evaluates hands8280e0a0/faces8280e3f8 into slices
of each entity's local post-adjust SQT buffer. Face update additionally reads
the two actual speech channels for gum flapping. Current simulation emits these
tokens. The regular presentation consumer now retains the original hand/face
ACS/database owners, copies right/left/face streams using native strict total
capacity300, checks6fetch/6blend limits, and writes into subranges of the SAME
post-adjust SQT allocation before body replay. Non-pose tokens use the pedestrian
pair-blend decoder, which differs from Actor NBlend semantics. The two live
enable cvars remain defaulttrue. Native Clip.SetFrame825ecdb8's ratio then length
multiply and SetTime828071f0 use the existing Clip/Fetch timing owner.

Regular descriptor82feff10 intentionally names PedestrianLeftHandRegular.abin
for BOTH hands, with pose clip `pose`; face uses PedestrianFaceRegular.abin,
also `pose`. Read-only original anim.big extraction confirmed RAW codec4452,
13frames at60fps and speed1 for both pose clips. Each actual part has5bones,
selectors[0,1]; hand stride20/mask1f rotations; face stride52/rotation2 and
translation1e. These are runtime files, not copied tuning or synthetic poses.
Shared ClipFetchPlan now admits original RAW codec and reuses the existing
Vector_ExtractPackedN decoder, frame interpolation and SQT operations. Existing
bounded multi-stream job batches now also support RAW SQT extraction/reuse,
using original hand20/face12 ACS capacities[12,6,0,0,0], averages[4,2,0,0,0].
No additional scheduler or skinning owner was added.

There is no selected speech playback backend, so no channels are playing and
no gum-flapping Jaw overlay is produced. Authored expressions remain active.
This is an explicit audio integration limitation, not a fabricated live speech
state. Startup/tokens/jobs were source-reviewed and syntax-parsed only.

CreatePresPedestrian82812e58 branches on livingworld_models ModelType
f6897cc7b637850c: zero uses regular, nonzero uses full hand/face presentation.
Original skater.vlt has57 category0 and41 category1 collections (category1 includes
ray_barbee, rob_dyrdek, slappy, and named pros). Recipe ModelCategory::Marquee
maps to character category0; current pedestrian spawning has no model-type
exclusion. This is a potential reachable gap, not evidence that those models
were actually spawned in a runtime frame.

Full startup8280d1c8 additionally loads MarqueeSkeletonPres.abin, creates
FullHandFacePedestrianACS, retains Marquee bind SQTs and the existing SkaterFace
playback, and allocates three face-bind buffers. UpdateFullHandFacePedestrians
82809738 combines those actual buffers with each entity's body/hand-prop slices
before PoseWithMatMult/ExtractMatrix. PedestrianPlaybackStartup now retains
those full owners as well as PedestrianSkeletonPres/Sim and FullPedestrianACS.
Full-model construction uses Marquee hierarchy/palette and spawn opacity0.
The shared SkaterFace playback selects CharacterScene.schema_name's original
face bind pose, falling back to rig_Tpose exactly as CreatePresPedestrian does.
No category-based census filtering was added.

The regular body pass extracts its50-bone SQT result into the prefix of the
full entity's existing buffer. The subsequent FullHandFacePedestrianACS pass
queues Set(adjustments), Set(full bind), Replace(face bind), AddSQT, Replace(body
prefix), Replace(right prop), SetMatrix(full inverse bind), PoseWithMatMult and
ExtractMatrix. The shared buffered evaluator now implements original partial
SetData/ReplaceSQT, including bounded allocation/reuse and actual SetMatrix job
dependencies. Native descriptor capacities[6,0,5,1,2], averages[3,0,2.5,.5,1],
three characters, nine dynamic SQT slots and the three face buffers are retained.
Regular body callback82805f58 uses the first registered regular inverse bind,
or the constructor's identity matrix registration if no regular model exists.

Read-only inspection of original anim.big confirmed MarqueeSkeletonPres has150
bones and RAW4452 rig_Tpose; SkaterFacePres has49 bones and the model-specific
RAW bind records. The runtime derives body replacement0..26, right prop67 from
regular source26, and face80..129 from the actual hierarchy names. Left/right
SkaterHand files each contain17 bones and RAW `poses` clips; PedestrianFaceFull
has49 bones across six parts and RAW `SkaterRig_Hand_Ctrl`. These reuse the same
pose token evaluator and destination subranges as regular hands/face.
Submission order is regular hands, full hands, regular face, full face. Source
Marquee hand cvar defaults true; Marquee face defaults false.

The renderer also admits actual defaultcharacter and defaulthair materials
through their existing shared GPU consumers, including the original hair alpha
texture and passes. Their Marquee objects now bind the actual lit surface
programs described below. defaultcharacter_ropa still requires its original
deformation owner.
All changes were source/asset reviewed and Rust syntax parsed only, not compiled
or runtime validated.

## Marquee character/hair lighting

Original final defaultcharacter/defaulthair VS/PS and UPDB/manifest assets were
read from render.big into `%TEMP%/skate2-character-shading-20260930`; final PS
microcode was decoded with the existing tools/xenos_disasm.py. Native pedestrian
BindConstants8280b5d0 was checked again in IDA: it retains the SH default probe,
instance colour/opacity and original renderer constants for every actual LOD part.
The shared `character_surface.rs` extends the existing stamp material binding,
palette, mesh, pass, camera and shadow owners; no second object roster or shadow
resource is created. Character tangent/binormal streams are52 bytes; hair adds
its original UV1 for60 bytes. Existing shadow caster layouts now admit both.

The final character PS binds seven m_params rows, nine i_shmult rows, three
g_charattributes rows, view/light, CSM/WorldShadow and diffuse/normal rasters.
Hair uses the same lighting, with WorldShadow only and its original alpha raster.
Both square diffuse RGBA (including its specular mask), decode normal AG*2-1,
reconstruct Z with the native sqrt(abs(1-dot(XY,XY))), transform and normalize
through the skinned tangent basis, and evaluate key/rim diffuse/specular plus SH.
Final character pc90 and hair pc55 floor SH RGB at(.036,.040,.042). Final
character pc106/hair pc51 saturate rim diffuse without the older embedded
source's extra diffuse-alpha multiplier. Character pc109/hair pc67 saturate the
combined specular after applying squared diffuse alpha. Gain, actual vertex fog,
exposure, tone curve and square root follow. Character opacity and hair
alpha.r*opacity retain their original ghost/default pass states.

Only objects supplied these actual material resources select the new lit shared
programs. Existing non-pedestrian diagnostic consumers remain explicitly
diagnostic until their own material/light owners invoke the shared binding API.
No test or WGSL/GPU validation was run; rustfmt checked Rust syntax only.

## Freeroam presentation scope audit

Shadow admission now retains each pedestrian's own position and world AABB in
its existing CharacterBinding. Previously ShadowMaps.record incorrectly gated
the entire character object list on ShadowCascades.casts, which describes only
the local skater. Ordinary pedestrian/security objects now call the existing
render_shadows.caster_cascades policy with their own inputs. The selected skater
keeps its existing admission. No duplicate shadow roster, GPU mesh or atlas was
introduced, and cached pedestrian objects refresh their bounds every frame.

Fresh IDA ShadowView.Cull82850ac0 confirms the per-entity GetPosition distance
test, IsInLightView82829b88 on the actual m_BB, and removal after complete
containment in cascade0. SceneRenderView.Cull828501c0 produces m_BB by adding
the entity translation to its local min/max, without rotating the box. The
snapshot derives those limits from its existing scene.level_bounds(0), whose
native owner is CalculateBoundsInfo827f2708's union of original LOD0 RModels.
This intentionally differs from the transformed midpoint used for opacity.
The ordinary cPresPedestrian constructor828122b0 does not replace that box with
the enlarged cube used by DynamicObject. Shadow skinning/layout remains the
same actual28/36/52/60-byte character stream selected for each GPU object.

The current main target already has Depth24PlusStencil8 and clears stencil.
Character pass pipelines now implement actual manifest STENCILENABLE/REPLACE255
for defaultlivingworld, stamp, character and all three hair passes. Ghost pass0
leaves stencil disabled; ghost pass1 writes255 after its alpha test. The shared
draw binds reference255; front/back compare ALWAYS, read/write masks255, failed
stencil/depth KEEP. This closes the former missing character stencil state rather
than allocating another depth target. Actual character/hair XML additionally
requires anisotropic4 and linear mip for diffuse and normal. Their shared lit
binding now selects that sampler; hair alpha and stamp retain their original
linear min/mag, point-mip pass defaults.

DoSimSubmit8280e8b0 writes category centre velocity through the SAME manager
virtual+120 at8280eae8 (pedestrians),8280ef50 (vehicles),8280f248 (props) and
8280f4dc (DMOs). The actual manager vtable820d4150+120 resolves GetSimulation-
CenterVelocity826f4e48. The existing local_skater_census_centre reader already
implements that producer and its small-speed zeroing. DoSimRestore82813c10..d08
copies those category values without interpolation. At this selected session's
whole-tick publication, reading that same completed producer is equivalent;
separate category storage is not a current freeroam blocker.

Pedestrian UpdateFromReplayData82809d98 ignores percent for post-adjust and
hand/face stream selection: it clears prior rotations and decodes CURRENT
records, as the adapter does. Fractional world-transform restore depends on the
global DataSystem presentation fraction; this host facade explicitly publishes
whole60Hz simulation ticks. An NPC-only invented fraction would not reproduce
the native frame restore and is not introduced.

CrowdImposterTarget.RenderPresEntity827e9298 obtains a cNoSimPresPedestrian,
temporarily applies kEntityEffect, renders an impostor target and restores the
previous technique. Initializer82f6bb10 names that effect `crowd`. Ordinary
cLivingWorldPresEntity.Update82811bd8 instead selects default/ghost at8281211c..3c.
Thus the distinct crowd technique is not a missing shader branch for normal
freeroam pedestrian/security entities.

## Remaining fidelity limitations

Global fractional replay restore remains unimplemented by the selected host;
the current whole-tick NPC path does not require a separate interpolation owner.
The distinct no-simulation crowd impostor renderer is not implemented or
substituted by this adapter. Marquee ropa materials still require their actual
deformation owner if selected; ordinary defaultlivingworld/stamp pedestrians
and guards do not use that shader branch.
Host f32 WGSL is not Xenos half/estimate bit parity. CSM border addressing follows
the existing renderer capability fallback to ClampToEdge when clamp-to-border
is unavailable. Material/g_charattribute loading uses the existing renderer
environment owner; the LOD array, SH probe and opacity model parameters
are fetched through the live core Vault publication. Runtime shader validation
and comparison against original frame output are still required.

## Final focused presentation integration review

Re-read the completed source chain after DMO lifecycle changes. `selected_player_tick` calls AnimMan update after World.Update1, sync after World.Update3, post-transform publication, then pedestrian presentation extraction. `present_pedestrian_animation`, `pedestrian_visuals`, and animation scheduling all enumerate the same retained `LivingWorldGrindServices.pedestrian_entities` EntityMan40. The facade publishes those palettes in `SelectedVisualSnapshot`; `LiveSkatingView` feeds that vector into `PedestrianRender` and the shared character/shadow scene. Geometry-LOD changes consume the synchronized presentation pose, rather than another animation evaluation.

Cull destruction prepares the native animation owner and drops its presentation palette with the entity. The renderer's next update keys membership by entity ID, recipe ID and palette identity, drops absent GPU objects from its retained suffix, and removes absent opacity entries. Each NPC draw supplies its own position/bounds to shadow admission; local-skater cascade admission no longer gates the whole character list. No new normal-freeroam source integration blocker was found in this focused review. Previously documented whole-tick replay, absent speech audio, optional Marquee ropa and unvalidated GPU/runtime behavior remain unchanged. This was source inspection only; no build/tests/runtime.
