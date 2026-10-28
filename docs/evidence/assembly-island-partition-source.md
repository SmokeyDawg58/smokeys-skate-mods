# Assembly-pair partition and table source packet

2026-09-07, read-only live IDA/PDB recovery. NOT yet implemented. Confidence:
exact for valid-domain scalar control/layout and table ordering; high for typed
mapping. Native concurrent arrival order and malformed memory remain unknown.

## Partition is recycled-label merging

SimulationManager::PartitionAssembliesInIslands (`0x82e0e2d0..0x82e0e5a8`)
creates a fresh local map keyed by native live Assembly pointers, not the
manager's mAssemblyIslandMap member. Consume the pair buffer in append order:

| Existing endpoints | Action |
|---|---|
| Neither | Assign both next; increment next; reset to zero only when equal to sign-extended mMaxIslands |
| Only A | Assign B A's label |
| Only B | Assign A B's label |
| Both, unequal labels | Replace EVERY map value equal to B's label with A's label |
| Both, equal | No change |

next is u32, cap is signed i8. An unseen self-pair assigns its one key and
advances once. Offline cap is 10. Recycled bins are observable: with cap2,
(A,B)->0, (C,D)->1, (E,F)->0, (D,E) changes ALL zero labels to1, including
disconnected A/B. Ordinary connected-components or union-find numbering is
not equivalent.

Afterward traverse Assembly pointer keys ascending, each Part ascending, and
write the label's low byte into each Part body slot. No state filtering; bodies
absent from the map retain their bytes. Live assembly instances need occurrence
identity; shared asset ObjectId is insufficient. Native pointer order affects
alias-last-writes unless distinct body ownership is established. Do not use
arbitrary host allocation addresses as recovered source ordering.

Layouts: manager maxIslands+54, map+58, pair pointer/counter/capacity+60/+64/+68;
pair stride16, pointers+0/+4; Assembly part list+8, count+1c; Part stride60,
body+4c; body strideb0; serialized record stride1780, live Assembly pointer in
Header+1c (not the host asset ObjectId).

## Pair provenance precedes Contacts

AssemblyAssemblyCollisionPipeline::Run `0x82e0bef8` passes shared manager
buffer/counter/capacity to jobs. Actual World pair capacity is500.
AssemblyAssemblyCollisionJob `0x827cc410..0x827ccb68` appends eligible distinct
pairs at `0x827cc7b4..824` BEFORE intersection `0x827cc94c`; self-pairs append
at `0x827cca04..4c` BEFORE `0x827cca98`. Thus zero-Contact eligible pairs still
participate in partition. Do not derive pairs from emitted Contacts.

Eligibility includes AABB overlap, frozen/static exclusions and optional global
group-culling bits; intra-pool iteration excludes reversed serialized pairs.
Self processing has a separate nonnull self-culling branch. Endpoint order is
ascending NATIVE live Assembly pointer, affecting merge orientation. A concrete
producer still needs these full gates and source-owned identities; this packet
is not a completed collision-job implementation.

One-slot append uses AtomicIncrement, index=result-1, then an EQUALITY check of
remaining capacity against zero. At exact capacity it restores the capacity
counter; otherwise it copies the16byte pair. It is not Contact's variable-size
Store algorithm. Valid serial appends saturate; arbitrary concurrent overshoot
is not established safe. Parallel arrival order is not deterministic pool order;
serial host scheduling must be documented, not called exact native ordering.

## Offline chronology and consumer

StartFrame resets Active labels0/static-1. Collision jobs produce pairs.
Step_Collision3 `0x827b4060` writes cap at `0x827b40b8`: flags3->10, otherwise
bit0->8, otherwise1. Step_Solver2 `0x827b35b8` selects single-player setup for
flags3, normalizes drives, optionally thaws, partitions, then calls
CreateSimulationIslandTables(simulation,0) at `0x827b36a8`.

SetupSimulationIslandsForSinglePlayer `0x827b2880` snapshots HookPool count,
visits pool/Part order and sets every body -1 via SetAssemblyIsland
`0x82e0aff8`, without state filtering. InitWorld `0x827b42c0` owns HookPool
but supplies only Skater/Dmo/Pedestrian/Vehicle pools to collision pipelines.
Hook bypass requires actual membership, not an inactive-state guess.

CreateSimulationIslandTables `0x82ae3d68..0x82ae4358` assumes reset counts:

- Bodies: Active right-link order, append only signed label>=0.
- Contacts: dense global order, choose A unless exactly-1 then B; Contact and
  Jacobian pointers keep original global ordinal, stride100.
- Joints/drives: active right-link order, require (stateA|stateB)&4!=0, read B
  then A labels and choose B unless exactly-1 then A. Admitted Jacobians get
  dense global ordinals, stride180. Each island is a stable source subsequence.

Null native map means label0. No merge occurs here. Compute prefix bases for
all10 islands. Offline argument0 uses original Contact-Jacobian stack and no
final swap; nonzero selects the separate stack and swaps pointers afterward.
Invalid labels/capacities are unchecked natively; Rust rejection is host safety.
Reuse existing Contact/Joint/Drive consumers rather than duplicating them.

Live byte SHA256: Partition
`b5996a8144a6348bda1d4af37a38a6495e14229afbc0c6cef877f762e592d93e`;
Create tables `6b26ef1063c1d8e6612f59ef2420853ee71e084278e371811c612cc8c642fbcf`;
Assembly collision job
`7f4a578bd6686af6ef4e283cc6b596d1119f4bcf521fc4e8a3352bcdfad793bc`.
