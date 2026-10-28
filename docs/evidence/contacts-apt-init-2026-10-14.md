# Contacts and Big Black timer startup InitActions

Date: 2026-10-14. PC only, solo.

## Report and cause

The preceding release could not boot:
`untranslated APT InitAction 103:8 0x1d38`.

`MarkerUi::new` constructs the Big Black timer before Contacts. The original
timer movie's first frame contains a `DoInitAction` for sprite 8 at `0x1d38`.
The class implementation existed in `BigBlackTimerMovie`, but its initialization
offset was incorrectly registered only in the ordinary timeline Action handler.
`Clip::apply` dispatches InitAction separately and therefore rejected it.

The new Contacts and basic-picker movies had the same registration mistake.
Their original InitActions were inspected together so the next menu initializer
would not retain the same defect.

## Original reference

Live IDA was unavailable. IDA 9.3 batch mode read the saved
debug database `world_probe_temp\waypoint_replay_20261014.i64`.
Input SHA-256: `3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.

- `AptCharacterAnimation::ExecuteInitAction` `0x829f0e20` searches first-frame
  controls for `DoInitAction` matching the sprite ID, exports definitions, then
  runs the initializer stream and marks that initializer as executed.
- `ExecuteInitActions` `0x829f7f98` resolves imported characters and executes
  their class initializers separately from ordinary timeline actions.
- `BigBlackTimerHUD::BigBlackTimerHUD` `0x826b4d28` starts hidden and selects
  the original timer screen. `Refresh` `0x826bbc38` supplies timer states/data.

The original APT control records identify these exact tuples:

| Asset | Sprite | Offset | Definition |
| --- | --- | --- | --- |
| 101 / phonelist | 74 | `0x26a4` | Guarded `phonelist` class |
| 102 / basic_menu_picker | 19 | `0xd14` | Guarded `menu_picker` class |
| 102 / basic_menu_picker | 20 | `0x1c28` | Guarded `basic_menu_picker` class |
| 102 / basic_menu_picker | 18 | `0x297c` | `Basic_Menu_Picker_Component` registration |
| 103 / bbtimer | 8 | `0x1d38` | Guarded `bbtimer` class |

The timer and phone classes' constructor/state methods and the picker control
are supplied by the existing typed Rust movie implementations. Added their
exact tuple predicates to the InitAction dispatch and removed the incorrect
ordinary-Action registrations. Unknown initializers still fail explicitly.
Reviewed the remaining timeline actions in all three movies: existing translated
actions and the shared Stop handler cover them. No timer/service/gameplay logic
or authored UI layout was changed.

The read-only `tools/inspect_apt.py` helper previously labeled both action types
as `action`, hiding this distinction in its output. It now labels InitAction
explicitly and prints the associated sprite ID.

## Evidence and validation

Native pseudocode/disassembly, original APT script dumps, exact control tuples
and asset hashes, source backups and scoped diffs are retained under
`.scratch/contacts-apt-init-20261014/`.

Confidence: **high** for this dispatch mismatch and its source correction.
The normal `cargo build --release -p skate2-pc -j 1 --timings` rebuild passed,
exit 0, in 3m35s. Core was reused (438 cached warnings); PC emitted 19 warnings.
The rebuilt `target/release/skate2-pc.exe` is 43,937,792 bytes, modified
`2026-10-14T04:10:20.2582104+03:00`, SHA-256
`A9DCD2F7023932896E391249D4EE601792B05651E113943DA794B90CE7B1C589`.
Build log/result metadata and the preceding executable backup are in the scratch
directory above. No test or game launch has been run; boot and Contacts/service
interaction still require runtime confirmation.
