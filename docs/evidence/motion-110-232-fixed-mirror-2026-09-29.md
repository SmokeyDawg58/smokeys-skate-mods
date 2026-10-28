# Motion 110/232 and HandsFace fixed-pose reuse

Source deliverable for the active five-error goal. No
Cargo, rustc, tests or game were run for this follow-up.

## Motion switch cases

IDA switch `8290aa8c` case232 targets `82910b4c`: read/cache condition320
(`sConditions + 0x500`), return its positive result. This is the existing
ShouldLeaveSlide(left) condition and existing actor slide-end flag reader;
neither a new condition implementation nor unconditional acceptance is needed.
The existing stance/fakie handling agrees with ShouldLeaveSlide `82c9c918`.

Sustain switch `8291e250` cases110 and232 both target `829221c8`, `li r3,1`.
These branches read no conditions. Pre110 already calls condition190 and was
left intact. Graph110 is N_PopShuvit206; expression232 is used by the left-slide
exit transitions. Added the three missing switch entries and regression checks
for true/false condition results, failure propagation and zero Sustain reads.

## Fixed Mirror was an invalid fixed-output producer

`AnimCommandSystem::Mirror82a2dfa8`, buffered branch:

- `82a2e098..e0a0` pops the command input before allocation.
- `82a2e0c4` retains its output-table reference.
- `82a2e0c8..e0ec` recognizes fixed input only for opcode12/15 with bit0.
- `82a2e0f8..e10c` allocates a fresh dynamic SQT for a fixed input; a dynamic
  input keeps its existing destination.
- `82a2e16c..e190` encodes input type/reference separately from the destination.

Our previous Mirror command retained a fixed ID as its output. `produced`
therefore registered an unsupported fixed graph root, and later compaction
reported ReuseFixedRootsPending. The executor also assumed every mirror ID
indexed dynamic storage. Both ends now retain separate input/output IDs:
the existing fixed-owner operand reader copies the fixed pose into the new
dynamic destination, then the existing mirror kernel operates there. A dynamic
input still mirrors in place. Fixed source poses remain unchanged.

GetNextLoc `82a24c28` and DispatchReuseBufferCmd `82a2fc18` track graph node
occurrences, not unique fixed frame IDs. Repeated references to the same fixed
frame are valid. Compaction now replaces the surviving stack-top occurrence
in place while retaining its new graph-node order separately. Previously a
duplicate-ID check rejected it; moving the replacement behind a popped operand
would instead make the subsequent consume remove the wrong occurrence.
Existing allocation limits, segment copies, owner checks and failure guards
remain active.

Regressions cover fixed-to-dynamic mirroring across compaction, preservation
of the fixed source, subsequent dynamic in-place mirroring, and identical
fixed IDs on the surviving stack and a popped operand. These are UNRUN.

## Validation and scope

Scoped rustfmt parsing/formatting passed for all seven edited Rust files.
Module/include audit passed: 1995 Rust files and 1989 literal paths.
Raw PPC, switch destinations and decompiles are saved in
`motion-110-232-fixed-mirror-ida-2026-09-29.txt`.
This establishes a source defect and its correction, not reproduction of the
reported input sequence or proof that every animation graph is complete.
The independent empty-command-stack reuse limitation remains guarded by
ReusePendingOperands; it was not the reported fixed-root error.

Other requested goal items are implemented and documented separately:
`skitch-physics-104-2026-09-29.md` (full skitch lifecycle/update integration),
and `ground-animation-wipeout-transition-2026-09-29.md` (103 to300 admission).
Next check: build-release.bat, then play-quiet.bat; exercise car grab,
towing, lateral shimmy, release/bail, nollie pop-shuvit, slide exit and repeated
crouch/grab/gesture inputs. Compiler and runtime correctness remain unverified.
