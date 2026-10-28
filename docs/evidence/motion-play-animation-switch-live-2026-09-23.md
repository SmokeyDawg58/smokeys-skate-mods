# PlayAnimation GetIsSwitch reached by the neutral preview

The profiled release preview completed 81 neutral frames before
`Motion(PlayAnimation(Backend IsSwitch, PendingAnimationOperation("GetIsSwitch")))`
in `artifacts/controlled-live-neutral-after-mirror-103-120-2026-09-23.log`.
An unprofiled replay reached the same query after one completed frame, so the
frame number is not a native timing claim.

IDA `SkaterAnim::GetIsSwitch` at `0x82cb7580..82cb758c` reads retained
`mRelativeStance` at `+0xe60` and returns whether it equals one. The existing
Rust animation player initializes and toggles the same `relative_stance` and
already uses `==1` for the physics output's riding-switch value. The Motion
PlayAnimation backend now reads that retained field from the same animation
owner instead of returning a pending operation. No second stance producer was
introduced. A fresh live replay is required to validate the bridge.
