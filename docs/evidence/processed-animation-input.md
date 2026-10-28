# Retained animation input into ProcessedPhysIn

Fresh IDA review covers `PhysicalPlayerHiLOD::SetPhysUpdateData` 82dfe688,
`ProcessAnimOut` 82e00030, `ProcessedPhysIn::Reset` 82b39098 and constructor
82e00de0. Original bytes and full disassembly are retained outside the repository
at `%TEMP%/skate2-process-animation-input-source.json`.

SetPhysUpdateData is exactly `stw r4,0x2b4(r3); blr`: retain the actual input
allocation, then consume its current fields. ProcessAnimOut reads that pointer
and the existing processed pointer at +2b0. It does not construct another
processed owner or gate copies on active flags.

| Store | Source input | Existing processed destination |
|---|---|---|
| 82e00058 | byte +2880 bit0 | +8c4 mask00100000 |
| 82e00068 | float +288c | time delta +93c |
| 82e00078 | float +2890 | compression +978 |
| 82e00090 | byte +28f0 bit0 | +8c4 mask8 |
| 82e000a0 | vector +28e0 | impulse +5b0, all four lanes |
| 82e000b8 | byte +2a10 bit0 | +8c4 mask4 |
| 82e000d0/d8/e0/e8 | matrix +29c0/+10/+20/+30 | matrix +5c0/+10/+20/+30 |
| 82e000f0 | byte +2a00 | onboard +600 |
| 82e00100 | float +2a14 | truck tightness +9c4 |
| 82e00110 | float +2a18 | wheel hardness +9c8 |
| 82e00128 | byte +2883 bit0 | +8cc mask00400000 |

The matrix uses four alternating SIMD loads/stores in ascending address order.
Inactive impulse and reset payloads are still copied. Float copies preserve
payload bits, including negative zero and NaN. Existing jump-event +8c4 and
physics-state +8cc/time-delta storage are reused, preserving other producers.

Reset writes compression +0 at82b39454, truck +0 at82b39514 and wheel +0 at
82b3951c. Existing physics-state Reset owns time delta bits3c888889 at82b3943c
and the complete zero +8cc word at82b39558. Existing jump Reset owns the complete
+8c4 word2000 at82b39428. Neither the complete Reset nor constructor writes
impulse +5b0, external matrix +5c0..5ff or onboard byte +600. Those fields remain
unknown initially and preserve their retained contents across later Reset.

This statement concerns generic construction/Reset. The actual HiLOD allocation
uses Sim category55 OR100: its dedicated selected allocation initializer now
establishes zero residual impulse/reset payloads before Processed ctor, while
generic new/Reset retains unknowns. See actor-physics-auxiliary-settings.md.

`SkeletonProcessedContact::try_process_animation_input` consumes the actual
`AnimOutPhysInOwner`. A host construction guard precedes source stores. If the
final +8cc read is unknown, it refuses that read-modify-write after retaining all
preceding writes; it cannot invent unrelated bits. Source scheduling and
retained pointer publication are separate caller responsibilities.

Focused tests distinguish inactive unconditional copies, all sixteen matrix
lanes, flag preservation across repeated calls, initial unknown/reset retention,
and the final-word failure prefix. Root owns integrated Cargo validation.
