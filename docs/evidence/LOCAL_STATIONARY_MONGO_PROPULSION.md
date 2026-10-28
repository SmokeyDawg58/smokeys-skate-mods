# Stationary mongo propulsion: local force preference

The owner rejected preview16: speeding the wind-up did not satisfy the request
for a stronger physical mongo push. Restore the original PlayAnimation Begin
path and remove its2x policy. Original animation timing is required here.

The active source solver already uses the original Skate3 contact-event target
and82D948B8 force calculation. Logs show stationary regular-X contact targets
peaking around3.91m/s, with the deck reaching about3.9m/s after one stroke.
The live ground input has the original8.5m/s push cap, mode-dependent speed-change
limits, actual body mass and timestep. The boost below is an explicit local mod
preference; no reference hardware trace establishes exact retail parity.

During B_MONGO_PUSH_CYC1, latch the signed starting speed at the first actual
unsuppressed push-contact flag (Processed2468 bit25). Only a stroke beginning
below0.5m/s gets the preference. Its target speed gain relative to that latched
origin is1.5x for this stroke, aiming for about5.85m/s from the observed baseline.
Do not scale the current target-minus-current gap: that merely reaches the old
terminal speed faster. Do not change global velocity, animation clocks or caps.
The native force kernel, queue, mass and solver apply the resulting force.

Clear the latch on contact release, another animation, Ground entry or board
reset/teleport. A stroke beginning while rolling is unchanged, even if it later
slows; a new stationary stroke may qualify. The first grounded contact supplies
the foot and timing: raw X/A labels and natural-stance inference are not used.

Validate the physical speed gained through the actual EXE and a fixed input
recording, retaining normal timing, full control-push behavior, original caps,
menus, replay, scoring, landing notifications and audio settings. Silent probes
use copied saves. Keep the result local until owner approval.
