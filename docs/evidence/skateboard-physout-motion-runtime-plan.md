# Actual-body motion to retained prior PhysOut

2026-09-08. Read-only original XEX/PDB and current runtime
inspection. No implementation/Cargo run. This narrows the earlier
`skateboard-physout-motion-source.md` to the next usable producer/consumer path.

## Smallest concrete dependency

The actual Part6 rigid body already owns current linear velocity XYZ. No new
force, contact, surface or velocity estimator is needed to supply the jump
consumer's prior deck velocity. The missing stage is the original guarded
publication to the SAME pair PhysOut Motion.DeckVelocity, followed by the
already implemented next-Update1 copy to processed+190. Do not replace that
prior copy with a live current-body getter inside GroundAnimation jump.

Use the actual board session's Part6 observation (same pool/occurrence) and
`SimulationRigidBodyPool::try_contact_spy_linear_velocity` as the existing
checked XYZ getter. Despite its original caller-oriented name, it reads the
same retained slot.velocity with occurrence/integrity validation and no
calculation. A source-neutral forwarding name could clarify reuse; a second
velocity store/owner is unnecessary. Retain W=None: native velocity+20's fourth
word is the intrusive body link, not a reconstructed numerical lane.

The same actual body owns angular velocity XYZ at+30, but no narrow angular
getter was found. Existing per-body snapshots contain omega; their bulk fields
and validity rules should not become an eager snapshot at the linear read.
A narrow observation-checked getter mirroring the current linear getter can
read that same slot.omega at the **later** source point. Angular W is also a
packed native link word; it must not be manufactured as zero.

The minimal adjacent publication run is angular velocity, deck velocity and
deck speed through82b2f7f8, retaining the earlier linear sample fromf550. It
naturally gives the prior copier three real publications and preserves the
observable deck-velocity-before-length-estimate failure boundary. This is a
selected FillPhysOut component run, not all preceding Fill or all output fields.

## Exact native motion publication order

FillPhysOut82b2f3e8..82b3076c captures the actual assembly Part6 body atf43c.
Source linear velocity read is **f550**, before two fresh effective-transform
calls f554/f568, direction dot/sign work, two guarded position-component
vector writes f638/f678 and conditional debug branches. Keep this retained
sample across those stages; do not reread velocity at the later storef788.

Velocity-direction cvar target/value f67c/f680 selects two more actual effective
transform calls f694/f6a0 and overlay existence checkf6a4. Look-direction target/
value f6cc/f6d0 similarly selects effective callsf6e4/f6f0 and overlayf6f4. There
is no drawing call in these listed branches. Missing native singleton still
asserts, so those selected effective calls/gates cannot be silently omitted
because the drawing output is absent. Source default/cvar sampling remains
owned by the actual configuration service; this packet does not invent values.

At the selected adjacent motion run:

| Stage | Source order | Existing output API |
|---|---|---|
| Angular velocity | sample latchf714; read retained Part6 angular velocityf718; check sampled latchf71c..734; store Motion+40f740 | sample_write_guard; late angular getter; publish_motion_vector(AngularVelocity) |
| Deck velocity | sample latchf744; checkf748..760; dot(retainedVelocity,retainedVelocity)f764; full Motion+50 storef788 | sample guard then check_write_guard BEFORE dot; publish_motion_vector(DeckVelocity) after dot |
| Deck speed | sample latchf78c/comparef790; estimatef794; two refinementsf7ac..c8; squared*inversef7cc; exact-zero selectf7d0; check sampled latchf7d8..ec; scalar Motion+a0 storef7f8 | sample guard; estimate+shared length finish; publish_motion_scalar(DeckSpeed) |

A failed angular source read occurs before that sampled-latch check. A failed
deck dot leaves AngularVelocity published but DeckVelocity old. A failed length
estimate leaves DeckVelocity published but DeckSpeed old. A locked speed guard
is checked only after estimate/refinements. Reusing one guard for all fields or
using a generic guard-before-source callback would change these stages.
`publish_motion_vector/scalar` already checks a sampled owner-qualified guard
and marks PhysOutOrigin::Publication. The explicit earlier deck check is needed
because the later setter alone would delay the failure until after the dot.

Reuse `skeleton_collision_numeric::length_from_squared_and_estimate` for the
length finish, with ReckoningNumeric dot and reciprocal_sqrt_estimate at their
original points. Even squared==0 requests an estimate, then selects full+0.
Do not use direct host sqrt, a NormalizeSafe helper, or an alternate epsilon.
The earlier source document's reference to a similarly named helper in
reckoning_numeric is historical: the currently exposed shared finish is in
skeleton_collision_numeric.rs.

## Existing same-pair consumer is already present

`physical_player_phys_out.rs` retains separate motion vectors/scalars with
constructor/reset/publication provenance and occurrence-qualified write guards.
`physical_player_phys_out_session.rs::try_with_pair_phys_out` verifies actual
board/skeleton membership and their shared PhysicalPlayerAssemblyOwner, then
borrows that owner's retained PhysOut. There is no buffer swap.

