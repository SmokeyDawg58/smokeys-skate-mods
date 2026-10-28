# SkateboardBody retained ground normal

2026-09-08. Independent source audit. No runtime edits or
Cargo execution. Raw IDA instructions and PDB layouts were read afresh.

## Finding and ownership

`Sk8::Physics::SkateboardBody::CollisionInfo` starts at Body+0x40. PDB fields
`averageGroundNormal` and `lastValidGroundNormal` are respectively +0/+0x10
within it, hence Body+0x40/+0x50. These are separate from outer Skateboard+c0
jumpNormal and the Reckoning normal. Body+0x50 persists when no qualifying
wheel sum is produced. Copying the latest contact winner, deck/truck fallback,
or Reckoning normal into it would change source behavior.

Constructor `??0SkateboardBody@Physics@Sk8@@...` at 0x82b38270 passes Body+0x40
to `CollisionInfo::Reset` at 382dc. Reset 82b23630 loads gJVector and stores
average at 236a4, then last-valid at 236ac. Actual bytes at 8227e900 are
`00000000 3f800000 00000000 00000000`. Both start full [0,1,0,0].
`TeleportReset82b2e7f0` calls the same Reset at 2e828. Ordinary postphysics
does not reset last-valid.

## Contact prefix and line dependency

`SkateboardBody::UpdatePostPhysics82b36a58` is called by outer Skateboard at
82b37e5c. Outer code reads its retained Reckoning+1b0 at 37e3c and passes that
up vector, and reads dt from processed+93c at 37e40. The callee asserts dt>0,
activates Assembly, calculates wheel compressions, then gets its actual
Island contact vector at 36ad0. This is after real contact export, not a
Skeleton collision-normal producer.

Reuse the admitted PhysicalContactInfo list and the source winner ordering
already established in `skateboard_contact_surface.rs`; extend that same
producer rather than perform a competing contact traversal. Additional state
needed here is the full selected normal, four valid-normal flags, and the
best-Y part index. Each contact retains its existing endpoint-oriented full
normal. No normalization occurs during selection.

Native initial loop clears seven inContact bytes at Body+34c..352. It also
stores one at Body+2f0+i at 36b14 for **seven** iterations. PDB calls
contactNormalValid a bool[4]; the last three native stores overlap the first
physicsSurfaceType word. Preserve this layout discrepancy as raw evidence;
do not silently infer a seven-element native validity array. Only four flags
are read by the later wheel accumulator.

For non-null other Body, source reads other owner then this owner and skips
same Entity at 36cac. Other Assembly group 15 or 16 clears the selected part's
validity byte at 36d94 (and sets the light-object collision flag). Group14
sets a sibling DMO flag but does not clear validity. Group4 executes separate
deck-end geometry/cvar work before selection. These earlier operations and
their possible failures cannot be claimed implemented by a surface-only
projection.

First contact per part wins; later replacement requires ordered current Y >
selected Y. Equal and unordered retain earlier. Full normal stores 36dd4;
other winner fields follow; inContact stores 36e68. Independently, after each
accepted contact, compare the retained selected normal Y to a running best Y
initialized -2.0. Ordered greater updates best part at 36e5c and best Y at
36e64. Thus this is traversal-order tie breaking across all seven parts, not
an ascending part scan after the traversal. Best part starts -1.

After contacts, wheel indices 0..3 execute 36ee4..36f24. If inContact is true,
skip the line fallback. Otherwise capture live LineDistToUseNormal target
(ref8308d8ac, target+28), read wheelToGroundDist at Body+2e0+i*4, then target
float+D8. Raw branch 36f00 is `40980010`, BO4 BI24: LT-not-set, including
unordered, clears validity. Only ordered distance < threshold copies the
full lineTestNormal from Body+2a0+i*16 to contactNormal Body+60+i*16. It does
not set inContact. Target/value are sampled separately for each missing wheel.

Those line fields come from `EndWheelLineTest82b24700`, not the contact list.
It Syncs the actual retained batch, initializes each normal [0,1,0,0] and
distance +0 before GetResult(i), then on a hit stores full result normal and
distance = lineParam * 0.2. Four wheels are followed by deck result index4,
then Release and nulling batch. StartWheelLineTest is 82b24530. This audit
does not certify either wheel-line stage integrated. A legitimate source miss
therefore has initialized normal/distance semantics; do not fabricate the
batch or manufacture those outputs as current without its producer.

## Wheel accumulation and last-valid commit

Source executes other postphysics stages, including seven velocity/acceleration
updates and grip flags, then ChoosePhysicsSurfaceType at 3709c. It counts
parts and publishes numPartsInContact at 37130 before normal accumulation.
The following graph must remain after these stages in a complete caller.

1. Start full sum [0,0,0,0], wheel-contact count0. Read Body+2098 bit0x80.
   If set, capture/read live kFreeSkateboardWheelInertia at 37148..54;
   otherwise contact-wheel angular drag input is +0.
