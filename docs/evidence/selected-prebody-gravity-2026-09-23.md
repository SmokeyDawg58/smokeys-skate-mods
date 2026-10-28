# Selected board gravity before body admission (2026-09-23)

The controlled live boot previously applied original Vault gravity only at
`try_prepare_selected_queued_frame`, after it constructed the physical board
and skeleton. Rust `SimulationRigidBodyPool::try_add_rigid_body` copies the
pool's current gravity into a new Active body's `force`; `set_gravity` updates
only the pool field. The pool begins with `Simulation::Initialize` gravity
`[0,-600,0]`. The original selected Vault vector in the mounted corpus is
`[0,0xc11ccccd,0]`, about `[0,-9.8,0]`. Consequently the late setter left a
first-tick body force at `-600`, explaining the observed neutral first-frame
deck velocity near `-10` and downward displacement about `0.1667` at 1/60.
The precise simulation response remains subject to the source-selected
solver/initial-state path, so this is a source-supported causal explanation
of the Rust run, not a matched native per-tick measurement.

Independent IDA/PDB review of `sk82_na_m.xex.i64` confirms `Island::Island`
binds physics attributes at `0x827bab70` and copies the four-lane Vault
gravity into Simulation `+0x90` at `0x827bad30..38`, before gameplay body
admission. `RigidBodyPool::AddRigidBody` Active loads that current Simulation
gravity at `0x82ae6f1c` and stores body force `+0x90` at `0x82ae6f24`;
initial velocity is zero. `StartFrame` sets time/counters but does not rewrite
force; later `DynamicUpdate` restores force from current gravity only after
the collision/solver phase. The free `CalculatePartBBox` used by the
SkaterPool bounds job reads the live body force as well as velocity; Rust
also includes both, so a missing force term was not the issue. Native W is
zero in the shipped gravity layout; this selected Rust owner transports XYZ.
See [native first-frame query gates](native-first-frame-ground-query-gates-2026-09-23.md).

The controlled facade now resolves its selected gravity immediately after
the retained boot prefix creates the world and assets, and sets it on that
same world before terrain selection, player/Actor construction, and physical
body Add. The later frame-preparation setter was removed, rather than
duplicated. The lower-level cross-owner original-data corpus, which bypasses
the facade, now makes its one gravity selection before constructing bodies.
An original-data facade assertion checks both the pool vector and initial
force of the first seven board bodies are exactly `[0,0xc11ccccd,0]` before
the first tick. It passed in
`artifacts/original-facade-prebody-gravity-2026-09-23.log`; the cross-owner
corpus passed in `artifacts/original-cross-owner-prebody-gravity-2026-09-23.log`.
`cargo check --workspace` passed in
`artifacts/check-prebody-gravity-2026-09-23.log`. An independent source/code
review found no material boot-order issue or duplicate facade setter.

The release two-frame query capture
`artifacts/controlled-skating-release2-prebody-gravity-2026-09-23.log`
still admits zero board triangles in frames 1 and 2 while the board descends
from the native `+0.15` spawn offset. In the release 30-frame pose trace
`artifacts/controlled-skating-release30-prebody-gravity-2026-09-23.log`,
physical state remains Ground at frame 30, processed wheel count is four,
board Part 6 is upright, and wheel-line height is about `0.03098`. This
replaces the earlier first-frame rapid fall/zero-wheel/WipeoutGround trace.
However, the frame-30 and frame-120 captures still render the skater
reclined, with a displaced-looking board. Their palette/root or animation
presentation remains under investigation. No matched native pose or contact
trace exists yet, so this correction does not establish first-test parity.

Later same-day presentation audit resolved that visual recline separately:
the skater model upload had omitted the native saved-Hips transform from
its local-to-world matrix. The corrected release captures now show an
upright skater at frames 30 and 120. See
`selected-hips-pose-handoff-2026-09-23.md`; this gravity trace remains a
historical pre-presentation-fix measurement.
