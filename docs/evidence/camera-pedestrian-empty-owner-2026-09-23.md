# Selected camera pedestrian query owner (2026-09-23)

The camera's `PathEvaluatorObstacleServer::AddPedestrianObstacles` at
`0x82e39eb0` calls `LW::Man::GetPedestrianMan` at `0x82e4f078`, then the
pedestrian manager's `GetPedestriansWithinRange` with a maximum of 20 and the
live `kActorSearchRadius` value. It skips candidate handling only when the
returned count is zero. `GetPedestrianMan` asserts when `gpPedestrianMan` is
absent, so a missing manager is not a valid empty result.

Native ownership and selected order:

- `LivingWorld::Manager::Manager` (`0x82721dc0`) constructs embedded
  `mPedestrianMan` at `0x82721e18`. The `LWPedestrianMan` constructor
  (`0x82e7c900`) constructs its base collision manager and group-perception
  attribute instance.
- `Manager::InitLWMan` (`0x82702da8`) publishes the same embedded manager via
  `InitPedestrianMan` at `0x827030c4`; `InitPedestrianMan` (`0x82e4f378`)
  assigns `gpPedestrianMan` at `0x82e4f37c`.
- `Manager::Init` (`0x8271f6f0`) calls pedestrian virtual `Init` at
  `0x8271f8a8`, before vehicle `0x8271f8bc` and DMO `0x8271f8e4`.
  The shared component Init (`0x82d2af28`) publishes its entity manager at
  `0x82d2af80` and initializes it at `0x82d2afec`. The pedestrian factory
  (`0x82e93678`) allocates `0x38c` bytes, its entity constructor
  (`0x82e92c40`) constructs an empty 40-entry map and null KD root, and its
  Init (`0x82e92cb8`) allocates a MemStore of 40 blocks, 80 bytes each.
- Pedestrian `GetPedestriansWithinRange` (`0x82e9ab88`) tailcalls generic
  `KDTree<uint,Vector2,2>::FindNear` (`0x82d28c00`), clamping the caller's
  requested maximum to 40 at `0x82e9ab8c..94`. `FindNear` stores zero to
  the result count at `0x82d28c34`, checks `maxItems <= 256`, then checks its
  root at `0x82d28c5c`. Null root branches to exit without writing the ID or
  distance arrays.

The Rust `LivingWorldGrindServices` is already the retained selected Manager
occurrence shared with the first session's camera. It now retains one
`EntityMan<40>` for pedestrians alongside its existing vehicle and DMO stores.
The selected entity initialization publishes the pedestrian manager field and
constructs/initializes its entity store before vehicle and DMO. Its checked
range method returns count zero only when that store was actually initialized,
its KD root remains null, and its roster is empty. Missing publication,
missing entity storage, a populated tree, or a populated roster returns an
explicit error. There is no second camera-specific pedestrian owner.

This is a **high-confidence** constructor-empty query path, based on named
symbols and PPC branch order. It does **not** claim full native
`LWPedestrianMan::Init`: execution-group creation and its attribute lookup
remain unrecovered. A populated KD tree still needs native traversal and
candidate conversion. The selected publication field represents only the
source `gpPedestrianMan` assignment, not all side effects of `InitLWMan`.

Focused validation: `cargo test -p skate2-core living_world_grind_ --lib`
passed **5/5**. The new tests cover pre-publication rejection, retained owner
identity across a repeated Init attempt, result count zero with untouched
output arrays, the native pedestrian clamp to 40, the generic max-items
boundary, and rejection of later populated state.
