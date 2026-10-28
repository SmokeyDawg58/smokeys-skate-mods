# Local rain/snow adapter from installed Rust Skate 3 Remaster

Owner requested rain and snow from the installed Skate3Remaster, local testing
only. Source package revision038ff33156fb86be5b86e37aac56deddea0b2400/build10
contains its custom weather WGSL as embedded text in skate3rust.exe. Its
CustomEngineLayer MIT notice is retained in weather_common.wgsl and alongside
the adapter. The public repository was unavailable, so this is an adaptation
of the shipped shader text and supplied prepared audio, not a claim of access
to the complete weather host source. Evidence/provenance is kept in the local
weather-port audit.

The exact hash, noise, cloud, wet/puddle/ripple and snow helper bodies are
retained. The precipitation shader retains its36x24x36m world-wrapped box,
rain/snow fall vectors, flake sway, minimum pixel width, edge/near fades and
alpha shapes. The host supplies WGPU scene colour, actual depth, camera-relative
projection and an independent vertical1024px static-world shelter map. Character
and vehicle stencil masks exclude those surfaces from the ground coating.
Sky, wet surfaces and settled snow are composited before the original final
blit and HUD. Clear bypasses the weather resources and original blit remains
unchanged. Existing physics, friction, scoring and HUD placement are untouched.

Adapter decisions: native Settings > Mods > Weather selects Clear, Light Rain,
Heavy Rain, Light Snow or Heavy Snow. Audio Settings > Rain Volume is a separate
0..100% slider, multiplying the existing SFX level. Both persist in weather.json
beside the active preferences; no profile or system volume is modified. Light/heavy
amounts0.35/1 control particle density; any snow setting lays the full cover.
Surface normals/light are reconstructed from depth/colour instead of modifying
the native materials. The shelter covers a64m square,160m vertically; a15cm
receiver allowance avoids self-shadow. It uses static map caster geometry, so
movable props do not create additional weather shelters. No multiplayer weather
synchronization, thunder/lightning, seasonal modes or skater spray are added.
The original renderer's output is preserved when Clear is selected.

Rain beds are8-second excerpts from the provided CC0 prepared bed_light_0 and
bed_heavy_0 recordings, crossfaded250ms into short stereo48kHz PCM loops.
CREDITS.txt retains the supplied recording notices. A separate voice enters
the existing device, silent dump and replay-export paths; it follows current
SFX volume and pause state without modifying any normal game/Windows settings.
Snow adds no rain bed.

This is an explicit PC visual mod with adapted host integration, not a native
Skate2 reconstruction or complete1:1 Skate3 weather-parity claim. Preserve
Easy Motorized repair480debc and scoring repairc0f0a37. Only app rendering,
native weather menus, weather preference and additional ambience change in this step. Short actual
game checks use copied windowed saves, inactive offscreen windows and per-run
audio dumps. Public Update21 remains unchanged pending owner review.

Local visual verification found that native g_FogColour is pre-exposure, while
the adapter samples the already graded scene. Using its raw low RGB made flakes
dark, and using dark ground albedo as baked irradiance suppressed snow coverage.
The post-grade PC adapter therefore floors atmospheric colour at 0.6*tint(),
approximates snow irradiance with min(sqrt(scene.rgb)+0.35,1), and uses the native
snow_cover's material-independent light=1 branch. These are explicit adapter
choices, not recovered host tuning. Distance haze blends into the same moving
cloud field at 1-exp(-distance*0.006*amount), avoiding a mismatched far-sky patch.
Actual per-pixel normal/shelter/stencil inspection confirmed exposed upward
ground, covered awnings, and protected skater geometry. Prepared screenshots
also show the native Weather selector and Rain Volume slider receiving pad
input and saving only weather.json, with the copied profile byte unchanged.
Explicit SKATE2_DIAGNOSTIC_WEATHER_SHADER and FRONTEND_CAPTURE hooks allow
bounded per-process inspection; ordinary launches use the embedded shader and
the unchanged frontend path. No background service or public update is added.

## Photographed storm-sky preview (2026-10-26)

PC mod only, authored appearance; no claim of original-game sky parity.
Poly Haven CC0 Approaching Storm (Greg Zaal) and Quarry Cloudy (Dimitrios Savva / Jarod Guest) replace the smooth procedural cloud field. Sky-only crops, provenance and hashes are in gpu_renderer/rain_sky/CREDITS.txt. Embedded JPEGs decode/upload once per rain target, then release CPU pixels; no runtime network or per-frame upload. Two 4K texture samples, slow seam-continuous azimuth drift, small lower scud modulation and isolated existing lightning. No volumetric ray marching or live reflection re-render. Native clear-sky MSAA edge replacement and all prior rain/performance/controller/audio/log changes remain.
Production WGSL, real GPU submission at1x/4x and16 geometry/ribbon/spray component checks passed. Real-map appearance/performance and private preview packaging are pending.

## Puddle/ripple appearance refinement (2026-10-26)

Owner screenshots show dark outlined broad noise lobes and uniform large rings. The PC shader now rotates and domain-warps three contour scales, carries analytic gradients through the warp Jacobian, and fades the complete water response over a world-space shallow shoreline. Heavy/Medium coverage is reduced while Light/AfterRain coverage is retained; no extra geometry or reflection pass is added. Reduced substrate/ripple contrast avoids dark rims and chalk outlines. The visual depth is an authored contour distance, not a fluid or terrain-drainage simulation.
Impact seeds vary lifetime, growth speed, ring width/frequency/strength, spacing and occasional secondary packets. Smaller rings and softer contrast retain a bounded3x3 neighbourhood; no new particle allocations or ray queries. The legacy CPU field/CDF remains a documented fixture, unused by runtime puddle contours; wet-board spray still uses the actual admitted wet terrain and roof queries.
First photo-sky capture exposed foreign terrain because an incorrect crop included83% of the full panorama. Corrected assets retain only v=0..0.415 of the full photograph. Sky-only image review, final real-map multi-location captures, and performance checks are required before delivery.

## Higher-preset water viewport and multi-location check (2026-10-26)

Owner requested multiple authored starts. Real-map Waterfront, Mongocorp, Permission Pool and Boneyard were captured with board/foot and different native camera angles. Mongocorp exposed83.6FPS with10.82ms rain overlay despite the Waterfront case reaching163.7FPS at1440p. Indoor cover queries dominated the water pass.
Medium/Heavy now use one quarter of the Light/AfterRain water viewport area. Geometry, stencil rejection, cloud photos and rain/spray stay full resolution; depth-aware upsampling smooths the water. Sky shading moved to the full-resolution compose pass, retaining photo detail independently of the water budget. Impacts older than every possible lifetime skip per-impact hashes/waves; fully Light/Heavy skies skip the unused second panorama sample.
A diagnostic WGSL prototype of that same grid/shading path reached140.3FPS at Mongocorp1440p (overlay4.41ms) and150.0FPS Waterfront4K (overlay3.59ms). These are short uncapped headroom checks on RTX3080Ti, not guarantees for all maps/machines. Final embedded build and multi-location capture receipts must accompany the private preview.
