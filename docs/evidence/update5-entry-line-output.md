# Update5 entry and actual hips-line output

Selected source **PhysicalPlayerHiLOD::Update5_ProcessOutput82e04ea8**.
The implemented entry ends before the virtual GetSkateboard call at82e04ef0 and
`Skateboard::FillPhysOut82b2f3e8` at82e04ef8. It does not publish physical state,
run conditioners, or claim a completed Update5.

The selected world adapter resolves the actual board/skeleton pair with the
existing backend authority checks and retains the same PhysicalPlayerAssemblyOwner.
It locks that owner's PhysOut and calls existing complete Reset82e2f740 at the
native call82e04ed0. All nineteen component resets run in their original order;
the write latch is cleared only after all succeed. A later NULL component retains
earlier reset writes and the prior latch. It then executes the actual
FillPhysOut_SkeletonLineTestResults82dfed78 callback at82e04ed8 using the same
owner's `PhysicalHiLodTail.line_records`, without accepting replacement payloads.

The first output guard occurs before obtaining the source line record. Every
subsequent output getter has its own guard. Source/output mapping:

| Source | Destination | Native store |
| --- | --- | --- |
|HiLOD line-result0 valid|Ground+124 belowHipsGroundDataIsValid|82dfedbc|
|HiLOD+210 line0 start position|Ground+c0 startPos, all four lanes|82dfedf4|
|HiLOD+180 line0 result position, if valid|Ground+d0 position, all four lanes|82dfee38|
|HiLOD+190 line0 result normal, if valid|Ground+e0 normal, all four lanes|82dfee70|
|HiLOD+1a0 line0 surfaceID, if valid|Ground+f4 surfaceID|82dfeea0|

Start position is copied even when result.valid is false. The conditional
position, normal and ID stay at their preceding Reset values in that case.
Surface ID is loaded at82dfee74 before the final output guard and is copied as
the same32-bit word, including a negative source i32. Vector copies retain
known W bits or unknown W; no W=0 invention, normalization, or numeric estimate
is involved. The fields already exist in GroundRemainder; no duplicate storage
was added.

`PhysicalHiLodTail.line_records` is initialized by the actual HiLOD constructor.
StartSkeletonLineTests produces each start_position and lease. Actual Update4
`try_end_skeleton_line_tests` Sync/GetResult, record copy/clear, Release and null
pointer sequence is the per-frame result producer. The existing Update1
`try_fill_skeleton_line_surfaces` fills ProcessedPhysIn and is a different call;
it cannot stand in for this output publication. No artificial valid result or
query miss is introduced here. This selected entry does not assert that the
whole Update4 schedule has completed.

The returned `Update5BeforeBoardFill` retains the same owner. It is a continuation
boundary, not permission to invoke the Physics conditioner early. State is not
stored until **82e05434**, after the preceding Fill callbacks and direct field
stores, from an actual current-state virtual GetType call at82e05400. Category
is obtained from a fresh GetType at82e055f4 and GetPhysicsCategory82d83b80, then
stored at82e056b8 after the conditional collision-wheel read and Air flag store.
No cached Ground enum substitutes for either query.

Four focused tests cover the actual reset/latch and output identity, full-lane
valid result and signed ID bits, invalid-result conditional omission with
unconditional start copy, guard-before-missing-record failure, and a later Reset
failure retaining its prefix and latch. Files were formatted; Cargo validation
and independent source review are delegated to the parent integration gate.
