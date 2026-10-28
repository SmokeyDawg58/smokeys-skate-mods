# Contacts projection, Big Black timer, and required model loading

Date: 2026-10-14. PC only, solo. **Source changes; no compilation.**

Playtesting found a flat, incorrectly placed Services menu with the marker menu
still visible underneath, an incorrectly projected Big Black timer, and a hire
that started the timer without producing the protector or a Dismiss option.
The original PS3 screenshot is visual context; reconstruction uses the original
Xbox debug executable and its authored APT/Vault data.

## Reference and confidence

Live IDA was unavailable during the investigation. IDA 9.3 batch analysis used
the separate debug database
`world_probe_temp\waypoint_replay_20261014.i64`.
Input SHA-256:
`3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.

Native pseudocode, disassembly, template data, and structure layouts are in
`.scratch/contacts-parity-20261014/native.json` and `native-2.json` through
`native-4.json`. Original phone/timer APT disassembly is retained under
`.scratch/contacts-apt-init-20261014/`; this investigation also decoded the
original `screenmanager.apt` into `screenmanager-apt.txt` in its scratch folder.
Prior service lifecycle evidence is in
[career-contacts-security-2026-10-14.md](career-contacts-security-2026-10-14.md).

Confidence: **high** for the identified source mismatches. The changed source
has not been compiled or run, so this is not a completed visual/runtime parity
certification. No arbitrary position, size, font, or timer tuning was introduced.

## Phone and timer layers

`sScreenTemplates` is 16 bytes: name at +0, movie URL at +4, transition at +8,
and `FEScreenType` at +12. `FrontEndManager::RegisterScreenTemplates`
(`0x82628098`) passes these values into the original screen manager.

| Screen | Template | Address | Original layer |
| --- | --- | --- | --- |
| PhoneList | 13 | `0x82fdb428` | Type 2, `g3dFocalContainer` / Focus Left |
| BigBlackTimerHUD | 73 | `0x82fdb7e8` | Type 6, `g3dHUDRight` / Focus Right |

The original screen manager creates the screen in the selected container. It
does not add a phone-specific scale or translation. PhoneList was erroneously
drawn as flat geometry; the timer used the generic left projection. Corrected
both submissions in `apps/skate2-pc/src/marker_ui.rs`, including the phone's
placement among the other layer-2 screens. Phone and timer retain the original
landing jolt used by these layers.

Projection references:

- `SetApt3DFocusLeftMatrix`, `0x826082b0`: the authored matrix, stage centre
  (640, 360), landing jolt, 180 minus the 3D angle, and the 3D distance.
- `SetApt3DFocusRightMatrix`, `0x82608748`: the corresponding 180 plus angle.
- Original defaults are 11.25 degrees and distance 624. The existing shared
  projection retains the native sine/cosine implementation and APT view camera
  projection (`ViewMan::Init`, `0x82848f30`; perspective setup `0x82840770`).
- Existing viewport aspect handling is retained. The supplied screenshot's
  enclosing render dimensions are not established, so its pixels were not used
  to invent an extra scale or translation.

Authored geometry checked:

- `phonelist.apt`: 1280 x 720 stage; `mcQuickMenu` starts at (88, -9), scale 1.
  The original menu/service/description pages, 38-unit row spacing, original
  font/button settings, and intro/outro timelines remain in use.
- `bbtimer.apt`: `mcTextTimer` at (1043.699951, 258.649994), Y scale
  1.00001526. Its timer child is at (-10.25, -39.35), Y scale 1.0001831;
  the text child is at (-128, 39), scales 1.00050354 and 1.00115967.
  The icon child remains at (-21, -5). These asset values are not overridden.
- `BigBlackTimerHUD` constructor `0x826b4d28`, update `0x826b2a28`, and refresh
  `0x826bbc38` confirm the existing data/text states, five-update refresh,
  rounded-up time display, and flash/sound threshold below five seconds.

PhoneList FE state 74 is at `0x82fdf348`, bytes
`0100000000000001000000000000000301000000`.
`sFEStateInfo::mShowHUD` is the word at +4: **Hide (1)**. Added Contacts to the
same gameplay HUD hiding condition used by other native states. This hides the
marker/phone HUD, minimap, timer, and other gameplay HUD while Contacts owns the
screen. Its existing pause and input ownership are retained.

## Why the timer ran without Big Black

Reproduced with the existing executable using a copy of `saves/SKATER.P`:

- `run-before/capture.png` shows **Hire Big Black ($500)** still offered while
  the timer reads **04:57**, after the $500 debit.
- `run-before/stderr.log`, line 555: `BigBlack { spawn: true }`.
- At that point `big_black_lw` is absent from the loaded model records. Around
  recipe updates 600 and 900, only five to seven required models are resident.

`Manager::SpawnBigBlackMsgHandler` (`0x82719598`) calls census spawn for
`big_black_regular01`, stores the returned entity, and assigns the local actor
as protectee. `IsBigBlackServiceActive` (`0x8271d3a8`) tests that real entity
pointer. `CensusMan::SpawnPedestrian` (`0x82720fe0`) refuses an unloaded model.
The port follows those contracts, including the spawn flags
initial=0, allowCull=0, force=1, worldCollision=1, ignoreNavmesh=0, handProp=0.

The missing step was **loading before gameplay**. In the original Vault,
`big_black_regular01` references required model `b7cdd46ee0b8e9b0`, recipe
`big_black_lw`. It is the last of 15 required models in this queue's original
pop order. The port initialized the queue but only pumped it in simulation
updates. Its 120-update interval therefore deferred the protector until well
after gameplay had started.

Native loading sequence:

| Function | Address | Loading behavior |
| --- | --- | --- |
| `GameModule::OneOffLoad` | `0x8273add8` | Pumps RecipeMan at `0x8273b6dc` until Required and Initial queues are empty, sleeping 1 ms per iteration; then waits on asset streaming. |
| `GameModule::GameLoad` | `0x8273be50` | Same queue wait, Update at `0x8273c7a4`, followed by the stream-focus loading barrier. |
| `StateReplay::LoadLivingWorld` | `0x82776328` | Initializes/pumps the same queues before advancing its loading state. |
| `RecipeMan::Update` | `0x82728380` | Preserves active-load gating, queue priority, and the 120-update interval. |

Moved the existing recipe update/callback body into
`SelectedLivingWorldRecipes::update_models`, shared by normal SIM updates and
the new startup loading wait. Mounted player startup now completes Required and
Initial loading plus the last outstanding real asset completion before Actor
construction/admission. It uses the same registry, request allocator, worker
results, and simulation/presentation model registrations as the existing path.
Secondary models remain on the ordinary in-game queue. This performs no physics,
population, challenge, or random-number updates.

The existing service then reads back the actual spawned entity. Contacts uses
that status to offer Dismiss and to avoid a second hire; the minimap follows the
entity's actual X/Z transform. No timer-based fake active flag or fabricated
protector was added.

## Second reproduced fault: protection attributes omitted from the Vault

The comparison in `run-resident-2/` used the same existing executable and a
fresh copy of the real save. It waited until all required models, including
`big_black_lw`, were resident before hiring. Spawn then reached the real
protector graph, revealing another integration error:

```text
pedestrian generated update 0x828f5748:
Callback("native BigBlack protection collection assertion")
```

The log records the spawn at lines 1480/1482 and the failure at line 1485,
after 7,898 completed gameplay frames. This run did not reach the planned
Dismiss-menu screenshot. An earlier launch attempt in `run-resident/` exited
before startup because its process environment lacked `SKATE2_GAME_ROOT`;
that directory is not a successful runtime result.

Native constructor `BigBlack::BigBlack` (`0x82e98960`, exact RefSpec sequence
`0x82e98b00` through `0x82e98b34`) obtains attribute `aa97a406833ba601` from
the entity and constructs its protection instance. In the original data this
references **livingworld_entities_protect** (`da06e0e2b89b86c0`), collection
**default** (`d7edbd362d7d2152`). The port's selected Vault class list omitted
that class. Export metadata existed, so reference lookup returned an instance,
but the class and collection had never been constructed and the instance was
null.

Added the real protection class to `selected_original_asset_classes.rs`, shared
by mounted startup and corpus readers. `GetProtectionRadius` (`0x82e707c0`)
and `GetProtectionRegionEnabled` (`0x82e70920`) confirm the retained instance's
runtime reads and null assertions. The class contains nine authored attributes,
including protection radius 80, maintain radius 100, block radius 25, imminent
radius 7.5, minimum radius 2, and fixed-region enabled. These remain live Vault
reads; no replacement values or assertion suppression were added. Decoded
original data is recorded in `protection-attributes.json` in the task scratch
directory.

## Lifecycle review

Reviewed the existing callers against the retained native service packet:

- `StartSecurityTimerHUDAndSpawnBigBlack`, `0x8289dc98`: authored 300-second
  timer and separate warning timer ten seconds earlier, then HUD and spawn.
- `DeactivateSecurityService`, `0x8289d020`: hides the HUD group, invalidates
  timer IDs, sends the ending message, removes the icon, and unspawns.
- `OnChallengeSelect` / `OnChallengeDecline`, `0x8289c670` / `0x8289c6d0`:
  pause/unpause the main timer.
- `OnChallengeAccept`, `0x8289d428`: keeps the service only when the challenge
  allows it; otherwise dismisses with the original goodbye/CIC delay.
- `GameUnload`, `0x8289d9d0`: the host consumes unspawn while the Actor and
  living-world owners still exist, before the next game.
- Contacts `OnMenuSelect`, `0x82689d78`: active security service selects
  dismissal. Source availability, selection, and label all use actual entity
  status after the physical spawn/cull operation.

No change to these already translated timer/lifecycle rules was needed to repair
the demonstrated unloaded-model failure.

## Validation

- Five changed Rust files parse successfully with Rustfmt `--emit stdout`,
  `skip_children=true`; output discarded, source hashes unchanged by parsing.
- Backups, scoped diff, and file hashes are in
  `.scratch/contacts-parity-20261014/source-review-final.{diff,json}`.
- No Cargo check, compilation, or runtime test of the changed source.
- The diagnostic used a copied save. The real save SHA-256 remains
  `1058c9859f41c1149f97e904d8617385c12ab65b94683dc982dc6c853c23d280`.
- Existing executable SHA-256 remains
  `A26093EC281E50C23EDB66E5A26782301D7804303620B59605E20B15304A0757`.
- The earlier HoM clothing matrix fix remains in source and also needs a
  rebuild; see [hom-cloth-matrices-2026-10-14.md](hom-cloth-matrices-2026-10-14.md).
