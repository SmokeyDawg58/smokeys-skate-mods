# Local rainwater revamp, 2026-10-26

Owner feedback: preview29 has invisible water, apparently static clouds, no
visible after-rain rainbow, and spray on any wettable floor. Treat all previous
weather appearance claims as subject to that correction. This is a PC visual
mod, not recovered Skate2/3 behavior or a complete fluid simulation.

## Researched open-source references

- Overflow (MIT): https://github.com/fema-ffrd/overflow and
  https://fema-ffrd.github.io/overflow/algorithm-details/fill/
  Published Priority-Flood (Barnes/Lehman/Mulla2014) provides fill-to-spill levels.
  New native Rust implementation uses its boundary-first depression concept and
  independent runoff/volume integration; it imports no Python/GDAL runtime.
- liquid-ripple (MIT, Copyright2026 Axistify):
  https://github.com/AsimRauf/liquid-ripple/blob/main/src/shaders.ts
  Crossing height-field swells, normal-driven refraction and damped ripple
  presentation are adapted to world-space WGSL and existing varied rain impacts.
- shadecore_dev's CC0 rain-puddle postprocess was inspected as a reference:
  https://godotshaders.com/shader/rain-puddles-with-ripples-and-reflection/
  No SSR implementation or separately authored Shadertoy code is imported.

Notices are retained in gpu_renderer/rain_sky/THIRD_PARTY_WATER.txt.
Research downloads/hashes and actual validation receipts remain in the private
local-gameplay-review-r30 audit folder, never uploaded by this work.

## Geometry and shared depth

Exact decoded/transformed map triangles rasterise terrain and overhead cover
separately on a257x257 grid at0.5m spacing (128m square). A camera-near field
is prepared off the render thread and cached up to4 entries, quantised32m.
The visible water fades before the field boundary. Floors on multiple storeys
use a bounded camera-height layer; this is a local single-valued height field,
not full3D drainage through tunnels, pipes or complex stacked interiors.

Priority-flood establishes closed depressions and spill levels; downhill routing
adds exposed catchment rainfall to each basin. Sorted bed heights convert volume
to one horizontal water level, limited to the spill and0.65m local maximum depth.
Light/Medium/Heavy/AfterRain presentation histories use6/22/55/12mm of rainfall.
The amounts are authored equilibria for each selectable look, not real-time
meteorological accumulation. Virtual0..45mm micro-relief creates shallow
pockets on perfectly planar collision slabs; it never changes skating collision.
Real bowl/dip geometry dominates the water shape and deeper capacity.

CPU wheel contacts and the GPU use the same solved levels and wet-bed threshold.
Spray requires at least9mm over the virtual bed and wheel-bottom height overlap;
carried/airborne boards and dry floors do not emit. Wheel offsets are authored
visual approximations (27cm length,9cm lateral,5.5cm below deck origin), not
recovered wheel collision shapes. Existing droplets finish their short lifetime.

## Rendering

The old per-pixel BVH wet-coat path is replaced by buffered height-field lookups
and ray/level intersection. Water is shaded at the actual basin level, including
water above a bowl's bottom, and tested against native scene depth. Transmission,
soft shallow shore, normal-driven refraction, static native terrain/environment
reflection and the photographed moving sky provide visible water. No live player
reflection, SSR, duplicate scene render or global simulation-clock change.
The bounded Medium/Heavy water viewport and full-resolution sky/particles remain.

Cloud azimuth now advances continuously at0.3515625degrees/second, with an exact
loop over the existing4096s clock. The accepted cloud photos/lightning stay.
AfterRain keeps its12mm basin history with no falling rain/audio/lightning;
an artist-placed faint rainbow anchors toward the clearing view on preset/map
entry and then stays fixed in world direction as the camera turns. This is an
authored rainbow placement, not a recovered physically exact sun-scattering model.

## Validation checkpoint

