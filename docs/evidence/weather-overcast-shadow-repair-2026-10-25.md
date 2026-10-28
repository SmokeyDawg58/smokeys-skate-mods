# Local weather lighting and stair-face repair

Owner screenshots show directional sunlight on the skater beneath storm clouds,
hard sun shadows and banded wet shading across near-vertical stair risers.
The first weather adapter returned original character-stencil pixels untouched
and composited clouds only after the native solar disc and character key/rim
lighting had already been shaded and bloomed. Native renderer paths are retained
in Clear. Weather is an explicit PC modification, not a reconstruction claim.

For selected precipitation, the same cloud-cover amount used by the weather
shader attenuates character key/rim diffuse and specular RGB while preserving
ambient SH. CAS hair has its separate direct-light branch. The otherwise unused
map animation-time W lane carries that cover into the sky shader before bloom,
world solar specular and CSM visibility. Direct hard sun shadows fade into
diffuse storm illumination; native baked lighting and contact/AO remain.
Original animation time X, physics, scoring, audio and settings are untouched.

Weather normal reconstruction selects short same-surface depth neighbours
instead of derivatives spanning a stair edge. Rain puddle/reflection evaluation
is limited to upward-facing surfaces, avoiding horizontal-water reflection
math on vertical risers. Characters remain excluded from snow/puddles but now
receive the common overcast colour response and distance haze.

Confidence: high for the identified integration omissions; the resulting
lighting adaptation is a PC weather choice, not recovered Skate3 host tuning.
Short silent real-map comparisons and shader validation accompany the private
preview; no public update is authorized by this change.

A further owner screenshot isolates the jagged snow boundary under overhead
geometry. The original binary 1024px/64m shelter test generated visible texel
steps. The private repair uses a world-anchored 2048px map, bilinear comparison
sampling with a 3x3 tent and fractional coverage in the surface blend. This
keeps the existing 15cm receiver allowance and 96m/160m coverage. Particle
shelter still tests the filtered coverage at one half. Multisampled character
coverage averages stencil samples instead of picking one binary sample, and
upward wetness fades smoothly instead of applying a horizontal-water reflection
model to vertical faces. No display preference or audio level is changed.
