# Actual selector COM and second-pass completion

Confidence **high** for source order/ownership, with the same explicit host
ReckoningNumeric estimate, dot reduction, reciprocal refinement and inverse-trig
boundaries. No deterministic reference trace or original-data run claimed by
this author; root owns validation. Continues the completed-query packet on the
same OnboardTrajectoryOwner and copied winner; no parallel selector state.

## Source

`TrajectorySelector::CalculateCOMTrajectory`82dc02f8 captures Global trajectory
layout and graph8 at60/80 evaluated using winning normalY864 **before** copying
the winning result's entire trajectory to winning COM. Corrected raw VMX128
operands at0404/0414/041c/0420/0444 are:

- delta=startTrajectoryPosition930-launchCOM650;
- abs(dot(ProcessedUp220,delta))*graph;
- offset=launchCOM650-old winning COM p0;
- desired=winning normal860*scaled height.

Then capture Global trajectory again, read winning frame40 and maxAdjustment1f4
at0478 **before** signed frame<=0 gate047c. For positive frame, adjustment is
desired-offset and position is **oldp0+offset**, stored049c before call to the
existing recovered tTrajectory.AdjustTrajectory82db6e78. This is not a direct
launchCOM assignment: floating-point cancellation is observable. Full vectors,
unknown W and trajectory time are retained. ShowWallInfo requires overlay once
if selected, then independently sampled ShowTrajectories requires it twice.

`IsSecondPassRequired`82db98a0: AI following2394 returns false before any tuning,
normal/numeric work or targeting-grind publication. Otherwise read trajectory
layout1f0, GetLandingNormal82db63a8 asserts signed best nonnegative and returns
winning normal860, then AngleBetweenVectors against live ground1d0 using shared
recovered unsigned-angle leaf. Publish targeting_grind=(bonus238e && best==0).
Return pass==1 && signed candidate count>=2 && NOT angle<threshold && !grind.
Raw BLT9978 gives unordered angle/threshold the non-rejection branch; this is
expressed through partial_cmp, not angle>=threshold.

`Update`82dc6804..6910 calls COM then second-pass decision, then samples
ShowTrajectorySelect. Its selected debug path GetWinningTrajectory82db6338
asserts best nonnegative, computes collision position before one overlay check,
checks overlay once for every positive signed candidate count, reasserts best,
then DebugDrawTrajectory82dc0b18. The latter tests all four time lanes>=0 before
an extra overlay check and collision-position calculation. No drawing command
remains in these source bodies; reproducing these lookups/assertions is their
actual behavior. Only afterward publish ready=true68fc.

`LaunchSecondPass`82dc3fe8: publish launch startVelocity600 from winning data850
(load4008/store4010). Separately capture Global layouts for coneX244/store4044
then coneZ23c/store407c. Call existing private Launch82dc2800 and only after
successful completion set pass2 at4088. The runtime reuses already integrated
same-owner prepare and continue functions. It releases the trajectories mutex
before predictor submission reacquires it to reset the exact retained futures.
Winning-buffer and ready publications remain during the in-flight second pass.

## Integration and policy

New runtime `try_complete_pair_selector` calls the integrated prefix once,
returns retained readiness or false for near landing, and continues BeforeCom
through actual COM/second launch. GroundPreparationGrindPolicy remains a required
argument forwarded to the prefix; no implicit native-to-parked downgrade.
Native grind scoring remains explicitly pending according to first packet,
and AI private-launch behavior remains the existing typed unsupported branch.
Raw pass0 trajectory drawing remains the prefix's separate explicit boundary.

Registration patch only adds selector_finish under retained owners, finish
runtime under constraint_session, and removes the now-consumed select-cvar
temporary dead-code expectation. Root replaces its temporary bool wrapper/call
sites with this complete API, passing the already available rounding service;
the packet does not edit registered call sites concurrently with root gates.

## Validation

Nine focused tests cover graph failure before COM copy, unconditional maximum
read at frame zero, subtract/add cancellation, unknown W and time retention,
AI lazy return preserving targeting, unordered threshold second pass, ready
and startVelocity surviving a later cone read failure, debug overlay failure
before ready and each selected overlay read. Source reviewer independently
checked raw COM registers and branches, second-pass/debug tail and relaunch.
Standalone rustfmt and patch check are author checks; Cargo, strict lints,
original corpus and reference traces are root validation and still pending.