Production WGSL/Naga and GPU submission passed at1x/4x;19 focused tests cover
closed bowls, increasing rainfall, level water, after-rain retention, covered
basins, open drainage slopes, flat-ground pockets, wet/dry/airborne wheel contact,
existing rain ribbon frame-rate invariance and cover/material geometry fixtures.
Actual map basin dumps, multi-location visual captures and performance checks
are required before installing the next separate local preview. Public28 and
installed preview29 remain intact during development. No background PC monitor.

## Smooth-edge and sampling correction

An actual PermissionPool capture showed that missing terrain samples could act
as false drainage channels. Small enclosed gaps (at most256 cells) now receive
solver-only support at the surrounding crest. They remain invalid for rendering
and wheel contact; open missing regions touching the domain boundary still drain.
Raster height intersections accept either native face winding, including slopes.
This is a bounded visual sampling repair, not a claim about hidden native drains.

Virtual micro-relief uses smooth interpolation on CPU and GPU. Shoreline opacity
widens with the pixel footprint; distant subpixel ripple detail fades out. The
low-resolution water pass uses physical terrain-plane distance for depth-aware
upsampling rather than radial distance across different rays, which created
stripes at grazing angles. Native scene MSAA remains intact. Unused legacy GPU
triangle/CDF uploads are removed; the CPU rain-cover BVH is retained. These steps
reduce aliasing; actual captures remain the acceptance check, not a promise of
mathematically zero aliasing at every resolution or camera position.

## Owner correction: water needs smooth depth, not a flat mask

The owner rejected the previous low-resolution water edges as jagged and asked
for near-volumetric-looking puddles/water (not clouds). Native Pool close-ups
confirmed staircase holes at invalid material/geometry samples. The final path
therefore removes the low-resolution coat target, pass and upsampler entirely.
Buffered local water lookup runs once per native pixel; expensive map BVH rays
and duplicate scene rendering remain absent. Sparse invalid-cell contours use
bounded5x5 neighbouring valid levels; actual native scene depth determines whether
water is in front of the floor/objects, closing sub-grid seams without drawing a
synthetic floor. The coarse hydrology/CPU contact remains conservative around
these missing map samples; visual continuity is not proof of exact collision data.

Water now measures view-ray entry/exit thickness, exponentially attenuates the
bottom colour and adds a small depth-coloured scattering term. Refraction uses
Snell's ratio1/1.333, projected scene colour and a depth-checked fallback to avoid
foreground skater/wall leakage. Water Fresnel starts at0.0204. Authored smooth
sum-of-sines swells are damped in the shallows; they leave the mean solved water
volume and game physics unchanged. These optical effects give water depth cues;
they are not a full3D volumetric fluid solver or exact multiple-scattering model.

Technique references (concepts only; no listing copied): GPU Gems chapter1,
Mark Finch, https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-1-effective-water-simulation-physical-models
and GPU Gems3 chapter19, Rusty Koonce,
https://developer.nvidia.com/gpugems/gpugems3/part-iii-rendering/chapter-19-deferred-shading-tabula-rasa
for depth-controlled water appearance, continuous waves and depth-tested refraction.

Prototype Pool close-ups and a4K Heavy check are retained in the private audit.
They establish neither a universal120FPS lock nor zero aliasing on all displays.
Final embedded-source map checks remain required before separate installation.

## Bounded native-resolution cost

Wider actual-city runs exposed high per-pixel seam-search cost. The25-neighbour
continuity calculation is now prepared once with the field on its existing worker
and cached as a second32-byte/cell optical buffer. CPU collision/contact samples
retain the original conservative data. GPU admission reads four cached cells;
it never searches a neighbourhood or traces the map per pixel. Far surfaces and
native geometry above a candidate water plane reject before optical shading.
A modest wet-bottom attenuation (.78/.84/.88) makes shallow water easier to read
while retaining transmission/refraction, rather than boosting global exposure.
The extra cached buffer is about2.1MB/field, four cache entries, with no new worker,
service, monitor or duplicate scene draw. Actual city/4K paired checks are kept;
variable GPU-sharing conditions do not establish a universal120FPS guarantee.
