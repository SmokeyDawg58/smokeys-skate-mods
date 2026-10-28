# Contact append reservation-counter correction

Independent live-IDA audit, 2026-09-07. This supersedes the earlier assumption
that every copy-count clamp also clamps the native global Contact counter.
That earlier source interpretation, and the tests derived from it, were wrong.

## Source evidence

`EA::Physics::GlobalBuffer<rw::physics::Contact,EA::Physics::ExternalMemory>::Proxy::Store`
is `0x827CB5C8..0x827CB678`, collision job buffer ownership. Live function bytes
SHA256: `91EC2FF8A204A024AA86030F5F51094833AD15FB8F682AF1125EC3BE6CDCA049`.
The matched executable SHA256 is
`3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`.

The independent audit inspected actual PPC branches, not only pseudocode:

- `0x827CB5E4`: zero requested count returns zero before the atomic operation.
- `0x827CB5FC`: `EA::Jobs::AtomicAdd`; `0x827CB604` subtracts requested count
  from its returned post-add value to obtain the old counter/start.
- `0x827CB608..60C`: signed `maximum - start`; nonpositive goes to the reset
  branch at `0x827CB628`, with `AtomicStore(maximum)` at `0x827CB630`.
- Positive remainder takes `0x827CB610..618` to `0x827CB638`. Its comparison
  at `0x827CB63C` chooses requested count or remaining capacity for COPYING.
  Neither positive-remainder route executes an atomic reset.
- The extra reset block at `0x827CB61C..624` would require the already-positive
  remainder to equal zero; it is unreachable in this serialized value flow.
- `0x827CB648` guards the destination pointer construction/copy on nonzero
  accepted count. An out-of-capacity returned start with zero accepted count
  must not even form a host slice at that start.

`EA::Jobs::AtomicAdd`, `0x82C2EE50..0x82C2EE78`, independently confirms a
reservation-loop addition and post-add return. It does not saturate.
CPU ground job `0x827CDB98` flushes each processed assembly's temporary tail
at `0x827CDECC..0x827CDEE4`. `TempContactBuffer::GetNextFreeContact`,
`0x827CB7E8..0x827CB89C`, ignores a Store truncation when flushing a full
50-record local buffer, then reserves local slot zero.

## Corrected typed behavior

The private append owner now keeps two different counts:

- `reservation_count`: native serialized atomic state, including overshoot.
- `published_count`: initialized accepted dense-prefix length, a host safety
  fact used to authorize slot reads and the existing sealed Contact import.

With initial counter 2, maximum 10, and request 50, Store returns start 2,
copies eight records, and leaves the native counter at **52**, not 10. A
zero-count Store leaves 52 unchanged. The next nonempty Store returns start
52, copies nothing, and resets the counter to 10. The accepted prefix remains
ten records throughout those latter calls. Exact fit reaches maximum through
the addition alone; it does not execute an atomic reset either.

The existing receipt's `clamped` field denotes the bounded COPY-count choice,
not whether the atomic counter was reset. The planner's `final_count` remains
the native raw reservation count. Unsigned addition wrap, an unrepresentable
signed difference, and a maximum outside the supported signed capacity domain
are explicit host refusals. Source-created overshoot above signed maximum is
accepted when the actual subtraction remains representable.

The root's import of the accepted dense prefix is explicitly a safe-host
ownership boundary; this correction does not prove that every native consumer
of a temporarily overshot global counter is safe. Concurrent job reservation
interleavings, aliasing copies, malformed counts, and eventual native recovery
after a final overshooting job remain outside the serialized parity claim.

## Validation and confidence

New discriminating tests exercise 2/10/50 followed by zero and successive
nonempty Stores, overshoot across the signed maximum, and a caught ground
composition panic permanently poisoning its already-reserved buffer.
Existing temporary buffer tests preserve accepted-prefix behavior while their
underlying reservation state now follows the source. Ground assembly tail
flush tests are owned by the root's real producer integration tests.

Focused validation: `cargo test -p skate2-core simulation_contact_append --
--nocapture` passed all 20 tests. The intentionally caught panic is printed by
the panic hook and its test passes. Subsequent clippy identified signed casts
in this correction; they were replaced with a checked positive conversion.
Final strict gate after that mechanical change belongs to the root integrator.

Confidence: **high** for the source-backed serialized branch/copy graph;
**exact** for the quoted addresses and independently hashed bytes. Rust unit
tests are not an executable runtime trace; concurrent native reservations and
full downstream overflow behavior remain unvalidated.
