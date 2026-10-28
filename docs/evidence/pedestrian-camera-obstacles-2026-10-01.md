# Camera avoidance for populated pedestrians

The hand-prop flag release build passed. Its walking capture reached11 roster
members and11 presentation entities (22 meshes at1200,21 at1500). Entity33554483
moved from837.6708,-61.428825,961.37964 at900 to832.27673,-61.432022,965.74036
at1200 and827.2444,-61.43137,967.9556 at1500. This is movement evidence, not
visual/collision verification. At1701 the camera stopped on the old explicit
PendingPedestrianCandidates(1) boundary.

IDA PathEvaluatorObstacleServer.AddPedestrianObstacles82e39eb0 decompilation
omits the populated loop. Raw82e39f30..a184 establishes:

- Query20 pedestrians within the authored actor search radius; preserve order.
- GetPedestrian can return null; skip it. Complete object is interface minusac.
- Core proxy vslot0c is IsInitialized82d1d248; skip missing core.
- Entity slots10/08 are GetVelocity82e94658/GetPosition82e94640, reading
  core+144 and core+64 respectively.
- Reject when dot3(pedVelocity-cameraVelocity,cameraVelocity)>0.
- Reject when dot3(cameraVelocity,cameraPosition-pedPosition)>0.
- LWBasicEntity slotA4 is GetWidth82e4ff40. Model slot10 resolves
  GetDimensions82d16218. Radius is full dimensions.X; absent model gives zero.
- Emit spheres at position, position+J*0.7, position+J*1.4; constants are
  3f333333/3fb33333. J at8227e900 is0,1,0,0. All share pedestrian velocity.
  Raw vmaddfp word11606b2e at82e3a100/a15c has VA0,VB13,VC12:
  J*height+position, despite the disassembler's printed operand order.
- AddObstacle82e39d90 admits up to50 total and stops immediately on capacity,
  retaining a partially admitted pedestrian's prefix.

Existing camera obstacle owner/list/dynamic evaluator reused. No new collision
or pedestrian owner; no avoidance-disable bypass. Numeric reduction retains the
existing host_dot3 policy (not a claim of bit-identical Xenon reduction).
Focused direction/height/full-width/capacity regression added. First test build
failed at three stale test APIs: skater channel initializer omitted the new
pedestrian=false argument, transition-only topology test accessed private search,
and startup corpus still referenced the removed ChallengeMessage.player_id field.
Updated the call/typed payload assertions and made search crate-visible (no
algorithm change). Test build restarted in artifacts/pedestrian-camera-obstacle-test.log.
Core release test compilation passed (14m46s). All five camera dynamic-obstacle
tests pass in pedestrian-camera-suite.log. The constructed-field inheritance,
stride and cache-rejection regression passes in pedestrian-attribute-projection-test.log;
shared registry namespace/exact-removal regression passes in
pedestrian-navigation-publication-test.log. Original-asset map test build is
running in artifacts/map-selection-aspect-test.log. Runtime executable rebuild
still required.

Additional old-executable capture pedestrian-visible-1500 completed1500 ticks
with13 submitted entities/24 meshes. Screenshot pedestrian-visible-1500.png
does not clearly establish visible NPCs. Added SKATE2_TRACE_PEDESTRIANS-only
renderer diagnostics (opacity, clip position, first palette rows) for the next
executable build. Renamed the existing misleading `visible` log count to
`submitted`. Geometry submission and movement are established, visibility and
physical interaction are still open requirements.


## Completed runtime follow-up

Release executable build PASS: artifacts/pedestrian-camera-visibility-build.log.
Runtime PASS3600 completed ticks: artifacts/pedestrian-camera-visibility-run.log.
Screenshot artifacts/pedestrian-camera-visibility-3600.png visibly shows a
walking pedestrian with a shadow in the plaza. Same-ID position samples prove
movement; roster changes continue during travel. The prior camera boundary no
longer stops this route. Physical contact remains a separate unverified check.
The original-asset map arrow/background regression also passed in
artifacts/map-selection-aspect-test.log.
