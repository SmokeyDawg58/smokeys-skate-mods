# Basic skating continuation, September24

Active objective remains the first usable controller skating test; source
parity is evidence-bounded and not yet numeric Xbox per-tick parity.

## Verified corrections and replay results

- Kickturn: use native strings Balance/Spin (not C++ global variable names)
  and collection A168897919752875 (not reversed halves). Original-data45-frame
  replay checks nonzero physical attributes and substantial board-part6 rotation.
- PrepareLand: Global+0xd4 animation provider class BABB3EC110849E08 replaces
  conditioner-animation class. Fresh60-frame Ollie launches and lands.
- HandBusy: actual ISkaterMotionGraph vtable82069688+30 resolves
  GetGrabStateWrite825fdfb8. Existing Motion counters are BS0/FS1; the previous
  animation allocation-word implementation was removed. FS80-frame held/released
  replay now passes, including the counter assertion. Right-trigger replay is
  newly authored against the same helper and awaits the next build.
- Shared Action Up/Down/horizontal BoardAdjust, Running/Dismount/OBAir leaves,
  required IsDoneAirDismount animation lookup, and29 Motion flip-family states
  were recovered/reviewed in batches, reusing existing behavior implementations.
- Pre95..107 use HasIntent173..185. These preceding candidate checks are needed
  even when the requested trick is a later nollie flip. Native direct case
  blocks8290d53c..8290d9e0 and constant Sustain829221c8 independently reviewed.
  All4866 nonignored core unit tests pass before the next Air child integration.

Logs: `artifacts/astra-original-ollie-steering-20260924.log` passes both;
`artifacts/astra-basic-routes-20260924.log` passes pushing and300-frame bail/recovery,
but initially fails a stale Action-parent assertion in the grab test and the
flip-family continuation. Correct Action leaf138 replaces parent73 in the
assertion; `artifacts/astra-family-candidates-replay-20260924.log` then passes
the full grab replay. All four flip families reach physical Air and their cycles,
then request physical502 at44/48/50/48 with release38.

Shorter release matters: `artifacts/astra-kickflip-release24-20260924.log` passes
the existing Kickflip continuation probe. Release30 still requests502 at44.
Thus long-hold runout is not evidence that every ordinary flip landing is broken.
The grouped four-family probe now defaults to release24 and accepts
`SKATE2_FLIP_FAMILY_RELEASE_STEP` for held/released coverage; this change awaits
the next build. The longer runout path still must be implemented, not suppressed.

## Remaining producer dependencies

Physics502 has constructor storage but needs its complete Enter/Update/Post/Fill
and transitions. Offboard selector draft reuses the actual existing predictor
queue, collision batches and future publication; it does not manufacture ready
results. Air feet/root draft reuses Ground services with source-specific order,
and fixes the existing root translation's COM ownership. LandingOnDeck runtime
and live bindings remain necessary. Walking likewise requires actual StepSelector
geometry/futures and the next-frame trajectory consumer, not only Motion778.

Root's `physical_player_landing_on_deck_math.rs` is an unregistered draft containing
only the independently reviewed optimal-time82dc8de8 leaf. It is not compiled or
a mounted manager. Agent artifacts remain unapplied until root source review and
integration; do not reconstruct duplicate versions of those existing drafts.

## Host speed

The current live window caps to one1/60 simulation tick per redraw and discards
overdue slots. Release profiling previously measured about34ms simulation plus
drawing, above16.7ms total. The latest human log confirms dropped slots. This
causes actual gameplay slow motion and must be addressed for meaningful normal-speed
controller evaluation without changing native simulation tuning.

The goal tracker now reports active on the September24 continuation. The earlier
attempt to replace the paused unfinished goal was rejected; no false completion
was used.
The reviewed Air combined and offboard selector patches are now integrated;
their core test batch is running. Full state502 lifecycle remains in progress.
