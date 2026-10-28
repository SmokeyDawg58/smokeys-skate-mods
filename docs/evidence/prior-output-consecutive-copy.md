# Consecutive prior-output input copy

The actual new consumer covers `PhysicalPlayerHiLOD::Update1_ProcessInput`
82e02430..82e02a28. It consumes the retained PhysOut and AnimOutPhysIn owners
and writes the existing SkeletonProcessedContact. It begins after the caller's
earlier Update1 prefix and ends before the next Ground field copy. Confidence
is high for reviewed raw instructions and PDB layouts; whole Update1 remains
unfinished and no final PhysOut write-lock is set by this helper.
Original bytes and disassembly are archived externally at
`%TEMP%/skate2-prior-output-copy-source.json`.

The old ten-gap projection remains for explicitly bounded probes. Six gaps
contain only the next source latch check/pointer read, already represented by
the actual guarded getters: BeforeSurfaceVelocity, BeforeSurfaceSpeed,
BeforeDeckSpeed, BeforeAngularVelocity, BeforeCentreOfMassVelocity and
BeforeReckoningUp. The new route does not invoke a Gaps collaborator.

The former BeforeDeckVelocity includes a large earlier prefix, not a no-op:
Reset, force-brake conditions, collision/OffboardJump flags, landing offset,
ProcessAnimOut, teleport check, input75, ground timers, SurfacePhysics.Update,
conditional AI transport and HiLOD counters. This prefix remains the actual
HiLOD caller's responsibility. The new bounded sequence begins with these
previously omitted straightforward copies:

| Stores | Source and destination |
|---|---|
| 2460/2468 | State.physics_state -> Processed.physics/current |
| 249c | independently read State.category -> Processed.category |
| 24d8/2514/2550 | independently guarded State teleport-finished/reverting/requested-teleport -> 8c8 masks80000000/00400000/00080000 |
| 258c | guarded State.handplant -> 8cc mask1 |
| 25c0/25f4 | independently guarded Grinds.type/substate -> 8f8/8fc |
| 2600 | reread Processed.category -> last_category |
| 263c/2678 | Motion deck/surface velocity -> 190/1a0 |
| 26ac | Motion+ac surface angle -> Processed+9b0 |
| 26e0/2714/2748 | Motion z/surface/deck speed -> 944/96c/968 |
| 2784 | Motion angular velocity -> 2d0 |
| 27c0/280c | Motion+80/+90 empirical acceleration/velocity -> 310/320 |
| 283c/284c | AnimOut+2a30/+2a40 COMdelta/animation -> 270/280 |
| 2864/287c/2894 | AnimOut+2aa0 high byte masks80/40/20 -> 8cc masks80/40/20 |
| 28a4/28b4/28c4/28d4 | leftIK/lefttarget/rightIK/righttarget -> 988/290/98c/2a0 |
| 28ec | AnimOut.next_step_valid lowbit -> 8d0 mask8 |
| 28fc/290c | next-step position/normal -> 370/380 |
| 2944/297c/29b4 | independently guarded System COM/COMvelocity/Up -> 250/260/220 |
| 29f0 | guarded SkateboardReckoning+b0 look direction -> 230 |
| 2a28 | guarded Ground+60 last valid up -> 1d0 |

The byte at big-endian AnimOut+2aa0 is the word's high byte. Using its low byte
would publish different flags. Every source component getter independently
checks the live PhysOut write latch before accessing that component, including
the two separate empirical vector loads and the repeated State reads. Missing
components and unknown processed flag words retain earlier completed stores.
No fallback output values or fabricated callback successes are used.

New retained fields use the original Reset writes: empirical vectors3936c/70,
COMdelta39320, COManimation39328, feet39330/38, next-step39394/98, look39274,
surface angle394ec, foot influences39474/7c, grind type394c0/substate394c8.
Original vector stores copy all lanes; unknown W provenance from PhysOut is
retained rather than synthesized.

The subsequent Ground dynamic-up/anti-flip, Air, HandPlantManager, collision,
wipeout, filtered/offboard flags, wheel-pair mapping, timers and world-transform
work through34xx remain outside this sequence. Old selected_copies_complete
does not claim those operations. The new copied_input_fields.complete marks
only the explicitly bounded sequence above.

Focused tests distinguish the high-byte flag mapping, copied payloads,
preservation of unrelated bits, late 8d0 failure prefix, early write-lock failure
and foreign output identity. Root owns integrated validation.
