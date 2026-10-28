# Darkcatch cancellation on an unsupported ground-animation transition

Owner report: a performed trick awarded no points. The retained ordinary-game
journal is public source02c638c, EXEcb35b793. At source tick100560 the online
darkcatch guard rejected a pending853.125-point sequence with landed=false and
state103. Source output100561 has zero wheel contacts, rising deck velocity
11.384372m/s, and a retained deck-contact bit. The completed deck probe at
100560 has deckY15.485283 and probeY11.806385, far below the physical deck.
States return to200/201 two frames later. The shared score output goes from
654HUD points at host101032 to zero at101040, without a wipeout.
Other preceding combinations did bank6357 and11344 points before completion;
zero banked at their later completion frames is not itself a scoring failure.

This is an explicit PC online rule, not retail reconstruction. Keep its existing
held-darkcatch landing rejection, but require actual wheel support or a board
contact with a nearby completed deck-probe plane. Plane separation uses the
existing native WHEEL_LINE_LENGTH reach; no damage or scoring value is tuned.
Use plane distance rather than point distance so tangential travel on ramps
does not invalidate a support result. Missing probes cannot reuse old points.
The physical Landing event remains authoritative. Unsupported GroundAnimation
blips do not clear the in-flight catch history or reset the same sequence.

Focused checks retain held-touchdown rejection and late-release behavior, valid
prelanding release, offline behavior and teleport reset, plus the logged
unsupported transition and nearby deck/wheel support cases. Confidence: high
for the observed cancellation; support policy is a scoped PC decision.
Keep the Easy Motorized landing fix from480debc. Diagnostics remain silent and
offscreen with copied saves. Public Update21 stays unchanged pending review.
