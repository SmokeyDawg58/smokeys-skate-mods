# Edit Skater gesture catalog (2026-10-12)

PC source fix, solo. Android is parked. No build, automated tests, or gameplay run.

## Confirmed mismatch

`EditSkaterFrontend::new` populated `available_gestures` by sorting and deduplicating
`record.data.gestures`: the four equipped direction slots. Consequently the player
could only rearrange those equipped gestures; duplicate assignments could reduce
the choice count even further. `set_available_gestures` had no callers. The existing
35-entry label and preview tables were present, but their unlock catalog was missing.

## Native evidence

Live Xbox debug IDA, including the numeric stores in the catalog branch:

| Native function | Address | Relevant behavior |
| --- | --- | --- |
| `FrontEndState_CaC::SetScreenState` | `0x82684b40` | State 10 reads the four equipped slots, clears `mAvailableGestures`, then queries every gesture in enum order. |
| Catalog construction | `0x82685138` to `0x82685f30` | 35 `GetStringHash` calls, followed by `IsUnlocked(category=14, id, includeDLC=true)` and conditional append of IDs 0 through 34. No sort, gender filter, or four-choice cap. |
| `UnlockablesBackEnd::AddDefaultUnlocks` | `0x82566030` | Grants 15 gesture IDs: 0-11, 17, 20, 28. |
| `UnlockablesBackEndCore::IsUnlocked` | `0x82a73c98` | Debug unlock-all, otherwise the main unlock list and optionally the DLC list. |
| `FrontEndState_CaC::ReceiveInput` | `0x8268d818` | Gesture branches `0x8268dd2c` to `0x8268e088`: find the equipped ID, use index zero if absent, add -1/+1, wrap, assign the selected slot, preview, and refresh the screen. |
| `FrontEndState_CaC::GetStringValue` | `0x826775f0` | Find equipped ID in the available list; when absent, display the first available gesture's label. |
| `FrontEndState_CaC::PlayGestureAnimation` | `0x82649a98` | Uses the same lookup and first-entry fallback for the preview body key. |

The gesture set is at `this+0x1094`, with Up/Down/Left/Right words at offsets
`0x1094/0x1098/0x109c/0x10a0`. The available vector's begin/end/capacity are
`+0x10a4/+0x10a8/+0x10ac`. There are four equipped slots and up to 35 choices
per slot. The native branches do not remove an ID merely because another slot
already uses it.

The catalog names are literal native unlock IDs, independent of localization
and animation names. For example ID 18 uses `gesture_flex`, IDs 21-23 use
`gesture_paper/rock/scissors`, ID 32 uses `gesture_raisetheroof`, and ID 34 uses
`gesture_axe`. Deriving unlock names from HAL suffixes would miss these entries.
The exact 35-name order is recorded in `GESTURE_UNLOCK_NAMES` and in the raw
catalog evidence.

## Implementation

- `session/edit_skater_frontend.rs`: the constructor now requires the existing
  unlock callback and builds choices from all 35 native names in their original
  order. Removed the unused setter and equipped-only fallback. An empty unlock
  list remains an explicit host error instead of fabricating unlocked choices;
  a valid native profile starts with the 15 defaults.
- `session/edit_skater_session.rs`: passes the same unlock callback already used
  by the clothing shop. The caller's `IsUnlocked(..., true)` retains DLC and the
  debug unlock-all behavior. The editor owns this snapshot while gameplay is
  paused; reopening reads the current career unlocks again. The new-career path
  already supplies `Unlockables::new`, including the native 15 defaults.
- Label, preview, and left/right selection share `gesture_index`. This preserves
  native wrapping and the fallback for an equipped ID absent from the available
  list. Merely displaying that fallback does not rewrite the equipped record.
- Existing four-slot serialization, record commit, player character-data reload,
  and preview body/animation loading remain the consumers of the selected ID.
  The existing asset-dependent test initializer was adapted to the required
  callback; no test cases were added or executed.

Source inspection also confirmed the ordinary editor entry points use the career
unlock snapshot or new-career defaults. Offline Free Skate and Party Play pause
hubs do not enable Edit Skater; this change adds no new menu entry points.

## Review and limits

Confidence: **exact** for the recovered catalog, category, DLC flag, native order,
default count, and selection/label/preview lookup semantics; **high** for source
integration. Runtime behavior has not been observed for this change.

- Compared the Rust catalog with the live-IDA extracted names: 35 entries,
  zero ordering/name differences. Default IDs resolve to
  `[0,1,2,3,4,5,6,7,8,9,10,11,17,20,28]`.
- Reviewed both constructor callers, slot order, wrapping, missing-equipped-ID
  behavior, duplicate assignments, save flow, and preview ownership.
- Both changed Rust files parse with `rustfmt --emit stdout` (files not formatted).
  This is syntax review, not a type check or gameplay verification.
- No compilation, tests, asset animation playback, or in-game cycling performed.
  The executable needs rebuilding. There is no claim that all 35 gestures are
  unlocked on every profile, or that every possible award source was audited.

Backups, scoped diff, six complete native function extracts, and the 35 catalog
branches with raw disassembly are in `.scratch/gesture-catalog-20261012/`.
This supersedes the equipped-only availability limitation in the October 1
Edit Skater UI evidence.
