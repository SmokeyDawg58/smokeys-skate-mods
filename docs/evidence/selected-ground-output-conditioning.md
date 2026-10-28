# Selected actual Ground output to retained Physics conditioner

2026-09-13. Confidence high for reviewed source stores. Unregistered adapter;
independent integration review and Cargo/original-data validation pending.

Fresh IDA raw PhysState_PhysicsGround.Fill82d8ddf0..e0e4 confirms the existing
Ground.Fill kernel and existing actual owner adapter. All fields already exist.
No new value defaults, tuning owner or field projection is necessary.
Ground antiFlipTorque is typed SkeletonRootVector produced at82d9a770 by actual
ordinary force continuation (physical_player_ground_ordinary_forces.rs), and
wallride-exit is actual retained Ground+f0, not reset PhysOut state.

Corrected the draft's world-grab ordering: raw82d8e050 samples output lock,
82d8e058 reads processed BYTE8cc, 82d8e05c extracts low bit, and82d8e060 branches
on the sampled lock. The kernel now uses the existing sampled-guard token.
That byte low bit is word flags8cc mask01000000, correctly retained in the adapter.
The second attribute pointer loads before the first; fadds rounds the sum before
ordered LT with actual absBodySpeed948. Null attribute pointers alone use the
existing DefaultDataArea4 capability. New test covers equality and NaN speed.

Fresh Update5 decompile and existing state-kernel review agree on distinct actual
GetType reads at5400 and55f4. The supported actual owner is PhysicsGround100;
category helper asserts signed0..702 then descending hundred ranges. Guarded
stores include type5434, category56b8, HiLOD update count56f4, offboard5724,
offboard-ground5764 and actual processed DT5798. These registered helpers do
not publish filtered state. No registered helper was edited.

New physical_player_selected_ground_conditioning.rs must be an ASSEMBLY CHILD
to borrow the existing private owner.conditioners. It consumes actual
Update5BeforeGrindManagerFill, invokes the existing selected type/category/dt
producers, actual same-owner Ground.Fill, and then calls the existing retained
Conditioners.try_update_physics_prefix once. The returned
SelectedGroundBeforeDampedCom retains owner and exact conditioner output token.
No direct filtered setter, reconstructed conditioner or history reset occurs.

This is deliberately a selected producer/consumer corpus closure. It does not
claim intervening Update5 managers, remaining output stores, or earlier UpdateAll
callbacks ran. GrindManager remains explicitly parked. Other manager fills are
pending, not fabricated constructed-empty results. Do not use this result as a
complete-frame token. Subsequent actual Action consumers may observe the result
in a selected check, but no hands-on skating readiness claim follows.

## Exact root registration and insertion

1. physical_player_phys_out_construct.rs: path
   physical_player_phys_out_ground_state_fill.rs as pub(crate) mod ground_state_fill.
2. simulation_assembly_constraint_session.rs: path physical_player_ground_state_fill.rs
   as pub(crate) mod ground_state_fill.
3. physical_player_assembly.rs: path physical_player_selected_ground_conditioning.rs
   as pub(crate) mod selected_ground_conditioning.
4. Reuse registered constraint-session update5_state_publication and the Pumping
   update5_pumping registration from the preceding packet.
5. After actual same-Actor Skeleton.Fill then Pumping, use the existing world
   try_with_board_force_runtime(board, |runtime| ... ) capability. Inside call
   selected_ground_conditioning::try_condition_selected_ground(before_grind,
   board, skeleton, Some(&runtime.push_attributes), references, numeric).
   Unwrap/propagate both the world access and selected producer results.
   This reuses the actual board force runtime's live Vault capability.
6. Verify result.physics.matches_output against result.owner.phys_out and inspect
   actual filtered state through the existing getter. For actual Ground100 it
   should follow the actual published wallRideExit (Ground or Air), never force
   the expected value. Feed the retained codec-loaded Action session afterward.

Potential concrete failure boundaries are preserved: missing Ground instance or
foreign pair, missing processed948/8cc/DT93c, missing antiFlipTorque producer,
actual attribute failure and absent constructor conditioners. If Physics prefix
fails, do not retry: its velocity/empirical histories may already have advanced.
Ground100 conditioner inputs are complete Board.Fill contact/surface/deck position,
complete Skeleton.Fill COM velocity, actual type/category/dt and Ground wallride
exit. Other manager-produced targeting-grind is read only for state201, which
this adapter does not admit. Full UpdateAll dependency review remains separate.

Validation by author: rustfmt on owned drafts only. No Cargo/original-data run.
