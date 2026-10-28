# BlendSpace plural attributes ? 2026-09-22

Independently source/code approved and registered; Cargo gate pending.

GetAttributes82d7fc18 uses active-volume indexes and original flags for all
children. Stack-local attribute lists do not consume the shared scratch pool.
The first list is multiplied once; later lists accumulate through shared
MultAdd primitives, with live per-attribute coefficient reads. Only completed
valid results append to caller output. Single GetAttribute82d77658 uses direct
children instead and remains explicitly pending.

The original sorted scan can read unwritten end scratch: current missing
attribute is invalidated before the following unknown read. This implementation
preserves that prefix and returns a typed admission error for unknown storage;
it does not claim defined native results for those reads. Packed-name
re-encoding remains the inherited admission boundary.

Shared Attribute MultBy82d6b0e8 and MultAdd82d6b198 were extracted from the
existing two-way blend. Time products/additions store before payload type
assertion; static MultAdd adds positive zero rather than resetting times.
All four vector lanes and fused payload accumulation are retained. Existing
PhaseBlend continues through the same multiply-then-add wrapper.

Fresh raw proof: artifacts/acs-nblend-attributes-independent-2026-09-22.
Four tests discriminate accumulation, time prefix on mismatch, exhausted
cursor invalidation and empty scratch admission. Confidence high for admitted
source ordering; real-data execution and exceptional host numeric parity remain
separate validation boundaries.

Root checkpoint: strict clippy-attrs-live-services-2 PASS41.17s; workspace-attrs-live-services-2 4,633 unit plus six doc PASS; original-attrs-live-services-1 18/20,5.01s. Neutral stops at real BlendSpace serialized normal tail; bail advances past Action23 to Motion expression268.
