# Skate 3 style and posture in Edit Skater

The user requested source preparation only, without building another game EXE.
The existing Test7 executable and installed settings are not replaced by this
change. This document describes the next source build.

Two PC selector rows are added to **Edit Skater > Skater Style**, alongside the
gesture and stance controls:

| Row | Authored profile order |
| --- | --- |
| Skate 3 Style | Default, Loose, Gonzo, Aggressive |
| Skate 3 Posture | Default, Stiff, Slouch, Buff |

Left/right changes and wraps the selections. These choices apply when the
Skate 3 movement solver is active, including its offboard replacement. They do
not change the native Skate 2 CAC `style` word or the selected physics toggles.
Edit Skater keeps its existing CAS room animation; this change does not replace
the editor preview with a Skate 3 character or movement scene.

## Source behavior and ownership

`GetCACSettings`82590D20..82590D84 selects the animation construction name:
0 is null/default, 1 Loose, 2 Gonzo, 3 Aggressive. The retained Skate 3
`SkaterAnimation::set_customisation` already implements that mapping.
Posture profile mapping82590B50 selects Stiff, Slouch, or Buff from1..3;
profile0 is the unmodified motion tree. The registered static pose order is
82858810. `PendingPosture` preserves the Play Begin request at82678990, consumed
while constructing an eligible main tree at82B984C8. Channel trees bypass that
service. Changing the profile does not manufacture a pending request or apply a
posture indiscriminately to gestures, bails, or other channels.

The host previously called `set_customisation(natural_stance, 0)` on every
selected-player reconciliation, forcing the default style. Natural stance
refresh now has its own method and leaves animation style intact. The actual
selected Skate 2 Actor's owned Skate 3 animation receives the saved style and
posture before advancing. No second character or process is introduced.
The existing embedded OnBoard/OffBoard banks already contain the authored
style clips and posture poses; no new runtime game folder is required.

## Persistence

`SKATER.P` stores a version1 `SK82_PC::Skate3Style` leaf with separate big-endian
style and posture words, both in0..3. The existing native CAC and independent
gesture settings retain their own storage. A missing leaf selects both defaults;
invalid or future versions fail explicitly rather than being overwritten.

Finishing Edit Skater merges the CAC record, gesture assignments, and these
animation preferences into the latest save, verifies the encoded read-back, and
commits them together through the existing save owner. Display and career
writers opened before this edit read the latest file before saving and retain
the extension. Returning to gameplay invalidates both preference caches, so an
existing selected player reads the new choices. Profiles are not read every tick.
Diagnostic events record selected style/posture IDs and the saved preferences.

## Validation scope

Focused checks cover profile format compatibility, invalid/future leaf handling,
real Edit Skater rows and saved choices, and sampling the embedded original
motion clips with the authored posture tree. They do not establish console trace
parity. Test outcomes are recorded below after the checks complete.


Completed validation:

- `skate2-formats` style extension tests:2 passed. All16 combinations survive
  serialization and independent gesture/display writes; unknown save data and
  native CAC values are preserved. Invalid/future data is rejected (19.22s).
- Actual original-data Edit Skater/save test:1 passed (6.11s). Both new rows
  display and cycle all four labels in each direction. Finishing and reloading
  preserves the selected style/posture and independent gesture sets; a writer
  opened before the edit retains them. The native S2 style is unchanged.
- Embedded original motion test:1 passed (5.43s). All16 combinations are
  sampled at both natural-stance selections; each nondefault choice changes
  the evaluated pose, samples are finite, and stance refresh retains the chosen
  style. The same application method is called by the real selected player.
  Logs identify the authored Hsu, Gonz and PJ clip variants and the three
  original posture poses. This is a component check, not a full moving Actor
  or console stance trace.
- `cargo check -p skate2-pc --locked`:passed (2m30s,18 existing app warnings).
  The core test target compiled successfully (927 existing warnings). No
  whole-workspace fmt/clippy/console parity gate is claimed.
- The original external save remains unchanged in both corpus runs. Unedited
  lines in the10 existing source files retain their original line-ending bytes.
- No new game EXE was built or launched for this source preparation. The
  installed Test7 EXE retains SHA256
  `cb363a082e1aa0b790bbf81d0c3db29e49957fff73ada585c9af9864e5b3bfda`.

Raw probe results are retained outside the repository in
`work/style134-authored-motion`, `work/style134-editor-save`,
`work/style134-profile-tests.log`, and `work/style134-pc-check.log`.
The initial133 test-only compilation referenced an upstream seek helper that
is disabled in the retained host port; the probe now samples through the
existing animation length/advance API. Only the corrected134 run is a pass.
