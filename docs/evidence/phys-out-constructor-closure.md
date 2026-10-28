# Physical output constructor closure

Complete typed constructor/reset implementation is registered and awaiting its
first gate and independent implementation review. No native skating tick is claimed.
The production owner is the existing `PhysicalPlayerPhysOut` retained in
`PhysicalPlayerAssemblyOwner.phys_out`. Its selected projection and explicit
`PhysOutRemainder` contract do not establish full constructor completion.

## Allocation and ownership

`PhysicalPlayerHiLOD` constructor82e03c50 calls `PhysOut` constructor82e2f2a8
at82e03f20 and retains it at+2ac. GetPhysOut82e01698 returns that same owner.
All nineteen child allocations use kMemCategory_Sim82ff1af0 and flags1.
Allocation NULL is stored and the constructor continues; non-NULL storage is
published only after that component's constructor returns. The final unlocked
latch store82e2f730 follows all nineteen components.

Flags1 alone does **not** request clearing. Independently checked allocator
closure: XCore_EASTL_Category_Alloc828b4bf0 gets the category allocator through
GetCoreAllocator8296f4e0; CategoryAllocator.Alloc82973570 applies
`flags & mMemFlagsMask | mMemFlagsOR`. The PDB-typed table
memCategorySetups82ff0988[61], stride24, has Sim row55@82ff0eb0:
name821678a0, limit80000, heap IDs7/0, maskFFFFFFFF, OR100.
AddCategory82975158 retains these mask/OR fields. Therefore this original Sim
setup supplies flags101. Internal.Alloc82972a18 calls AllocHeapMem82972240;
both its small-block and ordinary-heap branches call MemClear for bit100.
Zero initial storage is justified by this actual category setup, not by flag1.
Changing the allocator configuration would require preserving its changed
initialization semantics rather than assuming the original table still applies.

| Order | Component | Allocation bytes |
|---:|---|---:|
| 0 | SkateboardReckoning | 0x120 |
| 1 | SkateboardMotion | 0x120 |
| 2 | Air | 0x1c0 |
| 3 | Physics | 0x30 |
| 4 | Grinds | 0x140 |
| 5 | Skeleton | 0x250 |
| 6 | Collision | 0xd80 |
| 7 | State | 0x48 |
| 8 | Ground | 0x130 |
| 9 | SystemReckoning | 0x90 |
| 10 | Audio | 0xe0 |
| 11 | Camera | 0x70 |
| 12 | Trick | 0x80 |
| 13 | Intents | 8 |
| 14 | Animation | 0xb0 |
| 15 | Scoring2 | 0x4650 |
| 16 | Filtered | 0x58 |
| 17 | PathRecording | 0x8c |
| 18 | OffBoard | 0x50 |

`PhysOut::Reset82e2f740` visits all nineteen components in the same order.
Update5_ProcessOutput82e04ea8 calls it first at82e04ed0; TeleportPlayerNow82e00618
also calls it at82e00940. Filtered reset82e2d618 initializes INVALID, not Ground.
Animation reset82e255f8 initializes time-since-last-input to zero; its actual
later producer is Update5 copying ProcessedPhysIn+9bc into Animation+80.
Filtered history comes from PhysOutConditioner_Physics.UpdateFilteredPhysicsState
82e2d800, rather than a direct alias of the current physical state.

## Complete constructor implementation

The prior projection's `reset_selected` returns true for Motion, Collision,
Ground and SystemReckoning. Its omitted-stage collaborator therefore does not
cover those components' other fields. Extending the complete owner requires
explicitly covering them as well as all remaining component constructors. The new
typed remainders omit those already retained projection fields, so there is only
one canonical value for each existing motion/surface publication.
Scoring2's nested constructor must complete before publishing Filtered. A failed
later constructor cannot be bypassed to invoke an Action callback that the
native outer caller would never reach. `try_construct_complete` requires all
nineteen stages before enabling concrete output reads. NULL allocations continue
as in the source; a subsequent full reset stops at the first NULL component and
preserves the already executed reset prefix and old latch. The old selected
collaborator path cannot satisfy the concrete completion flag.

