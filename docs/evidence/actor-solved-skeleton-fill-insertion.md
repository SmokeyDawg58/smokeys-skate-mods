# Same Actor solved Skeleton output component schedule

New unregistered child `physical_player_actor_solved_fill_corpus.rs` reuses the
existing new-only `physical_player_skeleton_full_fill_solved_corpus.rs` as its
child. No registered module was changed and no Cargo command was run.

Root insertion, preserving existing selected startup assertions:

1. In `physical_player_ground_startup_corpus.rs`, retain `check_schedule` as a
   wrapper invoking a new `check_schedule_mode(..., solved_fill: false)` body.
   Add a separate fresh `check_schedule_mode(inputs.clone(), None, true, true)`
   call before the existing invocations. The mode allocates contact capacity256
   instead of0 only for that dedicated world. Other constructor capacities and
   all original Actor/animation/physical construction remain unchanged.
2. In `physical_player_update1_corpus.rs`, retain `check` as a wrapper invoking
   `check_mode(..., solved_fill: false)`. The dedicated caller invokes its mode
   with true. Register the new file as `mod solved_fill` under this corpus.
3. Immediately after the existing successful `try_start_board_wheel_lines`,
   before `try_end_skeleton_line_tests`, insert:

   ```rust
   if solved_fill {
       solved_fill::check(world, board, skeleton, &continuation.owner, &ground);
       grind_residency.finish(&world.grind_services, &cvars);
       return;
   }
   ```

   The mode should return
   from the dedicated outer check after this call, before the later original
   kickflip/startup-specific assertions. Keep those assertions in every existing
   invocation.

The new helper reads the same Processed dt93c, loads gravity from the same Vault
default Physics layout, and invokes the existing authored-terrain solver and
completed contact feedback. Fifty iterations and `ControlledNoFreezing` are
explicit inherited solver-test controls. Neither is claimed as the complete
native scheduler. It retains original pose/location and terrain; actual empty
contacts are allowed without translating the Actor to manufacture penetration.

After actual End collectors and the already-selected ChoosePhysicsSurface
producer, it calls complete Skeleton post-physics and existing full Fill proof.
That proof consumes only production-returned Update5 tokens, preserves captured
COM velocity through prefix/parts/suffix, checks the same output occurrence and
Processed Arc, and checks history/collision share the same completed solver frame.

Fresh raw `IsDeckInHand`82dc8358 reads controller+1bc, subtracts1, then cntlzw/
extrwi: result is state==1. HiLOD82e01d00 skips the SkateboardBody+360
`numPartsInContact` read when false. The older helper only admitted the
non-held branch; the production post-solve schedule now reads the retained
board post-physics count on the held branch.
It does not replace this predicate with an unconditional false argument.

Scope remains a component schedule. ManageHangUps/remaining controller forces,
complete Board.PostPhysics (including its currently missing live cvar adapter),
and outer HiLOD Update4 return/Update5 Pumping are not covered. The complete
Skeleton callee and mandatory IK/audio data publication are covered if this new
original invocation passes. It is not yet compiled or executed; the parent owns
registration and validation.

## Root acceptance checkpoint58

Latest accepted connected Skeleton output checkpoint: **workspace58 passes 4,085 unit tests and six doc tests; original50 passes all 17 core corpus tests (9.07s); Clippy94, format43 and diff44 pass.** A fresh real Actor-backed pair now executes actual ProcessData, original-terrain solve/feedback, complete Skeleton post-physics and full Skeleton.Fill through mandatory IK/audio collision-data publication. COM velocity is captured once at585b0 and carried through the same output tokens. Ground.UpdateSkateboard also reaches its a998 return through retained constructor hang state and complete pinning. Active hang geometry/pulse services remain explicit pending branches. This is a component schedule with controlled solver iteration/freezing settings; full Board.PostPhysics, outer Ground/controller/force scheduling, output conditioners and animated trick consumption remain pending.
Root independent raw review confirmed same-owner bindings, all mandatory IK/audio source calls, COM capture, guard ordering, actual controller non-held predicate and Hang/Pin defaults. Narrow records reciprocal uses final-negated VNMSUB at541a0/a8; generic drive policy remains unchanged. Original50 includes successful freshActor solve/fullFill and all prior fixtures.
