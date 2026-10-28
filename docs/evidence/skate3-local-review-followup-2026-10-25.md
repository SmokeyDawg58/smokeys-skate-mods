# Follow-up after owner reported no improvement in local preview 6

The installed EXE hash and the pinned game icon both identify preview 6 at the
requested E: path. The owner's report is a failed gameplay review, regardless
of the earlier successful compile. Shared publishing remains held.

The owner authorized short focused diagnostics on 2026-10-25. A copied-save
280-tick run completed in 11.66s total, with 4.67 simulated seconds in 5.04s
of admitted gameplay. Source Skate 3 physics was present in the trace.
The 120-tick main-menu diagnostic ended before the authored camera settled;
its failure is an insufficient observation window, not proof of a new crash.

## Corrected paths

- Menu ownership is checked explicitly, including pause/options/map/profile,
  hub, popup and career overlays. The earlier pause-stack-only predicate did
  not describe every frontend that takes local input while online simulation runs.
- Native Actor pin policy intentionally retains Derived controller history
  (`controls.rs` skips Fill). A menu now consumes neutral input through that
  update before the retained Actor policy filters intents. ForceBrake stays
  active; old axes are not held until unpin. Real script pins keep their semantics.
- The observer publishes source reckoning up (`Physical.reckoning.vector_96`)
  for the host Up contract, matching TU3 AirCollector82DA81E4's perpendicular
  takeoff speed calculation rather than using visual skeleton Y.
- Source mode now uses its TU3 spin accumulator82DA8BE0/82DAC880/82DAC9D0,
  including board projection on reckoning up and trick-active tracking. The
  host HUD/sequence events and authored TU3 values remain in place. The native
  S2 spin path is retained for S2 mode.
- Two direct tests of the actual source rotation module passed in under a
  second: 1080 degrees through heading wraps, and exclusion of board flick spin.
- Fast menu camera transitions are an explicit requested PC policy: a
  camera-only take takes at most 0.35 seconds in full S3 mode. Physics, skating
  animation, replay speed and the camera's authored path are not accelerated.

The opt-in local review controller checks actual steering, neutral source
axes under the online menu, subsequent input recovery, and replay elapsed
seconds against wall time. It is never constructed in ordinary play and uses
copied profile data plus a local server for the online menu check.

Exact whole-game/retail scoring parity is not established by these limited
checks. The owner must approve the local result before a shared release.
