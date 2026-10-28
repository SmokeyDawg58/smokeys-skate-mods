# Z-fighting: world-space effects, identical depth, hair program, part order (2026-09-27)

Status: the whole batch was built and run (09-27): the map-wide z-fighting is
gone (after section 7). Sections 1-6 shipped in
the same build.

Reference: debug `sk82_na_m.xex` in IDA, the shipped `render.big` shader
containers (`.vpo`/`.fpo` microcode, disassembled with `tools/xenos_disasm.py`;
the `.updb` text is only a cross-check), and `marquee.big` (tutorial_male).

## Report

After the CSM shadows were confirmed, z-fighting remained on
the hair and head, the hat and head, and some map textures.

## 1. Map: world-space effects and identical depth

**Original.** Every world effect's shipped `defaultVS` writes oPos with the
same four instructions, `dp4 oPos.{x,y,z,w}, c0..c3 (g_matVP), rN`, where rN
is the fetched position with no ALU write before it. Two draws of the same
vertex positions therefore get bit-identical depth under any effect, and the
pass-reset ZFUNC LESSEQUAL lets the later draw in the sort order win. The
instance transform (`i_matWorld`) never reaches these vertices.

Classification of all 58 shipped `*_defaultVS.vpo` (scratch script: the four
oPos dp4 must read c0-c3 and one vfetch register untouched by the ALU; in the
last row the position is computed first or not written by such a dp4, and
the `.updb` text names the transform):

| Class | Effects |
|---|---|
| World space | advertisement, alphatestdefaultenvironment, baseenvironment, baseincandescent, baseterrain, building_ceiling_tangentspace, building_ceiling_worldspace, building_cuberoom_tangentspace, decalenvironment, decalterrain, decalzbias, decalzbiasbacklit, defaultenvironment, flowingwater, ocean, proxyworld, scrollincandescent, trafficlight_one, trafficlight_two, transparentdefaultenvironment, transparentenvironment, transparentincandescent, tree, videoscreen, water, zbiasincandescent, zbiasincandescentbacklit (also WorldShadow, ZPrePass) |
| World space plus a wave offset (`P = In.pos + move`) | treeanimate, vertexanimate |
| Not world space | characters (bone rows), vehicles (`skinMatrix`), dynamicobject/alphatestdynamicobject (`partMatrix`), sky (`+ g_vViewPos`), glare, godray(spotlight), spline, visualindicator, vp6, crowdimposter |

**Port before.** `render_inspection.rs` `world_from_mesh` treated only 8 of
these as world space and multiplied the other 19 (baseterrain, decalterrain,
advertisement, baseincandescent, the zbias/incandescent family, traffic
lights, water, ...) by the instance transform ("pending shader review"). No
world pipeline marked its position `@invariant`, so the ported environment
effects and the diagnostic path could round the same position differently:
coplanar layers that tie in the original could flicker.

**Fix.**

- `WORLD_SPACE_EFFECTS` (29 names) from the microcode classification; those
  draws use the exact identity instance matrix (1*x + 0*y + 0*z + 0 is exact).
- Every world position output is `@invariant`: the environment effects go
  through one `world_clip` (`globals.clip_from_world * vec4(p, 1)`), and the
  diagnostic shader's output is marked too. naga 0.19 accepts it on the shared
  VS-out/FS-in structs; Vulkan ignores Invariant on the fragment input.
- Draw order already follows the original `InstanceSortKey` stable sort, so a
  tie resolves to the same draw as in the original.

## 2. Hair vertex program

- Shipped `defaulthair_defaultVS.vpo`: oPos = four dp4 of the skinned
  position against c0-c3. The `.updb` line `Out.Pos.z += GetZBias(Out.Pos.z,
  0.0)` (`zbias / z`) compiled away. The port's `z += 0.0 / z` is removed; the
  hair now computes its position exactly like every character program (the
  character `Output` was already `@invariant`).
- No character program has a real depth bias: only defaulthair, cac_hair and
  the cacstamp/livingworld/vehicle programs carry `GetZBias(z, 0.0)`;
  `defaultropa` alone uses `g_charattributes[2].w`.
- Hair alpha checked in the shipped `defaulthair_defaultPS.fpo`: `tfetch2D
  r2.w <- tf3.x` at UV.zw (i_alpha), `oC0.w = r2.w * c15.x` (i_params[0].x).
  The port matches.

