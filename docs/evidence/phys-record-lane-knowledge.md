# PhysRecord lane knowledge integration packet

New unregistered child `skeleton_physics_history_lane_knowledge.rs` owns no additional record. It tracks four W-known bits per existing transform and one W-known bit per existing derived vector. Existing raw arrays remain explicitly **host projections**. Unknown W storage is scratch; typed getters never promote it. Native PhysRecord constructor82b3d6f0 establishes all lanes as known: I/J/K basis and full zero translation, plus full zero derived vectors. The existing FRESH_VOLUME_TRANSFORM constant already has native zero W0; reuse it. The earlier handoff incorrectly assumed it had affine W1.

Required shared edits by root:

1. In `skeleton_physics_history.rs`, add `#[path = "skeleton_physics_history_lane_knowledge.rs"] pub(crate) mod lane_knowledge;`. Add `lanes: lane_knowledge::RecordLaneKnowledge` to `SkeletonPhysicsRecord`. Initialize with `lane_knowledge::RecordLaneKnowledge::constructor_values()` in its sole literal in `SkeletonPhysicsHistory::new`. Rename or explicitly document existing raw-array getters as host projection accessors; they are not source-facing known-lane getters.
2. Route real history reset/update through typed matrices. Native matrix assertions still examine only XYZ of all four columns and precede `record.store_transform_typed(part, &matrix)`. Production caller sites currently use `try_live_part_transform_host`: `dynamic_assembly_skeleton_reset.rs` reset closure; `dynamic_assembly_skeleton_history.rs` initial supplied root and part closures, and ongoing board root/part closures. Replace these with `try_live_part_transform_typed`, retaining their current source call ordering. Existing full-lane test APIs may wrap known input matrices, but an attached host-projection getter must never use such a wrapper.
3. `try_reset`: after matrix store, validate typed translation XYZ and store `Position(part)`; then store native literal `[0;4]` to `Velocity(part)` as known. Do not mark velocity delta or COM known merely because reset ran; source does not write those fields here.
4. `update_part`: obtain old typed Position/Velocity before overwriting those vectors. At original stores, publish matrix; compute `(next_position-old_position)*f32::from_bits(0x426fffff)` with `zip`/`map`, then velocity delta via `zip`; validate and store delta, validate and store velocity, validate and store position. Each validation failure preserves precisely the earlier matrix/vector stores and their metadata.
5. `try_update`: write proxy Position from typed live Part0 column3, at its existing store before parts1..23. Native supplied root remains the separately sampled root; do not replace it with a fresh getter.
6. COM sums: initialize known `[0;4]`; accumulate typed transform translations and typed velocities with each scalar mass via per-lane `mul_add`, propagating W knowledge even when weight is zero. If source projects COM, use typed root translation and typed Up; dot3 ignores W, projected W is known only when the Up/root inputs required by the FMA are known. Validate XYZ and store COM before validating/storing COM velocity. If `SkeletonHistoryInput` cannot supply typed Up, retain an explicit unknown/required-known boundary rather than promote host scratch.

Public helpers supplied by the child:

- `record.transform_typed(part) -> Option<SkeletonRootMatrix>`.
- `record.vector_typed(RecordVectorField) -> SkeletonRootVector` for all existing vectors.
- Parent-visible `store_transform_typed` and `store_vector_typed` for native store points.
- `history.post_wipeout_record_transform(part) -> SkeletonRootMatrix` and `set_post_wipeout_record_transform(part, &matrix)` for the pending root adjustment.
- `history.add_post_wipeout_skateboard_error_typed(offset)` for source `82b3d7b0`, updating only translation XYZ/W metadata for parts1..23. Root0, basis columns and cached Position/Velocity fields remain untouched. Unknown offset W does not prevent actual XYZ stores; it invalidates only each resulting translation W. Clear Skeleton3dd4 after this entire operation.

Replace the old unregistered offset child call in `physical_player_post_wipeout_prefix.rs`: remove `known_lanes().ok_or(...)`, pass the existing typed offset directly to `add_post_wipeout_skateboard_error_typed`. Do not simultaneously register two methods with the same responsibilities and leave the old full-lane path live.

Mandatory consumer amendments:

- `skeleton_fill_live_runtime.rs::physics_column`: use typed transform column. `physics_velocity` and other record-vector callbacks also use `vector_typed`, rather than the helper that marks raw arrays known.
- `skeleton_part_transform_query.rs`: physics-record branch must return `transform_typed`, not `.map(SkeletonRootMatrix::known)`.
- `skeleton_post_physics_records.rs::calculate_errors`: typed proxy/translation minus typed target/prediction; W uses `zip`, not prediction-W alone. Other record COM/velocity readers likewise must consume vector knowledge.
- `skeleton_collision_update.rs`: specific-part world positions currently evaluate four raw lanes. Preserve XYZ arithmetic and track W from the typed matrix composition; its raw world-position field needs a W-known bit or a typed vector. Never mark its scratch fourth lane as real later.
- Internal history COM arithmetic must use knowledge as above. Remaining external transform-array consumers found by audit are corpus/test host projections; keep them explicitly labeled.

Native provenance: Part6 GetPosition reaches Part.GetTransform `826c3480`. Attached body's columns load packed Simulation pointer, inertia pointer, tag and RF-row pointer as their W lanes. Board semantic/mutable inertia routes do not retain native image addresses. `try_live_part_transform_typed` correctly returns unknown W there. The existing source-routed DMO image provider is a separate authority and cannot be attached retroactively to this board by substituting host indices or canonical affine values.

Tests in the child cover known constructor zero-W and explicit typed-store invalidation, per-column knowledge establishment/invalidation, actual XYZ offset progression without root/basis/cache changes, and signed-zero preservation. No Cargo or shared-file edits by the drafting agent; root owns registration and integrated checks.

Raw constants: gI8227e8f0 = [3f800000,00000000,00000000,00000000]; gJ8227e900 = [00000000,3f800000,00000000,00000000]; gK8227e910 = [00000000,00000000,3f800000,00000000]. Translation store82b3d758 uses vspltisw zero. Proxy scratch stores zero at r1-40/-3c/-38 and integer zero at r1-34 (raw3941ffcc then912a0000), copied full4 at7a8.


Root integration accepted: workspace77 4,187 units +6 docs, original65 all17 in8.46s, Clippy132/fmt49/diff50 pass. Both typed children are registered and actual reset/history callers use typed Part getters. Record/COM/error/collision/Fill consumers preserve knowledge. Independent review accepted the consumer changes and revised corpus XYZ/knowledge oracles. Original61..64 exposed old host-projection assumptions in solved Fill, ongoing history, error records and diagnostic COM publication; those expectations now use actual typed producers, with independent XYZ bit checks retained. Existing native constructor constant was already correct (P.W0). UpdatePoseBuffer drafts remain independently reviewed, unregistered next work.
