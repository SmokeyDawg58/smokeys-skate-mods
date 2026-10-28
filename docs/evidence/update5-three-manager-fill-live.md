# Update5 retained TrajectorySelector, Reckoning and StateChanger output

Source review and draft implementation September13. Confidence high for field
mapping and callback order; unregistered/uncompiled until root gates. No complete
Update5/frame claim. Original-data validation is delegated to root's retained
same-Actor solved-output continuation; component tests are not native traces.

Read fresh IDA raw disassembly and PDB types for
`TrajectorySelector::FillPhysOut82db64d8`, `Reckoning::FillPhysOut82dd4150`
and `PhysicalPlayerStateChanger::FillPhysOut82dd3bc8`. HiLOD Update5 calls them
at82e04f3c,4f48,4f54 after GrindManager4f30 and before HandPlant4f60.

TrajectorySelector reads238f targetingGrind and2396 targetingWallRide into
Air1ab/1ac. Each samples output latch4c BEFORE source byte, checks captured
latch, resolves actual Air, then stores. Existing OnboardTrajectoryOwner owns
both constructor fields; bindings must match SAME physical/Processed owners.
No query completion, targeting inference, or fabricated false is substituted.
The adapter publishes real retained flags; it does not implement later launches.

Reckoning maps Up1b0->System50, Heading1e0->System60, dynamicLean358->System80,
spinSpeed354->Airbc, flipping36c->Air1a9, frontFlipping36d->Air1ad,
animTilt210->Animation0, groundTransform20->Ground0. Lean sign reads actual
Processed8c4 mask00100000 (PPC bit11, tail lifting): set preserves lean; clear
negates it. An unknown selected bit refuses after earlier publications. All
vector/matrix lanes are preserved. Up, Heading, flipping, tilt and matrix
sample latch before source reads, then assert. Lean, spin and front flipping
read source before latch. Matrix publishes four loaded columns only after the
captured latch assertion. Existing completed Reckoning phase is required per
field; constructor access is not substituted for current live update.

StateChanger samples/asserts latch before reading its revert flag34 and resolving
State; copies flag to3c. It next reads requestTeleport35. True samples/asserts
latch, resolves State, writes1 to3b; false leaves the prior destination intact.
Actual StateChanger weak owner identity and constructed fields are required.
Existing system-logic session owns request mutation; this Fill does not clear it.

The draft correction moves source calls relative to captured guards, and moves
State component resolution after its revert source read. Tests cover locked
source ordering, committed prefixes on late source failures, full lanes/signed
zero, and false teleport retaining existing true.

`physical_player_update5_manager_session.rs` consumes the real
Update5BeforeGrindManagerFill. RequireNative returns GrindManagerPending before
any callback. ExcludeForFirstSkatingTest explicitly records the deliberate
parked GrindManager omission, synthesizes no grind output, executes these three
callbacks in order, and returns Update5BeforeHandPlant with the exclusion.
The remaining HandPlant/FootPlant/controller/Wipeout/late Update5 and conditioner
stages remain dependencies. Retained original flags do not close omitted updates.

Root registration: construct::manager_fills -> physical_player_phys_out_manager_fills.rs;
constraint_session::update5_managers -> physical_player_update5_manager_session.rs;
reckoning child fill_read -> existing reckoning_fill_read.rs. The latter only
adds reads of existing heading/flipping fields; do not replace its existing file.
Pass the actual phase returned by try_update_pair_ground_reckoning to
try_continue_update5_managers(board,skeleton,before,&phase,policy). Downstream
selected conditioning should consume Update5BeforeHandPlant and retain explicit
GrindManager exclusion, rather than remaking a before-Grind token.