`skeleton_processed_prior_output.rs::try_copy_prior_output` binds the same output
occurrence across calls/resets, invalidates previous toolkit readiness, executes
its explicit preceding gaps, then copies Motion.DeckVelocity at82e02638/63c into
its retained velocity field (processed+190). Its `prior_output().velocity()`
returns the typed vector and PhysOutOrigin. Angular velocity similarly copies
Motion+40 at82e02780/784 to processed+2d0; DeckSpeed+a0 copies02744/748 to+968.
No change to this copier or a second processed velocity field is required.

The actual scheduling remains publication in HiLOD Update5_FillOutputs (board
Fill call82e04ef8), then next Update1 prior copies before toolkit. Native
Update1's output locktrue82e03ae8 is much later; selected-copy helpers must not
insert it early. Reset clears that latch only at its actual final stage. A
scoped test should explicitly run its already-authorized selected lifecycle,
assert Publication provenance and actual body XYZ, then execute the same prior
copy/grounded jump preparation. Constructor zero is not a current velocity proof.

## Why surface velocity cannot be filled from deck velocity alone

Later sourcefdf8..3004c first obtains guarded Motion pointer, actual retained
Reckoning ground normal+1f0 (fe1c) and previously **published** Motion.DeckVelocity
(fe24). Compute dot(V,G)fe2c; rounded G*dotfe3c; S=V-productfe40. This G is not
Body contact last-valid/average or the output GroundUp vector. Read processed
physical deckZ+60 atfe48, compute its ground-plane projection P with dotfe4c,
productfe54 and subtractionfe58.

NormalizeSafe(P,zero)fe60 is a real fallible numeric stage whose result at stack
+90 is **unused**. Subsequent source reloads original P at+80fe74/fe8c. Reuse the
existing typed NormalizeSafe graph and discard only its successful result;
never omit its callbacks or replace P by the normalized output.

Dot(P,S)fe94; ordered positive preserves P, equal/negative/unordered sign-bit
negates all four lanesfec8..d0. Fresh Reckoning ground transformfed4..ff28 supplies
windY to signed AngleBetweenVectorsff2c. Existing try_signed_angle implements
that graph. Angle wrapping uses **floor** (`vrfim128`ff5c), not the existing
truncating Math::Angle::Normalize: t=angle*g2PIReciprocal; fraction=t-floor(t);
if ordered fraction>.5 subtract1, then multiplyg2PI. This still needs a narrow
source floor primitive if none is added by the owning reconstruction before
implementation; nearest-even cosine rounding is not interchangeable.

Only after that numeric sequence/guard does source publish SurfaceVelocity+60
ff9c, SurfaceAngle+acffdc, then evaluate length and guarded SurfaceSpeed+a4
3004c. Current PhysOut exposes all these fields; existing numeric graphs cover
projection/dot/signed angle/normalization/length, but these actual Reckoning and
processed-state inputs and intervening source stages must be bound first.
No surface velocity producer was found already integrated into this PhysOut.

## Z speed has another later source transform

After surface speed, wobble/tilt outputs intervene. A fresh Part6 body-position/
transform call300e8 and current processed flip flag8c4&100000 at300f0 determine
effectiveX/Z signs. At30170 source rereads published Motion velocity, computes
dot(velocity,effectiveZ)30174, then guarded ZSpeed+a8 store301a0. Do not eagerly
compute it from the effective transform nearf554 or derive it from deck-speed
magnitude. Reuse actual Body transform getter and existing sign-bit operations;
require the retained processed flip producer at this source stage.

## Concrete next implementation and validation proposal

Introduce a retained selected motion sample containing the actual Part6
observation plus typed linear velocity captured atf550. It should be produced
by the actual world/body bridge at that source stage, not publicly constructed
from arbitrary vectors. The subsequent adjacent motion publisher takes the SAME
pair output, numeric capability and a late actual-angular getter for the retained
body. Bind world/board occurrence and never reread velocity for the deck store.
This sampler/publisher remains explicitly selected scope until all earlier
position/effective/cvar stages are supplied by the real Fill caller; do not hide
those behind production no-op gaps or claim full function readiness.

If root instead closes the complete Fill prefix now, reuse existing same-owner
effective transform producer and add the missing position-component fields/
configuration captures with their exact guards before the adjacent motion run.
The first playable dependency is the real deck-velocity prior field; surface
projection should be a separate continuation, with explicit unresolved inputs
rather than guessed contact normals or default tangents.

Tests: unequal live body/prior constructor velocities; mutate body velocity after
sampling to prove retainedf550 sample; mutate omega before late getter to prove
f718 timing; locked angular source-read-before-check; locked deck check-before-dot;
dot failure leaves only angular publication; estimate failure leaves deck velocity
but old speed; zero length still estimates; unknownW survives output and prior
copy; wrong pair/occurrence rejected; source stages do not mark surface/Z fields
published. Original corpus should assert actual Part6 XYZ and Publication origin
before next processed+190 copy, then use that same processed owner in jump.

Fresh raw read confirmed all listed f550..f7f8 orders. Whole Fill function
reference hash remains8c27321995ce79faf4d251aaf9c5aeac49182b7d076084730d04572b282f18c7
for82b2f3e8..82b3076c. Confidence exact for listed source reads/guards/stores and
existing API inspection, high for the bounded implementation proposal. No
new live motion publication or complete cyclic frame is claimed.
