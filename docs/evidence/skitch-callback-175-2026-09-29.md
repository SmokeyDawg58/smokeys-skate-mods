# Car-grab Motion callback175 — September29

Reported: `Motion(PendingCallback { index: 175, address: 2194306600 })` while
grabbing a car. Decimal address is `82ca7628`, EnterSkitchingBehaviour's instance
constructor. It is code-folded with FilterMotionGraphIntent; matching by address
would select the wrong behaviour. Existing constructors already retained the
correct anim_skitching Vault instance. No replacement Vault/physics owner added.

## Source checks and implementation

- Entry: Create82ca7628 allocates0x14 and zeros three floats. Begin82ca7620
  forwards to Initialize82c95de8: initial PhysOut.Animation.skitch_crouch,
  zero elapsed, reach blend=-1. Update82cb4b10 reads actual ground spline height,
  predicted contact time, base-animation time/length, and authored long/short
  reach times. First blend is unrestricted; subsequent updates cap change at
  exact float0x3dcccccd. Publishes Crouch then reachspeed as settable attributes.
- Holding: Create82ca2be8 allocates0x10, two zero floats. Begin82ca7618 forwards
  to82c95c80: retain crouch and seed absorb-speed from authored PointNegGraphData8.
  Update82cb48d0 maps current contact speed then caps against the live cvar.
  Publishes Crouch then absorbspeed. Both End callbacks are BLR825187c0.
- Full64-bit generic attribute keys verified from raw PPC (decompiler drops
  upper halves): absorb1c04030e5b00a8ea, height6e6efe7cf9bef4b8,
  longbdfd21f2a96828fd, short8738e31e171e2a87. Reused PointGraph evaluator,
  x+0x10/y+0x30, and existing generic attribute access/default-zero behavior.
- Cvars:82f88690/82f88740/82f887a0, startup slots82fd5108/5110/5114.
  Added unique registry labels for both false-by-default debug flags and
  kSkitchMaxAbsorbSpeedDelta (raw default0x3d23d70a, null callback). Existing
  debug-overlay service is reused. Normal play adds no logging.
- Adjacent conditions: IsSkitchingWithAbsorb82ca6f10, SkitchingPosition82ca7228,
  IsSkitchShimmying82ca73e0. Preserved state-before-attribute guard, regular/switch
  parity, and ISkaterAnim.GetIsMirrored (vtable+1c), distinct from PhysOut mirror.
- Pre expressions202/204..210/342 were read from raw switch8290aa90 because
  full Pre evaluator failed decompilation. Preserve lazy right-before-left
  checks for208 and expiry-before-absorb for342. Their independent Sustain
  switch entries8291e254 all select the original constant-true829221c8.
- Verified update thunks:828ef070/f0c0,828f01e0/0240/0308,
  828f1948/19e8/1a70/1ad0/1b20/1b98,828f2850/28f0/2990/29f0,
  828f3678/3718/37a0/3840. Wired their28 authored state occurrences with the
  existing packed ancestor/behaviour order, once per slot. Existing steering,
  PlayAnimation, hand ownership, EndShimmy and attribute callbacks reused.

## Validation and limits

Rustfmt parsing passed for edited integration files; new callback/kernel files
formatted. Static checks confirm all28 update addresses and active sets match
the existing graph declarations, and all960 cvars register exactly once.
Added regression tests for first-entry versus repeated reach blending, completed
animation fallback, absorb delta, hand/stance parity, and lazy expression order.
**No Cargo build or test execution, and no game launch in this task.**
Compile with build-release.bat.

This closes the reported entry callback and adjacent entry/holding wiring.
It does not establish complete end-to-end skitch physics parity. The separately
declared SkitchShimmyingBehaviour's animation lifecycle is still pending and
has not been replaced with a no-op. Its state update slots are mapped, but an
actual lateral-shimmy selection can still report its missing callback. Further
errors must be traced at their own source boundary, not suppressed.
