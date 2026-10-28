# Initial collision scene

Last updated: 2026-09-02

## Stream selection

`GameModule::InitStreamerLocation` (`0x8272e888`) initializes the offline
stream focus from the game-info position. `cAssetStreamSystem::GetCollectionId`
(`0x824d73b0`) converts that position to a high-detail grid collection:

```text
center(axis) = 200 * (truncate(axis) / 200)
             + (axis < 0 ? -100 : 100)
name = c<stream suffix>_<center x>_<center z>_high
```

Integer division is truncation toward zero, matching the PowerPC routine. The
reference `BAM_Sim.loc` entry `A_RobsPark` is at
`[588.8617, -54.231647, 996.5048]`, selecting `cSim_500_900_high`.

That current-cell calculation is distinct from the full focus set.
`cAssetStreamSystem::AddStreamFocus` (`0x824dacb0`) assigns presentation and
simulation a horizontal radius of `195`; texture streaming uses `70`.
`cStreamFile::FindCollections` (`0x824df068`) queries a box with horizontal
half-extents equal to the focus radius and vertical half-extent `8000`, then
sorts candidates by XZ distance from the focus to each hull center. The default
simulation cap is four (`kSimMaxFocusCollections`, initializer `0x82f4e310`);
the presentation cap is sixteen (`kPresMaxFocusCollections`, initializer
`0x82f4e2b8`).

The four simulation collections selected at Rob's Park, nearest first, are:

1. `cSim_500_900_high`
2. `cSim_500_1100_high`
3. `cSim_700_900_high`
4. `cSim_700_1100_high`

## Runtime ownership proof

The PC application opens all four selected XSF collections, decodes their
arenas, and copies collision data into platform-neutral `skate2-core` runtime
structures. The runtime scene owns expanded mesh triangles, primitive volumes,
and the instance-to-collision-model object graph; it has no pointers into
temporary XSF or RW4 decode buffers.

Current Rob's Park high-detail scene:

| Measurement | Count |
|---|---:|
| RW4 arenas | 68 |
| Aggregate collision roots | 71 |
| KD primitive sets | 45 |
| Primitive volumes | 451 |
| Clustered meshes | 26 |
| Expanded triangles | 354,382 |
| Collision models | 26 |
| Instances | 477 |
| Collision-bound instances | 26 |

A native vertical segment query at the serialized spawn position returns one
hit. The nearest point is `[588.8617, -54.431, 996.5048]`, normal
`[0, 1, 0]`, packed surface ID `133`. The spawn is therefore approximately
`0.199` world units above the decoded ground, a strong end-to-end consistency
check across location parsing, stream-grid selection, archive reads, arena
decoding, mesh expansion, and collision math.

## Implementation boundary

The reference uses an octree to accelerate focus-box overlap. The native
selector currently tests the same serialized hull AABBs directly. This
preserves the candidate set and ordering while keeping console-specific octree
storage out of the source reconstruction; a native spatial index can replace
the linear scan when dynamic streaming needs it. Collision queries already use
the owned serialized KD trees. The next runtime step is collection lifecycle
and collision-island behavior as a focus crosses cell boundaries.
