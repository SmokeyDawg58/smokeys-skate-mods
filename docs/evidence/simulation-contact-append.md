# Collision Contact append and temporary buffer

Date: 2026-09-05 (Europe/Helsinki)

This checkpoint reconstructs the shared append operation used by every
non-network collision Contact producer and the 50-record temporary buffer used
by the assembly/assembly and assembly/ground families. It is private typed
storage only: collision candidate enumeration, Contact construction, body-role
binding, island partitioning, and the production frame coordinator remain
separate boundaries.

## Provenance

The authoritative packet is
`world_probe_temp\contact_producer_seal_evidence.md`, SHA-256
`9EC8914EABBC001C7AE3AD160977E0DA27D5B1F6A721559769C31014D9DB196E`.
The frozen Store and pipeline-route raw extracts hash to
`478E15E54306A10BC2EF82F32E0B4EC1CD0AEE09B5D57A5CE430F4479167C29A`
and
`B83E8B2E73B9D20463279B86C02B4E74579FF8C40F7F066CAC8FB6FE39931548`.
The reference XEX, PDB, and linker map hash to
`3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`,
`0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`,
and
`51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.

## Recovered append behavior

`GlobalBuffer<Contact, ExternalMemory>::Proxy::Store` is
`0x827cb5c8..0x827cb678`. For nonzero requested count `q`, it performs an
atomic add, derives the pre-add `start`, and tests `maximum - start` as a
signed 32-bit value. A nonpositive remainder accepts zero and clamps the
counter to the maximum. A positive remainder accepts all `q` only when
`q < remaining`; otherwise it accepts the fitting prefix and separately
clamps. Exact fit therefore takes the clamp route. Accepted records retain
source order and use a `0x100` stride. When source and computed destination are
the same address, the copy is skipped. The function returns `start`, including
when zero records fit. The unique `q == 0` route returns zero and performs no
atomic or copy operation.

The Rust owner preallocates the complete logical destination and reproduces
the same ordinary valid-count accepted prefix, final count, and return value on
the non-aliasing producer route. The private planner separately preserves and
tests native's destination-alias no-copy decision; the owning destination does
not expose an operational aliasing source. For nonzero requests it rejects
`maximum > i32::MAX`, `count > maximum`, requested lengths not representable
as `u32`, and `u32` post-add wrap as safe-host policy. It serializes Store calls through exclusive
ownership; this does not claim that native's AtomicAdd followed by a separate
clamp store is a linearizable concurrent-container algorithm, nor does it
invent the runtime order of colliding producer jobs.

## Recovered 50-record local behavior

`TempContactBuffer::GetNextFreeContact` is
`0x827cb7e8..0x827cb89c`. Its layout is a count at `+0x00`, 50 complete Contact
records at `+0x80..+0x327f`, a copied Proxy at `+0x3280..+0x3288`, and the
`mCanFlush` byte at `+0x328c`.

Below 50, native increments the local count before returning the selected
slot. At 50 or above with flushing disabled it returns null without changing
the count. With flushing enabled it Store-publishes the complete local count,
ignores any global accepted-prefix truncation, resets the local count, and
then reserves slot zero with count one. Job-tail callers publish only a
nonempty local prefix and reset it afterward.

The typed host buffer retains that reserve-before-fill chronology but does not
expose uninitialized memory. It binds the copied destination identity at
construction. A reservation token carries its buffer identity and a checked,
non-reused generation; its reserved slot must be initialized before a flush and becomes stale
after discard/re-reserve or reset. The private discard-last operation models
the decrement used after `Contact::IsValid` rejects an assembly/assembly
candidate. The specialized 80-record skater/DMO cache performs the same local
count decrement through its separate storage. Assembly/ground does not call
that predicate or discard path; this module does not normalize the families
into one admission rule.

The private Assembly/ground write port now narrows this storage into a
non-generic reserve-before-compose operation. A null slot does not invoke its
infallible final-value closure, while a successful reservation keeps the slot
token hidden through the complete write. The port has no production issuer
yet and exposes neither flush nor discard authority. See
`docs/evidence/assembly-ground-contact-write-port.md`.

## Producer census and next boundary

The complete Store call set contains:

- `0x827cb84c`, the shared temporary-buffer mid-flush;
- `0x827ccb08`, the assembly/assembly tail flush;
- `0x827cdee0`, the assembly/ground tail flush; and
- `0x827cf70c`, the specialized skater/DMO tail flush.

The first ordinary ground-contact record builder is
`AssemblyTriangleIntersectionTester::RunIntersectionTests` at
`0x827cd1e0..0x827cd7c8`. It consumes `Sk8ComputeContactPoints`, fills a
complete Contact through this 50-record buffer, and intentionally has no
`Contact::IsValid` filter. Reconstructing that builder requires its geometry
inputs and four independent occurrence-qualified roles; Store alone does not
authorize arbitrary Contact values.

The free-skate route launches `SkaterGroundCollisionPipeline` first from
`Island::Step_Collision1` and synchronizes it first in `Step_Collision3`.
Evidence proves that it uses the skater Assembly pool and this ground pipeline,
but board-specific attribution still requires an independent trace of the
deck/truck/wheel Assembly registration into that pool.

## Verification

Seventeen focused tests bind storage to the complete `0x100` Contact input and
distinguish zero-before-validation, under-capacity, exact-fit,
overfull-prefix, already-full, planner-only alias-no-copy, zero-maximum, and
invalid host routes. They also cover slot 49 to count 50, the full no-flush
null route, a full mid-flush followed by slot-zero reservation, tail and
mid-flush global truncation, invalid-candidate count rollback, cross-buffer and
discard/re-reserve token ABA, non-last discard, foreign destinations, and
refusal to publish an uninitialized or post-reset stale slot. Exhausted
reservation identity is refused before a required mid-flush.

Four of those tests specifically freeze Assembly/ground no-slot/no-compose
behavior, later-null prefix retention, full-buffer flush-before-new-slot
behavior, and the absence of a ground validity discard.

The audited `simulation_contact_append.rs` source hashes to
`09A4995CA3399DC5978BEF2C4B37055848084ED19CF1001BEBF4068C0A823C12`.

## Explicit nonclaims

- This append checkpoint itself implements no collision candidate enumeration,
  manifold reduction, or material/contact-value construction. The complete
  private `Sk8ComputeContactPoints` value path now exists in its separate
  evidence checkpoint but is not wired here.
- No arbitrary public Contact constructor or producer exists.
- The four table/RF roles remain independently validated future inputs.
- Native cross-job dense order and overlapping overflow races remain unknown;
  deterministic serialized ordering is host policy.
- The specialized 80-record skater/DMO cache and the producer-specific
  validity predicates remain separate next steps.
- This does not yet connect the prepared Horus owner to a production World
  frame.