## 3. Inverse-bind fill order (`FillInvBPBuffer`)

IDA chain:

- `CharacterEntity::BuildTInstanceArray` 0x82d088a8 walks `mCompositeAsset`,
  an eastl map keyed by the component name (`PooledStringTemplate<24,0>`).
  Its `less` 0x82590cc0 compares `SharedString::mHash` unsigned; `GetString`
  0x8259b8c0 sets `mHash = RwHash32Buffer(name, len, 0x811C9DC5)` (FNV-1,
  0x82bb5790). A "Beard" component is skipped when "Rostral" exists.
- `PresentationEntity::InitFromRecipe` 0x827faab8 -> `GetTInstances` ->
  `AddLODModelInstances` 0x827fa998 / 0x827fa1c0 keep that order in
  `m_vLODMdlInstances[lod]`.
- `SkaterPresEntity::InitSkinning` 0x827e4b48 -> `Sk8::FillInvBPBuffer`
  0x827f8338: every hierarchy bone starts at identity; each part in order
  writes `m_pIBPMatrices[i]` for each of its `m_iNumBones` named bones (not only
  the palette), a missing name going to "Hips". The last writer wins.
- `UpdateBodies` 0x827de988 feeds `InvBindPoseBuffer` (+0x214) to
  `PoseWithMatMult`; `UpdateBoneTransforms` 0x827efb78 copies the shared
  hierarchy-ordered result into each part's palette.

tutorial_male (LOD 0):

- Original order (FNV-1): OuterTorso 09410273, Feet, InnerTorso, Arm,
  Rostral, Hat, Hair, Organ, Pants 4dbff91d, SkateBoard. The port used recipe
  order (Arm, Hair, Rostral, Feet, Organ, SkateBoard, Pants, Hat, InnerTorso,
  OuterTorso).
- Shared bones agree across parts to 6e-7, except RightArmTwist,
  RightForeArmTwist and RightForeArmTwist1: Pants' copies differ from Arm and
  OuterTorso by up to 0.052 (about 3.2 cm of translation). Pants never skins
  with them; its name table lists them. The lower-lip and tongue-tip bones
  differ between Rostral and Organ; Organ wins in both orders.
- So the original skins Arm (RightForeArmTwist1) and OuterTorso (all three)
  with Pants' stale matrices; the port used OuterTorso's.

