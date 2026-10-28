# Ground queue-to-hook prediction

New-only draft, not compiled or executed. Ground.Update82da1024 calls distinct
UpdatePredictedSkateboardPos82d84b88, after UpdateSkateboard returns and before
the conditional RequestPushExit1028..1038. This is not Update3's earlier linear
board prediction. No new cvars or Vault bindings.

## Numerical and source-read order

Source ba4 tests retained Ground.exiting_wallride+f0. True returns without mass,
DT, queue or hook reads. Otherwise bc8 loads Processed mass970, bd4 DT93c, bdc
reciprocal estimate of mass. The first refinement is native final-negated
VNMSUB atbec followed by FMA bf0. GetCombinedForces is called bf8 between the
two refinements. Second residual c18 precedes fresh groundNormal1d0 load c24;
second FMA c28 follows that load. Numeric collaborators provide the original
estimate/dot policy boundary; host estimates are not Xbox bitwise parity.

The remaining graph is separate rounded operations:
inverseDt = inverseMass * DT (c2c);
velocity = combinedForce * inverseDt (c30);
projection = dot3(velocity, groundNormal) (c34);
normalComponent = groundNormal * projection (c38);
tangent = velocity - normalComponent (c3c);
offset = tangent * DT (c40).
It does not normalize groundNormal, use current velocity, halve the displacement,
or skip zero/NaN forces. Unknown W propagates through full-vector stores; dot3
alone ignores W. The same offset is sent to AddOffsetToHooks atc48.

## Combined queue source

GetCombinedForces82b26650 has hidden output r3, board r4. Board+120/+124 are
retained vector begin/end. Raw6674/78/7c store +0 at sp-16/-12/-8, 6658 sets
r8=sp-4 (IDA labels this back_chain), and6680 stores integer0 there. Thus all
four initial accumulator lanes are actually +0. Loop66a4 loads each record+10
world force, advances by30 bytes, adds at66ac in insertion order. No kind filter,
attribute lookup, local-position transform, queue clear or force application.
Existing SkateboardForceQueue already owns exactly those retained records.

## Actual hook producer

AddOffsetToHooks82b4d600 loads Skeleton.drives1934, drives.hooksAssembly e0,
then first Part from assembly+8. GetTransform d62c is the ordinary Part getter.
Translation is old + offset atd648; SetTransform d658 commits before predicted
board position3ce0 is read atd664 and updated atd66c. Only hook Part0 changes;
Part1 and drive dynamics are not replaced. This order preserves Part/body/cache
partial writes before any later error; predicted position cannot change first.

Existing same HookPool/session, try_live_part_transform_typed and typed hook
setter implement the selected route. The older hook-pool host getter promotes
host matrix W to known and is deliberately not used by this adapter. Attached
Part XYZ and unknown W are preserved; detached typed-cache provenance remains
unchanged. Existing actual SkeletonRootTransforms owns predictedBoardPosition.

## Registration plan

- skateboard_force_queue child combined: skateboard_force_queue_combined.rs;
  private-range access adds combined_forces() without exposing records.
- drive_owner (dynamic_assembly_skeleton_drives.rs) child hook_offset:
  dynamic_assembly_skeleton_hook_offset.rs.
- skeleton_members (dynamic_assembly_skater_pool_skeleton.rs) child hook_offset:
  dynamic_assembly_skater_hook_offset.rs.
- constraint_session children ground_force_prediction and
  ground_force_prediction_session, corresponding physical_player_*.rs files.
- cfg(test) child of ground_force_prediction:
  physical_player_ground_force_prediction_tests.rs.

The world method try_update_pair_ground_force_prediction(board,skeleton,
GroundSkateboardComplete,numeric) checks the SAME actual pair/currentGround,
consumes its exiting flag, samples queue through existing actual force runtime,
then mutates the retained skeleton hooks and predicted position. It returns
GroundBeforePushExit, not Ground.Update completion. Owner/Reckoning locks are
released before mutable body operations; skater-pool refresh invalidation reuses
existing mutation behavior. No fresh animation/body owner or diagnostic frame.

Three focused tests are drafted but unrun: insertion-order sum including all
kinds/unknown W without consumption; normal projection/read sequence; wallride
gate and late missing-normal failure. The actual Part0 update followed by
predicted-position update still requires integrated same-owner corpus coverage.

Independent review corrected the world wrapper's exiting-wallride route: it
returns its token immediately after validated Ground flag read, before any
Reckoning/Processed acquisition. The kernel gate alone did not protect against
an eager owner-lock error on the native no-op route. Child placements above also
preserve the private drive-runtime and skeleton_mutation_index access boundaries.
