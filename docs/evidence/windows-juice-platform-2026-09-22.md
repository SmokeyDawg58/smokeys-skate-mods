# Windows Juice platform mapping

Confidence: high for the recovered native control flow; explicit PC platform
policy for networking and identity. This packet does not establish a complete
application session or recover successful Juice protocol handshaking.

## Ownership and source

`windows_juice_platform.rs` implements the existing `JuiceBootPlatform` contract.
It supplies the same retained boot/remap owner documented in
`juice-input-boot-admission-2026-09-22.md`; it adds no readiness flag or input
owner. The shared patch exports only that platform contract and its value types.
The application session caller remains pending, with a narrow module-level
dead-code expectation until that caller exists. The already connected core
queued-input disabled branch remains the acceptance path for boot/remap behavior.

Native `JuiceBase` construction calls network initialization even when the
configuration disables Start. `XNet::Initialize` at `82a74660` combines
`XNetStartup` and `WSAStartup(0x202)` results. `UnInitialize` at `82a746e8` calls
`XNetCleanup` at `82a746f4`, then `WSACleanup` at `82a746f8`, ignoring status.
PC has no Xbox XNet startup counterpart: this adapter explicitly maps network
initialization to actual Winsock 2.2 initialization, without inventing an XNet
success result or binding a dummy socket.

`EstablishConnection` at `82a74500` creates an IPv4 TCP socket and performs a
blocking connect. `Connect` at `82a766a8` first rejects `inet_addr`'s all-ones
result. The adapter admits canonical dotted IPv4, rejects the all-ones address
without opening a socket, and uses an actual blocking `TcpStream::connect`.
Unsupported address syntax is a typed admission error, distinct from actual
connection failure. Legacy `inet_addr` textual forms remain unsupported.
Actual connect failure retains its `io::Error` and returns the existing failed
connection result. No timeout or retry policy is introduced.

Endpoint/debug identity is explicitly supplied; disabled Start never requests
it. Missing identity is an error for enabled startup, not an invented local
address/name. Native endpoint selection and failed-connect 2000 ms wait remain
in the existing core owner. Successful transport retains the real stream and
still ends at that owner's explicit handshake-pending boundary.

## Safe dependency and lifetime

The Windows-only dependency is pinned `rustix = 1.1.4`, with `std` and `net`.
The exact cached source `rustix-1.1.4/src/net/wsa.rs` was inspected: its safe
`wsa_startup()` requests `0x202` and succeeds only when `WSAStartup` returns zero;
its safe `wsa_cleanup()` performs the actual cleanup. Repository
`unsafe_code = forbid` remains unchanged. See the
[rustix API](https://docs.rs/rustix/1.1.4/x86_64-pc-windows-msvc/rustix/net/fn.wsa_startup.html)
and Microsoft's [Winsock lifetime contract](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsastartup).

Each successful initialization owns one `Arc` lease. Failed initialization
creates none. The platform retains its lease through disabled startup; the
eventual session caller must retain the platform for its desired lifetime.
Every successful connection clones that lease. The connection's stream field
is declared before its lease, so stream destruction precedes releasing its
lease. The private stream is exposed only by a shared borrow, preventing direct
replacement or ownership transfer of that field. Standard-library socket clones
remain possible and use the standard library's independent networking lifetime;
the accessor does not claim to enforce lease association for such clones.
Final lease destruction performs exactly one cleanup for this adapter's
successful startup. Other standard-library/process startup references are not
claimed to be cleaned up by this lease. Cleanup status is ignored as in the
source. The dependency's failed-startup error-number behavior is not claimed
as source parity; the native caller consumes only the success boolean.

## Validation and remaining boundary

Focused tests exercise real Winsock initialization and final lease lifetime
without connecting, plus explicit identity/name status preservation. They do
not call `connect`. The author has run formatting and `git apply --check` only;
Cargo and execution belong to the root integrator. No remote connection has
been attempted during development. The original enabled INI is unchanged;
explicit host disable remains distinct from its actual `enable=1` value.

Artifacts: `artifacts/windows-juice-platform-2026-09-22.patch` and its Python
generator. Independent source/code review approved the source mapping,
dependency, lifecycle and shared patch; no reviewer Cargo or network execution
was performed. A production selected-session caller
and successful Juice handshake/device registration remain separate work.
