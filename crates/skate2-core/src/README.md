# Core source map

Start with the subsystem folder below. Existing descriptive filenames are retained
so searching an older source note still finds the same implementation.

| Folder | Contents |
| --- | --- |
| [actors](actors/) | Actor construction, ownership, breadcrumbs and skater services |
| [animation](animation/) | Animation trees, channels, pose evaluation, locomotion, playback, jobs and Action/Motion graphs |
| [assets](assets/) | Shared asset and character loading; Vault readers in `attributes/` |
| [camera](camera/) | Camera calculations, with `frame/`, `graph/`, `rig/` and `shots/` |
| [input](input/) | Input routing, devices, recognition and shared live tuning references |
| [living_world](living_world/) | Vehicles, DMOs, entities, navigation, population, interactions and assets |
| [physics](physics/) | Player states, board and skeleton physics, collision, assemblies, Reckoning and solver |
| [rendering](rendering/) | Presentation contracts, materials, shadows and skater presentation; GPU implementation remains in the PC app |
| [session](session/) | Session/player lifecycle and the live startup, streaming and LivingWorld adapters |
| [streaming](streaming/) | Collection residency, activation, world streaming and simulation arena adapters |
| [world](world/) | World contracts, spatial indexing and WorldPainter data |
| [math](math/) | Shared random-number and numeric parsing helpers |
| [diagnostics](diagnostics/) | Logging, debug overlay, evidence types and scaffolding |

The folder hierarchy is a **physical source layout**. Rust module identities,
visibility and public APIs are preserved in [lib.rs](lib.rs) and the existing
module owners through explicit `#[path]` declarations. A folder does not create
a second Rust module or subsystem instance. Tests and corpus files remain with
their corresponding subsystem; inline tests remain inline.

See [the maintenance guide](../../../docs/CORE_LAYOUT.md) for examples and rules,
[the relocation index](../../../docs/core-file-moves.tsv) for old paths, and
[the named ids guide](../../../docs/NAMED_IDS.md) for the graph and physics id names.