Fix: `CharacterScene::build_skinning_state` fills the buffer in
component-name hash order (stable, so models and LODs keep recipe order within
a component); the returned parts keep recipe order. The default probe in
`main.rs` now checks each model's own inverse binds for the bones its meshes
skin with (the merged buffer would read about 0.045-0.052 against its 0.05
limit, as the original's does).

Not ported: the Beard skip (tutorial_male has no Beard); the order of models
and LOD nodes inside one component (unverified; recipe order kept).

## 4. Hat, hair and head: ruled out

No port-vs-original difference was found for the head area besides the hair
term above. Measured and checked:

- Geometry (bind pose, the port's decode `SHORT4N * 2 + (0, 0.8, 0)`): the
  LOD 0 hat never enters the head, 0.63 mm at its closest; head vertices
  under the brim are at least 0.22 mm away (Head plus eyebrow weights). The
  hair is the hat-hair model below the hat line; 91 card vertices are rooted
  inside the scalp by design, the nearest surface gap 0.04 mm. LOD 2 is
  tighter (0.006 / 0.004 mm).
- The hat is 100% Head; the hair is 98% Head, 2% Neck1.
- No blend shapes on any tutorial_male mesh.
- Depth setup is the original's: `XBox2GraphicsMan::Init` 0x8282c3d8 uses
  D24S8, viewport MinZ 0 / MaxZ 1 and clears to 1.0. The near plane is
  `CameraPresentation::GetNearPlane` 0x827ef8d8: `mICENearPlane` for the ICE
  camera, 0.8 for an NIS sequence camera, otherwise `kNearClipPlaneDistance`
  (0.1), which the port uses. Depth32Float has the same 2^-24 step at these
  depths: about 0.005 mm at 3 m and 0.02 mm at 6 m.
- The per-schema face pose and the per-frame replay reset are ported
  (2026-09-23), so face bones cannot drift.
- Every part's palette updates from one published generation per frame.
- The ghost technique (depth prepass, then blended LESSEQUAL, pass-major) is
  ported as the manifest states.

The remaining gaps (hair roots 0.04 mm, brim 0.2 mm) are authored and the
same in the original. The character is still the diagnostic unlit diffuse,
without the original's lighting over those gaps. A screenshot or clip with the
camera distance is needed to find what is left.

## 5. Files

| File | Change |
|---|---|
| `apps/skate2-pc/src/render_inspection.rs` | `WORLD_SPACE_EFFECTS` from the microcode |
| `apps/skate2-pc/src/gpu_renderer/environment_shading.rs` | `world_clip`; `@invariant` on every environment position output |
| `apps/skate2-pc/src/gpu_renderer/map_draw.rs` | `@invariant` diagnostic position |
| `apps/skate2-pc/src/gpu_renderer/character.rs` | hair `0/z` term removed |
| `crates/skate2-core/src/assets/character.rs` | inverse binds filled in component-name hash order; test |
| `apps/skate2-pc/src/main.rs` | default probe validates per-model inverse binds |

## 6. Tests

- New `character` test: Pants wins a shared bone over OuterTorso although the
  recipe lists Pants first; parts keep recipe order.
- New `gpu_renderer` test: the camera-relative matrix reproduces the world
  clip position (section 7).
- Not run (no cargo in this workflow).

## 7. Follow-up: camera-relative clip math (the map-wide z-fighting)

Report (same day): tree leaves, wall textures, the hoodie sleeve over the arm
all flicker wherever two surfaces are nearly coincident.

Cause, measured (scratch `depth_precision.py`, float32 FMA chains as the
GPU evaluates `clip_from_world * (p, 1)`; near 0.1, far 2000). The city's
coordinates are thousands of metres from the origin (stream cells at
x -3500..-100). Each product rounds by ~1e-4, and the projection turns an
absolute error in clip z or w into a depth error of about
error * distance / near:

| Camera | Distance | World-space depth error | 2 mm gap order flips | Camera-relative error |
|---|---|---|---|---|
| (10, 1.7, 5) | 3 / 20 m | 0.06 / 0.57 mm | 0 / 400 | 0.01 / 0.66 mm |
| (-3500, 1.7, -900) | 3 m | 11 mm | 143 / 400 | 0.18 mm, 0 flips |
| (-3500, 1.7, -900) | 10 m | 30 mm | 70 / 400 | 0.21 mm, 0 flips |
| (-3500, 1.7, -900) | 20 m | 57 mm | 14 / 400 | 0.52 mm, 0 flips |
| (-3500, 1.7, -900) | 40 m | 74 mm | 110 / 400 | 2.4 mm |

The depth buffer itself is not the limit: the original's is D24S8 over 0..1
(section 4) and the port's Depth32Float has the same 2^-24 step there.

Fix (`gpu_renderer.rs` `CameraRelativeClip`): the clip matrix's translation
column is rebased at the camera in f64 on the CPU, and every GPU position is
transformed as an offset from the camera. The product is the same;
only the rounding changes.

- World effects and the diagnostic path: `Globals` carries the rebased
  matrix (`clip_from_camera`) with `g_vViewPos`; shaders transform
  `position - g_vViewPos` (`world_clip`). The sky's positions are already
  camera offsets. Fog, lighting and CSM receivers keep absolute positions.
- Skater: each bone row's translation lane is rebased at the same camera on
  upload (`row_bytes`), and parts use the rebased matrix.
- Shadow casters: the cascade matrices are rebased at that camera too.
- CPU culling keeps the true matrix.

## 8. Skater cloth (ROPA) is not ported

tutorial_male's OuterTorso (hoodie) is a `defaultcharacter_ropa` part. In
the original, after `SkaterPresEntity::ClothInit` 0x827d7d88 it is a CPU
cloth simulation, `ROPA::Effect` (particles, distance constraints, bone
drivers, sphere/tube/plane collision volumes; `TubeSolve` 0x8285be88,
`SphereSolve` 0x8285c438), gated by `cv_EnableRopa`. Its skinned input
comes from `InitSkinParams` 0x827d2938 with the model instance's merged
`m_matrices` (so Pants' right-arm twist matrices reach it too). The port
draws the part through its GPU branch (i_vtx_params.x > 0) only, so a
sleeve can still intersect the arm where the cloth would have collided.
