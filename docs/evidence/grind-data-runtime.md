# Retained GrindData lifecycle and query

Draft `grind_data_runtime.rs` is initially unregistered. Confidence **high** for
ownership/order and corrected primitive arithmetic. Host VMX min/max exceptional
selection and dot3 rounding remain explicit numeric boundaries. Graph agent owns
the separately reviewed native octree. No Cargo/original-corpus gate has run.

Init82cfcfb8, called from World.Initialize827b12a4, publishes an actual empty
implementation on successful allocation. Raw repeated Init merely overwrites
mpImpl without destroying the old allocation. The selected Rust bootstrap now
explicitly refuses repeated initialization with AlreadyInitialized, preserving
all current records/arena owners; it does not claim native Reset semantics or
model abandoned old implementations. Allocation-failure null publication is
outside this selected successful-allocation path. AddSplineData82cfd190 completes each record before appending,
including duplicate GUIDs. Remove82cfb8b8 raw b8e8 LD/b8ec CMPLD compares full64
GUIDs, destroys matches, swaps the final element into place and rechecks it.

GrindSplineSource retains Arc<SimulationNormalArenaPlan> and the actual normal
kind6/static TOC/dictionary locator. It parses original dictionary-backed bytes
and reads the full TOC GUID, never flattened diagnostic GrindAsset primitives.
AssetRecord82cfb640 copies first segment bounds (empty is an explicit missing
source access), unions later bounds in authored order with full4 min/max,
expands maximum by[1,1,1,0], subtracts that vector from minimum, then initializes
the octree. Each authored segment index is inserted before primitive creation.

Primitive82cfb538 uses folded SplineEval_Zero82e6bb30, copying full D including W.
Corrected raw SplineEval_One82e49908:992c177df035 VD59/VA61/VB62 gives C+D;
9930175fe035 VD58/VA63/VB60 gives A+B;993817fad835 gives(A+B)+(C+D).
993c173ff825 permutes the same sum through constant82fd6e90 bytes
00010203 14151617 00010203 00010203, yielding[X,Y,X,X]; the mask2 insert writes
Z, so final W=X. Root independently confirmed this decode. Existing diagnostic
grind.rs grouping differs and remains unchanged pending separate scope/tests.
Vertical classification requires no ordered X/Z absolute displacement greater
than SMALL_FLOAT82319320 bits37800000. Only flag bit31 is initialized.

BoxTest82cf97e0 clears count before the full-vector subtraction/dot3 extent gate.
Only ordered squared extent>37800000 traverses; NaN/equality skip both static
and dynamic. Overlap is absence of ordered XYZ separation, ignoring W. Asset
vector order and actual octree GetNext order are retained; vertical filtering
precedes accepted-count increment. At40 accepted static results no further
GetNext runs, but the installed dynamic provider is still invoked. Lower flag
bits and old result slots survive count reset; accepted copies retain full
endpoints and original header provenance. Source debug enum is unused here.

The dynamic provider is an actual explicit collaborator; absent implementation
must not become native null or empty results. Frame owns the LivingWorld adapter.
Focused draft tests cover original-byte source identity, high GUID differences,
duplicate removal, endpoint cancellation/W, expanded bounds, static cap followed
by dynamic invocation, NaN extent and ordered-only overlap. Root owns the gate.

Root independent review, 2026-09-12: raw AssetRecord82cfb640 confirms first full-lane bounds, ordered min then max union, maximum+expansion before minimum-expansion with W+0, octree insertion before each primitive constructor. Folded GetSeekDirection82e6bb30 copies segment+D full16 bytes. Primitive82cfb538 and SplineEval_One82e49908 agree with retained endpoints/vertical bit. BoxTest82cf97e0 clears count before dot gate, tests ordered separation on XYZ, preserves native octree order, stops at40 accepted static candidates, then calls installed dynamic provider even at40. Add82cfd190 publishes after construction; Remove82cfb8b8 compares full64-bit GUID and rechecks swapped tail. Native allocation-null publication/abandoned repeated Init lifetimes remain outside selected successful host admission. Combined Cargo gate pending.
