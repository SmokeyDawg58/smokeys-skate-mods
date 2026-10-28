# Independent remaining constructor review

2026-09-12, grounding agent independently reviewed input agent's
`physical_player_remaining_constructor_owners.rs` and root's
`physical_player_anim_out.rs` against fresh IDA decompilation, critical raw
instructions and PDB field layouts. No blocking mismatch found in the
constructor-only fields reviewed. No authored implementation was modified.
This is not evidence that offboard queries/updates or the complete HiLOD
constructor caller have executed.

## Source caller order and provenance

After SkateboardController constructor82e0437c and publication82e0438c:

| Owner | Allocate / bytes | Constructor call | Publication |
| --- | --- | --- | --- |
| OffBoardFeetIK | 82e043a0 / 0x140 | 82e043b4 | 82e043c4, HiLOD+2dc |
| LandingOnDeckManager | 82e043d8 / 0xe0 | 82e043f0 | 82e04400, +2e0 |
| OffboardLocationTest | 82e04414 / 0x50 | 82e04424 | 82e04434, +2e4 |
| OffboardJumpingTest | 82e04448 / 0x70 | 82e04458 | 82e04468, +2e8 |

SkeletonController allocation follows82e0447c. Each of the four allocates
PhysicsSkater with flags1, calls its constructor only on nonnull and then
publishes that return or null. Raw category row40 at82ff0d48:
`821677d4000199990000000800000000ffffffff00000000`, ORflags0.
Thus category allocation alone cannot establish zero payload.

## Four constructors

OffBoardFeetIK82dcd048 first initializes both SurfaceQueryResult objects,
then stores actual ProcessedPhysIn and Skeleton, then calls Reset82dc8728.
PDB size320 confirms ten vector fields at0..90, results+a0/+d0, scalar and
counter fields+100..128, eight boolean fields+12c..133 and bindings+134/+138.
Raw Reset initializes all vector W lanes by integer-zero stores. Both
ground normals at80/90 are [0,1,0,0]; all other named vectors and both
result position/normal vectors are [0;4]. Source li2 at82dc88ec and store
82dc88fc establish FEETIK_STATE_NO_FOOT_FIXED=2. Seven floats, three counters,
eight flags, both surface IDs and valid flags are zero. Rust final values
and actual owner/Processed/Skeleton identity bindings match.

LandingOnDeckManager82dce3a0 size224 first finds and constructs its retained
attribute instance. Raw pair construction82dce3ac..3dc proves class
2377cef0ff59c4d2 and collection d7edbd362d7d2152 despite the decompiler's
incorrect 64-bit argument grouping. It stores Processed, Skeleton, IWorld,
null trajectory batch and constructs both trajectories with three zero
vectors and [-1;4] time. Raw42c/43c loads after explicit -1 stores confirm
both time vectors. Reset82dc8c80 resets the ordinary trajectory again,
but preserves the ideal trajectory. It zeros trajectory time, landing
error/velocity, the three landing-time/error scalars and all five result
flags. The Rust constructor's two `RetainedTrajectory::constructed()` values
and remaining fields match; no later Reset semantics are claimed by this
constructor-only module.

OffboardLocationTest82dd1f08 size80 initializes ground position, look
direction and jump velocity to four-lane zero. Raw1f28..1f64 includes each
W integer-zero store, so zero W is not inferred from category memory.
Jumping/collide-only/sync flags and line/trajectory batches are zero/null;
IWorld binding is actual. No query is launched.

OffboardJumpingTest82dd4fd8 size112 likewise initializes input ground
position/direction and result landing position in all four lanes through
raw4ff8..5034. Jump distance, two result booleans and valid counter are0;
one line batch and two trajectory batches are null, and IWorld is retained.
The typed host owner/world bindings are lifetime/identity guards, not
invented query results or native pointer addresses.

## Physical AnimOut

This is distinct from PhysOut and Actor AnimOutPhysIn. Source allocates24
PhysicsSkater bytes at82e040ec, calls ctor82e040f8 and publishes HiLOD+2b8
at82e04104, after Pumping and before Skateboard allocation. Current root
mounting location follows that source order.

Constructor82e03be8 initializes the FastString30 member then calls
Reset82b3a1a0. Reset copies all five packed words from Andale::nullStringName
at82b3a1b4..1d4, then loads flags+14 and performs clrlwi5 at82b3a1dc,
preserving low27 bits. PDB size24 confirms five high named flags and an
unnamed low27-bit field. PhysicsSkater OR0 means those low27 bits remain
unknown in a fresh constructor. Rust known_mask=f8000000/bits0 is correct;
Reset preserves already-known lower bits and does not make unknown ones known.
The source nullStringName is literal `nullString`, established by initializer
82fabfd0, not an empty packed name. The reused constant has that meaning.

The helper/corpus presence and constructor field checks do not establish
full actor publication, prediction execution, query hits, or offboard physics.
Root owns caller attachment, independent full-constructor admission and gates.
