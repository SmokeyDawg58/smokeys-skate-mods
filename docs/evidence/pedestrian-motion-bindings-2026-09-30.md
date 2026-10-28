# Pedestrian motion binding audit, 2026-09-30

Confidence **high** from original executable/PDB and static source comparison.
No compilation, tests, runtime or deterministic trace was run. Rustfmt syntax
parsing of the three changed files succeeded with stdout output only.

## Reachable behaviour binding correction

`TemplateHC<HCDelegate_MotionGraph_Pedestrian>` constructor82930ce8 constructs
472 behaviour occurrences. Its factory call82930e84 passes **null parent**.
`StateGraph::Base::Base`82cf6f88 uses parent-enabled only for a nonnull parent,
then reads the local enabled attribute, defaulttrue. Generated state `active`
is an implicit-child-selection flag (`Evaluate`82932560), not the behaviour's
parent-enabled value. Explicit transitions enter states with activefalse.

The previous `Behaviours::bind` incorrectly combined every ancestor's active
flag into each behaviour's enabled field. Static declaration inspection found
63 affected occurrences. JumpAirUp state39 was one concrete reachable failure:
it has activefalse yet contains TriggerPedestrianJump and
PlayPedestrianTimeScaledJumpAnimation. Skipping these caused a missing slot in
generated StateUpdate828f5990 and failed to produce the real jump trajectory.
The binding now uses only each occurrence's local enabled attribute. The table
contains no authored local enabled attributes, so all472 bind enabled, matching
the native generated constructor. No graph selection rules were changed.

## Branch predicate order and comparisons

GetCurrentBranchWindow82e6e668 rejects negative start and time-before-start with
BLT at82e6e6f4/fc; it accepts the upper bound with BLE at82e6e708. This admits
unordered values through the first two checks, but not the last. The same
predicate now governs both branch-foot matching and InTurnBranchWindow.

InTurnBranchWindow82e6e750 obtains animation properties and accepts expiration
before calling GetCurrentBranchWindow. Its adapter no longer fetches a body
animation clock eagerly on that shortcut. InBranchWindow827127b0 rejects a
transition before reading the current animation clock; its adapter now does
the same. BranchPoint uses BLT/BGT rejection with a lazy cvar read after the
lower bound; cycle branch uses BLE acceptance at827128e0/ec, which rejects
unordered comparisons. The prior shared probe and not-greater helper erased
those differences. Separate property, clock and transitioning accessors retain
native order and avoid unnecessary missing-current-animation errors.

## Jump and job producer-to-consumer audit

The existing navigation intent producer initializes SpatialAdjust trajectory
and downward=false before adding ChaseJump. Motion state38 JumpInto transitions
to39 JumpAirUp; TriggerPedestrianJump calls entity JumpNow82e80f90, whose actual
SpatialAdjust JumpNow82e7d5f0 fills up/down times and sets jumpingtrue. The timed
animation Begin82718520 writes mirror override before reading the jump duration
and computes originalLength/duration. SpatialAdjust's update advances the phase
and clears jumping at landing. Conditions41/42 read those same retained fields;
the constructor's unknown downward byte is not replaced by a fabricatedfalse.

All nine declared condition families, fourteen behaviour families and eighteen
distinct generated update addresses have concrete bindings. The transition
hook guard is valid because all441 authored sections have zero hooks. Static
inspection is not proof that every asset or frame-state requirement is met.

SIM EvalCurrAnimForSimLOD uses existing FetchVbr, SqtBlend, SetData, ReuseBuffer
and PoseMatMult execution. `submit_sqt_programs` recognizes fixed-matrix VBR
pedestrian batches and delegates to their dependency graph, before the generic
single-chain mixed-set/fetch rejection. Sync publishes the actual extracted
matrix owner; PRES replays the retained stream into the existing full hierarchy.
No second scheduler or animation evaluator was added. The destruction
PendingBlendSpaceJobWait guard still represents an unsupported outstanding
blend-space-job teardown, rather than silently skipping its native wait.
Full hand/face presentation and geometry LOD are separate remaining gaps.
