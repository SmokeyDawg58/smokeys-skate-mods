# Update5 post-category stores before Ground.Fill

Bounded source interval: `82e0579c..82e05b5c`, returning before the current-state virtual Fill at `82e05b74`. Raw Update5 instructions establish the ordering below. These adapters extend the actual retained producer-to-consumer chain after existing category/dt publication; they do not claim completion of the rest of Update5.

- Output OffBoard surface IDs at `57cc/5800` both read Processed **left** ID `+934`, independently after their respective guards. The second source is not right ID `+938`.
- Fresh Processed flags `+8d0` precede each output guard. Masks `00200000`, `00100000`, `00000800` publish mounting, unmounting, air dismount.
- A further fresh flags read selects mount validation at `58cc` only when `00200000` and `00040000` are both set. The adapter reports pending at that exact boundary, after previous stores, without inventing mount transforms, live cvars, or validity.
- Fresh flags mask `00400000` publishes interactable validity at `5a44`. The next guard is sampled before reading Processed object ID `+8c0`, then the ID is stored at State `+18` (`5a78`). Missing ID therefore preserves preceding stores.
- Exit state `+8d8 == 500` refreshes actual HiLOD dismount count `+144` to 3. The subsequent fresh count read decrements positive values before checking the output guard and setting the State request true. Nonpositive values retain the old request.
- Processed flags `+8c4` mask `02000000` is read before the guard. True reads existing output ZSpeed `+a8`, then Processed animation push speed `+940`, subtracts, samples another guard, reads actual AnimOutPhysIn dt `+288c`, multiplies, validates the guard, and stores Reckoning contact error `+c4`. Subtraction and multiplication round separately. False checks the first guard and stores literal positive zero without touching push speed/dt.

Actual adapters reuse `SkeletonProcessedContact::line_surfaces`, retained physics/animation fields, HiLOD prefix count, existing output motion scalar, and registered animation physics input. Unknown required fields stop at their first source read. No parallel owners or synthetic flags are introduced.

Integration requires retaining Processed object ID `+8c0` in `ProcessedPhysicsStateFields`: `Option<u32>`, initially `None`, reset to `Some(0)` by native Reset `82b39420` (r30 zero established `82b391ac`). Existing positive interactable publication remains pending until its actual payload is implemented; reset zero is an actual producer, not a fabricated query result.

Tests cover independent repeated-left reads, false-branch laziness and positive-zero output, unchanged dismount latch, partial publication at mount-validation pending, exit-count refresh/decrement before a late dt failure, and push distance using published ZSpeed and actual dt. Cargo and shared registrations are left to the root integrator.

Ground.Fill remains the immediate next call. Update5 completion still requires later trick/stance/COM/push/intents stores and surface processing, `ProcessIntentsForPhysOut` at `61f4`, animation output Fill at `6200`, Reset at `6208`, then wrapping output count increment at `6214`. There is no final output lock in this Update5 return span.
