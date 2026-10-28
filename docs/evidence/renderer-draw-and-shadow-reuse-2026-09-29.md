# Renderer draw batching and world-shadow reuse — September 29, 2026

Scope: further renderer/chunk-loading performance work, SOLO, with compiled
diagnostic runs. No physics,
asset, visibility-distance, streaming-residency or shading-quality changes.

## Existing work reused

The city CPU loader, GPU builder and old-map retirement already run on workers.
Existing shared mesh arena, lighting arena, exact ordered map render bundles,
world-shadow command bundles and culling caches remain in place. This pass does
not add another streaming implementation or repeat the rejected command-chunk
experiments documented in the earlier performance evidence.

## Changes

- `apps/skate2-pc/src/gpu_renderer/map_draw.rs`: combine adjacent selected island
  draws only when object/instance and vertex base match, index ranges touch, and
  each range is a complete triangle list. Never bridge a culled gap, reorder
  objects, change materials or combine different objects. Both main-map and
  world-shadow bundles use this helper. Preserve original island statistics;
  append actual main-map draw-call count as `worst_map_calls` to optional CSV.
- `gpu_renderer/shadow_map.rs`: reuse the resolved static world-shadow image
  when the exact command-bundle identity and all projection-row bits match.
  A different selected caster list, immutable map generation, binding/pipeline
  or projection causes normal rendering. This is host redundant-work elimination,
  not a claim that the original console implementation cached this texture.
  Existing ported native caster selection and shadow shaders are unchanged.
- Cache only a Weak reference to the command bundle: no extra retention of
  retired streamed geometry. The key is confirmed only after queue submission;
  invalidate before queued uniform writes, and discard unsubmitted pending keys.
  Dynamic skater/DMO/vehicle cascades still render normally every frame. They
  overwrite the shared intermediate target, not the retained resolved world map.

Optional comparison switches (presence enables old behavior):

```
SKATE2_SEPARATE_ISLAND_DRAWS=1
SKATE2_REDRAW_WORLD_SHADOWS=1
```

These were set only inside diagnostic child processes. Normal launchers need no
changes. No additional ordinary-play logging was introduced.

## Validation

- Release renderer tests: **36 passed, 1 ignored, 0 failed**. Includes ordered
  index/instance/base stream equivalence, culled-gap and incomplete-triangle
  boundaries, adapter-backed map/effect validation, exact shadow-key invalidation
  and proving its Weak reference does not keep geometry alive.
- Release executable built successfully; pre-existing warnings remain.
- Five CPU-profile camera runs, plus two GPU-profile camera runs, each completed
  2400 simulation ticks without errors. Same binary, same spawn locator
  `m_2c_03_goto_find_pool_locator`, `offboard-street-look`, uncapped presentation,
  1280x720. No compiler was running during captures.
- Before/after final screenshots are **pixel-identical below the top 40 rows**
  containing the FPS overlay. Skater, board, foliage, buildings and shadows are
  visible. This checks this captured view, not every map/effect combination.
- 1800-tick `offboard-city-walk` completed without errors: recorded position
  (886.306,48.197,-391.913) -> (827.987,46.384,-338.919), approximately 79 m;
  map object population changed from 5985 to 5947. Inspected final screenshot.

Logs: `artifacts/renderer-batching-{tests,build}-20260929.log`.
Captures: `artifacts/batch-{baseline,enabled,walk}-20260929.png`.
Runtime logs: `artifacts/*batch*-runtime-20260929.log` and
`artifacts/shadow-only-runtime-20260929.log` (quiet, no reported errors).

## Measurements and limits

Means of full one-second windows with elapsed time >5 and <40 seconds. CPU times
are milliseconds per redraw. CSVs: `artifacts/frame-profile-<name>-20260929.csv`.

| name | FPS mean | record CPU ms | scene CPU ms | world-shadow CPU ms |
| --- | ---: | ---: | ---: | ---: |
| batch-baseline | 377.73 | 1.2929 | 0.9675 | 0.1278 |
| batch-enabled | 327.95 | 1.2407 | 0.9907 | 0.0458 |
| shadow-only | 364.03 | 1.3232 | 1.0666 | 0.0477 |
| batch-baseline-repeat | 302.96 | 1.4573 | 1.1026 | 0.1430 |
| batch-enabled-repeat | 333.96 | 1.3236 | 1.0586 | 0.0511 |
| batch-gpu-baseline | 272.44 | 1.5212 | 1.1456 | 0.1452 |
| batch-gpu-enabled | 423.87 | 1.1703 | 0.9180 | 0.0377 |

GPU-profile pair: mean world-shadow GPU time **0.0500 -> 0.0045 ms**;
main-scene GPU time **0.8291 -> 0.8049 ms**. GPU samples are asynchronous,
approximately one in sixteen frames, using the existing profiler.

First enabled run's worst-frame diagnostic counts averaged 3325.31 original
islands and 2169.74 emitted draws (about **35% fewer**). Counts come from each
window's worst redraw, not a per-frame draw-count average.

The world-shadow stage consistently improves. Total FPS varies substantially,
including between two runs of unchanged baseline behavior; client, acquisition
and submission timings vary too. Do NOT advertise the GPU pair's 272->424 FPS
as a reproducible overall gain, a universal minimum, or a solved stutter issue.
Adjacent-island batching's standalone FPS benefit is not established; it safely
reduces command count and preserves the captured output. Remaining CPU scene,
submission/acquisition and simulation work still dominate total frame time.

No new streaming architecture was needed in this pass. Broad city traversal and
the exact reported slow location remain useful follow-up measurements. The current
release executable includes both changes and is ready for normal playtesting.
