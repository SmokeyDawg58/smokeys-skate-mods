# Live maximum-iteration sampling placement

Read-only audit; root owns implementation and registry indices299/300.

Fresh IDA raw `Island::Step_UpdatePrePhysics1` at `827b36e8` compares the
retained Island flags at+8 with exact3 (`827b36fc..3720`). Equality selects
single-player; every other flags value selects online. It reads the selected
cvar target+28 then value+d8 at `827b3738..3740`, and stores the raw32 bits to
the same Simulation's `m_MaxIteration` at+b0 at `827b3744`. This is a live read
each reached call, not constructor25 or a fixture50 substitution. The store
precedes DMO/Vehicle/Pedestrian BeginUpdate (`375c`, `3770`, `3784`) on flags3.
The other branch starts optional ghost work afterward.

Call chain and chronology from fresh source:

1. `Simulation::SimUpdate1` (`8289f410`) calls GameInputManager.Update at
   `8289f4b8` when skater updating is enabled. Challenge/AI/recipe and enabled
   LivingWorld.Update1 work follows before BeginDMOUpdate.
2. `World::BeginDMOUpdate` (`827adec0`), called at `8289f5e0`, calls authoritative
   NetworkSimulation.StartFrame at `827adf24` for mask bit1, optional admitted
   visual StartFrame at `827adf38`, then simulation-island prephysics at
   `827adf48` and admitted visual-island prephysics at `827adf58`.
3. Only afterward, SkaterManager.UpdateAnimation1 is called at `8289f618`.
   It filters the actual Actor list by flags and invokes StartAnimUpdatePhase1
   at `825ffd7c`. That function's serial branch calls GenerateActionGraphIntents
   at `825ffc50` then Actor.UpdatePhase1 at `825ffc5c`; job submission is also
   after the iteration store. Thus Actor action/input-intent work is later.
4. Animation2 at `8289f674`, EndDMOUpdate at `8289f6ac`, Animation3 at
   `8289f6e4`, SkaterManager.UpdatePhysics at `8289f71c`, World.Update1 at
   `8289f758` all occur later. Sampling only at pre-solve/solver is too late.

Current narrow implementation seam:
`SelectedPlayerTick::try_step_input` already performs queue delivery, then
GameInputFrameSession immediate step or `try_recognize_delivered` (which calls
the retained GameplayInputUpdateManager::try_update), then
`try_selected_actor_frame_prefix`, then `try_prepare_ground_pre_solve`, then
`try_step_authored_ground`. Insert the retained manager iteration producer
after successful input recognition and before the `let hands` Actor block.
This preserves relevant input-before-read and read-before-Actor ordering within
the explicitly admitted selected slice. No-command mask0 returns before it;
unsupported selected mask2 still rejects. The cvar selector uses actual Island
flags, not the command update mask (mask1 can update an offline flags3 island).

Retain the raw sampled value on the actual manager/simulation owner before the
Actor call, so an Actor error does not erase the native store. Pass that retained
snapshot onward through the existing max_iterations parameter consumed by
`try_step_authored_ground` / `simulation_horus_pipeline`; do not re-read the cvar
after Actor callbacks or clamp it independently. Existing diagnostic request
overrides may remain explicitly separate. A caller-side read before invoking
the whole selected tick would be premature relative to input callbacks.

This seam does not establish the omitted challenge/AI/LivingWorld/DMO schedule
or relocate the existing backend StartFrame implementation; those remain the
selected slice's documented schedule boundaries. A full scheduler must preserve
StartFrame-before-store and all native intervening stages separately.

Cvar metadata freshly verified:

| Field | Single player | Online |
|---|---|---|
| initializer | 82f6a620 | 82f6a678 |
| constructor call | 82f6a654 | 82f6a6ac |
| initializer table xref | 82fd224c | 82fd2250 |
| section | Physics_MaxIterations | Physics_MaxIterations |
| label | kMaxSinglePlayerIterations | kMaxOnlineIterations |
| title | Max Single Player Iterations | Max Online Iterations |
| default/min/max | 50/25/200 | 25/25/200 |
| callback | null | null |

No production code, registered files or Cargo changed during this audit.
