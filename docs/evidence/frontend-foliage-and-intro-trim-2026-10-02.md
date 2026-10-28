# Frontend foliage lighting and requested intro trim

## Cause and original evidence

The new main-menu world path retained the ICE camera axes in
Inspection.camera_axes, but the g_ViewDotLight producer used shadow_inputs
or normalized(camera_position - loading_focus). The main menu deliberately
has no skater shadow caster; its loading focus is the final authored camera
position. At the settled menu, subtraction therefore produces exactly zero,
normalization produces NaNs, and the tree/treeanimate lighting receives an
invalid multiplier. Textures were resident and alpha cutouts still worked.

Live IDA RenderMan::UpdateRenderParams82852780, freshly checked:
- GetMatrix at82852870 writes the affine camera at stack+0xb0.
- 82852874/8285288c load stack+0xd0, i.e. camera axis2.
- 82852894..828528f8 normalize and publish g_vViewDir at global+0xd0.
- 82852d6c/82852d88 reload that direction;82852d94 dots the authored
  rendering direction. Decode raw VMX128 register operands: IDA's printed
  high VA bits are wrong (v95 is v63 here).
- 82852e00..82852e0c pack response.x + dot*response.y, minimum, backlit,0
  into g_ViewDotLight at global+0xf0. No dependency on a shadow caster.

The existing render_inspection.rs producer now prefers the explicit camera
axis2. Gameplay without explicit axes retains its existing actual-camera
shadow input; the standalone inspection camera retains its look-at direction.
This reuses EnvironmentLighting::view_dot_light and the shipped foliage
shaders; no material recoloring, light clamping, texture replacement or
brightness compensation was introduced. CAS also uses its supplied axes.

## Intentional intro preference

The PC intro starts further along the alley camera animation. Startup
now skips BootToPressStart (1.888895s) and starts PressStartToMainMenu at
0.75s (45/60), leaving2.25s of its original3s curve. This trim is explicitly
a PC presentation preference, NOT a recovered original timing constant.
The remainder's authored positions, shake, FOV and playback speed, and the
Quit-return track, are unchanged.

## Validation

Current ICE source standalone alley_probe.rs sampled0.75,1,1.5,3 seconds
from the original asset. All camera axes and view-light responses finite.
Final axis[-0.6511618,0.18124115,-0.73698026] gives0.56444496 with the
authored direction/response. Release build PASS6m37s, plant-lighting-build.log.
Original-asset main-menu startup/navigation run exit0. Same settled-camera
capture main-menu-foliage-fixed.png inspected: formerly white plants are green.
Fixed region(415,475)-(575,545) in1280x720 capture has0 bright-neutral pixels
versus1084 previously (minRGB>200, maxRGB-minRGB<5). No exposure change.

## Startup flash follow-up

Gameplay appeared briefly before the alley intro. The window
initializes with the retained gameplay map; controlled boot can return Pending
for multiple redraws, and those frames previously had cover alpha0.
LiveSkatingClient::initialize now applies the existing fullscreen cover at1
when startup has a main-menu focus. Pending boot preserves this cover; after
update_main_menu has replaced the scene, the existing end-of-frame fade
publication reveals the prepared ICE view. This is host loading protection,
not an invented original-game fade speed. Quit/return uses existing fade flow.
PC app rebuild PASS2m15s, startup-cover-build.log. First-present screenshot
startup-first-present.bmp is black except the FPS overlay (nonblack bounding
box10,10..185,35); all world pixels below y55 are exactly0. The one-frame
capture intentionally ends before any SIM completion, so the existing generic
bounded-test postcheck reports no completed skating frame; this is not a boot
failure. Full startup-covered-roundtrip run exit0,213 gameplay ticks,
settled menu restored. Only isolated diagnostic saves used.

## Dullness report: follow-up audit, not declared fixed

The menu and possibly gameplay then looked dull in
brightness/saturation. Fresh live IDA confirms missing shared postprocessing:
- PostFX::Apply8284c7a0: SetParameters, GenerateBloomBuffers, normal or fast
  Uber processing, then Blit_ToDeviceRT82831c18. Colour-cube end is gated by
  activeEffects bit4.
- GenerateBloomBuffers calls Tonemap82828f68 at828332e0. Tonemap reads its
  low-resolution buffer and adjusts environment/character exposure within
  authored bounds, with ICE override support. Our EnvironmentLighting exposes
  static Vault envattributes[2] instead; material shaders already have their
  native local tone curves, so this is specifically missing adaptation.
- UpdateColourCubeBegin8284ed78 selects original32-cubed colour data and
  blends district targets through the native cache/state flow. No equivalent
  colour-cube GPU pass is mounted in the PC final renderer.
- Blit_ToDeviceRT also adds max(bloomCoefficient[0],0) to vignette intensity;
  current freeroam reader has no bloom producer and leaves this lane0.
- ApplyToneMap82829350 writes fixed exposure2 ONLY in the no-PostFX branch
  of RenderModeScene82857a78 (call82857e10). Do not use this as a universal
  exposure correction for the missing active PostFX chain.

The PC final-blit default brightness1.188 remains the earlier PC
adjustment, not an original value. No new exposure/saturation tuning was made
in response to this report. These are confirmed parity gaps and plausible
contributors, not proof of each stage's visual contribution in this screenshot.
Exact matched-view comparison and full source-backed PostFX integration remain.
