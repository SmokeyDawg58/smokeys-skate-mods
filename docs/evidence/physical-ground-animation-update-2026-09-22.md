# Reached GroundAnimation 103 Update

Confidence: high for composition; existing numeric policies remain unchanged.
The original queued input runs now reach frame20 after actual Air exit,
GroundAnimation enter and Update3 prefix, stopping at StateUpdatePending(103).

`PhysState_GroundAnimation::Update` 82d98d68 clears retained jumpedThisFrame
at 8d80, calls same Reckoning fixed-heading evolution at 8d94, updates the
animated Skeleton with true at 8da4, stores add-skateboard-error at 8db4,
and enters UpdateSkateboard at 8db8. Fixed-heading uses the existing shared
82ddd468 body without the ordinary Ground wrapper's early heading/dynamic
stores. No new Reckoning, Skeleton, board, animation or state is constructed.

UpdateSkateboard 82d8a7b0 calls actual steering with target zero at a7d8
(not a no-op), selected wheel material at a7e0, then CalcGroundJump at a7ec.
CalcGroundJump 82b315a8 reads the existing live maximum-error cvar at 31600,
before local false/height/velocity zero stores 31604/08/0c and the actual
Processed +8c4 StartJump 00400000 gate at 31614/18. Unknown bits are errors;
known true stops at an explicit active-jump boundary. No successful result is
published on that boundary. Known false returns the real initialized result,
then the retained JumpInfo writes a7f4/a800/a80c occur.

The existing skateboard_ground_force_tail implementation supplies the six
source-ordered producers: front a8b8, rear a8cc, brake a8d8, slide a8ec,
drag a8f4, collision a968. Collision selects only its response; the ordinary
branch queues front/rear/slide/brake then sets drag. Its real Processed reader
now exposes the same late stateTimer. No force math or queue is duplicated.
Successful state update enters the existing shared controller continuation,
prephysics and retained Reckoning completion certificate on the same pair.

The shared animated path receives an explicit fast-blend argument: the old
Air call remains false; the actual GroundAnimation call supplies true. Both
admit only their same retained state/pair allocation. Existing animation root,
GeneralUpdate, trajectory reset, attributes and numeric services are reused.

Focused tests distinguish maximum-before-flags and lazy error behavior, full
positive-zero inactive result, and unknown/active bits refusing a publishable
result. Existing Reckoning, animated blend and six-force-tail tests cover the
reused numerical bodies. Standalone rustfmt and patch apply-check passed;
Cargo and original-data acceptance belong to the root integrator.

Explicit subsequent boundaries: UpdatePostPhysics 82d83fb8 tail-calls the
actual Wipeout.CheckForAnimatedGroundWipeout (not a no-op). Fill 82d83fc0
conditionally publishes jumped/velocity, and current103 selection has its own
CalcSuggestedState branch 82ddc370..400. These are separate follow-up work;
this packet does not claim a completed103 frame or the enabled-jump launch.
