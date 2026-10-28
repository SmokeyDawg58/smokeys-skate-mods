# Rain and close-water preview31, 2026-10-26

Owner corrections to preview30 are authoritative: ground flickers, close shores
step, underwater silhouettes distort, screen bottom stretches, water is too blue,
wheel spray is coarse, rain impacts are faint, and collection sometimes floods
implausible rectangles. Keep distant accepted appearance, controller/gameplay,
cloud photographs, lightning and existing audio preferences. Local review only.

## PC-authored changes (hypothesis for appearance; not retail parity)

- Refraction never clamps an offscreen projected sample onto a screen edge.
  Displacement is limited to3 native pixels and fades at borders/near the water
  line. Skater/native character stencil protects destination and source samples.
  Submerged/near-surface views use undistorted transmission; mean-plane coverage
  stays stable while waves perturb normals. Absorption/scattering and wet-bed
  tint are nearly neutral; close reflection is less blue.
- Candidate levels and their support interpolate continuously with quintic
  weights. Coverage fades across missing/dry samples instead of selecting any
  nonempty corner for an entire0.5m cell. Ground confidence distinguishes real
  receiving floors from optical seam interpolation; wrong stacked surfaces
  reject water before shading. Physical floor/depth still occludes the water.
- Fields anchor to the published local skater's feet/bounds rather than camera
  bob. Horizontal/vertical hysteresis avoids tile churn. Additional geometry
  streaming for the same map keeps current water until its replacement arrives.
  Large unknown holes remain drainage outlets: only narrow bounded seams may
  receive solver support. This local single-valued model does not recover all
  city drains, catchments, stacked interiors or physically timed rainfall.
- Eight bounded contact wakes perturb the actual water normal/highlights.
  Published native board pose, skater hips/bounds and solved depth gate entry,
  wheel and walking effects. Feet are approximated by native world-bounds minimum
  and hips XZ; wheel offsets remain authored. They are presentation approximations,
  not recovered foot collision shapes. Replay, stale poses and teleports reject
  trails. Carried/tilted boards do not create wheel spray.
- Wheel spray uses up to128 fine velocity-aligned droplet ribbons with gravity,
  short life, near-camera fade and soft analytic edges. Large round billboard
  blobs are removed. Rain impacts are more frequent/varied and remain filtered
  by pixel footprint. Existing short-lived contact waves fade naturally.
- Fine rain streaks vary width/length/taper. Coherent17-second gust envelopes
  gradually introduce sideways birth velocities; each droplet keeps its exact
  trajectory for cover caching. This avoids per-pixel map traces or repeated
  shelter rays for continuously changing direction. Native roof/geometry cover,
  bounded density, camera rejection and frame-rate invariant integration remain.

No physics, scoring, network protocol, simulation-clock or master/Windows volume
change. Rain/audio settings remain native; tests route audio only to per-run files
and scratch copied profiles. Existing MIT water notices and CC0 cloud credits
remain. No new third-party code or game assets imported.

## Validation scope

Focused production GPU pipelines at1x/4x, narrow geometry/drainage tests, actual
wet/dry/air/replay/teleport contact cases, walking wake case, wind bounds and
frame-rate particle integration checks. Preview30 pool approach trace proves the
native controls enter water (wheel-spray event between frames180 and240). New
close/low-camera, bowl/walking and city/4K captures are required before installation.
GPU fixtures/captures cannot establish exact retail parity or a universal120FPS
lock. Private receipts retain all attempts, including failures.

## Native close-water correction

The native pool approach/braking/walking trace reached Root1323.76/87.81/-469.39
with solved water88.404. It exposed a carried-board orientation branch clearing
all foot waves each tick. Only discontinuous/teleported poses now clear trails;
a carried board simply stops wheel emission. The focused contact test includes a
moving actor AND a carried board, and checks that wake age exceeds0.12s. Close
ripple highlights are stronger while distant response stays unchanged. Minimum
projected rain width with opacity compensation reduces subpixel streak popping.
GPU water-coverage probing confirms water over the floor; a clear material is
not treated as a missing puddle merely because the native bottom remains visible.

## Native multilevel placement probe

A contact/water-only diagnostic overlay exposed surface-height switching at the
pool rim: two successive height samples could leave an unconverged plane on a
vertical wall. Reject a final surface/level disagreement above25mm, and fade
native-floor disagreement continuously from10cm to28cm, including inferred seams.
The same trace retains a smooth low-pool boundary and expanding walking rings
while removing the tall wall curtains. Close contact-ring highlights have an
additional bounded neutral term; distant appearance remains unchanged. These
are conservative visual admissions, not recovered fluid geometry.

## Owner-requested After Rain birds

After the render checks, the owner requested very faint birds only after rain.
Four two-second excerpts of iwanPlays CC0 MorningBirds.wav are embedded as
mono24kHz PCM. Linear resampling retains original pitch in the48kHz mixer;
source fades,2.5s entry delay and authored7-17s quiet gaps avoid constant chirps.
Both channels receive identical low-level signal, maximum0.0096 before existing
SFX/weather gain. Pause/hidden/mute/non-AfterRain gates disable the calls.
Existing rain bed levels, normal profile, master and Windows volumes are unchanged.
See weather_audio/AFTER-RAIN-BIRDS.txt for source/license and preparation.
