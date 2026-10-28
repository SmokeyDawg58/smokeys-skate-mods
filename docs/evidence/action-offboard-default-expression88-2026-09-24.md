# Off-board Default expression 88 (mounted raw Y)

The raw Y replay in `artifacts/post-reboot-raw-y-action36-breadcrumb-2026-09-23.log` reaches Action state 9 (`TransitioningToOffBoard`) at frame 34 and stops on `PendingExpression(88)`. The generated Action table gives state 30 (`Default`, parent 12 `OBGround`) expression 88; this is a candidate target of state 9's zero-priority transition 101. State 9's transition index range 15..19 resolves through `INDICES` to transition IDs 100..103. No equivalent expression 88 implementation existed in `generated_action_runtime_selection.rs` before this change.

IDA's `HCDelegate_ActionGraph_OnBoard::EvaluateExpression<1>` at `0x828ff990` uses jump-table entry 88 (`0x828ffb2c -> 0x8290403c`). `<2>` at `0x82906048` uses entry 88 (`0x829061e4 -> 0x82908e98`). Both blocks have the same ordered, cached condition logic:

1. Condition 134, pointer offset `0x218`, calls `StateConditionDirect_PhysFilteredState`. A true value returns true immediately.
2. Otherwise condition 135, offset `0x21c`, calls the same condition class. False returns false.
3. Otherwise condition 143, offset `0x23c`, calls `StateConditionDirect_OBTimeToLand`. Its true value returns true; false returns false.

The resulting expression is `c134 || (c135 && c143)` in both masks. Generated original attributes identify c134 as filtered state `offboard` (attribute 452), c135 as `offboardair` (attribute 385), and c143 as `lessEqual=0.03` (attribute 455). Existing filtered-state dispatch already supports both states. `OBTimeToLand::ActivationFunc` at `0x82c9aff0..0x82c9b0e4` resolves `IPhysical`, gets PhysOut, reads the off-board output float at `+0x1c` (`time_to_land`), and calls the retained comparison operation. Unlike the separate `TimeToLand` condition, this callback has no air trajectory validity gate. The new dispatch uses the existing OffBoard PhysOut owner and comparison helper.

The selection test checks lazy condition order for both masks: OffBoard true reads only 134; neither physical state reads 134 and 135; OffBoardAir with landing time reads 134, 135, 143. The mounted replay and Cargo compilation are pending the root agent's build; this file records source correspondence, not a gameplay parity claim.

Next state 9 route, read-only: priority-two transitions 102/103 target states 23 `Wipeout` and 24 `Teleport` (existing Pre expressions 27/28). Priority-zero transitions 100/101 target state 32 `Dismount` (existing expression 36) and state 30 `Default` (expression 88 here). After those candidates, state 9 Sustain expression 71 is already implemented. The actual next boundary must come from a fresh replay because the branch depends on runtime cached conditions and the parent/child selection order.
