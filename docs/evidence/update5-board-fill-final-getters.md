# Final BoardFill getters and output

Unregistered drafts finish3065c..3074c after the ground/contact suffix. This
would finish BoardFill only; remaining Update5 callers still follow. No new
physics category or forced Ground state is published here.

## GetPartSlideSpeed82b242f8..243e8

The actual selected part is6. Source asserts signed part<7 then reads contact
byteBody+34c+part. False returns positive zero without loading vectors or
calling numeric operations. True loads relative velocityBody+220+16part at
24378, then normalBody+60+16part at2437c. Dot3(normal,relative) precedes separate
normal*dot and relative-minus-product, then dot3(tangent,tangent). Estimate and
both Newton steps happen unconditionally before the squared-zero selection.
Raw243a8=10cb6a2f and243b8=106a696f are standard VMX VNMSUB residuals with
squared length and inverse square, matching the local reviewed board-speed
helper. The draft reuses that helper; shared numeric policy is unchanged.

Actual collision_state.relative_velocity is preserved unknown by Reset. Thus a
true deck contact without its genuine PostPhysics relative-velocity producer
fails at the real used field; false does not require it. Only XYZ participates
in returned scalar math. No packed W is synthesized or published.

## GetDeckToWheelBase82b2e718..2e7f0

This is an attribute subtraction, not a geometric body query. PDB identifies
GlobalAttributeCollection+9c as physicswheels_default and+dc as
physicstrucks_default. First singleton assertion/capture precedes wheels lookup
key12646f5e842c4db3 at e770. A fresh singleton capture precedes trucks lookup
key2be08dd7a9207622 at e7c8. Only then are the retained first float e7dc and
second float e7e0 loaded, followed by fsubs e7e4. Missing pointers use actual
DefaultDataArea(4); malformed present payloads remain errors.

The draft reuses the actual construction-owned SkateboardAttributes instances
and existing lookup/float methods, retaining both pointers until both lookups
finish. It introduces no separately bound global instance and no tuning value.
Tests distinguish failure of the second lookup before either float read.

## Final stores and retained request

Fresh body call30668 and slide getter30670 precede output assertion30674 and
Collision+14 store306a0. Fresh body306b0 and wheel-base getter306b4 precede
assertion306b8 and Reckoning+c0 store306e0. Assertion306e4 precedes actual
Processed+9c8 wheel hardness3070c and Motion+c8 store30710; draft needs a narrow
getter for the existing ProcessedAnimationInput.wheel_hardness field.

Outer WantsRespawn+101 is read30714. False neither writes output nor clears
input. True asserts output, writes State+3b30748, then clears outer+1013074c.
Fresh PDB names State+3b mPhysicsRequestedTeleport. The source output field
already exists. Outer constructor initializes WantsRespawn false at38b0c;
bad-data path later sets true at37d68. Proposed single storage is an added
constructor-backed field in existing SkateboardEntryState, with getter and
clear. No repeated reset or assumed successful PostPhysics is implied.

Tests cover lazy false-contact getter, unknown relative velocity on true,
unconditional zero-length estimate, actual attribute read order, output-locked
slide evaluation, request publication before a failed clear, preservation of
earlier hardness, and false-request preservation of previously set output.
All draft tests remain unrun; parent owns registration and Cargo gates.

Root independently reviewed complete raw GetPartSlideSpeed242f8..243e4,
GetDeckToWheelBase2e718..2e7ec and caller3065c..30768 against these drafts.
Lazy contact reads, separate projection multiply/subtraction, unconditional
zero-length estimate, retained attribute pointers across both lookups and
final output/respawn-clear ordering agree. Source leaves and adapters are
approved pending the shared registration window and validation. The proposed
new respawn storage still requires root review of constructor/setter wiring.
