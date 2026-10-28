# Skeleton IK constructor, Reset, and retained attribute ownership

Read-only source recovery, 2026-09-08. This packet supplements
[the active IK continuation packet](skeleton-general-ik-next-source.md).
It does not claim that the active IK solver/contact/drive continuation is wired.

## Evidence and source scope

IDA raw bytes, decoded operand fields, and the loaded PDB were read without
database mutations. Half-open source spans and SHA-256:

| Function | Span | SHA-256 |
| --- | --- | --- |
| `Sk8::Physics::SkeletonIK::SkeletonIK` | `82b550f0..82b55398` | `fe5ec937a3845bb4c3de546b194979c77a1702795ade9eb74be981315d5c85f6` |
| `Sk8::Physics::SkeletonIK::Reset` | `82b51370..82b51710` | `138aafd84e1f4f04454f94a5af1e945fe7c0a160c63aa5943bb5316d29030e06` |

`SkeletonIK::Init` at `82b5eb80` remains the existing mapping producer;
`CalculateTargetTransforms` at `82b4a140` remains the current pose target producer.
Neither is replaced by Reset. The owner is `Sk8::Physics::SkeletonIK`, heap
allocated at82b57420, constructed at5743c and stored by Skeleton at57448.
It shares the Skeleton lifetime, retaining references to Skeleton, processed input,
SkeletonData, and SkeletonState. The native generated
`Attrib::Gen::physics_skeletonik` type has exactly one base subobject,
`Attrib::Instance` at offset zero: no additional generated cached-layout payload.

## Constructor order and failure prefixes

1. `5513c`: `FindCollection` for class `c8be3349f2bd493e` and default collection
   `d7edbd362d7d2152`; `5514c`: initialize the Instance with null message port.
2. Store owner references: Skeleton `+10` at `55150`, processed input `+14` at
   `55154`, SkeletonData `+18` at `5515c`, SkeletonState `+1c` at `55164`.
3. `55168`: clear exactly `+20..+1d0`, length `1b0`, including all DataIn
   matrix/vector lanes and pointer words. `55170`: call full Reset below.
4. Four sequential reads from `GlobalAttributeCollection.sInstance` at
   `830591b8`, using its `+94` physicsdeck Instance. Each read independently
   samples/asserts the singleton and then requests element index zero. A missing
   attribute pointer selects `DefaultDataArea(4)`; a malformed present payload
   is not evidence for using that fallback.

| Order | Key | Get call | Value load / calculation | Retained store |
| --- | --- | --- | --- | --- |
| 1 | `d76d9db5ee1fd008` | `551c0` | `551d8`; multiply by half at `551e0` | `+c60` box half-width, `551e4` |
| 2 | `c94d65091dcb828b` | `55228` | `5523c`; multiply by half at `55240` | `+c64` box half-length, `55244` |
| 3 | `1d53bc53932190e7` | `55288` | `5529c`; reload retained `+c64` at `552a0`, add new value + old half-length at `552a4` | `+c68` total half-length, `552a8` |
| 4 | `80203de9f9b732ca` | `552ec` | vector load `55304`, splat X; degree product then ordered clamp and tangent | `+c6c` end slope, `55388` |

Half is bits `3f000000` from `8200fd30`. The end-angle degree factor is
bits `3c8efa35` from `82261ed4`. Crucially, raw `5534c` multiplies the fetched
angle by this factor; `55350` stores the PRODUCT and `55354` reloads it.
Do not follow decompiler text that clamps the constant instead of the product.

Let `p` be that rounded product, `lo = bits(3c23d70a)` from `8200fdf0`, and
`hi = bits(3fc7c82d)` from `82268b7c`. Scalar source graph:

```text
t = lo - p
a = if t >= 0 then lo else p          // fsel 5535c
u = hi - a
b = if u >= 0 then a else hi          // fsel 55364
slope = XMVectorTan(splat(b)).x       // 55374
```

Comparisons describe `fsel`, including either signed zero as nonnegative;
unordered selects the final operand. Thus NaN product becomes `hi` at the
second selection. Positive infinity becomes `hi`; negative infinity becomes
`lo`. Every actual constructor tangent input is finite and bounded by these
endpoints. The full tangent graph is in
[skeleton-ik-tangent-source.md](skeleton-ik-tangent-source.md).

All reset writes precede the first global read. A failure on a later read or
numeric service must retain the earlier geometry stores, without inventing
values for geometry fields whose store has not happened. An optional/fresh
construction phase can represent these unknowns; a successful owner has four
known geometry scalars. No repeat attempt is implied by the native constructor.

## Reset store map

