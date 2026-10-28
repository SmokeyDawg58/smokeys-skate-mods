# Post-wipeout hand IK

2026-09-19 bounded draft, confidence **high** for source graph. Unregistered,
uncompiled; no runtime or corpus claim. Root owns registration and gates.

Fresh IDA symbol `?DoPostPhysicsHandIK@SkeletonIK@Physics@Sk8@@QAAXABVMatrix44Affine@vpu@math@rw@@@Z`
at82b56310; sole code caller82b57bf4 in PostWipeoutCheck82b578b8. Raw caller
selects !teleport && airborne && board-contact, after any selected foot helper,
before unconditional IK.UpdatePostPhysics82b57bfc. No offboard restriction in
the hand branch; no do_ik or wipeoutStarting gate inside this helper.

Visit indices2,3 ascending: primary mapping starts IK+468, part_to_deck Z+720,
on-deck read primary-pointer+7d0=IK+c38. Pointer steps4/40; loop counter340,
360, then380 exceeds360. On-deck zero skips all reads and debug for that entry.
Existing original Init maps LeftHand/RightHand and sets optional parents -1.

Compose deck with retained part_to_deck. Raw VMX128 operands at563c8/cc/d4
are v61/v59/v57, not IDA aliases v93/v91/v89. Axes X multiply then Y/Z FMA;
translation X FMA into deck.P then Y/Z FMA. Reuse typed Matrix.compose,
preserving unknown W; inner W is not a coefficient. Record P capture5641c
uses SkeletonState+40*(part+25), before target writes56438/3c/40/44 (K,I,J,P).
The complete target write precedes optional-parent assertion56448..64.
Delta56470 word16f8f877 is v55=v56-v127 (not IDA v88), all four lanes.

DoTwoBoneIk_PostPhysics82b536c0 called56484 receives PRIMARY hand, not optional
parent. Reuse post_wipeout_foot::two_bone unchanged: its actual genealogy,
PhysRecord bone positions, solver, arm parent/grandparent writes and debug
reads remain authoritative. Only true commits hand Part.SetTransform82b25480
at564e0 then checked PhysRecord.SetTransform82b3a800 at564f0. No rollback.
Read kDrawPostPhysIK after true/false at564f4..fc. Enabled branch requires
actual overlay. Independent reviewer confirms both apparent drawing calls
5654c/56564 target shared BLR825187c0. Existing FootIkSource debug adapter may
retain a stricter explicit DrawingPending boundary; draft does not broaden it.
The solver separately samples kDrawPostFeetIK. No new cvar/Vault field needed.

New child physical_player_post_wipeout_hand_ik.rs reuses FootIkSource,
FootIkError, two_bone and commit. Proposed parent skeleton_ik_state module
name post_wipeout_hand. Runtime should reuse foot live Source and retained
field-split owners, with world/pool pair identity checked by the root caller.
No runtime adapter authored in this bounded task.

Tests exercise the SAME production hand_entries caller with controlled solver
outcomes, not a duplicate algorithm: skipped hands require no providers;
record failure before publication versus sentinel failure after publication;
left/right order and primary IDs; nonzero on-deck predicates ignoring unrelated
gates; solver-false debug and unknown delta W; successful solve followed by
record-store failure retains Part prefix and stops right hand. Existing solver
tests/source review remain separate. Actual original-data airborne/contact
producer-to-consumer coverage is still required; neutral ground does not prove it.
