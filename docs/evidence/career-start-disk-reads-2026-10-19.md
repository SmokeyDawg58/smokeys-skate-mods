# Career start on a hard disk: reads on the tick threads — October 19, 2026

On a hard disk the first seconds after a career start stuttered. A tick runs on
the SIM thread, and the window thread consumes it before the next one starts,
so a file read on either thread holds the simulation until the disk answers.
At a career start the disk is busy: the city's models and textures
(`city-resource-loader`, about 90 MB), pedestrian recipes and AI skaters all
load at once, and a read on a tick thread queues behind them.

## Measurement

- `SKATE2_TRACE_READS`: every archive read (`MountedBig::read_entry` and
  `read_entry_range`) on stderr with its thread, size and time. On the threads
  the variable names (`main,sk8 SIM`) the read's stack is printed as well. The
  hook is `skate2_formats::mount::observe_reads`; the observer is
  `diagnostics::read_trace`.
- `SKATE2_DIAGNOSTIC_READ_DELAY_MS`: every archive read takes that much longer,
  one read at a time, as on a busy single-spindle disk. The runs below used
  15 ms.
- New Career on a scratch save (`--live-controls-demo career` with
  `SKATE2_DIAGNOSTIC_PROFILE_NAME` and `SKATE2_DIAGNOSTIC_MOVIE_SKIP`),
  `SKATE2_TICK_TRACE` and `SKATE2_PRESENT_TRACE`, measured from 2 s after the
  career start to the end of the run.

## Reads on the tick threads after a career start

| Thread | Reader | Files |
| --- | --- | --- |
| window (inline ticks), SIM | `MountedCollectionProducer::poll_parser`, resident collection streaming | the `cSim_*_high.xsf` tiles around the start, and each mission's `cSim_Global.xsf` as it activates |
| window | `VisualIndicatorView::frame`, first draw | `visual_indicators_sk82.rx2`, `vitextures.rx2` |
| SIM | `plugin::load::load`, a prop's spawn | `livingworld\PluginDescriptor\*.stategraph` |
| SIM | `AiRequest::LoadPathPackage`, a challenge's start | `data\content\aipaths\AIPATH_Challenge_*.rx2` |
| SIM | `MountedPlayerRecipeLoad::try_load_selected_inner`, an ambient AI skater's spawn | its marquee recipe (`data\content\recipe\marquee\*.recipe`) and face (`anim.big` `data\anim\faces\*.abin`) |

With a warm file cache these reads take well under a millisecond and the ticks
stay even. With the simulated busy disk the mission collision loads gave
46–89 ms ticks, and the first frames after the opening scene hitched by 95 ms
and 61 ms.

## Changes

1. **Collection streams** read on the stream reader
   (`streaming/stream_reads.rs`): one "sk8 stream read" thread runs reads in
   queue order, as `cAssetStreamer`'s device thread does.
   `MountedCollectionProducer` queues the length probe and the next 256 KB
   stage there, and parses a stage on the first poll after it has arrived
   (`MountedCollectionProducerPoll::Reading` until then). A traced run
   (`diagnostics::traced_run`: a lean trace or `SKATE2_TRACE_SELECTED_STATES`)
   and the unit tests read on the polling thread as before.
2. **AI path packages**: `LoadBinaryPathPackage` (0x824b8f28) queues an arena
   load (`cAssetActivationManager::LoadArenaAsset`) that arrives frames later.
   The package now reads on the stream reader and joins the path database once
   it has arrived (`deliver_path_packages`). The challenge asset load already
   waits in `WaitingForAiPaths` for `AreStaticAIPathsLoaded` (0x825b2c50).
   Traced runs read in place.
3. **Plugin descriptors**: `LivingWorld::Manager::PreLoadStateGraphs`
   (0x82725480) loads all ten `livingworld/PluginDescriptor/*.xml` behaviours
   (`BehaviourMan::Load`, 0x82724c00..0x827254e4) before `InitPropData`. The
   world startup now does the same, so a spawning prop finds its descriptor in
   the cache.
4. **Visual indicators**: `VisualIndicatorView` reads and parses both files on a
   "sk8 indicator load" thread from its construction. The first frame after it
   finishes adds the splines and uploads the textures.
5. **AI skater spawns**: `AI::SkaterAsset`'s `CharacterRecipeLoader` streams each
   held recipe on an "sk8 AI load" thread before any skater spawns with it
   (`try_prepare_marquee_scene`). That thread now also keeps the parsed recipe
   and reads the face (`LoadFaceData`'s `anim.big` entry,
   `CharacterFaceManager::archive_path`), and the spawn's selected load uses them
   (`PreparedRecipe`) instead of reading them again on the SIM thread.

## Results

New Career, 15 ms simulated disk, about 56 s of play after the start (changes 1–4):

| | Before | After |
| --- | --- | --- |
| Reads on the window and SIM threads | 13 | 0 |
| Ticks over 25 ms | 5 (worst 89 ms) | 0 |
| Ticks started over two ticks late | 12 (worst 73 ms) | 0 |
| Presented-frame gaps over 33 ms | 2 (95.5 and 61.3 ms) | 0 |

The same with the career's first dialog answered, about 85 s of play: reads 9
to 0, ticks over 25 ms 3 (worst 88 ms) to 0, late ticks 8 to 0, frame gaps 2
(97.8 and 70.2 ms) to 0.

Continue Career from a save after the first mission, 15 ms simulated disk:

| | Before | Changes 1–4 | All five |
| --- | --- | --- | --- |
| Reads on the window and SIM threads | 10 | 4 (AI spawns) | 0 |
| Ticks over 25 ms | 6 (worst 54.8 ms) | 3 (worst 37.1 ms) | 0 |
| Ticks started over two ticks late | 3 (worst 39.8 ms) | 0 | 0 |
| Presented-frame gaps over 33 ms | 4 (worst 55.1 ms) | 3 (worst 42.3 ms) | 0 |

Follow Slappy's start (the sweep abandons the first mission and signs up
`m_1a_07_follow_slappy`), 15 ms simulated disk, all five changes:

| | Before | After |
| --- | --- | --- |
| Reads on the window and SIM threads | 35 | 0 |
| Ticks over 25 ms | 17 (worst 82 ms) | 0 |
| Ticks started over two ticks late | 33 (worst 125 ms) | 0 |
| Presented-frame gaps over 33 ms | 14 (worst 88.7 ms) | 2 (36.8 ms, the challenge's teleport) |

The path package arrived from the stream reader 57 ms after it was asked for;
the challenge went on from `wait_for_assetload` once it had, and Slappy's
`StartPath` ran on it.

Parity: the October 2 pool route's 6000 `selected (board|states|offboard|manual)`
lines are byte-identical before and after, with changes 1–4 and with all five (a
traced run keeps the synchronous reads, and the descriptor preload changes no
simulated state).

The career's own start (`CareerSession::new`: the global locators and vaults)
still reads on the window thread, under the fade: 1.35 s with the simulated
disk, about 120 ms with a warm cache.
