# Actual ordinary channel removal producer

Fresh IDA decompile `RemoveChannel82d7d8a8`, independently aligned with
`animation-channel-removal-live.md`, 2026-09-20. New unregistered source
`actor_animation_channel_remove.rs` intended construction child `channel_remove`.
No registered edits or Cargo. Existing actual pool `runtime.clear()`, free vectors,
channel payload, retained body reference and diagnostic mirrors are reused.

Method `try_remove_channel(&FastString36)->Result<(),GestureChannelError>` returns
without mutation on dictionary miss. Found slot appends free_channels d914 BEFORE
capacity assertion d94c; clears actual channel tree d968; appends free_trees d994
BEFORE assertion d9c4. Inline transitioning branch d9dc currently returns explicit
missing-producer error after that committed prefix, before dictionary erase.
No fabricated inline destination or body transition slot is substituted.

Ordinary path erases dictionary da7c then reads base da80, parent da88. Middle
updates parent.base dadc, base.parent dae0, parent runtime.base dae4 in that order.
Bottom updates parent.base=None dab8 before resolving actual GetBaseAnim and
publishing its actual reference dad0. Root updates base.parent=None daa4 before
root daa8; single sets root=None da9c. Root diagnostic mirror updates alongside
the actual root. Final source stores dae8..dafc null both tree handles, clear
transitioning, parent/base, then priority0. Runtime animation, clocks, flags,
coefficient, evaluated and transition payload are preserved stale, as native.

Dictionary API coordinated with foot_ik: replacement ChannelDictionary retains
get/remove/iter interface; remove unlinks exact native chain and does not rehash.
Producer erase is already at native source point. Root handles ordered expiry
traversal and replaces old refusal only after dictionary mount.

Optional cfg(test) `check_remove_channels_corpus()` runs LAST on disposable
post-teardown Actor after existing six-channel + body/blend checks. It verifies
missing no-op, middle/bottom/root/single relinks, native free append ownership,
real pool clearing, preserved stale animation/flags/time, and final root mirror.
Needs >=3 existing ordinary channels and a retained body for bottom relink.
No fake animation/tree fixture and no permanent-runtime state pollution.
