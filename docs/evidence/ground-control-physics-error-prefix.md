# Ground control physics-error prefix

Source review September 12, 2026. Ground.Update82da101c calls
PhysicsGround.UpdateSkateboard82d9a018. This selected continuation ends after
CalcTruckDamping at82d9a074, before CalcSteeringTilt82b27ae0 at82d9a088.
It neither runs the later truck setter nor completes the state update.

## Actual owner and ordered reads

The selected Ground owner already retains the same board/skeleton pair.
The adapter checks those identities and uses the board constructor's retained
physical owner, whose Reckoning holds the existing processed Arc. No second
processed buffer or fresh board state is constructed.

At a048..a060, kDrawSBJoints is sampled. Enabled calls DebugDrawJoints82b23910
at a064: it obtains the actual body's Assembly and calls825187c0 twice. That
callee is a literal blr. There is no debug singleton check or line query here.

UpdatePhysicsError82b23aa8 at a06c consumes processed orientation+0, distinct
from the captured deck+40. Its rigid inverse is computed before fresh
Body.GetPosition(6). The relative matrix is inverse(orientation) times the
current Part6 transform. Existing typed affine inverse/composition carry the
actual unknown-W policy; source output quaternion depends only on basis XYZ.

The relative rotation is orthonormalized in reverse axis order Z,Y,X using
CLASSICAL projections (dot with original current input, subtract from the
progressively shortened candidate). Products and subtractions are separate.
Every axis runs an unsafe dot and two-refinement reciprocal-root graph.
No finite/degenerate repair or alternate axis is introduced.

The same existing typed normalization body now accepts a refinement callback.
Existing callers keep their existing policy; this source call supplies native
VNMSUB final-negation residuals. No second orthonormalization loop is added.

The quaternion graph is the existing world::try_simulation_quaternion_candidate.
Fresh raw82b23da4 word10aa69ef and3db4 word1049692f are standard VNMSUB:
residual = -(magnitude * rounded(seed*seed) - 1), with fused inner arithmetic.
The single candidate evaluator uses -magnitude.mul_add(square,-1), preserving
the negative sign of an exact-zero residual. The preceding half-seed multiply
and following fused estimate refinement retain their original ordering.
Candidate construction uses sign-bit XORs and strict trace/diagonal choices.
All four candidate estimate lanes execute, including subsequently unselected
zero or negative magnitudes. Host estimate/reduction/NaN-payload behavior
remains an explicit numerical policy, not Xenon bit parity.

Raw3e14 stores the selected quaternion at outer Skateboard+d0;3e18 then sets
the blending byte+100. There is NO final quaternion normalization. Using the
body conversion helper that normalizes the candidate again would be incorrect.
All earlier source/numeric errors leave both retained output fields unchanged.

CalcTruckDamping82b23980 is three instructions: load board+110 processed,
lfs processed+944 into f1, blr. The current Z-speed value is sampled only after
the physics-error stores. SetTruckDamping82b29dd0 is later at a118 and must not
run in this prefix.

## Constructor and cvar evidence

Outer constructor82b38a94 sets r22=gLVector;38af8 loads it;38b00 stores it at
offset d0. gL is [0,0,0,1]. At38b04 r30=0 clears byte100, followed by existing
deck tilt+e0 positive-zero store38b08. These fields extend the same retained
SkateboardSteeringState; no duplicate physics-error owner is introduced.

Cvar slot114 is reserved: reference830d9620, initializer82f8f028,
static table82fd5718. Source strings are section Physics_Skateboard,
label kDrawSBJoints, title Draw Joints. Rawf04c passes null callback r8=0,
f050 defaultfalse r7=0. Sampling uses the actual live linked target.

## Validation and remaining stages

Focused leaf tests cover constructor state, fresh Part failure ordering,
every one of thirteen explicit numeric failure boundaries, and typed unknown
affine W. A deliberately inaccurate quaternion estimate yields W3f5e6800,
proving no extra normalization was inserted. Parent owns all Cargo gates;
this packet does not imply an original-corpus pass before that gate runs.

Following stages are CalcSteeringTilt, COM-to-deck length, SpeedWobble,
SetTruckDamping, existing UpdateSteeringTilt and wheel material update, then
WallRideForce and jump/collision/ordinary-force decisions. Existing
GroundAnimation six-force orchestration has a different call order and is not
substituted for this PhysicsGround controller. Its individual force services
remain reusable at their actual later positions.


## Root integration review, September12

Root independently reviewed the full Ground caller a018..a12c, the steering
and speed-wobble arithmetic, and the live same-owner adapters. The cvar
initializers82f8f028,82f6c450,82f7bc00 were freshly checked. SetTruckDamping
82b29dd0/4 is an unchanged store into the existing Body+1dfc owner.
Frame agent independently reviewed root registrations, cvar ordering, retained
truck ownership and the selected same-Actor call after Pumping: PASS, no defects.
Clippy51 PASS; workspace37/original34 remain pending at this record.
