# Modern PC media saving, 2026-10-13

Intentional PC parity differences: shareable MP4 replays,
1080-line photos/video, higher image quality, and local save-path wording.
PC only, solo. Android and the actual player profile are not edited.

## Reference boundaries

Live IDA checked:

- `ReplayEditorBackEnd::CalculateFootageLengthInSeconds`, `8255e6f0`:
  trim frames at 30 Hz, per-interval speed and rounded duration.
- `StateMemcardReplaySaveFlow::SaveFlow`, `82750568`: freeze submissions,
  screen/thumbnail capture, then editable recording serialization.
- `StateMemcardPhotoSaveFlow::SetScreenShotAndThumbData`, `8274ad18`:
  153600/15360-byte buffers and descending JPEG quality from 75.
- `PostFX::SetWatermarkTexture`, `82828610`: resolves the global presentation
  registry asset and retains its texture pointer. Watermarks do not belong to
  an individual streamed city generation.

The modern encoder is deliberately a host adaptation, not claimed as native
console behavior. Recorder/physics cadence is unchanged. Existing native
editor, ICELITE, replay restoration and audio consumers produce each sample.
The output clock uses f64 to avoid drift during long slow-motion exports.

## Changes

- Dedicated 1080-line render target, previewed in the existing window.
  Capture follows PostFX and the selected watermark, preceding frontend UI.
- H.264 CRF18/60 Hz plus 48 kHz stereo AAC192k, streamed to FFmpeg without a
  shell. Two encoder threads, bounded frame memory, nonblocking finalization.
- CPAL temporarily relinquishes the mixer to deterministic export sampling;
  recorded world/skater audio is consumed through the normal game-audio path.
- JPEG95/4:4:4 full photos, native thumbnails. Career photo captures also use
  the high-resolution target.
- Complete-slot publication, cancellation/error cleanup, editable archives
  retained, old/new slot deletion supported, actual-path completion messages.
- Temporary export state saves/restores author, replay clock, camera mover,
  game-camera restoration and collision history. Normal input resumes only
  after the export is ended.

See [media export usage](../MEDIA_EXPORT.md) for paths and dependencies.

## Runtime correction

The second controller run saved and decoded a real 1920x1080 photo, then
exposed `map scene is still loading` when export rewound into a city update.
The renderer transfers its mutable map/texture cache to CPU/GPU workers;
looking up a watermark through that cache on every export frame was invalid.
Both original watermarks are now retained by the window from boot, as the
native global texture pointer is retained. Video sampling also waits for
both map workers and the exact requested focus before capturing or advancing
audio. Photo/poster capture retries this temporary state too.

## Validation status

Full PC `cargo check -p skate2-pc --bin skate2-pc -j 1 --offline` PASS three
times; check3 includes the map/watermark fix (24.74 s). Normal release builds
PASS: build1 821.64 s, build2 113.92 s, final build3 227.52 s. Existing
core437/PC18 warnings retained. No optimization flags were reduced.

Final executable `target/release/skate2-pc.exe`: 43,769,856 bytes, SHA256
`36A82EE22F3FF138325A8C2D25E93A59964AADD120F30EB5445D1E33D88DB21D`.
FFmpeg sidecar: installed 4.2.3 build, `libx264` and AAC present; SHA256
`52F1733E400A20F8C0C39BD3B21E183D4EED88A967FC1CB2B2FF5F034DC5ECFF`.

The controller regression **PASS**, exit0:
`.scratch/replay-entry-test-20261013-180216-021/`. Entered actual Career and
Replay Editor; saved photo; cancelled MP4 without publishing a slot; saved
MP4 plus editable archive; asserted cursor/trim restoration; returned to a
visible, responsive pause menu and ran 120 resumed simulation ticks. The real
profile's SHA256 remained
`13B6A6E9316724AE9ACE456679A7244FBD1D79A1920459CED5E58C3F4D5D8576`.

Export file checks:

- JPEG: 1920x1080, 4:4:4, 797,552 bytes; visually inspected, no HUD.
- MP4: H.264 High, 1920x1080, 60 FPS, BT.709 limited range; 362 decoded frames
  with 362 distinct hashes. Full decode exit0. Video sampled visually: actual
  recorded skating, selected watermark, no editor UI/FPS counter.
- AAC-LC: 48 kHz stereo, non-silent (mean -26.0 dB, peak -7.8 dB). Container
  audio padding is present as expected from AAC; raw source is exactly 800
  stereo sample frames per video frame.
- MP4 `moov` precedes `mdat`; 17,036,428 bytes. SK2R archive retained at
  10,963,201 bytes. No temporary slot remains after cancellation/completion.

First regression run used the wrong scripted photo button (Y instead of the
authored X); corrected in build2. Second run exposed the streaming defect
above; fixed and rerun successfully in build3. The final scripted route uses
an unedited, normal-speed Career recording. It does not independently validate
every advanced camera/speed edit, ultrawide resize or sound-event edge case.
No full campaign playthrough or new physics-parity claim is made.

Backups, live IDA packet, checks and build/run evidence are under
`.scratch/modern-media-20261013/`. The career audit is separately documented
in `career-progression-audit-2026-10-13.md`; it is not a campaign playthrough.
