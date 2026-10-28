# Assembly/ground reserve-first Contact write port

Date: 2026-09-06 (Europe/Helsinki)

This checkpoint reconstructs the narrow mutation boundary entered by
`AssemblyTriangleIntersectionTester::RunIntersectionTests` at
`0x827cd4e4..0x827cd4fc`: obtain one temporary Contact slot, return an explicit
unavailable signal which the future Run owner must propagate to stop the
entire invocation, and only then begin final Contact composition. It does not
yet issue the port from a live collision frame or connect the complete Sk8
point list and final-value compositor.

## Authority

- Xbox 360 executable SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`
- full PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`
- linker MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`
- complete Run raw packet SHA-256:
  `B4FC2DF4223001B753A1B702BDB9478DF281C4CD9108B1C925A40273018C3A8E`
- full `0x827cd1e0..0x827cd7c8` native span SHA-256:
  `81C7E79EA8038EC1A61F45F03A3DC203A18B7742EFCA214EBA493C83B9FE4983`
- audited `simulation_contact_append.rs` source SHA-256:
  `09A4995CA3399DC5978BEF2C4B37055848084ED19CF1001BEBF4068C0A823C12`
- audited `simulation_contact_jacobian.rs` source SHA-256:
  `93230F8EFEB096524AB99230F5C3BE4E33272B4FA488F7E3B5E0E73E021908BB`

The latter file changes only the `#[cfg(test)]` sibling visibility of its raw
word constructor and inspector. Its production compositor is unchanged.

## Native chronology and typed boundary

For every accepted Sk8 point, native calls
`TempContactBuffer::GetNextFreeContact` at `0x827cd4e8`. A null result branches
at `0x827cd4ec..0x827cd4f0` directly to the whole Run epilogue. Native neither
composes that point nor visits later points, volumes, or triangles in that Run
invocation. Earlier initialized temporary records remain present. A successful
reservation is followed by the two `dcbz128` operations at
`0x827cd4f4..0x827cd4fc` and then the final Contact construction.

The private `AssemblyGroundContactWritePort` accepts only an infallible closure
that returns one complete opaque `SimulationContactBuildInput`. It first calls
the already-proven temporary-buffer reservation operation. `Unavailable`
returns without invoking the closure. Success invokes the closure while the
fresh generation-bound slot remains exclusively hidden, then consumes that
slot with the complete value. The port exposes no destination, temporary
buffer, slot token, generation, flush receipt, tail flush, or discard
operation.

At a full 50-record local buffer, the inherited native behavior remains:

- with flushing disabled, return unavailable with the complete local prefix
  unchanged;
- with flushing enabled, submit the old prefix to Store, which may publish only
  its fitting prefix, ignore that receipt, reset the local count, reserve slot
  zero with count one, and only then invoke the compositor.

The ground port has no `Contact::IsValid` call and cannot access
`try_discard_last`. This preserves the producer-family distinction: native
Assembly/assembly can decrement after invalid admission, while
Assembly/ground does not do so.

## Safe-host policy

Destination/generation/storage errors are checked before the closure runs and
have no native-valid counterpart. Once a reservation succeeds, the private
token cannot be invalidated through safe code before the write. A composing
panic would leave a reserved uninitialized slot; a future port issuer must
treat such unwind as terminal or poison the owning job. It must not invent a
ground rollback through the discard operation.

## Discriminating tests

Four focused tests prove:

- a full non-flushing buffer returns unavailable without composition and keeps
  all 50 earlier records;
- after 49 earlier records, one new point is written and a later unavailable
  reservation retains that complete 50-record prefix without rollback;
- a full flushing buffer publishes its old 50 records and places the next
  value in newly reserved slot zero;
- material words that a normalized validity policy could reject are still
  retained and published unchanged by the ground port.

The lower-level temporary-buffer suite separately covers global truncation,
foreign destinations, generation exhaustion before a required flush, stale
and reused tokens, and refusal to publish uninitialized slots.

## Remaining boundary

No production constructor or issuer exists for this port. The live owner must
still validate and retain four independent occurrence-qualified roles, call
Sk8, iterate its returned points in order, acquire the final live body/table/RF
values only after reservation, propagate `Unavailable` to the whole Run
epilogue, and perform the single job-tail flush. Typed runtime
`SerialisedAssembly::VolumeData`, `ExtraVolumeData`, `GroundData`, and their
real `CreateGPInstance`/body ownership remain prerequisites; this checkpoint
does not replace them with callback-shaped stand-ins.