The function has no attribute/cvar/numeric callbacks or assertions. It first
sets `mDoIK +1d0 = true` at `51384`, then `mSkateboardDontPush +c70 = false`
at `5138c`, then `mTellAnimDontPushOrBrake +c71 = false` at `51398`.
Scalar zero comes from `8220a90c` bits `00000000`; temporary vector W words
are explicitly cleared, so these vectors have known positive-zero W.

The first loop completes each entry `i=0..3` before advancing. In exact order:

| Store address | Field base / stride | Value |
| --- | --- | --- |
| `51408` | mode `1f4 + 4*i` | disabled, integer 0 |
| `51414` | on-deck blend `1d4 + 4*i` | +0 |
| `5141c` | external blend `1e4 + 4*i` | +0 |
| `51420` | external normal blend `33c + 4*i` | +0 |
| `51434` | contact position `be0 + 16*i` | zero full vector |
| `51438` | contact status `c20 + 4*i` | integer 0 |
| `5143c` | world target position `220 + 16*i` | zero full vector |
| `51440` | local target position `260 + 16*i` | zero full vector |
| `51444` | local target delta `2a0 + 16*i` | zero full vector |
| `51448` | normal-set `320 + i` | false |
| `5144c` | external-set `324 + i` | false |
| `51454` | local-set `328 + i` | false |
| `51458` | target blend `32c + 4*i` | +0 |
| `5145c` | part-on-deck `c30 + 4*i` | unsigned 0 |

The next eight loops each complete four matrices in ascending entry order,
with per-matrix stores X, Y, Z, P. I/J/K are from `8227e8f0`, `8227e900`,
`8227e910`; P is full positive zero. All four W lanes are known zero.

| Array order | Base | Field | First-entry column stores |
| --- | --- | --- | --- |
| 1 | `360` | target transform | `514b0/514b4/514b8/514bc` |
| 2 | `7c0` | parent-to-animation | `514ec/514f0/514f4/514f8` |
| 3 | `8c0` | parent-to-world | `51520/51524/51528/5152c` |
| 4 | `9c0` | parent-to-world external | `51554/51558/5155c/51560` |
| 5 | `ac0` | parent-to-deck | `51588/5158c/51590/51594` |
| 6 | `580` | part-to-world external | `515c4/515c8/515cc/515d0` |
| 7 | `480` | part-to-world | `515f8/515fc/51600/51604` |
| 8 | `680` | part-to-deck | `51630/51634/51638/5163c` |

Then separate ascending loops clear full part-position vectors at `+780`
(`51658`), full external-normal vectors at `+2e0` (`51678`), scalar contact
target offsets at `+bc0` (`51698`), and scalar contact offsets at `+bd0`
(`516b4`). Finally full last-left-foot-deck-position `+c40` is zeroed at
`51704`, then full last-right-foot-deck-position `+c50` at `51708`.

Reset does **not** write Instance/owner references, DataIn, bone IDs `+204`,
target-bone IDs `+34c`, Part IDs `+460`, parent-Part IDs `+470`, or any of the
four deck geometry scalars. IDs are genuinely unknown before Init; their
absence must not be reinterpreted as a constructor store of zero or -1.

## DataIn is a retained snapshot, not reset tuning

Offsets below are relative to DataIn `+20`; total length is exactly `1b0`.

| Offset | PDB member |
| --- | --- |
| `000` | animation-to-world matrix |
| `040` | world-to-animation transform |
| `080` | physics skateboard matrix |
| `0c0` | inverse skateboard matrix |
| `100` | hips transform in animation space |
| `140` | skateboard matrix |
| `180` | most-vertical toe contact normal |
| `190` | hips offset |
| `1a0` | actual animation-volume pose-buffer pointer |
| `1a4` | actual animation pose-buffer pointer |
| `1a8` | island pointer |
| `1ac` | Skeleton assembly pointer |

Constructor matrices are all-zero, not identity. Later UpdateIKData replaces
the snapshot from the actual owner authorities. Island/assembly pointer copies
have no null assertion at this boundary. Retain real identity separately from
whether the host implements the later line-query service; stop lazily at an
actual query, not while copying DataIn. Missing collision-normal data is a
separate source read boundary at `82b41798`.

## Runtime provider and integration proposal

Retain a single native physics_skeletonik Instance for the new complete IK
owner, bound from the actual physical owner's `Arc<AttributeRuntimeVault>`.
Use existing `AttributeRuntimeInstance::try_get_data(key,0)`, preserving its
node, inheritance, original-child layout, null-pointer, and checked-payload
semantics. There is no evidence to cache selected scalar/vector values across
Update calls. Fetch absent values as `DefaultDataArea(4/16)` zero only after
the lookup actually returns no pointer; report malformed present payloads.

