# Asynchronous stream-file loader

This record covers the mount-backed production `InnerStreamBackend` boundary.
It preserves the recovered `xcore::FileLoader` admission and release behavior
while expressing host work as safe Rust jobs. It does not emulate Xbox device
threads or reproduce their seek optimizer.

## Reference evidence

| Address | Function | Recovered behavior |
|---:|---|---|
| `0x82973630` | `xcore::FileLoader::ASyncLoad` | fully initializes an operation, appends it to the pending tail, then starts admission |
| `0x8296fa88` | `AllocAsyncLoadOp` | obtains one of exactly 128 operation records |
| `0x82972f30` | `StartNextLoad` | admits the pending head while fewer than three operations are active |
| `0x8296dc58` | `ASyncLoadOp::GetStatus` | reports pending/busy, error, complete, or cancelled status |
| `0x82975470` | `FileLoader::ReleaseASyncLoadOp` | decrements the active count and then admits more work |
| `0x82975510` | `ASyncLoadOp::Release` | transfers terminal data and releases operation ownership |
| `0x82bbcaf0` | `Device::ThreadEntry` | performs I/O and publishes terminal status on a device worker |
| `0x82d5db60..0x82d5de40` | `cGameStream::Update` | polls, transfers bytes, parses/registers on the caller, then starts the next manifest read |

PDB layout fixes `xcore::FileLoader` at `0xd080` bytes with 128 embedded
`0x1a0`-byte operations, a free vector at `+0xd000`, pending list at
`+0xd010`, locks at `+0xd018/+0xd048`, and the active atomic at `+0xd078`.
The reference load worker is distinct from the filesystem device threads.
Filesystem callbacks only make operations terminal; manifest parsing and
registry mutation occur later on the polling load thread.

## Native Rust boundary

`MountedStreamBackend` owns a fixed 128-operation scheduler and a mandatory
`StreamRegistrationSink`. Admission is FIFO and never exceeds three active
leases. `begin_read` publishes only a fully initialized operation. Three host
workers exclusively call `MountSet::read_path`; `poll_read` is nonblocking and
all parsed XSM/XST/XSS/XML plus unload callbacks remain synchronous on its
caller. Each callback forwards the same stable file identity, stream kind, and
exact retained paths supplied by the owning `StreamFile`; the backend neither
reconstructs nor normalizes that context.

Completion alone does not free a lease. Complete and failed operations continue
to occupy their active slot until the owning handle is dropped. This composes
with the reconstructed `cGameStream`: XSM and XML terminal paths clear their
handles and admit queued work, while the exact XST/XSS failure stalls retain
their handles and therefore retain an active slot. Dropping an in-flight host
handle is an explicit PC abandonment policy: its lease remains active until
the worker finishes.

Requests preserve the original path bytes and require the proven stage flags
(`1` for XSM/XST and `0` for XSS/XML), `0x10` alignment, no interior NUL, and a
maximum of 255 path bytes plus terminator. `MountSet` does not yet implement the
reference `bigFile|path` namespace, so that form is rejected explicitly rather
than guessed. Archive lookup, I/O, RefPack, and size errors become the native-
shaped sticky file failure; scheduler poison, worker panic, pool exhaustion,
foreign handles, and invariant corruption are typed backend errors.

## Confidence and deviations

Confidence is **exact** for the 128-operation pool, three-active cap, FIFO
FileLoader admission, release-gated next admission, request flags/alignment,
and parsing/registration thread split. The safe handle lifetime correspondence
is **high**.

Using exactly three host workers, joining them at shutdown, abandoning an
in-flight handle without cancellation, and returning typed scheduler errors are
explicit PC policies. Backend destruction and `into_registration_sink`
synchronously join workers. Because cancellation is not reconstructed, neither
has a finite completion guarantee if a host read never returns. The defensive
internal `ShuttingDown` status is not ordinarily observable through safe public
use.

Native device seek/location reordering, cancellation, `bigFile|path`
resolution, guest allocator alignment, and production archive mount timing
remain **unknown**. No claim is made that host thread scheduling is identical.

Twenty-three focused tests cover pool and admission limits, FIFO order, terminal
lease retention, all pending/admitted/reading abandonment states, XSM/XML
release admission, XST/XSS sticky retention, one-shot byte transfer, worker and
poison faults, ID wrap/foreign handles, shutdown waiting, exact requests, and
caller-thread registration/unload callbacks. A final independent review passed
after the missing cross-module XSM/XML and shutdown-liveness coverage was added.
