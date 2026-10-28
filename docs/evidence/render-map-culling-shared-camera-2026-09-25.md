# Map draw list: clip-volume culling, shared camera, per-texture bindings — 2026-09-25

SOLO. Renderer-only; no simulation, input, physics, animation, camera producer or
tick scheduling code changed. Not compiled or launched.
Rustfmt parse PASS on every edited file. Unit tests added, NOT executed.
Pre-change copies: `artifacts/render-perf-backup-20260925/`.

## Measured baseline (release build, Vulkan, 164 Hz, offboard-city-walk, 900 frames)

tick ~6 ms, view ~0.25 ms, present 7–9 ms scaling with 3481–4416 map draws,
~65 fps. City swap frame present 30.9 ms; builder/loader window present 10–15 ms.

## Reference (debug `sk82_na_m.xex`, IDA)

- `Sk8::Render::Frustum::ComputeFrustum` 0x8282bd28: six planes from the combined
  matrix (w+x, w-x, w-y, w+y, w+z, w-z), optional normalize.
- `Sk8::Render::SceneRenderView::Cull` 0x828500a8: per presentation entity AABB
  gathered, `VectorizedCullingJob` 0x82859370 over the list; visible entities call
  their per-mesh cull (vtable) with flag 8 and are appended to the visible list.
- `Sk8::WorldPresEntityOptimesh::CullMeshes_VMX` 0x827d0ae8: when
  `kCullOptimeshIslands`, `Frustum::Cull_VMX_NEW` 0x82825948 on the island list.
- `WorldPresEntityOptimesh::FillMtlSrtKeyListWithCulledKeys` 0x827e3068: compacts
  visible island indices in ascending order; pushes the mesh sort key only when
  at least one island is visible.
- `Frustum::UpdateCullParams` 0x82853970 also feeds anti-frustum occluders and
  minimum pixel size/area/distance (layout attrib words 7, 8, 11, 12, 13, 21) into
  the same job. These remove visible geometry.

## Implemented (`apps/skate2-pc/src/gpu_renderer/map_draw.rs`)

- Entity -> island culling shape kept: object AABB test, then per-island test only
  when the object straddles a plane; object skipped when no island survives.
  Order is the retained render-queue order; islands ascending. Nothing re-sorted.
- Planes are the rasterizer's own clip volume (-w<=x<=w, -w<=y<=w, 0<=z<=w for
  wgpu depth 0..1), tested in f64 with 1e-5 relative tolerance; a rejected box
  provably produces no fragment, so the image is unchanged. Deviation from the
  reference, deliberate: near plane z>=0 instead of w+z; anti-frustum and
  min-size rejection NOT applied (they would change what is drawn).
- Island bounds: exact positions each island's indices reference, gathered in
  the existing `upload_mesh` admission loop. Non-finite or projective -> never culled.
  Instance-space effects: affine world bounds padded by 1e-5 of term magnitudes.
- One camera uniform per frame (64 bytes) replaces the per-tick N x 256-byte
  uniform batch; per-object world-from-mesh + color are static instance records.
  World-space effects (defaultenvironment, alphatestdefaultenvironment, tree,
  treeanimate) use identity, so their clip positions are the same product as
  before. Staged instance-space effects now evaluate clip*(world*p) on the GPU
  instead of a CPU-premultiplied matrix: last-bit rounding only, still pending
  shader review as before.
- One bind group per uploaded texture (was one view + bind group per object).
  No per-object uniform buffer/bind group for map objects (previously created,
  then immediately replaced by the batch, on every city generation).
- Draw state: wgpu 0.19 already elides repeated pipeline/bind-group sets
  (`set_and_check_redundant`, wgpu-core render.rs); vertex/index buffer binds are
  now skipped when unchanged.
- Character path unchanged (own pipelines/uniforms), drawn after the map as before.
- `SKATE2_PROFILE_REDRAW` line now also prints acquire/record/submit ms and
  `map_objects=drawn/total map_islands=N`.

## Validation to run

`cargo test -p skate2-pc map_draw` (property test: 3000 random boxes x 216
samples, no kept point in a rejected box), then the same profiled city walk;
expect lower record ms and map_objects well below total when facing away from
most of the loaded city. Visual check: no popping at screen edges.

## Result

Release build run: starting area ~80 fps before -> 165 fps after
(vsync Fifo cap on a 164 Hz display, so true headroom is higher). Dense-city,
section-swap and edge-popping observations not yet reported.

## Opt-in uncapped presentation

`SKATE2_NO_VSYNC` (any value) selects Mailbox, else Immediate, else Fifo when
unsupported; default stays Fifo. Presentation cadence only: the live client's
fixed 1/60 accumulator, input sampling and simulation are unchanged. Rustfmt PASS,
not compiled by agent.

Result with SKATE2_NO_VSYNC: ~1000 fps reported (presented-frame counter,
location not stated; earlier baseline ~65-80 fps with vsync on this route).

## Default image-quality enhancements (not original-parity settings)

- Shared diffuse sampler `anisotropy_clamp: 16` (all filters already linear;
  wgpu-core 0.19 clamps to 16 and forces 1 without ANISOTROPIC_FILTERING).
- 4x MSAA by default: multisampled color resolved into the swapchain image
  (StoreOp::Discard on samples), multisampled Depth32Float, every main-pass
  pipeline (map solid/diffuse/alpha, legacy object, character) uses one shared
  `multisample(sample_count)`. 4x only when both the surface format and depth
  format report it in the device's guaranteed AND adapter features, else 1x.
  `SKATE2_NO_MSAA` forces 1x. FPS overlay keeps its own 1x pass on the resolved
  image; capture copies the resolved image.
- Alpha-to-coverage deliberately OFF: alpha-tested foliage keeps the authored
  ALPHAREF discard exactly, so its cut-out edges stay aliased.
- Original 360 anisotropy/MSAA settings were NOT checked in IDA; these are
  PC presentation enhancements, not parity claims.
Rustfmt PASS (character.rs parse-only; its pre-existing style left unchanged).
NOT compiled by agent.
