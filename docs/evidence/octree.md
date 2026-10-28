# Local collision Octree

This record covers the concrete file-local `rw::collision::Octree` constructed
by `cStreamFile::Update` after XSS processing. It reconstructs resource
planning, initialization, sequential insertion, and sequential removal as
readable native Rust. It does not claim guest ABI layout or expose the
still-unaudited query API.

## Evidence scope

The supplied debug XEX, PDB, and linker map identify the implementation as
`rwccore:BB_runtime.rwccore.cpp.qU2JzdA4RoCoWZVRIITeg.obj` and provide these
anchors:

| Address | Function |
|---:|---|
| `0x82acc0c0..0x82acc118` | `rw::collision::Octree::Node::Init` |
| `0x82acc120..0x82acc188` | `rw::collision::Octree::Node::DecPushCount` |
| `0x82acc190..0x82acc20c` | `rw::collision::Octree::AddEntryToLeaf` |
| `0x82acc210..0x82acc35c` | `rw::collision::Octree::Remove` |
| `0x82ad6738..0x82ad6820` | `rw::collision::Octree::FindChildContainingBBox` |
| `0x82ad6828..0x82ad68d8` | `rw::collision::Octree::BBoxInCorner` |
| `0x82ad68e0..0x82ad6ae8` | `rw::collision::Octree::SplitLeaf` |
| `0x82ad6af0..0x82ad6cc0` | `rw::collision::Octree::Insert` |
| `0x82adbee8..0x82adbf50` | `rw::collision::Octree::GetResourceDescriptor` |
| `0x82adbf58..0x82adc0c0` | private constructor |
| `0x82ae0760..0x82ae0774` | `rw::collision::Octree::Initialize` |

PDB layout is exact. The Octree is `0x40` bytes and holds the AABB at `+0x00`,
maximum entries at `+0x20`, maximum nodes at `+0x24`, free-node head at
`+0x28`, and node, entry, and bbox pointers at `+0x2c/+0x30/+0x34`. A node is
`0x20` bytes, an entry is eight bytes, and an AABB is two 16-byte vectors.
Packed node and entry links reserve `0xffff` as the null value.

## Resource plan and initialization

For capacity `n`, the descriptor creates `(n >> 1) + 1` nodes. The single
nonempty resource has 32-byte alignment and, over the checked valid domain,
size `40*n + 32*floor(n/2) + 96`; the other four descriptor slots have size
zero and alignment one. Storage order is the `0x40`-byte header, `32*n` bbox
bytes, the aligned node array, and `8*n` entry bytes. Tests pin exact plans for
capacities 0, 1, 2, 3, 4, 284, 285, and 65,535.

The constructor preserves the recovered floating expression order. It computes
the four-lane half extent and center, chooses the greatest xyz half extent, and
forms a cube with `[radius,radius,radius,+0]`; the fourth-lane center is
therefore retained. Node zero is initialized as the root. Remaining nodes form
a packed free list through their parent storage, including the native raw-head
edge for capacities zero and one.

## Insertion and subdivision

`insert_next` exposes the proven stream-file contract: compact entries arrive
once in ascending order. It copies all 32 bbox bytes before routing. Root bounds
are inclusive; an outside or child-spanning box is pushed to the current
node's stuck list. Stuck and leaf lists are both LIFO, entry group is zero, and
only xyz select the child bits (x=0, y=1, z=2).

Each node uses 20-percent overlapping children. The code retains the native
`size * -0.4`, lower, and upper operation order. A child is rejected when a bbox
spans both overlap boundaries on any xyz axis. Otherwise the high side wins
only for a strict larger delta, so a tie selects low. `BBoxInCorner` also keeps
its strict comparisons. Corner-eligible insertions saturate the per-child byte
counter at 255.

The fourth corner-eligible push splits a leaf when a free node remains. The
implementation consumes the free-list head, converts the parent child from a
leaf reference to a node reference, then redistributes the former newest-to-
oldest list. Corner entries are reclassified into the new node's children;
non-corner entries become stuck. Destination push-front insertion produces the
reference reversal, and redistribution never recursively splits. Exhausting
the node pool suppresses splitting without failing the insertion.

## Sequential removal

The state-5 `cStreamFile::Update` loop at `0x824e1420..0x824e1438` calls
`Remove` for the compact accepted indices in ascending order. `remove_next`
exposes that proven monotonic contract. It selects the list recorded by the
entry: child `0xff` uses the node's stuck head, while child values zero through
seven use the corresponding leaf head. Head, middle, and tail removal update
only the selected head or predecessor link. The removed entry's eight bytes
and its separate 32-byte bbox remain untouched.

For a corner entry, `Node::DecPushCount` runs before unlinking. A count below
`0xff` is decremented directly. At `0xff`, the routine instead recounts corner
entries in the still-linked child list and caps at 255. This retains the unusual
but exact valid-domain sequence: removing from 255 live corner entries stores
255 while 254 remain; the following removal recounts 254 before unlinking and
stores 254 while 253 remain. It is deliberately not replaced with a generic
saturating decrement.

After unlink, only the current non-root node can collapse. Its stuck list must
be empty and every child must be a leaf with an empty list. The parent slot is
then reset to an empty leaf with push count zero, and the released node is
pushed onto the packed free list. The reference does not recursively inspect a
parent made empty by this collapse. Other bytes in the freed node are stale in
the console allocation; the typed Rust `NodeSlot::Free` intentionally exposes
only the reused next link.

## Safe native boundary and confidence

Confidence is **exact** for symbols, addresses, type fields, descriptor sizes
and alignments, packed sentinels, valid-capacity node/free-list construction,
branch comparisons, child numbering, insertion/list order, saturation, split
timing, redistribution, node-exhaustion behavior, removal-list selection and
unlink order, pre-unlink saturated recount, single-level collapse predicates,
and free-list insertion.

Confidence is **high** for ordinary finite host `f32` results because the
recovered expression order is preserved. NaNs, infinities, signed zero,
denormals, and exact PPC/VMX exception behavior remain **unknown** and are not
claimed bit-identical.

Rust uses initialized typed vectors rather than pretending to provide an
ABI-identical single allocation. Capacity is checked at 65,535 entries and a
full sequential tree returns a typed error. Once successful removal begins,
later insertion is rejected. Removal validates the selected list, counts,
parent linkage, and free-list head before making any logical change. These are
high-confidence safe deviations: the reference accepts arbitrary indices and
malformed internal links without checks and can underflow, loop, read out of
bounds, or corrupt memory. Allocator/null continuation, arbitrary indexed
insertion/removal, the concrete AssMan host-allocation topology, and all spatial
queries remain outside this slice. The pre-free call at `0x824e1440` resolves
to `rw::collision::Octree::Release` at `0x825187c0`; that ICF-folded function is
exactly one `blr` and has no observable behavior before allocator release.

Thirty-four focused tests cover the descriptor snapshots, cube/fourth-lane
normalization, root/free-list edges, all children and strict ties, strict corner
boundaries, inclusive root handling, list order, fourth-push splitting,
redistribution, exhausted-node suppression, actual insertion-path saturation,
and nonmutating capacity failure. Removal coverage includes leaf and stuck
head/middle/tail unlink, unchanged entry/bbox storage, ordinary and saturated
count behavior, root/non-root and two-level collapse cases, monotonic phase
enforcement, and atomic rejection of malformed lists, counts, parent links,
and free heads. The saturated recount is exercised with both exactly 255 and
more than 255 live corners, and a nonempty sibling leaf independently blocks
collapse. A deterministic runtime differential trace remains required before
claiming exceptional-state or machine-level parity.
