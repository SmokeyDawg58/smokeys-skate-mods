# Ordinary Update3 prephysics tail

2026-09-13. Confidence high. New unregistered constraint-session child
physical_player_update3_prephysics_tail.rs; independent review and root gates
pending. No Cargo run by author.

Fresh raw Update3_State82dfe708 confirms Controller return precedes shared
empty stube7f0, actual UpdatePrePhysics call e808, actual StoreForceDisplacement
call e820 and timer increment e824..e834. Fresh PrePhysics82b37cb8 confirms
capture actual Reckoning.up+1b0, body lookup/StartWheelLineTest37cfc then
ApplyForces37d04. No queue clear occurs in these calls.

The new adapter consumes actual Update3BeforePrePhysics, validates the same
pair/owner and uses actual late Reckoning up. It then calls existing wheel Start,
ApplyForces and root's current try_store_board_force_displacement method.
Errors retain prior query/force/displacement effects; no retry or rollback is
performed. Only after displacement succeeds does it capture the retained
Processed Arc, read old physics_state.state_timer (HiLOD14c), read fresh DT93c,
add DT+old in f32 and store HiLOD timer. Processed timer974 remains unchanged.
Missing inputs remain explicit failures, including old timer and DT after
prephysics effects have already committed.

Returned OrdinaryUpdate3Complete retains actual owner, produced displacement and
new timer. It reports completion of this ordinary selected Update3 path only;
solver, Update4, Update5 and conditioners remain independent phases.

Root registration in simulation_assembly_constraint_session.rs:

```rust
#[path = "physical_player_update3_prephysics_tail.rs"]
pub(crate) mod update3_prephysics_tail;
```

Integration signature:

```rust
world.try_finish_update3_prephysics(
    board, skeleton, before_prephysics, &ground,
    &mut crate::collision_line_numeric::HostCollisionLineNumeric,
)
```

Use actual before_prephysics from try_update3_controller_after_ground. Replace
the existing selected wheel Start/Apply/Store sequence on this completed branch
so none runs twice. Assert returned owner identity, displacement matches retained
force runtime value, queue remains unchanged, and processed timer974 keeps its
earlier snapshot while owner timer advances. A failure before timer store must
not advance timer. Broader incomplete branches must retain explicit boundaries.
