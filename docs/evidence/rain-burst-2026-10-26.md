# Rain burst correction preview33,2026-10-26

Owner clarifies the steady rain continues while a dense separate burst cycles
through it. Preview32 render-clock change alone did not resolve it.

## Reproduced source defect

Rain volume follows the camera. A particle above its new top (after a camera
move downward) entered the vertical wrap branch. max(low-y,0) made residual zero
for every such particle, resetting all to the identical top height. Their common
18m/s downward velocity then repeated the sheet every23.2/18=1.289s.

A120Hz production-generator probe drops the camera10m after one second, then
keeps a small smooth bob. Of6236 droplets,2627(42.13%) concentrate in one of64
height bins; the phase-distribution regression fails in Preview32.

## Correction and scope

Use signed periodic wrapping, low+(y-low).rem_euclid(height), for both lower and
upper crossings. Derive the birth-time residual from this wrapped position;
retain the existing bounded wind and trajectory-cover cache behavior. Do not
change density, rain audio, water material, surface contact effects, native
physics, controller settings, online code or Preview32's monotonic render time.
This is an authored PC weather correction, not a retail-parity claim.

The formerly failing production test, existing weather component tests and real
GPU checks gate a separate local preview. A short silent actual-map run checks
the final binary. Full workspace regression is outside the owner's short-test
scope. Contact-effect overhaul and grazing-angle dark-water work remain pending
while the owner reviews rain continuity. No public publication.
