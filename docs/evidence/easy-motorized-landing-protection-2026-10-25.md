# Easy Motorized landing protection

Explicit PC mod policy requested by the owner: Easy Motorized should use the
working Motorized no-hard-landing behavior while retaining Easy handling.

The selected S3 runtime maps Easy Motorized to an Easy-derived test slot via
`set_motorized_profile`. It previously overlaid only MotorEnabled and
MotorTopSpeed. The embedded physics_mode/easy WipeoutCheckForBadLanding is true;
physics_mode/motorized is false. Copy that exact Motorized field into the
Easy-derived slot, preserving asset resolution rather than inventing a new cap.
All other Easy fields remain inherited, including auto-push, perfect body flips,
jump heights, grind lock, pump and collision thresholds. Other difficulties do
not call this hybrid profile path and are unchanged.

The existing source Wipeout check, TU3 82D90358 and bad-landing 82D90D08, consumes
this field through Mode.check_bad_landing. The guarded branch requests reason 6
for excessive normal impact speed, 5 for landing angle and 8 for grind landing.
This matches Motorized landing protection; ordinary collision, explicit bail and
other physical wipeout logic still use their original rules. No global damage
or wipeout suppression is introduced.

Confidence: high for the live profile dispatch, embedded values and guarded
native check. Focused validation uses the exact profile method, real embedded
collections, the production settings loader and the native wipeout kernel.
Game diagnostics, if run, use copied saves, inactive offscreen windows and
per-run audio dumps. Keep playing copies and all volume settings unchanged.
This repair is local pending gameplay approval; public Update 21 stays live.
