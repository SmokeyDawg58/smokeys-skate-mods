# Session marker availability during a Skate 3 bail

Integrates the owner-supplied `3-skate3-marker-during-bail.patch` after the
on-foot marker and prop collision patches. Both earlier patches are retained.

The Skate 3 physical-output bridge published `is_wiping_out` while leaving
`can_leave_wipeout` stale. The session marker's rule check rejects a wiping-out
player when that second value is false. The native wipeout timers that normally
publish this permission are not the producers for the Skate 3 frame bridge.

The supplied patch explicitly publishes `can_leave_wipeout = true` alongside
the Skate 3 wipeout status. It preserves the other marker rule checks, including
challenge, HOM and pending-reset checks. The ordinary Skate 2 output path is
unchanged. This is a PC bridge policy, not a new claim of console parity;
runtime behavior for this delivery remains unverified.

The existing marker-return-after-bail corpus gains an assertion of the output
permission during a real air bail. Its case is retained in the normal future
automatic validation list. At the owner's request no gameplay or regression
tests are executed for this release, and skipped checks are not marked passed.
