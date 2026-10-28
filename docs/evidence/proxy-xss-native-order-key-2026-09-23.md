# PROXY XSS native order key and ties (2026-09-23)

Scope: source audit of `cProxyWorldManager::SyncLoadProxyWorldStream` and its `tProxySort` merge sort. This resolves the key algorithm; a comparison over the 147 observed XSS rows remains unrun because this session has no mounted game-data root.

## Native row and key

`SyncLoadProxyWorldStream` (`0x824df648`) starts the XSS table at file offset `0x18` and advances each authored row by `0x50`. In the row loop, the loads at `0x824df898..8c0` read `hull_min.xyz` at row offsets `0x10/0x14/0x18` and `hull_max.xyz` at `0x20/0x24/0x28`. These match `CollectionSpaceEntry` in `crates/skate2-formats/src/stream.rs`. The vector operations at `0x824df918..948` form `midpoint = min + (max - min) * 0.5`.

The routine constructs a `0.5f` vector from `__real_3f000000` at `0x824df870..928`. It adds this value separately to midpoint X (`0x824df950..960`) and midpoint Z (`0x824df970..99c`) before calling `floor` for each axis. Each floor result passes through `frsp` and `fctiwz` to a signed integer (`0x824df978/9a4` for X; `0x824df9a0/9b4` for Z). At `0x824df9b0..9d0`, `mulli` multiplies X by `0x2710` (10,000), signed `divw` divides Z by 10 (truncating toward zero), and `add` stores their sum in `tProxySort::mOrder` at offset 8. Thus, for ordinary finite coordinates:

```text
x = int32(float32(floor(midpoint.x + 0.5f)))
z = int32(float32(floor(midpoint.z + 0.5f)))
mOrder = int32_wrap(10000 * x + trunc_toward_zero(z / 10))
```

The sort record also receives the row's full 64-bit collection ID at `0x824df9d4..9dc`. The ordered records feed membership insertion and collection loading at `0x824dfa64..a88`. This is scheduling order for the XSS rows, distinct from their authored order and from parent-expanded dependencies.

## Comparator and equal keys

`ProxyCollectionOrderCompare` (`0x824d6888`) uses signed `cmpw` and returns true only for `left.mOrder < right.mOrder`; it has no ID tie breaker. The merge called from `eastl::merge_sort_buffer<tProxySort>` (`0x824dca40`) compares `right` against `left` at `0x824db4c8` and chooses the right row only when that comparison is true. Otherwise it chooses the left row (`0x824db4ec..f8`). Equal keys therefore retain XSS row order through the merge sort.

## Existing Rust and validation limit

`skate2_formats::stream::collection_stream_order` (`crates/skate2-formats/src/stream.rs:332`) already implements the same X/Z rounding and signed `10_000 * x + z / 10` key. `CollectionSpace::focus_collections` computes the same hull midpoint, but also applies focus/LOD selection; it is not the PROXY row enumerator. A PROXY owner can compute the midpoint for every authored XSS row and apply a stable ascending sort by the existing key. For exact edge behavior, native arithmetic wraps at 32 bits and VMX midpoint/addition rounding can matter at half-integer boundaries; the current Rust helper has not been checked against those boundaries in the PROXY corpus.

`SKATE2_GAME_ROOT` is absent in this session, and the existing `artifacts/proxy-parent-census-2026-09-23.log` contains IDs and counts but no XSS hull words. Consequently this audit does **not** assert a 147-row key sequence, a tie count, or corpus equality. The missing validation is to parse the mounted PROXY XSS with the existing `CollectionSpace` parser, emit each authored row's hull midpoint, key, and ID, then stable-sort and compare the complete sequence (including tie groups) with a native trace or independent PPC-equivalent evaluator. No duplicate parser or registered code change is required for that check.
