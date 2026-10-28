# GroundAnimation 103 postphysics, Fill and next selection

Confidence: high; original frame20 now completes actual103 Update and solver,
then stops at the unsupported postphysics dispatch. No Cargo run by author.

GroundAnimation vtable8234a1b8+20 points82d83fb8: it tail-calls
Wipeout.CheckForAnimatedGroundWipeout82de1930. That calls the existing Ground
detector first1948, then looks up arms d0317181ce40bfbc1998 and body
68e0e915d4f3af6619f0 through the same global wipeout attribute owner. It reads
arms1a0c then body1a14 and calls the already implemented collision-force leaf
82b3fd90. On true it requests0 BEFORE display-debug/overlay. Then it samples
the actual board contact opposing delta+35c1ad4, reads threshold
5d308c99bc0232e2 at1b14 and compares1b2c. Raw1b30=40990098 is GT-clear:
unordered skips this branch. Ordered greater optionally rereads board/delta
1b54/64 and requires overlay BEFORE request20. The actual debug calls target
the existing no-op leaf; overlay access remains real and fallible.

The draft is a child of the existing Ground detector Source, sharing actual
world/board/Skeleton/Wipeout/attribute/cvar/numeric/contact allocations.
Only visibility of four existing Air attribute/debug readers broadens to its
parent; no duplicate collision kernel or request owner is introduced.

FillPhysOut82d83fc0 tests jumpedThisFrame before any PhysOut access. True
requires the first write guard3fe0 then stores Air.stateTriggeredJump4010;
the second guard4014 precedes all four velocity lanes copied4040 from retained
state+50. False reads no jump velocity and requires no output component.
The actual remaining-state allocation and pair bindings are checked, and the
current-type producer reads its native GetType103 (82d83e50). Tests preserve
false-path laziness and the triggered prefix on late velocity failure.

CalcSuggestedState current103 begins82ddc368. Ordered priorities: processed
flags8c4 mask40000→300; flags8d0 mask20000→602; flags8cc mask20000→500;
flags8cc mask100→502; framesSinceJump91c==1→200 or201 from8c4 mask400.
Otherwise a noncolliding body executes the existing actual offground query;
true→200, otherwise actual IsSkateboardAnimated→103 if true,100 if false.
There is no Ground100 teleport-countdown admission in this branch. Tests
distinguish priority/lazy reads and the two future query continuations.

This admits only selecting these destinations. Existing unsupported transition
errors remain for unrecovered103 Exit/next-state Enter; no forced stay or
invented no-jump result is used. Enabled Ground jump remains the explicit
previous packet boundary. Sharedpatch is artifact-only; root integrates and
runs strict/workspace/original gates.