The following own-Instance reads are unconditional at their stage prefix:

| Stage | Ordered key | Bytes | Get / value load |
| --- | --- | --- | --- |
| UpdateBlendValues | `aa378f2ebf7aa313` | 16 | `51d98 / 51db0` |
| UpdateBlendValues | `7ce328814f9309ae` | 16 | `51de0 / 51df4` |
| UpdateBlendValues | `39fa8c07e8356284` | 4 | `51e28 / 51e40` |
| UpdateBlendValues | `094ace5de06851fc` | 4 | `51e5c / 51e74` |
| CalculateInitialPartTransforms | `121e67e2b4096a41` | 16 | `517a8 / 517c8` |

The first blend vector is loaded and added to the constructor geometry before
the second lookup; the second vector is loaded and combined before the first
scalar lookup. Each scalar load precedes the next lookup. No retained IK row
stores occur until all four blend reads complete. The initial-transform vector
is loaded before its first draw cvar access. Immutable typed `scalar(key)` and
`vector(key)` methods can collapse lookup plus immediate load here without
moving a fallible or externally visible operation between them. Do not prefetch
the initial-transform vector before the preceding status/blend stages.

The existing actual global deck Instance already persists in admitted board
`SkateboardBodyConstruction.attributes`, reached via `attributes_mut()` and
the pool's retained construction owner. Borrow this existing
`SkateboardAttributeRead` provider for the four constructor reads. Do not
construct a second board definition, copy derived deck geometry as substitute
tuning, or bind a fresh deck Instance for each frame. Board construction precedes
Skeleton construction in the paired factory, so its lifetime is available.

Extend one logical IK aggregate around the existing `SkeletonIkTargets`, which
continues to own its mapping IDs and target transforms. Add only the missing
retained mode/history/intermediate arrays; do not duplicate the target array.
Root owns aggregate/target interfaces and the exact constructor insertion.
Full Reset should call the existing target-reset operation at array-order 1
and reset the new histories in the source sequence. It must preserve mappings,
DataIn, and constructor geometry. The current teleport/reset caller only
resets target transforms and must be upgraded at that existing boundary.

The pre-physics two-bone caller additionally reads global physics_animation
keys `b76b5101768e856c` then `a175875dec9d6db7` after its own clamp/debug prefix.
That provider should borrow the existing global animation Instance lifetime;
its exact trait and delayed-load requirements belong to the two-bone author.
No new eager all-IK-tuning snapshot is justified. Contact and external stages
also sample cvars lazily; their detailed source stages remain in the parent
packet and are not absorbed into the Vault provider.

A final scan of all named SkeletonIK functions found additional attribute
sites outside this active pre-physics provider subset: `DoesBoneIntersectDeck`
reads own-Instance vector key `693453cdb831119b` at `534cc` (load `534e4`);
`FillPhysOut_FeetDeckInteraction` reads own-Instance vector key
`77dd00c18946c6cf` at `53c1c` (load `53c4c`);
`DoPostPhysicsFootIK` reads global physics_animation `+cc` scalar key
`3b2641b64fcf3485` at `55cd4` (load `55cec`); and `UpdateWipeoutAdjust`
reads own-Instance scalar key `54c356704afa0a9a` at `59cf0` (load `59d08`).
All use index zero and matching 16/4-byte missing-pointer fallback. Their
enclosing control flow is not recovered by this lifecycle packet, and none
should be prefetched merely because the provider exists.

## Validation proposal and confidence

Focused tests should seed every retained field, invoke Reset, and prove the
changed arrays plus retained DataIn/IDs/geometry. Constructor tests should
fail each of the four reads and each tangent service, checking geometry/store
prefixes; vary degree values including infinities and NaN through the actual
fsel graph. Provider tests should cover inheritance, exact scalar/vector sizes,
missing fallback, present malformed payloads, and per-stage lazy order. An
original-data check should read the same admitted board provider and original
physics_skeletonik instance, compare independent geometry arithmetic, then
consume that owner's targets/rows through the active continuation.

Confidence: **exact** for raw spans/constants and enumerated Reset stores;
**high** for source control/order, typed ownership proposal, and immutable
provider timing. No Rust changes, Cargo invocation, original-data validation,
or Xenon per-tick trace was performed for this packet. Numeric estimates,
NaN payload propagation, denormal mode, mutable native Vault notifications,
active contact queries, and completed active-IK scheduling remain explicit
separate validation/integration boundaries.
