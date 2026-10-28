# Post-wipeout head/spine orientation suffix

2026-09-19, confidence **high** for source graph; bounded unregistered draft,
no Cargo/runtime/native trace claim. Root registers only after independent review.

Files physical_player_post_wipeout_head_orientation.rs and _tests.rs were
checked absent before creation. Main file is a child of SkeletonNormalAssembly.
Root must expose existing skeleton_ik_state::external::normal_rotation as
pub(crate) without changing its arithmetic. No registered modules edited here.

Actual caller82b578b8 always invokes HeadTracking82dca850 at57c78 after IK post
update. Inputs are same constructor.head, captured actual global pose Head
named_bones[6], and root WorldToAnim. This draft starts AFTER successful update.
It accepts actual axisAngleOut and captured head; never fabricates either.

Gate57c9c/57cfc/57d0c is dot3(axis,axis)>VECFLOAT_SMALL, exact37800000 from
SMALL_FLOAT_82@821f0c20 and initializer82f76340. Equal/zero/unordered skip all
fractional reads/estimates/writes and proceed to caller's fresh pose cvar.
Active path rsqrt estimate57d10 precedes head target capture57d4c. Same target
value is read scalar57d58 for remainder=1-h then vector57d68 for h. Two existing
rsqrt refinements and squared*inverse with zero select yield length; use
length_from_squared_and_estimate, not another dot/try_length. Reciprocal estimate
57da8 precedes h*length and remainder*length; two reciprocal refinements then
scale axis XYZ. Inner axis W does not participate in the rotation construction.

SinCos calls in order57dd8(h*length),57e8c(remainder*length),57f6c(spine*remaining).
Spine target/value captured/read57ecc/57ee8 after SECOND SinCos. Exact existing
normal_rotation graph matches all three fresh decoded VMX128 blocks: columns
Rodrigues XYZ with W equal to each column X, translation0; no canonical affine W.
Corrected VMX128 operands rather than IDA's aliased high register display were
used. First rotation updates captured head basis only, preserving captured P.

AdjustPhysicsBoneRecordOrientation82b3fbe8 is invoked first for head_parent_part
at580ac then head_grandparent_part580bc. Each transforms original record basis
by direction(WorldToAnim), direction(rotation), direction(AnimToWorld), three
separate X-product/Y-FMA/Z-FMA graphs. Original record P is copied unchanged.
Checked PhysRecord.SetTransform82b3a800 calledfd70 follows all local work; no
Part, cached-position, velocity or COM write. First record survives second
failure. Do not combine matrices or feed translation through full composition.

After both records, actual head global I/J/K/P stores580e8..58100. Then fresh
same-owner parent global (animation head_parent, not physical parent mapping),
fresh just-published head global, rigid inverse(parent).compose(head), and head
local stores581dc..581f0 (K/J/I/P). Typed pose setter publishes one complete
matrix under exclusive borrow; no fallible/native external call separates the
column stores. Existing init fields source Head3e1c,parent-bone3e20,parent-Part
3e24,grandparent-Part3e28; no guessed fixed indices. Missing/invalid indices
stop only at their reached adapter boundary, preserving prior writes.

Returned HeadOrientationBeforePose authorizes fresh kUpdatePoseBuffer read at
581fc..58200, after skip OR completed orientation. True dispatches existing
complete pose method58210. Token is not a frame-completion claim. Pool caller
must retain SAME constructor bindings.animation_input and verify world pair.

## Reserved live cvars

All callbackNULL. 222: kUpdatePoseBuffer, title Update Pose Buffer, section
Physics_Skeleton, defaulttrue; ref8308e6c4, init82f7c5a8, ctorcallc5d4,
table82fd4550. Strings section82262a1c,label82262a08,title822629f4.

223: kFractionalHeadAdjust, title FractionalHeadAdjust, section
Physics_Skeleton_HeadAdjust, default3e4ccccd, range0..1; ref8308e92c,
init82f7c788, ctorcallc7c8, table82fd4568. Strings section82262ce0,
label82262cc8,title82262cb0.

224: kFractionalSpineAdjust, title FractionalSpineAdjust, same section,
default3f19999a, range0..1; ref8308eccc,init82f7c7e8,ctorcallc828,
table82fd456c. Strings section82262d2c,label82262d14,title82262cfc.

Native table82fd4540..4570: DrawContactInfo,DrawCentreOfMass,
DrawCentreOfMassAnim,PSK_DrawHookToHips,UpdatePoseBuffer,
AddSkateboardErrorToSkeleton,ShowDangerZone,SkeletonDisplayTrajectoryNode,
WipeoutOnCollision,ShowCollisionSkeleton,FractionalHeadAdjust,
FractionalSpineAdjust,ZeroTrajectoryTransform. UI section ordering is separate.

Tests exercise same production orient helper: threshold/equality/NaN lazy skip;
distinct two live head reads and exact service sequence; independent geometric
oracles for three fractions and noncanonical W; preserved captured translation;
SinCos and second-record/global/local failure prefixes; orientation ignores
frame translations and preserves unknown output W. Generic output collaborator
tests are distinct from real retained owner/corpus proof. Dot/estimates retain
existing host numeric boundary, not claimed bit-identical to Xenon.
