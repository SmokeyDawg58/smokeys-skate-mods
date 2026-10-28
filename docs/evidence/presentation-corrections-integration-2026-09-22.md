# Retained replay root and twist correction integration

Confidence high for source ordering and same-allocation mutation; original-data
execution and numeric validation are pending root Cargo. This packet reuses the
existing recovered MoveRoot, inverse, TwistThatBone and Expressions drafts.
No second pose, approximate replacement math, or renderer publication is added.

## Production chain

The existing selected frame probe now invokes `PresentationAnimation::try_correct_replay`
after actual WriteData/retained gather/ReadSkater/Extract/Sync. The method executes
all `MoveRootToHips827d3a10` individuals, then all `ComputeExpressions827d9db0`
individuals, matching `UpdateBodies827deb24/deb2c`. It does not add model gating
inside these loops. Zero count requires no indices, bind or attribute owner.

`PresentationAnimation` retains nine optional hips matrices, initially unknown.
The already allocated even replay and authored bind buffers are borrowed through
the existing `SqtChannelEdit`; the added scale setter uses its existing write
path. First mutation invalidates successful publication, late failures retain
stores, and successful completion publishes the generation. No read/modify/write
copy of the full pose is used. The old slice fixture entrypoints call the SAME
arithmetic through test-only channel access, retaining all existing tests.

Native signed GetBone results remain raw until checked indexed use. Projecting
-1 to its unsigned address-index bits does not cause an eager error: invalid
board access follows hips publication/reset; later invalid twist accesses follow
four ratio reads and prior corrections. Unknown constructor indices remain
out-of-range and are not defaulted to a valid bone.

## Native store/read sequence

MoveRoot_Individual827d35d8 publishes hips matrix3690..3708, then hips rotation370c,
scale3710, translation3714. It samples board only afterward, evaluates the existing
full inverse82600f38, writes board scale3820, then quaternion39f4 and translation39f8.
Scale is never used in either source rigid-matrix construction. Same-index alias
continues to observe earlier writes. Existing numerical graph is unchanged.

TwistThatBone827d9540 reads bind source955c, norm9564, current source9570. It retains
both reciprocal/root refinements and separate existing atan/sine/cosine helpers.
Intermediate destination rotation9950 precedes bind-destination read9954 and final
composition/store9984. Channel write errors remain typed owner errors.

ComputeExpressions_Individual827d99a0 reads four actual live ratios in order:
38317705e5a4571a,246fc198b6f1624d,5cdb40ca8c5a5044,3eb708886e6e4607; eight calls are
left arm/forearm1/forearm2/upper-leg, then corresponding right side. Missing global
is an error; missing attribute pointer uses actual DefaultDataArea(4).

The existing PresentationTwistAttributes generated binding is retained once on
the selected presentation startup, from the SAME player services Vault. It is the
selected GlobalAttributeCollection+d8 capability (ctor828b27cc..2824 class
babb3ec110849e08, collection2db7709645a19ce8), not reconstruction of the entire
global singleton lifecycle. Each individual uses that retained instance for four
fresh reads; ratios are never cached in the pose owner.

## Numerical limits and reached-input check

Existing host dot4 delegates to the explicit rounded-product/left-associated
profile used by ReadSkater. Existing host estimates/conversions and separate
trigonometry remain unchanged; their Xenon bit parity is not newly claimed.
The optional no-op numeric observer records actual transverse-length operands
before reciprocal970c, with source/destination indices. The corpus collects all
eight, prints bits even when a later service fails, and checks actual corrected
channels for finiteness. See skater-presentation-twist-expressions.md for the
independent zero-length audit. No zero ratio/zero length skip, finite seed clamp,
identity repair or bind-pose substitution is introduced.

## Tests and next boundary

All previous inverse/root/twist/expression tests continue through shared math.
Additional tests cover actual allocation invalidation after a late missing board,
retained hips matrix, same-allocation scale/channel alias reads, signed board=-1
failure timing, zero-count no dependencies, and missing later twist index keeping
four reads plus the first correction. Author checks rustfmt and patch application;
root owns Cargo and mounted runtime results.

After successful original-data validation, the next producer remains actual
model admission/frame mapping and inverse-bind SetMatrix/PoseWithMatMult queue
integration. Existing interpolated matrix allocations and CharacterScene palette
remapping must be reused. This packet does not publish a complete render palette.
