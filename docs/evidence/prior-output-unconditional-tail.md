# Update1 unconditional prior-output tail

Confidence: high; raw PPC and PDB member offsets agree on the selected
`PhysicalPlayerHiLOD::Update1_ProcessInput`82e02a2c..2e5c continuation. It stops
before the first Skeleton copy at2e60. These Ground, Air, HandPlant and Grind
operations have no category gate. Every output access independently checks the
PhysOut write latch, including adjacent reads of the same component.

Actual `PhysOut_State::SetResetValues`82e25550 stores category INVALID0 at556c
and physics state INVALID0 at5570. These are distinct from the retained
PhysicalPlayerState owner's current Ground100 state. A first frame cannot skip
Air or Ground copies merely because either category looks inapplicable.

| Source store | Actual source owner/member | Processed destination |
|---|---|---|
|2a60|Ground+40 dynamic up|210 vector|
|2a98|Ground+80 anti-flip torque|340 vector|
|2ad4/2b10|Ground121 anti-flip nudge /122 pinning|8cc masks20000000/02000000|
|2b44|Air+a8 trajectory collision time|9d0 float|
|2b80/2bbc|Air1a9 flipping /1ae found handplant|8cc masks10000000/2|
|2bf8/2c34|Air1af landing footplant /1b0 footplanting|8d0 masks04000000/02000000|
|2c4c|retained HandPlantManager5d4 flag10000000|8d0 mask80000000|
|2c88/2cc4|Grinds13a grind-on-front /13e did-jump|8c4 masks200/100|
|2d00/2d3c/2d78|Collisiond75 skeleton /d73 deck /d70 truck contact|8c4 masks8000/10000/20000|
|2dac|Collision0 signed wheel count|90c|
|2de8|Collisiond74 anything-in-contact|8c4 mask4000|
|2e24|Physics20 impulse-for-wipeout|8c8 mask2000|
|2e5c|Physics10 wipeout impulse|330 vector|

HandPlant is sampled only at2c38 after all preceding Ground and Air writes.
The implementation takes the same retained physical owner and its output, and
validates the Processed/output occurrence before copying. Missing HandPlant
preserves all prior writes and stops before Grinds/contact/Physics. Known-mask
tracking on the existing8c4 word remains in its existing jump-events owner.

New retained payload comprises only dynamic-up, anti-flip torque and trajectory
collision time. Wheel count, impulse and flag words reuse their existing owners.
The impulse is a single vector; no invented output-publication origin is attached
to an existing concrete component that did not track such host metadata.

Reset evidence is asymmetric: dynamic-up210 is zeroed at82b3925c (r18 loaded210
at391a4); collision time9d0 is zeroed at3952c. Anti-flip torque340 is not written by
Reset. Raw39378 and39380 both store to330 through unchanged r10. Generic
constructor/Reset therefore preserves unknown or earlier340 contents. The actual
Sim allocation's existing pre-constructor hook establishes zero residual bytes;
the actual Update1 copy always overwrites340 before subsequent Ground reads.

Focused tests distinguish invalid-category execution, late missing-HandPlant
failure prefix, first-getter write-latch failure, and preserved Reset torque.
Root owns Cargo and original-corpus integration; no independent Cargo process
was started by this implementation agent.
