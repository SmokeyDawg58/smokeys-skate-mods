# New-career menu, replay timeline and capture textures - 2026-10-13

## Status

Four PC source files changed; no build, tests or game launch.
Syntax parsing passed for all four files with rustfmt output discarded.
Runtime appearance and saving still need confirmation after rebuilding.

Solo work in this repository. Source backups, live IDA records, original
asset metrics and scoped review: `.scratch/replay-cac-followup-20261013/`.
No Android, physics or actual `SKATER.P` writes.

## Trick Guide alignment reverted

The earlier panel offsets are the original's, so the deliberate alignment
adjustment is reverted. Restored exactly the
pre-adjustment `trick_guide_movie.rs` from the previous task's backup:

`DB33900A8B63B14B96898D970A6DB53A2180DC7D4FCD96932DD6EF7A3154F35A` (SHA256).

This removes `align_panels`, its extra gesture origin, the lower-background
transform, and the text-wrap/padding override. It preserves the earlier
authored 48px gesture base and all separate pause-input/replay-camera fixes.
The original panel geometry/timelines are again used directly.

## New-career Edit Skater options

The reported screenshot is the final creation page after Pete's and Slappy's.
Native `FrontEndState_CaC::SetScreenState` **82684b40**, menu9 branch
**82684eb0..826850cc**, checks `mSubState` at offset **1010** against **1**:

| Entry context | Native option IDs | Rows |
| --- | --- | --- |
| New career, substate1 | 33,34,35 | Edit Style, Trucks, Wheels |
| Ordinary editor | 0,1,33,34,35 | Pete's, Merchandise, Edit Style, Trucks, Wheels |

`OnEnter` **82659a08** initializes a fresh record for substate1.
`OnLoaded` **8268ae20** opens Pete's for that substate, otherwise menu9.
The retained controller already carries this context as `new_career`, supplied
by `career_after_editor` through `EditSkaterSession::open`.

The missing branch is now in `EditSkaterController::push`: menu9 uses the
three original options only for new career. This affects the actual selectable
rows, not just their visibility. Returning from Edit Style retains the same
list; reopening the ordinary editor starts from its full five-row list.
The existing creation order, menu-stack clearing and finishing flow remain.

Confidence: **high**, native branch checked in decompilation, PPC and debug
member layout. No new-save playthrough performed.

## Replay timeline height

The colored interval fill was hardcoded to end at Y51, copied from the marker
icon geometry. It becomes visible when entering timeline editing, causing the
bar to extend over the controls below it.

Original `AptRenderingIntegration::CustomControlRender` **826363d8**, call
**82636860..826368a4**, reads the first rendering unit's first vector strip:
vertex1 supplies X/Y, vertex4 supplies the right/bottom values to
`ReplayEditorBackEnd::Render` **82579988**. `DrawHardCuts` **82571018** uses
the supplied bottom value and a fixed top3. `DrawMarkers` **825714b8** separately
uses its own fixed height51 for the marker/key textures.

Original `replay_editor2.apt/.geo`:

- `mCustomRender` is clip176 at (4.5,17); its depth1 child is shape175.
- Shape175's first element has vertices (0,30), (0,0), (817,0), (817,0),
  (817,30), (0,30). The native height input is therefore **30**.
- The port now reads vertex4.y from that same loaded geometry and includes the
  child's matrix. Interval fill ends at30; marker icons retain51.
- All four marker/key textures are already supplied by the loaded
  `data\fe\source\images\icons\minimap.rx2`; no new substitute art is needed.

Confidence: **high**, direct original geometry and native caller/callee match.
No rendered comparison performed for this source change.

## Replay/photo watermark residency

`LiveSkatingView::prepare_replay_capture` already resolves the correct original
texture name through the retained world inventory for both photo and replay
media. `load_loose_presentation_collection` decoded `globaltextures.rx2` but
only kept missingtexture, vignette and xray. Both watermark choices were absent
from that collection, producing the reported fatal residency error.

Native `SetWatermarkStatus` **8255d7f0** selects the default or pro name and
display dimensions; `PostFX::SetWatermarkTexture` **82828610** resolves the
corresponding `.Texture` asset from the presentation registry.

Both textures exist in `miscboot.big` / `data\content\textures\globaltextures.rx2`:

| Texture | TOC GUID | Source dimensions | Native display dimensions |
| --- | --- | --- | --- |
| logo_watermark.Texture | 9df5f895b0cdc6fc | 128x128 | 64x64 |
| logo_watermark_pro.Texture | 12b77e6baa817dcc | 128x64 | 128x64 |

They are 2D texture objects (000200e8), surface format20. Both are now decoded
through the existing exact-TOC owner and retained alongside the other global
textures. The existing cached GPU lookup, watermark selection, dimensions and
capture-only overlay are preserved. This repairs both save actions and both
watermark choices without disabling the watermark or swallowing a missing asset.

Confidence: **high** for the missing inventory and source fix. Successful media
capture, disk saving and reopening have not been exercised on this change.
