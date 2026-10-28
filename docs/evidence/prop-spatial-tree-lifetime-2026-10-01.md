# Prop AddEntity TreeCapacity during pedestrian census

Reported at18326 ticks: LivingWorld.Census.World.PedestrianCull
`Prop AddEntity: TreeCapacity`.

## Confirmed cause

The prop EntityMan had no scheduled spatial clear/rebuild caller. AddEntity
inserted a KD node on each spawn; RemoveEntity removed map membership only.
Old nodes therefore accumulated across successive culls/spawns until the
256-block tree filled, even with fewer than256 live props. The pedestrian
census error wrapper did not mean its40-entry pedestrian roster was full.

## IDA evidence

- Prop Init82e67788 calls MemStore.Init(0x50,0x100).
- Prop ClearSpatialLookups82e677b0 resets the MemStore and nulls the root.
- Prop UpdateSpatialLookups82e677f0 iterates the live map in its native order
  and inserts each ID and CommonEntityCore's XZ position.
- Manager.ClearSpatialLookups826f5320 and UpdateSpatialLookups826f5398 both
  visit **pedestrian, vehicle, prop, DMO** in that order.
- Manager.Update182727ee0 ends population processing with four entity-census
  updates, then ClearSpatialLookups and UpdateSpatialLookups82728354..8360.
- CensusMan.Update82702780 separately updates settings and calls those four
  entity-census updates8270288c..28c8 earlier in the frame. Both passes are real;
  the later one must not decrement reserve timers/update settings again.
- CharacterPrepareState's initial census uses clear/evaluate/update, with the
  phases separated around its initial population evaluation.

## Change

`selected_living_world_dmo.rs` now shares one four-manager spatial phase for
boot and Update1. Replaces the old partial rebuilds rather than adding a second
prop index. It clears all four, then rebuilds all four at the Update1 boundary;
boot retains its separately scheduled clear and update.

Extracted existing entity-cull processing into one shared helper. Update1 now
drains all four after population evaluation, where previously only the DMO
cull map was drained. Reuses existing destructor/free paths and actual rosters.
The early Census.Update pass still updates settings exactly once before culls.

No capacity increases, silent insertion failures, spawn suppression, or eager
RemoveEntity KD deletion. Existing native-style delayed spatial retirement is
preserved. Rebuild also publishes moved live props' latest XZ positions.

## Validation and limits

Scoped Rust syntax parsing passed. Added an unrun regression for1024 sequential
prop culls/replacements with the256-node tree, checking retired IDs disappear
and the surviving moved entity remains queryable at its current coordinate.
Static scheduling review confirms shared boot and per-frame callers.

No build or game launched. A long
city traversal is still needed to confirm the reported runtime failure is gone.
