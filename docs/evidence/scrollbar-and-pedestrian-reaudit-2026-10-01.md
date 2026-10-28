# Thumbnail scrollbar and pedestrian follow-up

## Corrected thumbnail row count

The Bottoms screenshot showed the last selectable row with the scroll thumb
still near the top. The editor passed individual item count to a scrollbar
whose selection and visible window were expressed in rows.

Original createskater APT InitThumbBrowser00f242..00f255 computes
`floor((mNumItems - 1) / 3) + 1` into register3. At00f2a1..00f2be,
SetNumberOfItems receives that row count and window3; SetValue receives
`floor(mCurrentIndex / 3)`. The earlier integration note followed the consumer
without tracing register3's producer. That note is corrected too.

`edit_skater_movie::thumbnails` now passes `item_count.div_ceil(3)` to the
existing basic_slider owner. No second scrollbar or replacement geometry.
Lists retain their original item-based eleven-entry window. Empty thumbnail
lists are hidden by the existing owner.

The existing original-asset UI regression now exercises the actual thumbnail
entry point with8,9,10,20,21,22,40 items: complete and partial final rows,
hidden scrollbar for one window, final thumb at the shared row-based endpoint.
The test passed using the mounted original createskater assets. Evidence:
`artifacts/scrollbar-pedestrian-ui-test-20261001.log`.

## Confirmed pedestrian defect: contained-prop transforms

`LWComponentMan<Pedestrian>::UpdateTransforms82e82be0` invokes Pre, Update,
Post consecutively for each entity in native map order. The selected runtime
called only Update. Ordinary pedestrians therefore failed to propagate their
new world transform to retained interaction Props after moving.

Fresh live IDA vtable reads:

| LWBasicEntity table | Pre +88 | Update +8c | Post +90 |
| --- | --- | --- | --- |
| Pedestrian823a4e70 | empty825187c0 | 82e5b358 | 82e4c2b8 |
| SecurityGuard823a5410 | empty825187c0 | 82e5b358 | empty825187c0 |

PostUpdateTransform82e4c2b8 passes the current core transform to virtual
UpdateContainedPropTransforms. Both +34 entries bind the base82e503b0;
the guard's empty Post override means that call must NOT be added to guards.
82e503b0 walks retained non-null children in vector order and calls each
Prop.SetTransform. Raw82e503f4..82e50404 confirms the incoming matrix is
forwarded unchanged (Hex-Rays omits the call argument).
Prop.SetTransform82e68e80 forwards to the existing core transform setter.

Restored the ordinary-pedestrian Post immediately after its core update.
Extracted the already implemented base pass from the DMO override into
PropMan.update_contained_prop_transforms. DMO callers reuse it and retain their
own authored waypoint/centroid transformation pass82d2e510. Ordinary peds
must not execute that DMO-only pass. There is no second entity or prop owner,
no extra per-frame child-vector clone, and missing retained IDs remain lifetime
errors rather than silently disappearing.

Fresh native capture:
`artifacts/pedestrian-reaudit-post-transform-20261001.txt` includes the vtables,
component loop, base/post/setter functions and argument-forwarding assembly.

## Existing pedestrian flows rechecked

These were compared to fresh IDA output and existing call sites before deciding
whether edits were needed. They were not reimplemented merely for this audit.

| Area | Native reference | Review result |
| --- | --- | --- |
| Population and distance culling | EvaluatePedestrianCensus82723d90, EvaluativeCull8271ef70, EvaluativeSpawn8271ea90 | Retains authored density/range calculations, initial versus regular attempt budgets, RNG order and non-cullable callbacks. |
| Spawn/contained props | SpawnPedestrian82720fe0, Pedestrian.Spawn82e9a510 | Existing factory, roster, navigation admission, contained-prop creation and guard post-init reused; ACTIVE_BODY4 correction already present. |
| Frame scheduling | UpdatePreFrame82e96fc0, UpdateCurrentFrame82e96ef8 | Reviewed integration phase order, navigation tasks, query fences, modifiers and intents. |
| Navigation | Update82e6bb70, Mesh.Seek82e75b68, Update82e7dc70, ResolvePath82e76120 | Existing speed-ramp order, path completion and retained tile/cell lifetime checked. No guessed speed changes. |
| Animation scheduling | BuildAnimTree82e6e878, AnimUpdate82e95da0, AddTime82e904f0 | Two alternating graph pools, elapsed-time carry, forced initial tree and saved base-expiration semantics agree with reviewed source branches. |
| Pose sampling | SetUpSimLODAnimation82e6ea28 | Existing FETCHBLEND1 retained; no second interpolation scheme. |
| Animation-driven transforms | UpdateTargetTransform82e7f660, SpatialAdjust82e745a0 | Existing root-motion/ground/spatial adjustment order reviewed. No unconditional ground snap or duplicate correction added. |
| Avoidance | EvaluateNavModifiers82e80080, UpdateIntent82e94f70 | Existing per-evaluation reset and modifier priority checked. |
| Collision targets | FillWorld82e90e30, FillSkater82e91530, OnEntityCollision82e95158 | Existing collision-body ownership and branch conditions checked; kept the native nonzero lock-distance branch rather than reversing it by intuition. |
| Held props | UpdateHandProp82e9d320 | Separate held/released, throw, tazer and idle branches reviewed; retained DMO/monitor ownership reused. These are separate from contained interaction Props. |
| Cull/destruction | RequestCull8271a5e8, OnCullRequested82e85fd0, destructor82e94380 | Existing detach/callback/contained-prop teardown, waypoint release, animation and physics cleanup reviewed. Outstanding query retirement remains tied to rig destruction. |

Supporting fresh captures are the `artifacts/pedestrian-reaudit-*-20261001.txt`
files (native, navigation, modifiers, collision, addtime, props, post-transform).
Do not use decompiled VMX temporaries as proof of vector arithmetic; preserve
the existing raw-instruction evidence where the decompiler loses operands.

## Validation and limits

- Original-asset editor regression PASS, including final-row scrollbar cases.
- `cargo check --release -p skate2-pc -j1` PASS after the pedestrian edits;
  log `artifacts/scrollbar-pedestrian-check-20261001.log`.
- This pass does not rebuild the release executable or claim a new pedestrian
  runtime comparison. A compile is needed before playtesting these changes.
- This is a broad review of the existing integrated lifecycle, not an exhaustive
  equivalence proof of every generated condition, guard mission, plugin or rare
  interaction. Existing explicit unsupported paths remain visible. In particular,
  speech/jaw playback has no audio backend and was not fabricated here.
- The earlier documented zero-distance fallback for six assets with missing,
  natively uninitialized crosswalk distance is unchanged. It is not evidence of
  a deterministic original value.
- Previous shadow speck/edge same-location comparison remains a separate open
  issue; this work does not establish that it is fixed.
