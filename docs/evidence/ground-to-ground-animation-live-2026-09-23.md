# Ground 100 to GroundAnimation 103 in the public skating session

The original-data public facade now drives a Kickflip, releases it, and then
continues neutral frames before an Ollie. On the fourth neutral continuation,
the selected frame stopped at `SystemLogic(Selection(PendingTransition(103)))`
in `artifacts/public-facade-ollie-expr13-2026-09-23.log`. Here 103 is the
destination physical state ID, not a transition-table index.

IDA `CalcSuggestedState` at `0x82ddbd58` checks the Ground100 off-ground case
at `0x82ddc2ac`, then calls `Skeleton::IsSkateboardAnimated` at `0x82ddc2d4`.
With animation active and teleport countdown zero it returns 103 at
`0x82ddc2f0`. `CheckForStateChange` writes framesSinceJump=100 at
`0x82e005f8` before `SetPhysicsState` at `0x82e005fc`. The generic native
state switch calls the controller, publishes transition fields, exits Ground,
then enters GroundAnimation. Two independent IDA reviews agreed on this path.

The existing Rust selector, skateboard-animation query, Ground exit, and
GroundAnimation entry already cover these operations. The only new runtime
admission is `(Ground, 103)` in `selected_transition` and the corresponding
stage guard of `physical_player_ground_to_air.rs`. Other unsupported
predecessor/destination pairs remain rejected. The existing route unit test
was updated; the public-session replay is the stronger lifecycle check.
