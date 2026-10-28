# Actual Air update and state dispatch integration

Confidence: high for source-ordered composition, not a claim of continuous-frame parity.
Independent source review approved the composition and individual COM, board/jump,
Animated, KnownAir and COM-launch packets. Runtime evidence below remains bounded.

## Update and ownership

`Sk8::Physics::PhysState_PhysicsAir::Update` 82d9f4b0 executes the already
reviewed collision/selector/Reckoning prefix. The suffix consumes its same-owner
continuation and verifies its completed Reckoning phase before any further mutation.
It fetches the own-Air COM gate, then freshly reads use-COM velocity. The false
branch calls existing Animated Skeleton(false); true advances the retained COM
trajectory and calls KnownAir Skeleton. Both reuse GeneralUpdate and actual board,
root, animation, Processed, IK and hook owners. A complete GeneralUpdate return
is required before board control and an Air completion token.

At82d9f7d0 UpdateSkateboard runs, then82d9f7e0 marks the real Skeleton error flag.
GetTransform BL at82d9f7e8 targets82b237f8, which reads Part6 Body.GetPosition;
it is NOT GetEffectiveTransform and does not read the board-flip flag. Raw branch
40990008 at82d9f7f8 is BO4 BI25 (GT-clear): maxY uses old only when old > new,
and uses new for equality/unordered. TimeInState is captured before maxY is stored;
Processed DT93c is read afterward, then dt + old_time is stored.

`PhysicalPlayerHiLOD::Update3_State`82dfe708 calls the selected state virtual
Update at82dfe7d0, the retained SkateboardController at82dfe7d8, then ordinary
PrePhysics/StoreForceDisplacement/state timer. The Ground and Air wrappers now
share that same controller helper; Air does not manufacture a Ground token.

## Current-state post and output publication

Update4 loads CURRENT HiLOD state+2a8 at82e01d50 after Skeleton.Post at1d4c;
vtable+20 call at1d60 precedes the wipeout request check and Skeleton.PostWipeout.
The actual selected state is read at this boundary, dispatching Ground or Air.
Unsupported states remain explicit. Earlier test snapshots calling Ground.Post
on Air must not be treated as Air post-physics evidence.

Air constructor82d8aa5c installs vtable8234a2a8. GetType slot+14 points82db40e0:
raw386000c8/4e800020 returns200. Current-type publication validates the actual
selected Air bindings, and later category publication makes a fresh type read.
Update5 reads the current state at82e05b60, then vtable+1c call5b74. Air.Fill
runs there before the shared trick payload, animation, push intent, surface,
finish and conditioner steps. No state is inferred from previous output.

PhysicsAir.Post passes literal false to CheckForAirWipeout at82d8b2c8.
This remains true when its Update used Skeleton.UpdateKnownAir. The distinct
PhysState_KnownAir.Post82d99c10 passes true; it is not the same state/path.

## Verification and limits

- original-air-prefix-2026-09-22-1:16/17 original tests; frame1 reaches explicit
  AirContinuationPending after actual prefix/Reckoning.
- original-air-finish-2026-09-22-1:16/17 (4.75s); frame1 completes Air pre-solve
  and solver but output stops at UnsupportedState(Air). This snapshot still used
  Ground.Post; no complete Air frame or correct Air post claim follows from it.
- check-air-dispatch-2026-09-22-1: cargo check core tests passed, including
  KnownAir/COM launch and corrected Post/Fill/type dispatch. Lint cleanup follows.
- New strict gates and original-data run remain pending. Predictor update timing,
  completed-query scoring, full animation schedule and rendered skating are not
  discharged by these local state changes. Host numerical approximation limits
  remain unchanged; deterministic reference per-tick traces remain required.

Latest gate evidence: strict `clippy-air-dispatch-2026-09-22-3.log` passes
(42.48s). `fmt-air-dispatch-2026-09-22-1.log` finds formatting only in five
files from the integrated batch (pool registration, Reckoning cvars, Animated
entry, StateChanger attributes and processed Grounded adapter). Formatting
will be applied after the running workspace/original gate finishes. No accepted
checkpoint is claimed from clippy alone.

Workspace-air-dispatch-1:4,152 core tests passed, one failure in source title-order
oracle; the inserted expectation used registration grouping instead of the
already-recovered (section,title) order. Independent review rechecked actual
initializer titles and confirmed corrected order. The production registry was
unchanged. Formatting-only differences were subsequently corrected; rerun active.

Final current-batch result: workspace-air-dispatch-2 passes4,409 unit+6doc;
fmt-air-dispatch-2 passes. Original-air-dispatch-1 passes16/17 (4.73s). Actual
frames0 and1 now finish; frame2 reaches Motion Sustain expression200 pending.
Thus the Air state/post/output chain executes on original data. This does not
prove full input, the separate COM-selected scenario, trajectory readiness
scheduling/scoring, native trace parity or an interactive build.
