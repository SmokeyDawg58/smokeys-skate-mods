# PC replay and photo exports

Save from the Replay Editor as usual. The completion popup shows the actual
file path. Default locations, relative to the Skate 2 Rewrite directory:

| Media | Location |
| --- | --- |
| Shareable replay | `saves/skate_reel/replays/<slot>/replay.mp4` |
| Editable recording | `saves/skate_reel/replays/<slot>/replay.sk2r` |
| Replay Editor photo | `saves/skate_reel/photos/<slot>/photo.jpg` |
| Career mission photo | `saves/career_photos/<slot>.jpg` |

`SKATE2_SAVE_ROOT`, when set, replaces `saves`. Slot directories retain the
existing eight-digit hexadecimal names, thumbnails and metadata. Old saved
recordings remain readable; their next save also creates an MP4. The MP4 is
for ordinary video players; reopen the accompanying recording to edit it in
Skate.Reel. Keep the whole slot directory for in-game browsing/editing.

## Quality and behavior

- A separate render target uses 1080 vertical pixels. A 16:9 window produces
  1920x1080; other window shapes preserve their aspect ratio and camera framing.
  This renders the scene again at the export resolution instead of enlarging
  a screenshot of the window. An open export keeps its initial dimensions.
- Video uses H.264, CRF 18, the `fast` preset, 60 output frames per second,
  BT.709 limited-range YUV 4:2:0 and AAC stereo at 48 kHz / 192 kbit/s.
  MP4 uses FFmpeg's `+faststart` layout for playback before a download finishes.
  See [FFmpeg format documentation](https://ffmpeg.org/ffmpeg-formats.html).
- The selected trim, edited camera, speed, shake, PostFX and watermark are
  rendered through the ordinary replay presentation. Each completed video
  frame advances an independent output clock, including 800 stereo audio
  frames from the game mixer. Time spent loading/rendering/encoding does not
  advance that clock. The game's original recording cadence is unchanged.
- Photos use JPEG quality 95 with 4:4:4 chroma. Full photos no longer reduce
  quality to fit the console's 150 KiB limit. In-game thumbnails keep their
  original 160x120 dimensions and 15 KiB limit.
- Export pauses editor controls and simulation. A or B on the Cancel popup,
  or keyboard Escape, cancels video export. The editor cursor, trim and camera
  state are restored after success, failure or cancellation.
- New media is written into a temporary slot and published only after all
  files finish. Failed/cancelled exports remove their temporary files. A power
  loss or forced process termination may leave an ignored `.pending-*` folder;
  it is never listed as a finished save.

These are intentional PC improvements. They replace the console's media
encoding/size limits and Xbox/EA instructions, while retaining the authored
editor UI and existing editable-recording format.

## Encoder dependency

MP4 export requires an FFmpeg executable with `libx264` and `aac` encoders.
Photos and editable-recording playback do not require FFmpeg.

The game searches in this order:

1. The executable path in `SKATE2_FFMPEG`, if set.
2. `ffmpeg.exe` beside `skate2-pc.exe`.
3. `tools/ffmpeg/ffmpeg.exe` under the save directory's parent.
4. `ffmpeg.exe` on `PATH`.

The current local release includes a copy of the already installed FFmpeg
beside the game executable. The repository does not vendor this binary.
Release packages should supply their chosen FFmpeg build and its accompanying
license/source information, or document the dependency for users.

Encoding uses two CPU threads and one frame at a time; a long clip does not
accumulate uncompressed video in RAM. Temporary audio and video use disk
space in the save directory until the final MP4 has been written.

## Focused regression

`powershell -File tools/test-career-replay.ps1 -Media` uses a copied profile,
enters Career and the actual editor, saves a 1080p photo, cancels a video,
saves another video, checks retained edits, returns to Pause and resumes
simulation. It writes only to a fresh diagnostic save root. Run the script
without `-Media` for the existing camera/Trick Guide regression.
