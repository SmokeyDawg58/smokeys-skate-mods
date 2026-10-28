# Actual inactive controller update

2026-09-13. Confidence high from fresh raw Update82dd1d68..1df8 and
Update3_State82dfe708..e84c. New children are unregistered; independent review,
Cargo and original corpus validation remain pending. rustfmt passed.

`?Update@SkateboardController@Physics@Sk8@@QAAXXZ` samples mSystemOn+1c0 at1d7c;
false branches1d84 directly to return1de8. No other native field read, store,
assertion or callback occurs on this path. Existing actual constructor stores
false, and actual Ground Enter calls existing StopController. The new child
does not stop the controller or manufacture a false value. It requires the
actual constructed phase and reads only the retained system flag.

True reaches explicit PendingUpdateState before reading controller state.
Native UpdateState82dd18e0 runs at1d88, then state+1bc is sampled1d8c. Only then
states1/3/4 dispatch to UpdateDriveFrames82dcc408, UpdateSkateboardHiding82dc7b68,
UpdateSkateboardRetrieval82dc7cf0. Those enabled branches remain pending.

New physical_player_update3_controller_session.rs consumes the actual
GroundUpdateComplete from ground_finish_session, obtains its existing owner,
checks the same world/pair Arc and calls that owner's controller. Its
Update3BeforePrePhysics token preserves that owner. It does not claim wheel
Start, ApplyForces, StoreForceDisplacement or timer completion.

Three field-isolation unit tests cover inactive with unrelated fields absent,
active pending before dispatch-state reads, and refusal of allocation-only or
missing system flag. Actual complete constructor validation remains in existing
controller tests/corpus; those tests do not fabricate a production owner.

Root registration:

```rust
// skateboard_controller.rs
#[path = "skateboard_controller_inactive_update.rs"]
pub(crate) mod inactive_update;

// simulation_assembly_constraint_session.rs
#[path = "physical_player_update3_controller_session.rs"]
pub(crate) mod update3_controller;
```

Mount immediately after actual try_finish_pair_ground_update returns:
world.try_update3_controller_after_ground(board, skeleton, completed).
Keep/read returned owner; then execute existing actual PrePhysics components:
late retained Reckoning.up+1b0, try_start_board_wheel_lines, try_apply_board_forces,
try_store_board_force_displacement. Shared native empty stub at82dfe7f0 precedes
PrePhysics. Foreign-pair rejection precedes controller reads.

## HiLOD state timer follow-on audit only

The timer owner is NOT the generic HiLOD prefix. It already lives in
PhysicalPlayerPhysicsStateOwner.state_timer:Option<f32>, representing HiLOD+14c.
Actual ctor initialize_constructor_scalars sets +0; real state transitions reset
both this timer and processed timer974. ProcessInputControls copies this owner
to processed.physics_state_fields.state_timer at its normal earlier phase.

Fresh raw Update3 suffix is:

- e824 capture actual ProcessedPhysIn pointer+2b0;
- e828 load old HiLOD timer+14c;
- e82c load DT93c;
- e830 fadds DT + old timer;
- e834 store only HiLOD timer+14c.

Future suffix must run after successful actual StoreForceDisplacement and reuse
the SAME owner's physics_state and reckoning.processed Arc. Capture processed
Arc first, read old timer before DT, add DT+old in f32 and retain source NaN/zero
behavior. Do not update processed timer974 here: it remains the earlier snapshot
until the next native input publication (or an actual state transition).
Do not advance timer before a force/displacement error, substitute Ground's
separate time_in_state, or prefetch DT from the earlier Ground callback.
