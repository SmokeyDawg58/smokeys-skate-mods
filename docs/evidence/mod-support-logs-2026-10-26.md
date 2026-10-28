# Shareable mod crash and disconnect diagnostics

The owner asked for logs friends can send after online boots and hard crashes.
This extends the existing bounded native journal and panic hook, not a separate
monitoring application or service. Ordinary launches store sessions under
`Mod Logs/sessions` beside the executable. Diagnostic save-root overrides keep
their reports entirely in the isolated scratch profile. `LATEST.txt` and a
short readme explain which folder to zip; there is no automatic upload.

Client transport disconnection and game-loss paths save the actual reason or
notice, in-game/search/location context and recent journal immediately, before
the session clears its state. This does not require console/online tracing.
The existing journal still bounds producer queues, rolling chunks and its
recent-history buffer. No transport, matchmaking, scoring or recovery behavior
changes. Logs do not add tokens, session keys, account/chat contents or arbitrary
environment dumps.

The existing panic hook retains source/build identifiers, GPU, thread, reason,
backtrace and recent history. Fatal non-panic errors now write reports on all
platforms, and GPU warning/error logging is enabled for the journal. A scoped
session marker distinguishes clean exits, reported failures and abrupt exits.
An OS-held per-run file lock prevents another live instance from being marked
as crashed; it releases automatically on termination. The next launch marks an
unclosed/unlocked session as an unexpected exit. Native aborts, forced closes
and power loss may not provide a backtrace or exact cause; their already-written
context remains. This is not a native exception/minidump implementation.

Seven isolated helper processes exercise clean exit, a synthetic disconnect
snapshot, process exit without unwinding, subsequent recovery marking, panic
backtrace, and fatal error reporting. They do not run the game or open an audio
device. No resident task exists outside the game process.
