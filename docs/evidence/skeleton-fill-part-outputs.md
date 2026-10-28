# Skeleton Fill part outputs, 0x82b58618..0x82b589b4

This bounded continuation follows the retained COM publication at 0x82b58614 in `Sk8::Physics::Skeleton::FillPhysOut` (0x82b58230). It ends before the actual assembly Part angular-velocity lookup at 0x82b589b8. It does not complete Fill or publish a complete Update5 result.

The new `physical_player_phys_out_skeleton_part_fill.rs` is a proposed child of the existing PhysOut `construct` module. Root owns registration and the actual paired Skeleton adapter. `SkeletonPartFillSource` borrows the actual data/query/history owners; it must not supply a cached transform snapshot. Source admission errors preserve previous stores.

## Source reads and stores

| Source operation | Destination in PhysOut Skeleton | Store |
|---|---|---|
| Data volume-to-bone translation19, then fresh GetPartTransform19 and transform point | right foot bone position +70 | 58684 |
| Data volume-to-bone translation15, then fresh GetPartTransform15 and transform point | left foot bone position +80 | 586f0 |
| Fresh GetPartTransform19, column I | right foot look +100 | 58734 |
| Fresh GetPartTransform15, column I | left foot look +110 | 58778 |
| Fresh GetPartTransform1, column J | head look +10 | 587b8 |
| Fresh GetPartTransform1, column P | head position +20 | 587f8 |
| Fresh GetPartTransform23, column I | hips look +30 | 58838 |
| Fresh GetPartTransform23, column J | hips matrix Y +50 | 5887c |
| Fresh GetPartTransform23, column K | hips matrix Z +60 | 588c0 |
| Fresh GetPartTransform23, column P | hips position +40 | 58904 |
| Fresh GetPartTransform15, column P | left foot position +90 | 58948 |
| Fresh GetPartTransform19, column P | right foot position +a0 | 58984 |
| Skeleton +2840, physics record velocity23 | hips velocity +b0 | 589b4 |

Skeleton Data begins +40; its volume-to-bone array is +670. Thus part19 translation is loaded from Skeleton +b70+30 at 58618; part15 from +a70+30 at 58688. The local translation is captured before its transform query. Raw VMADDFP at 58638/58648/5864c and 586a8/586b8/586bc computes local.X*I+P, then local.Y*J+previous, then local.Z*K+previous. Coefficients remain the first fused multiplication operand. Local W is ignored; transform W is preserved, including unknown W through the existing typed vector carrier.

The actual `try_query_part_transform` samples its live animation attribute on every call. The twelve calls therefore remain distinct, including four consecutive part23 calls. The true branch reads actual physics transforms; the false branch composes actual animation-to-world with the retained animated volume transform. No synthetic identity or cached choice is admitted.

Each output write retains the native latch check. The two bone-point sequences sample the latch before the FMAs and check after them. Column-copy sequences query first, then sample/check, then copy the selected column. The final velocity samples the latch at 58988, reads +2840 at 5898c, checks at 58994, and stores at 589b4. The actual velocity owner is `physics_history().record().velocities()[23]`: Skeleton +1f50 plus record velocity array +780 plus 23*16 equals +2840.

## Integration and validation

Register only under `physical_player_phys_out_construct.rs`:

```rust
#[path = "physical_player_phys_out_skeleton_part_fill.rs"]
pub(crate) mod skeleton_part_fill;
```

Adapter requirements are `volume_to_bone_translation(part)` from the retained SkeletonData matrix translation, `part_transform(part)` delegating each time to the existing actual normal query, and `hips_velocity()` from the actual physics history record. The output entry is `try_fill_skeleton_parts(&mut source)`. No new retained output fields are needed.

Four focused tests cover all thirteen destinations with distinguishable fresh query results, local-before-query order, failure after earlier stores, locked-output source reads, known-W arithmetic, unknown-W preservation, and ignored NaN local W. They are saved but not run by this agent; root owns compilation and gates. Host f32 fused arithmetic is used; no new estimate or transcendental numeric policy is introduced.

## Separate ClampJump review

Independent raw review of 0x82b23a58..60 confirms `fcmpu cr6,f13,f12`, branch word `40990008`, then `fdivs f0,f12,f13`. BO4/BI25 branches when GT is clear, including unordered. Consequently division occurs only for `denominator > candidate_length`; otherwise the retained scalar is one. Root owns that correction and regression tests, independently of this Fill segment.

Root integration review: independently inspected raw82b58618..589b4 and reviewed actual pool/world adapters. Confirmed twelve fresh queries, local translation capture before query, three-FMA order, latch timing, same owner Arc and actual history velocity. Registered validation: workspace40 3,952 units +6 docs PASS; original37 17/17 PASS; Clippy61/fmt32/diff32 PASS. Stops before angular-velocity continuation589b8.