| Component | Native constructor/reset closure |
|---|---|
| SkateboardReckoning | SetResetValues82e252d8 |
| SkateboardMotion | SetResetValues82e251e8 |
| Air | ctor82e2a300, Reset82e24ce0, inline trajectory/hand-plant values |
| Physics | inline82e2f2a8 /82e2f740 |
| Grinds | ctor82e2f028, Reset82e2d388, GrindChromosome.Reset82b38fd0 |
| Skeleton | SetResetValues82e253a8 |
| Collision | SkeletonCollisionInfo ctor82e2c690, Reset82e24e50, outerReset82e24f20 |
| State | inline descriptor/special-wipeout constructors, Reset82e25550 |
| Ground | SetResetValues82e250c8 |
| SystemReckoning | inline82e2f2a8 /82e2f740 |
| Audio | ctor82e2a370, Reset82e25028 |
| Camera | inline82e2f2a8 /82e2f740 |
| Trick | ctor82e2f118, Reset82e2d530, AnimOutTrickData.Reset825f68f0 |
| Intents | inline82e2f2a8 /82e2f740 |
| Animation | SetResetValues82e255f8 |
| Scoring2 | ctor82e2c768, Reset82e2a3f8, nested closure below |
| Filtered | ctor82e2f208, Reset82e2d618 |
| PathRecording | ctor82e2a6b0, Node ctor824b6538/Reset824c7d20 |
| OffBoard | SetResetValues82e256c8 |

Scoring2 retains all110 PDB-declared fields and all32 constructed Scorables in
eight fixed-capacity lists, even with count zero. Scorable ctor82deda90 invokes
TrickScoringSettings82de97f0: difficultyNone=3, scorable/fakie IDs=-1,
book order1, six intent tuples value1/endFrame=-1. SwitchVariantID and unused
connector bytes remain zero from the actual Sim allocation, not from an invented
reset store. Raw82de9850 stores a genuine zero64-bit attribute key.

AirPeriodData ctor82df2358/Reset82dede08 retains the initial Scorable settings
except `isValid`, and preserves underlying context UIDs while clearing counts.
ContextGroups ctor82de8c10 constructs five groups of two UIDData arrays;
UIDData ctor82de89b0 constructs eight UID/type pairs with typeInvalid=-1.
ContextData.Reset82de8b20 and Groups.Reset82de8c58 clear counts only.
Scoring2's list reset clears counts only, preserving inactive Scorable elements.
Threshold.Reset82dea760/Set.Reset82dea7b0 clears fifteen value/two-bool entries.
PatternMatches.Reset82de97d8 clears three bools.

BailScoringOutData ctor82df0de8/Reset82debf88 includes25 bone outputs,
three eight-entity sets, three AirDrop values and six damage-level scores.
BoneSet ctor82df0d98 and EntitySet ctor82ded488 are concrete field constructors;
bone damage levels, entity types and current body tweak reset to -1.
CompressedCharacterDamage.Reset82debf20 clears seven damage/intensity pairs.
Object/Grind GUID stores are full64-bit sign-extended -1, independently checked
in raw `li`/`std` instructions; decompiler split-register pseudocode is misleading.

Important reset discriminators:

- Audio Reset preserves both wheel-landing histories; only its constructor zeros them.
- Air Reset preserves hand-plant flag word bits30 and0..27 (mask4FFFFFFF), while
  resetting position and apex time. Trajectory duration is splatted -1.
- OffBoard's final raw stores82e2575c/760 write starting-jump=false and
  locomotion-viable=true. A decompiler combined-word expression obscures this.
- Node constructor quaternion bytes are0,0,0,255, but Node.Reset packs the ZERO
  quaternion through PackComponent824b2588, producing127 in each byte. Reset
  preserves ExtraData1..3 and deletes any extended node pointer.
- NULL_SKATINGGRINDNAME initializer82f7bd30 assigns empty bytes8225fe43;
  Andale.nullStringName initializer82fabfdc assigns `nullString`. The existing
  checked packed-name constants retain that distinction.

Four registered tests discriminate complete order/defaults, allocation-failure
prefix before Scoring2, reset preservation, and NULL-child reset failure position.
No Cargo was run by the implementation agent; root owns the pending gate.

