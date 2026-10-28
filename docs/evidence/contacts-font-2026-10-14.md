# Contacts button-help font lookup

Date: 2026-10-14. PC only, solo.

## Report and source mismatch

After the startup InitAction repair, a playtest reached the Contacts entry path
with LB/Y, then hit `map button font absent`.

The original `phonelist.apt` requests `HelveticaBold`, size 16, colour 13034748
in `UpdateButtonHelp` (`0x2d32..0x2d47`), followed by left justification.
The Contacts translation correctly retains those authored arguments.

The shared `map_movie::styled_button_items` helper measured button labels with
a strict `assets.fonts.get(font_name)` lookup and treated a miss as fatal.
The actual text renderer already used `Assets::apt_font`, so layout and drawing
disagreed about how the same requested font name is resolved.

## Reference and correction

Live IDA was unavailable. IDA 9.3 batch mode inspected the saved
debug database `world_probe_temp\waypoint_replay_20261014.i64`.
Input SHA-256: `3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.

`cFontManager::GetFontByAptName` (`0x82851790`) compares Flash names without case.
If there is no match, it selects the first mapping whose file is not `debug`.
`InitializeFontDetails` (`0x8282b920`) supplies those mappings in the original
font group's reference order. The existing `Assets::apt_font` and retained
`font_fallback` already implement this contract for the loaded font group.

Changed the shared styled-button helper to use `assets.apt_font(font_name)`.
Label width/baseline calculation and actual rendering now resolve the same font.
The requested name, authored size/colour, glyph metrics and button spacing are
retained. This also fixes the same lookup contract for other users of the shared
helper. No font alias, substitute host font, or Contacts-specific fallback added.

The original APT script dump is in
`.scratch/contacts-apt-init-20261014/phonelist-typed-apt.txt`.
Fresh IDA pseudocode/disassembly, the source backup/diff and build log are in
`.scratch/contacts-font-20261014/`.

## Validation

Confidence: **high** for the source mismatch and correction. Normal
`cargo build --release -p skate2-pc -j 1 --timings` passed, exit 0, in 3m15s.
Core was reused (438 cached warnings); PC emitted 19 warnings. The rebuilt
`target/release/skate2-pc.exe` is 43,937,280 bytes, modified
`2026-10-14T04:20:01.9656004+03:00`, SHA-256
`A26093EC281E50C23EDB66E5A26782301D7804303620B59605E20B15304A0757`.
No tests or game launch were run; controller entry and visible Contacts
navigation still need runtime confirmation. Android, simulation and real save
data were not changed.
