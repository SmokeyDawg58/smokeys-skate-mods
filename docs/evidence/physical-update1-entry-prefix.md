# Actual Update1 entry and teleport check

Confidence: high for the selected consecutive path; the complete physical update
is unfinished. This module executes the retained physical pair, without replacing
the Actor input, offboard managers, processed input, or previous output owners.

`PhysicalPlayerHiLOD::Update1_ProcessInput` (82e01f98) calls Location.Tick
(82dd1fa0), Jumping.Tick (82dd5f98), Landing.Tick (82dc8ed8), then
StartSkeletonLineTests (82dfe850). Location only sets sync permission when its
retained line pointer exists; a null pointer preserves its previous permission.
Jumping decrements the signed counter with native wrapping and clamps negative
results to zero. A retained line pointer requires CalculateResults and remains an
explicit dependency; constructor-null is the genuine selected initial path.
Landing clears can_land. The three line requests use the actual world API and
existing query ownership before any Processed Reset.

The State getter at2008 captures the previous category before Reset200c. Forced
braking2014..208c short circuits on actual input force_braking, signed state_count
greater than5, DeckSpeed strictly less than0.1 (bits3dcccccd), physics state100,
and signed wheels_in_contact greater than2. The raw BGE2044 skips on NOT-LT,
including unordered floating comparisons. No later getter executes if an earlier
predicate fails. Result updates only Processed8cc mask04000000.

One Collision getter2090 supplies skeleton partial-ragdoll and fence-impaled
booleans for Processed8d4 masks10000000 and00400000, in that order. Positive
Jumping result counter updates mask20000000. Landing.GetDeckOffset82dc8f10 copies
all four lanes from the retained manager+b0 landing_error to Processed+360.
ProcessAnimOut82e00030 runs at2134 on the same bound input. A fresh State getter
then copies teleport_exit_state (State+8) to Processed+8d8 at216c.

CheckForTeleport82e01db8 is called unconditionally at2170. It freshly guards State
and reads is_teleport_finished. False returns normally. True freshly guards Motion
and asserts did_teleport; another independently guarded Motion read copies all four
teleport-transform vectors before TeleportPlayerNow82e01ebc. Raw82e01e28 and e74
both load PhysOut+4: this is SkateboardMotion, not SystemReckoning. The selected
implementation preserves a failed assertion and otherwise reports the missing
TeleportPlayerNow consumer with the captured transform. It never forces the
finished boolean false.

The continuation at2174 retains the actual physical owner, Actor input, Processed
owner, and old category for the next consecutive AI/input prefix. It is not a
certificate of a complete Update1. Failures preserve earlier manager writes,
launched line batches, Reset, and ordered processed stores; callers must not retry
the entry as though those effects had not occurred.

New Processed fields +360 and +8d8 are reset by native stores82b39390 and82b39480
respectively; both zero. Existing flags8cc and flags8d4 remain the only owners of
their native words, including preserved unknown lower bits.

Validation pending root integration gates and actual corpus invocation; no
independent Cargo process was started by the implementation agent.
