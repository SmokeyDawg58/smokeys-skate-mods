# Wheel sphere eligibility for Assembly/ground Contact (2026-09-23)

Read-only audit of the matched `sk82_na_m.xex.i64` IDA/PDB, current Rust, and
the adjusted-spawn release traces. **Exact** identifies the cited native
stores and branches; **high** identifies the connected Rust path and its
logged result. No matched native per-tick wheel Contact trace exists, so the
native outcome at this spawn is **unmeasured**. No code or Cargo command was
changed or run.

## Observed Rust producer result

The frame-2 CPU ground job processed two assemblies and submitted four
terrain triangles. Its 13 pre-import records are nine board records from
volume ordinals 11, 13, and 15, all bound to deck Part 6/body ID 7, and four
Skeleton records from ordinals 15, 16, 19, and 20, bound to bodies 24, 25,
28, and 29. No board volume 0..3/body ID 1..4 produced a row. This is the
producer result, not merely the later positive-force spy filter. Frame 1
produced zero rows with two submitted triangles. Source:
`artifacts/controlled-skating-release2-ground-records-2026-09-23.log`,
`ground-records count` and `ground-record[0..12]`; see also
[adjusted-spawn first 30](adjusted-spawn-first30-runtime-trace-2026-09-23.md).

Before frame 2, the four Rust wheel Part centers have Y values `-54.486816`,
`-54.522614`, `-54.503006`, and `-54.504196`, while the deck is at
`-54.44767` (`artifacts/controlled-skating-release2-wheel-parts-2026-09-23.log`).
The spawn line hit near the deck was about `-54.431`, but its Y at each wheel
X/Z and the four selected terrain triangles' planes have not been captured.
Thus the center values alone neither prove sphere/triangle overlap nor show
which ground Run gate rejected each wheel. A center below a nearby terrain
height is not a Contact record.

## Original volume and serialization

The extracted original `skater.vlt/bin` has `WheelCapsuleLength=0` and
`WheelRadius=0x3cfdf3b6` (about `0.031`); provenance is in
[wheel definitions](skateboard-wheels.md). `SkateboardBody::CreateWheels`
`0x82b344f0..0x82b34a8c` takes the sphere arm and calls
`SphereVolume::Initialize` at `0x82b34790`. That initializer sets the sphere
type, local identity axes/zero translation, surface 0, and **`m_flags=1`** at
`0x82ad9438..0x82ad94c8` (flag store `0x82ad9498`). `CreateWheels` sets the
volume's own `groupID=5` at `0x82b3479c`, keeps the one allocation at
`mVolumes[2]`, and assigns that same pointer to Part definitions 0..3 in its
loop (`0x82b34880`, terminal check `0x82b34a78`). Four shared-pointer Parts
are four volume **occurrences**; the collision volume is not just the debug
sphere.

`Skateboard::SetStandard` calls `SkateboardBody::EnableCollisionVolumes`
(`0x82b355c0`). Enable ORs bit 0 into wheel `mVolumes[2]` at `0x82b241d8`,
as well as the truck and deck volumes. The inverse method clears bit 0 from
all three shared top volumes and each deck child (`DisableCollisionVolumes`
`0x82b24128..0x82b241a4`; wheel store `0x82b24158`). Its direct callers are
`SkateboardController::InitSkateboardHiding` `0x82dc76f0` and
`InitSkateboardRetrieval` `0x82dc77d8`; Enable also has standard/free/wipeout
and controller callers. Therefore an actual tick's enabled state must be read
from the live shared volume. The fresh/standard source does not intentionally
disable just the four wheels.

`SerialisedAssembly::Create` `0x82e0c850..0x82e0ce28` walks Parts in order.
Its direct non-Aggregate branch copies each Part volume into a separate
`VolumeData` record, up to the 26-record cap; the four wheel Parts therefore
occupy ordinals 0..3 on the seven-Part authored board. It stores `enabled =
m_volume->m_flags & 1` at `0x82e0c95c`, padding `0.05` at `0x82e0c978`,
surface ID at `0x82e0c988`, Part transform, and `group =
Part.m_userData` at `0x82e0cb04`. It calculates a Part bbox and contributes
it to the Assembly bbox even if the enabled bit is clear. `Update`
`0x82e0cf30..0x82e0d35c` re-walks the same selected Part volumes and refreshes
enabled from the live flag at `0x82e0cfe8`, plus transform, material, group,
and frozen state; it does not select only enabled volumes. The wheel's
`Volume.groupID=5` is **not** the ground Run culling group. The latter is Part
userData, initially 4; `Skateboard::CollideWithWorldOnly` changes Assembly
and all Parts to 2 (`0x82b26898..0x82b268b0`), and `CollideNormally` restores
4 (`0x82b268e0..0x82b268f8`). Rust's
`skateboard_collision_publication.rs::source_volumes/try_create/try_update_volumes`
preserves the four occurrences, the shared flag, Part group and ordered
record identities.

## Native pipeline and per-volume gates

`World::World` supplies `GroundPoolParams.collisionGroup=0` at `0x827b1a00`;
`GroundPool::GroundPool` stores it at `0x827c8d14`. `Island::InitCullingTable`
`0x827b8c10..0x827ba464` first looks for an inventory BitTable and otherwise
constructs a 19-by-19 table. `rw::BitTable::Initialize` zeroes its words at
`0x82bc3308..0x82bc336c`. The fallback table's row/column writes do not set
the `(4,0)` bit; the `(2,0)` pair is explicitly cleared at
`0x827b8e98..0x827b8ea4`. `InitCullingTable` installs the resulting table on
the skater-ground pipeline at `0x827ba438`. A reused inventory table may have
different live bits and needs observation. The board's separate self-culling
table is not this pipeline table.