Live IDA/PDB schema extraction is retained externally in
`%TEMP%/skate2-physout-schema.json` and `skate2-physout-nested-schema.json`.
Reset pseudocode is external in `skate2-physout-reset-*.txt`.
No executable data or PDB is copied into the repository.

## 2026-09-12 constructor mounting and nested review

The existing assembly owner now invokes the complete constructor between its
selected ProcessedPhysIn and Reckoning stages, before constructing either Body.
Fresh HiLOD82e03c50 decompilation confirms ProcessedPhysIn publication precedes
the outer PhysOut allocation (0x50, PhysicsSkater, flags1), call82e03f20 and
publication at+2ac. PhysOutConditioners and SurfacePhysics follow before
Reckoning. Those omitted outer collaborators remain separate unfinished work.
The selected host path chooses successful component allocations; native outer
NULL/OOM behavior is not claimed. Errors retain the CompletePhysOut phase and
the same partial output; later Reckoning failure preserves completed output.

The original pair corpus consumes this mounted owner and calls complete Reset,
instead of attempting the old late selected constructor (AlreadyStarted).
It verifies all nineteen component statuses and the actual Animation default.
The assembly Reckoning-failure test now checks that PhysOut has already
completed while no Body exists. These additions await root Cargo gates.

Fresh PDB inspection covered all31 saved nested schemas and the additional
GrabSplineDescriptor (8), SpecialWipeoutScoringData (8), pattern matches (3),
CompressedCharacterDamage (14), damage area (2), ObjectGrindInfo (24) and
GrindInfo (48). Saved bitfield member sizes describe their storage type;
fresh UDT bit lengths confirm HandPlant bits188..191 and trick bits20..31,
with no schema change. One real typed mismatch was corrected: damage area
+0 is signed `char` (`tinfo.is_signed() == true`), so DamageArea.damage_level
is now i8; intensity+1 remains u8. Constructor825efe30 and Reset82debf20
both clear the actual seven damage/intensity pairs.

Fresh source review agrees with UIDData82de89b0 (eight UID0/type-1 pairs),
ContextData.Reset82de8b20 (two counts only), Groups82de8c10/82de8c58 (five
groups), Threshold82dea760/82dea7b0 (fifteen value/two-bool entries),
PatternMatches82de97d8 (three flags), BoneSet82df0d98 (25 zero positions and
two -1 damage levels), and EntitySet82ded488 (eight ID0/type-1/score0 entries
and three zero counters). PDB enum values independently confirm difficulty
None3, animation gesture Count35, scorable/entity/damage/body-tweak Invalid-1,
special-wipeout Invalid-1, and grabbable/physics-state/category Invalid0.
Confidence: high source agreement for these constructors and typed schemas;
no native per-tick runtime parity claim.

### SurfacePhysics stage follows the retained output

Fresh raw HiLOD calls are PhysOut82e03f20, Conditioners82e03f58,
SurfacePhysics82e03f90, GrindAirAdjust82e03fc4 and Reckoning82e03ff8.
The selected assembly now mounts the existing concrete SurfacePhysics material
constructor before Reckoning, retaining its partial owner before any tuning read.
The still-omitted conditioners and GrindAirAdjust are not claimed completed.
The earlier reference to Reckoning82e04008 identifies publication, not its call.

The six original surface bindings and five ordered material constructors use
the same Vault-backed provider as before. That exact provider is retained on
assembly construction and moved into the actual board force runtime later;
rebinding would create foreign collection capabilities and break the live
surface-to-force path. The board runtime constructor now accepts that provider.
The old late pair-surface initializer was removed; the original wheel-material
corpus consumes the already mounted surface and its canonical material pointers.

A focused failure discriminator supplies the actual smooth collection key with
a truncated layout. All five source restitution writes survive before the first
friction-layout read fails; no Reckoning/board constructor runs, the retained
output remains complete, and replay is rejected. The prior Reckoning-failure
test also checks that its earlier surface owner remains present. No Cargo was
run by the implementation agent; root owns the new validation gate.
