# Testing logs

Ordinary no-argument game launches and `--live-skating` runs automatically
create `saves/test_logs/run-<timestamp>-<pid>/` under the working directory.
`SKATE2_SAVE_ROOT` moves the save directory; `SKATE2_TEST_LOG_ROOT` overrides
only the logging directory. Logging remains enabled in the quiet game.

Each run contains:

- `build.txt`: public source fingerprint, Git revision, build time, compiler,
  target/profile, executable and save paths. The source fingerprint includes
  modified and new Rust sources, including the captured Skate 3 source.
- `executable.sha256`: hash of the actual running EXE, calculated in the
  background. A very early termination can precede completion of this file.
- `events-*.jsonl`: rolling journal with sequence, elapsed milliseconds,
  thread, event kind and diagnostic detail. Preserve the folder as a unit.
- `failure-*.jsonl`: immediate recent-history snapshots for errors, panics,
  detected nonfinite movement output, or a stage that stops progressing.

The journal records raw controller polls before conversion, native tick
boundaries, action/motion/physics IDs, board position and speed, manual and
offboard flags, mode and difficulty, frontend/settings/pause/replay/editor
state, native presentation stages, gesture-set changes and independent D-pad
assignments, and verified save commit outcomes. The currently integrated
Skate 3 solver also records completed transforms/velocities, state/category,
ground/collision publication and pose validity each tick, plus its full
physical output once per 60 solver ticks. This instrumentation follows the
current integration and must move with the physics owner during migration;
it is not evidence that source integration is complete.

Disk writes run on a dedicated thread and flush to disk every second. The
simulation producer uses a bounded nonblocking disk queue. A heartbeat reports
lost journal records if that queue fills; the separate recent-history ring
still retains them within its bounds. Events roll through eight 8 MiB files;
the memory ring retains up to 32,768 records and 16 MiB. Up to eight failure
snapshots are retained per run. Logs from older runs are preserved.

A watchdog checks explicitly active stages without taking any game/session
locks. After ten seconds in one stage it saves the active stage, thread,
duration and recent events. It repeats at most every thirty seconds while
that stage remains stalled. A long asset load can also produce this report;
`stage_stalled` means slow work or a possible hang, not a proven deadlock.
Idle time outside monitored work is not classified as a hang. Returned
errors and Rust panics save history immediately, even before normal shutdown.
Panics additionally retain the existing forced-backtrace crash report.
Termination without unwinding can lose roughly the latest flush interval;
power/storage failures and uninstrumented third-party faults have the usual
limits. A log write failure is reported to stderr rather than halting SIM.

After closing the game, collect a single report with:

```powershell
powershell -NoProfile -File tools/collect_test_logs.ps1
```

Run it from the game's working directory, or pass `-RunDirectory` pointing
to a particular `run-*` folder. The script compresses only diagnostic files
and refuses to overwrite an existing report. It does not include game assets,
the EXE, the player save, or the private network build secret.

Inspect a folder or collected report without launching the game:

```powershell
python tools/summarize_test_log.py path/to/report.zip
```

The summary reports retained event counts, last observed input/state/stage,
errors/stalls, dropped records, and any partial JSON lines left by termination.
Counts cover the retained rolling window rather than necessarily the whole run.

Validation is recorded in `docs/evidence/pc_skate3_runtime_integration.md`.

Source integration performance records additionally distinguish terrain decode,
terrain world construction, grind refresh, grab refresh and the source tick.
`slow_stage` records a scoped stage lasting at least eight milliseconds; this
is solely a logging threshold and does not alter physics timing. Terrain
refresh records include retained/redecoded mesh counts and triangle counts;
grind refresh separates payload conversion from source octree construction.
