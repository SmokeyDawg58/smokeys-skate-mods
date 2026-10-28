# Native grind-data octree construction and ordered query

Confidence: **high**. Root independently reviewed and accepted constructor,
FindChild, Corner, AddLeaf, Insert, SplitLeaf, query construction, ProcessNode,
both GetNext stages and XYZ overlap against fresh raw instructions. Original
resident-asset integration remains under the combined gate. The implementation
is new `grind_data_octree.rs`, initially
unregistered. It does not replace the older diagnostic linear grind query.

## Source and scope

`GrindData::Implementation::AssetRecord82cfb640` computes the asset extent,
expands XYZ by one, initializes the octree for the authored segment count,
then inserts each segment's original full-vector AABB in authored order before
constructing its corresponding grind primitive. `BoxTest82cf97e0` constructs a
query for every overlapping resident asset and consumes it lazily until forty
accepted nonfiltered primitives. This module covers the octree dependency;
asset retention, primitive construction, result copying, and the dynamic
provider belong to the retained GrindData owner.

| Source operation | Address |
|---|---|
| Resource descriptor / Initialize / constructor | 82adbee8 / 82ae0760 / 82adbf58 |
| Node Init | 82acc0c0 |
| FindChildContainingBBox / BBoxInCorner | 82ad6738 / 82ad6828 |
| Insert / AddEntryToLeaf / SplitLeaf | 82ad6af0 / 82acc190 / 82ad68e0 |
| BBoxQuery constructor / ProcessNode | 82adc0c8 / 82ad6cc8 |
| GetNextInIntersectedNodes / GetNext | 824d65c0 / 824d6af8 |

PDB: Node is32 bytes (parent0, childOfParent2, stuckEntries4, childTypes6,
eight childRefs8, eight pushCounts24). Entry is8 bytes (group0, next2, node4,
child6, corner7). Query has211 48-byte stack elements at48, top10176, nine
u16 result heads10180, current result10200 and nextEntry10204. The selected
Rust owner retains fields consumed by build/query; unused removal/group
metadata is not a claim of complete RenderWare Octree support.

## Corrected vector arithmetic and constants

IDA's VMX128 source-operand rendering adds misleading register aliases. Raw
operand decoding uses VA low5 at16 plus instruction bit5 for register bit5 and
instruction bit10 for register bit6. For example `177cf075` at82ad675c reads
actual v60 minus v62, despite the displayed v92 operand. The formulas below
follow decoded raw operands, not those pseudocode names.

Static initializer82f7a710 writes inbox XYZ `becccccd` (-0.4), W positive zero.
82f7a748 writes axis bits `[1,2,4,0]`; 82f7a780 writes low masks
`[aa,cc,f0,ff]`; 82f7a7b8 writes high masks `[55,33,0f,ff]`; 82f7a7f0
writes all masks ff. 82f7a820 writes eight child masks: each XYZ lane is all
ones iff that axis bit occurs in the child index; W is always zero. These are
source static-initializer constants, not cvars or configurable geometry.

For every lane, in the source separate operation order:

```
offset = (node.max - node.min) * inbox
high_min = node.min - offset
low_max = node.max + offset
```

For an ordinary finite extent this produces overlapping children with the
two inner boundaries at40% and60%. FindChild separately computes
`lower=bbox.min-high_min` and `upper=low_max-bbox.max`. If any XYZ lane has
both `0 >= lower` and `0 >= upper`, it returns255. Otherwise each high-child
bit is selected by strict `lower > upper`; equal and unordered comparisons
select the low child. It does not compute an algebraically substituted center.
The child's high sides use high_min/parent.max and its low sides use
parent.min/low_max, preserving the source W mask.

Corner status requires each XYZ lane to satisfy strict
`low_max > bbox.max || bbox.min > high_min`. Query child masks start atff;
`query.min > low_max` applies the low mask, and `high_min > query.max`
applies the high mask. Each separating-axis test rejects only an ordered
greater comparison, so unordered lanes do not become an invented rejection.
W is retained and participates in vector arithmetic, but never enters XYZ
containment, corner, child-selection or overlap reductions.

Constructor82adbf58 computes `(max-min)*.5` and `(max+min)*.5`, then selects
the cube radius with two explicit rounded `fsubs`/`fsel` operations, first XY,
then Z. An unordered condition selects the second operand. The radius vector
has XYZ equal and W positive zero; the final center-minus/plus-radius stores
retain all four lanes, including signed zeros.

## Ownership, ordering and checked boundaries

The constructor allocates maxEntries boxes/entries and
`(maxEntries >> 1)+1` node slots. Only root node0 is initialized; remaining
slots retain their source free-list links. It installs free-list head1 even
for a degenerate one-node allocation. The Rust implementation does not repair
that value: a later actual request for an unavailable slot refuses explicitly.
Entry index65535 is the native sentinel; the selected source producer's usable
index range is checked rather than silently truncated.

Insert stores the full AABB first. Out-of-extent or child255 entries prepend
to the current node's stuck chain. Leaf entries prepend to that child's chain;
only corner entries increment the saturating byte push count. A count greater
than3 with a nonempty node free list causes one SplitLeaf. Splitting removes
the free-list head, initializes that node, converts the parent's leaf reference
to a node reference, then walks the old chain in order. Each entry is prepended
to its new leaf/stuck chain. It does not recursively split during redistribution.

Query processing pops a node from its LIFO stack, appends its stuck head first,
then visits child indices0..7. Leaf heads append to the local result array;
node references push onto the stack. Local heads are consumed backwards and
each entire next-entry chain is consumed before the next head. All local
heads finish before any queued child node. Finally GetNext performs the exact
stored-AABB overlap and continues after rejected entries. The iterator returns
one accepted index at a time, so the caller can stop at its native forty-result
boundary without scanning later entries or nodes.

This proves the diagnostic authored-order scan differs even below forty
results, where candidate sets can agree while downstream tie order differs.
Checked allocation/index/stack failures preserve already completed stores.
Unused source storage is represented as absent, not initialized geometry.
No complete octree mutation/removal API or portable native allocator parity is
claimed.

## Validation

Eight focused tests passed using standalone `rustc --edition 2024 --test`
against this dependency-free module. No Cargo run or shared module registration
was performed by the implementation agent. Tests distinguish overlapping
children and exact boundary comparisons; reversed heads and stuck ordering;
fourth-corner splitting and redistribution reversal; leaf results before child
nodes and LIFO child traversal; NaN admission and retained W bits; constructor
FSEL/signed-zero behavior; checked index/duplicate failures; and an explicit
rounding case where the native two-subtraction child selection chooses high
while an algebraic center comparison incorrectly ties and chooses low. Original
resident-data traversal signatures and the root's combined validation remain
required before claiming the integrated query boundary complete.