`AssemblyGroundCollisionPipeline::Run` `0x827cab00..0x827cada4` supplies that
pipeline table and ground group to each job (`0x827cac54`, `0x827cac60`). It
does not select volume types. The CPU `AssemblyGroundCollisionJob`
`0x827cdb98..0x827cdf48` skips a whole Assembly if all Parts are frozen or
all rigid bodies are static (`0x827cdcc8..0x827cdce4`), then prepares every
non-Aggregate volume in ordinal order, including its body binding and bbox
(`0x827cdd60..0x827cddbc`). It does not apply an enabled, wheel-type, or
per-volume static filter during this preparation.

For each ripped terrain triangle, `AssemblyTriangleIntersectionTester::RunIntersectionTests`
`0x827cd1e0..0x827cd7c8` visits every volume ordinal. It skips frozen
and Aggregate records (`0x827cd2a0..0x827cd2ac`), then a set pipeline bit
at `columns * VolumeData.group + groundGroup` (`0x827cd2b0..0x827cd300`).
After material selection and ground-body validation, it skips exactly a
`STATIC_BODY` volume body (`0x827cd3a0`). It constructs the volume GP, then
requires `enabled || numVolumes == 1` (`0x827cd3c0..0x827cd3dc`). The board
has multiple selected volumes, so the sole-volume exception cannot rescue a
disabled wheel. Next come inclusive triangle/Part-bbox overlap, effective
padding from the live body velocity and triangle normal, `Sk8ComputeContactPoints`,
and contact-buffer reservation. A zero point count or exhausted reservation
also yields no wheel record. Source ordering and numeric boundaries are in
[Assembly/ground Run](assembly-ground-run-integration.md) and the external
`world_probe_temp/assembly_ground_run_astra_proof.md`.

There is **no original wheel-specific exclusion** in this inspected
Assembly/ground path. With a live enabled, unfrozen, non-static wheel Part,
the normal group 4 (or world-only group 2), ground group 0, and source-created
fallback culling table permit its sphere to reach geometry tests. This proves
eligibility under those conditions, not that the native game generated wheel
Contacts on this particular frame.

## Rust correspondence and precise gap

`skateboard_collision_publication.rs` emits records 0..3 from the four live
wheel Parts, sets `enabled` from the shared volume flag, `frozen` from each
body state, `group` from `BoardCollisionGroups` Part userdata, and retains
the live transform, radius, material and `0.05` padding. It computes bounds
without filtering disabled/frozen/static Parts. `simulation_ground_contact.rs`
checks Assembly all-frozen/all-static first, prepares every non-Aggregate
volume, then its Run checks frozen/Aggregate, culling, static state,
constructs GP, applies the enabled/sole-volume rule, tests the triangle AABB,
computes padding, asks narrow phase for points, and reserves/writes records.
That is the native *eligibility* order for the inspected CPU route.

The selected live Rust facade currently requires `no_pipeline_culling=true`
and passes `pipeline_culling: None`
(`selected_session_public_facade.rs:763,960`; live client policy sets this in
`apps/skate2-pc/src/live_skating_client.rs:299`). Native Run receives the
Island's live pipeline table. This is a **source-confirmed culling-policy
parity gap**, although the source-created table's `(4,0)` and `(2,0)` bits
are clear, so that gap by itself does not explain the missing wheel rows
under the default table. The actual inventory-selected bit and actual native
frame state remain unmeasured. The separate flags-3 adjacent Assembly pair
and asynchronous scheduling gap is documented in
[first-tick support](wheel-support-first-tick-source-2026-09-23.md); it is
not a wheel-volume exclusion rule.

The existing Rust report counts only submitted triangles and final rows.
It does not expose the first rejecting gate for each wheel. The narrowest
next Rust diagnostic is a frame-2, wheel ordinal 0..3 trace for each of the
four submitted triangles, in source order: publication `enabled`, `frozen`,
body state and Part group; the pipeline bit and ground group; calculated
Part bbox and triangle AABB overlap; GP sphere center/radius and effective
padding; narrow-phase point count or rejection; and reservation result.
Also retain the exact triangle vertices, face normal and wheel X/Z. This
would identify why **Rust** produced zero wheel rows. A matched native
post-serialization and pre-solve ground-job trace with those same fields is
still required to call any contact difference a native parity failure.

## Subsequent Rust gate capture

The proposed wheel gate trace was subsequently run with
`SKATE2_TRACE_WHEEL_GROUND` for two controlled release frames; see
`artifacts/controlled-skating-release2-wheel-gates-2026-09-23.log`. For both
frame-2 board triangles, volumes 0..3 are enabled, unfrozen, non-Aggregate,
unculled, and bound to non-static body state 12. The two triangles have plane
Y `-54.431004`; all four swept Part bboxes end below them, with maximum Ys
`-54.45053`, `-54.48543`, `-54.46631`, and `-54.46747`. The inclusive bbox
overlap gate returns false for all eight attempts, before effective padding,
narrow phase, or reservation. This identifies the Rust rejection for those
triangles. The trace's triangle ordinal is local to each Run batch and it does
not report assembly-level skips, so it does not establish why the board had
no wheel row in frame 1. It also does not measure native contact behavior.