2. Capture global physics_reckoning_default (+e4). Lookup full attribute key
   `0e508708cc7f5249`, index0 at 371a8; missing pointer uses DefaultDataArea(4).
   Read first scalar at371cc, multiply attribute * degrees-to-radians, then
   call XMVectorCos at371ec. Constant at8225c454 is bits3c8efa35. This yields
   the up-vector acceptance cosine; do not replace live tuning by a literal.
3. For wheels0..3, count each inContact wheel regardless of normal validity.
   For valid normals only, read passed up, read retained full wheel normal,
   dot(wheelNormal,up), and require ordered dot > cosine (37258 GT-not-set
   rejects unordered). Then call unsigned AngleBetweenVectors with best-part
   normal first and current wheel normal second at37270. Require ordered
   quarterPi > angle; exact constant8225c468 bits3f490fdb. Accepted normals
   accumulate full-lane `sum + normal` in wheel order at372b4. Equal and NaN
   angle are rejected. These are equal weights: no averaging coefficient.
4. The best-part -1 sentinel deliberately addresses Body+50 via
   `Body + 16*(index+6)` at3725c..6c. It therefore uses lastValidGroundNormal
   when no accepted contact beat -2; it is not an invalid index to reject.
5. Every wheel also updates its actual body's inertia angular drag after its
   normal work: non-contact input bits3bc49ba6 (~.006), contact input from
   step1, multiplied by bits426fffff (~60), then mAngularDrag store372e0.
   These writes interleave the four wheel numeric graphs. Their authority and
   failure prefix are required for a complete implementation, not optional
   tuning effects that can be moved after normal publication.
6. Publish numWheelsInContact Body+361 at372ec. Only if count>0 compute dot3
   sum squared at37304 and require ordered squared > bits37800000 (actual
   constant8225c450, 2^-16). If accepted, rsqrt estimate then two existing
   Newton refinements, then full sum * refined reciprocal length. Store
   average Body+40 at37378, then last-valid Body+50 at3737c. Failure before
   these stores preserves both previous values. No extra post-normalization.

Use existing `reckoning_numeric::try_unsigned_angle` for the two-vector
angle overload (829b0528), with its existing documented exceptional behavior
boundary. Reuse the established rsqrt refinement graph, typed vector W, and
actual per-Part mutable inertia owner. XMVectorCos still needs an explicit
source numeric capability; host cosine is not automatically source parity.

## Fallback changes average only

If count is zero or sum squared fails its strict gate, 37384 begins full
zero sum. Add selected contact normals only when inContact, in exact order
deck6, front truck4, rear truck5. No validity or angle gates here. Compute
length through dot, rsqrt estimate, two refinements, squared*refined-rsqrt,
then select exact zero when squared==0. Estimate still executes for zero.
Compare length to bits3c23d70a (.01) at37434/38: only ordered greater enters
reciprocal estimate plus two refinements, then scalar-left reciprocal*sum.
Otherwise, including unordered, write full gJVector. Both branches store
only average Body+40 (37480 or37498). **Neither writes Body+50.**

Afterward time-since-wheels-touching is reset to zero if wheel count nonzero,
else incremented by dt; truck dynamics/frames follow. This packet does not
claim those siblings reconstructed.

## Minimal integration boundary and remaining proof

Retain constructor/reset-backed last-valid and average on the actual board
body occurrence. Extend the existing contact projection with full normals,
source best-index traversal and validity/group reads. Require actual retained
wheel-line outputs and passed up vector. Keep the surface vote before the
wheel-normal work and the actual angular-drag writes within it. Do not mark
all body postphysics complete merely because this narrow normal is available.

Current reusable authority is the actual sealed frame/list, late Entity-owner
resolver, seven actual board Parts, and their retained mutable inertia. The
surface leaf alone lacks other Assembly group reads, full normal retention,
wheel-line state, normal tuning/cvar captures, cos service and angular-drag
consumer. These dependencies must be closed or explicitly refused, not
replaced by a generic ground-normal function or Skeleton history output.

FillPhysOut's later copies are owned by its separate output buffer; this audit
does not establish that buffer or scheduling. Parent is withholding publication
until that ownership is recovered. Even if average falls back to up, the
published last-valid field must retain the actual earlier wheel result.

## Validation and confidence

Exact: PDB field positions, constructor/reset raw bytes, listed store/branch
order, full keys and embedded constants. High: bounded integration dependency
analysis. No live corpus run or newly implemented producer is claimed.

Fresh raw SHA-256 anchors (end exclusive):

| Span | SHA-256 |
|---|---|
| 82b36a58..82b374d0 | 2e999a33027dcdcf25dd3578a2ca48cac787f72f73b1d63fa1181d43400728ae |
| 82b23630..82b23764 | 3d9af0b58c0da8d16c2de1a59947587816d1dee9b9d38d4ffde18d68558d0188 |

Original Vault value/name for key0e508708cc7f5249 and cvar initializer values
were not independently recovered in this bounded audit; keep them unresolved
until original-data/source inspection. Constants above are actual raw bytes,
not values inferred from symbol names.
