# Career Contacts, protection services, and mission security

Date: 2026-10-14. PC release built afterwards. Android unchanged.

Later runtime investigation reproduced missing bodyguard model residency and an
omitted protection-attribute class. It also corrected the phone/timer 3D layers
and Contacts HUD hiding. See [the source-only follow-up](contacts-parity-2026-10-14.md)
before treating the historical build below as a working protection service.

A subsequent boot failure exposed missing InitAction registrations for the
new timer/Contacts movies. Corrected and rebuilt; see
[the startup repair](contacts-apt-init-2026-10-14.md). The first successful build
record below is historical and did not establish successful game startup.

The next run reached Contacts and failed in the shared button font
lookup. The helper now uses the original case-insensitive/fallback resolver;
see [the font repair and rebuilt release](contacts-font-2026-10-14.md).

## Scope and evidence

The requested work covers mission security disabling, the pumping tutorial's
counter, and the complete offline career Contacts path, including the living
Big Black protector. HoM unlock findings are in
[hom-unlock-followup-2026-10-14.md](hom-unlock-followup-2026-10-14.md).

Live IDA was unavailable. IDA 9.3 batch analysis used the
separate saved debug database
`world_probe_temp\waypoint_replay_20261014.i64`, with input SHA-256
`3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.
Decompiler output and PPC disassembly are retained in
`.scratch/career-services-20261014/native.json` and `native-2.json` through
`native-26.json`. The original linker map and original Xbox Vault, Lua, and APT
data were also read. VMX register display mistakes in IDA were checked against
the actual instruction operands before transcribing vector calculations.

Confidence: **high** for the recovered source contracts and integration described
here. This is not an exact runtime/visual parity certification. The normal
release build passed after the integration repairs below. No game or regression
test was run. At the initial source handoff, Rustfmt was used only to parse
source, with output discarded and `skip_children=true`.
Existing compiled format/core libraries were used for original asset inspection;
those tools do not validate the changed implementation.

## Security in missions

There is no universal rule that all missions remove or disable guards. The
following controls have different owners and must remain distinct:

| Reference | Address | Contract |
| --- | --- | --- |
| `Chasee::GetGroupEndChaseReason` | `0x82de1de0` | For the actual local chasee, the local force-terminate flag overrides the stored reason with reason 0. It does not overwrite the stored field. |
| Pedestrian chase-end query / guard conditions | `0x82e6fdd0`, `0x82e69ea0` | Query the virtual group-end reason, rather than only the stored integer. |
| `Chasee::CanAddNewChaser` | `0x825ef830` | The resolved group-end reason participates in admission. |
| Chasee ongoing update | `0x82de39f0` | Resolved group-end state gates ongoing mood delivery. |
| Challenge character spawn permission | `0x825a3160` | Mission owner, alternate character/hide flags, game mode and census constraints decide which characters can spawn. |
| Challenge force-cull decision | `0x825a85f0` | Uses authored remove/hide/clear/avoid/census decisions. Being in a mission alone is insufficient. |

The port previously read the stored chase-end reason in several consumers,
omitting the local forced-termination override. Connected the shared query to
Actor chase updates, guard conditions, new-chaser admission and capture detection.
Local identity comes from SkaterManager, not an assumed player zero. The original
global termination getter is called but its result is discarded in this debug
build; do not silently make it another termination rule.

Existing character spawn/cull branches matched the inspected reference. No
blanket mission guard ban was added. The specific reported mission was not
identified or reproduced, so this is a confirmed shared mismatch, not a claim
that its particular encounter was necessarily forbidden.

## Pumping tutorial

Inspected the original `m_2a_01_tutorial_slappy` objective set, its Lua conditions,
and `cChallengeObjectiveGroup` evaluation (`0x82d4c508`, `0x82d4e978`). Inspection
logs are `inspect-grabs.log`, `inspect-pump-conditions.log` and
`inspect-grab-objectives.log` in the scratch directory.

The displayed `ID_CHALLENGE_M_2A_01_TUTORIAL_PUMPING_OBJ1` counter is driven by four
sequential hidden objectives. Each increments `vars.objcounter`, checks `>= 90`,
then tests volume `1854678612805382`,
`DIST_Projects|m_2a_01_tutorial_slappy_vol05`. The surrounding pump-area conditions
use volumes 02/03. These original conditions contain neither a grab predicate
nor a requirement for a distinct takeoff/landing for each counted step.

No speculative one-grab-per-air filter was added. The exact reported movement
was not reproduced. This finding concerns the inspected pumping tutorial, not
every mission that counts grabs. The shared objective actor selection correction
in the HoM follow-up is independently supported by the native group update.

## Contacts frontend and challenge choices

| Reference | Address | Recovered behavior |
| --- | --- | --- |
| `PhonelistHUD::CanShowPhoneListOptions` | `0x826b5d90` | Career flags 4, original Uncapping Available setting, no main challenge except type 9. |
| `PhonelistHUD::UpdateInput` | `0x826c0848` | Visible maximized phone HUD, Y release, cellphone sound, push FE state 74. |
| `FrontEndState_PhoneList::OnEnter` | `0x826876b0` | Original sorted leaf collections, progression settings, sponsor and rank filters. |
| `FrontEndState_PhoneList::SetState` | `0x82681fc0` | Menu stack and choice construction. |
| `FrontEndState_PhoneList::ReceiveInput` | `0x82689cf8` | Intro gating, selection and back navigation. |
| `FrontEndState_PhoneList::OnMenuSelect` | `0x82689d78` | Service/cash handling, location and opponent choices, film dispatch. |
| `GetOptionText` | `0x8265d828` | Native labels, service prices, unbeaten-pro prefix. |
| `StartPickAProChallenge` | `0x8265de20` | Selected profile/root category, pro-film identity, speech character, progression result message and challenge key. |
| `StartThrowdownChallenge` | `0x8265e168` | Location's SKATE challenge and each selected pro's SKATE profile and completion message. |
| `cFilmManager::ReevaluateActivityPrereqs` | `0x82db0f88` | Actual available/completed AnywhereFilm lists. |
| `cFilmManager::Start[Completed]FilmChallenge` | `0x82da3470`, `0x82da3510` | Starts the chosen main activity directly. |

Implemented `challenge/contacts.rs` and `challenge/film.rs`, then connected them
through session input, FE pause/audio state, intro callback and screen ownership.
The controller path is **hold LB, then release Y** while its phone options are
available. It uses the original input-map meaning; it is not an always-active
shortcut.

The main list supports Services, Throwdown, Film Anywhere, Completed Films and
Pick-a-Pro as their native prerequisites permit. Throwdown requires the Alphabet
Shingo completion plus available locations/opponents. Pick-a-Pro retains the
original rank and board-sponsor exclusion. Single/multiple-pro flows preserve
chosen opponents through the existing challenge AI loader and result messages.
Insufficient-cash handling, back navigation, selection sounds, saving-busy menu
availability, and service-in-progress gating are connected.

Original APT movies are used: `phonelist` (`fbbb45c1`), its
`basic_menu_picker` import, and `hud2/bbtimer` (`bcb2eeb7`). Text locations,
page skins, item exports, 38-unit spacing, visible-row counts, scrollbars, button
style, intro/outro and timer placement come from the original movies/scripts.
The original pro prefix is `#\xbb %s`; `#` is the literal-string marker, not a
displayed hash. Missing authored timer glow children remain inert writes, as in
Flash, rather than being fabricated visual layers. No visual screenshot test
was performed for these screens.

## Service lifecycle

| Reference | Address | Behavior |
| --- | --- | --- |
| Service availability | `0x8289c388`, `0x8289c3e8`, `0x8289c448` | Progression settings; offered pool/uncapping key; chase, main challenge, AI release and introduction checks for security. |
| Service activation | `0x8289d5c8`, `0x8289d6d0`, `0x8289d7f0` | Original request message, cash debit and statistics: security $500, uncapping $100, drainage $200. |
| `StartSecurityTimerHUDAndSpawnBigBlack` | `0x8289dc98` | Authored 300-second timer, warning 10 seconds before expiry, HUD activation and physical spawn. |
| `DeactivateSecurityService` | `0x8289d020` | Hide timer group, invalidate timer IDs, send ending message, remove icon and protector. |
| Challenge select / decline | `0x8289c670`, `0x8289c6d0` | Pause/unpause the main security timer. Warning timer is separate. |
| `OnChallengeAccept` | `0x8289d428` | Original `AllowSecurityServiceToBeCalled` flag; allowed service continues, otherwise goodbye and a 1.5-second CIC wait. Inactive service graph reevaluates prerequisites. |
| Challenge completion | `0x8289c5c8` | A 0.5-second delayed service notification refresh after a main challenge. |
| `GameUnload` | `0x8289d9d0` | Clear introduction/menu-disable state and deactivate active protection. |
| `StateManagerFrontEnd::CheckForOptions` | `0x82656718` | Reads `mbDisableBackpackAndPauseMenu` before opening StartHub. |

Authored service Lua still owns conversations, movies and uncapping/draining
work. The new phone choices invoke those real activities. World-painter changes
and the delayed refresh now reemit their original service offers. Existing
dynamic-hull loading/state persistence is retained; service statistics, cash
totals, pool percentages and native achievement thresholds are connected to the
existing saved statistics backend. The challenge-replay spawn/unspawn natives
use the same physical protector owner.

Big Black's active status is read back from the actual LivingWorld entity, not
set optimistically when a request is queued. Timer IDs are invalidated instead
of deleting unrelated timers, so an old expiry cannot dismiss a later hire.
The live map icon follows the real entity. Early career unload flushes the
unspawn/cull while the Actor and LivingWorld owners still exist.

The port already wrote `pause_menu_disabled` from service/mission scripts, but
the StartHub opening path never read it. Added that missing reference-backed
check; it does not suppress input to an already-open menu or to the phone HUD.

## Big Black entity and protection

The previous physical spawn output had no complete protector implementation.
Added the actual BigBlack specialization of the retained pedestrian, sharing
its collision, animation, navigation, state storage and EntityMan.

Key evidence:

- Constructor/reset: `0x82e98960`, `0x82e86300`; original protection RefSpec,
  BigBlack memory store 11 and AI flag `flags[1] |= 0x20`.
- Original `HCDelegate_Big_Black`: 19 states, 34 behaviors, 18 conditions;
  generated expressions `0x828fbeb0` / `0x828fc1c0`, original update schedules
  and callback order, TemplateHC seven time slots / sixteen active slots.
- `ProtectProtectee::Begin` `0x8270d0a0`: acquire an unprotected IProtectee,
  set its reciprocal protector ID and initialize the zone center.
- `PreStateGraphUpdate` `0x82e9f828`: base update first; assigned/wanted owner,
  fixed/moving protection region, original protection and outer radii.
- `UpdateZoneInterests` `0x82e70a78`: threat intercept/block/current position,
  otherwise protectee position; retain the old interest point outside the zone.
  `LWCircularPatrolZone` `0x82e4d040` / `0x82e4d130` uses full XYZ, inclusive
  radius comparisons and the authored outer radius. Shared zone predicates and
  mood/threat processing now dispatch to that zone too.
- `EvaluateThreatToProtectee` `0x82e96b00`: determine threats from the actual
  protectee's chasers, evaluate geometry, native partial sorting/primary switch,
  choose action; clear threats when the IProtectee no longer exists.
- Threat ordering/action: `0x82e7b8c0` / `0x82e7b778`; block geometry
  `0x82e7b4d0`; original geotest helpers `0x82e559b8`, `0x82e55dd0`,
  `0x82e56340`. Strict comparisons and unusual native arithmetic are retained.
- `GetProtectorThreatBlockPoint` `0x82e82520` resolves the assigned protectee,
  with the protector's own position when absent. Following/watch/interception/
  block callbacks `0x8270d1d8` through `0x8270dee8` feed the shared nav rig and
  original takedown prediction, intent and animation callbacks.
- `AddModelsToWorld` `0x82e704a0` always uses world collision and ACTIVE_BODY
  for both bodies (raw PPC `r5=1`, `r6=4`, `r7=4`).
- `AddRAINTestPoints` `0x82e92508` adds a compulsory-stop point on roads,
  using target translation, half model Z length and the real pedestrian ID.
  Connected the manager's previously absent pedestrian producer as a whole:
  ordinary pedestrians `0x82e91c88` use their Colliding flag; manager
  `0x826f5218` visits pedestrians before vehicles/props/DMOs, in EntityMan order
  (`0x82e82fb8`). `IsOnRoadNetwork` `0x82e6e378` retains the cell-score gate.
- `TestLOSim` `0x82e968a8` retains full simulation while a live threat exists.
  Big Black is not population-census managed. His original nav attributes have
  BigBlack avoidance disabled, so no invented self-avoidance exception is needed.
- `OnCullRequested` `0x82e863c0` clears the reciprocal protector before base
  cull; destruction releases the same BigBlack memory store and shared owners.
- `KnowAboutProtectee` `0x82e96c08` was inspected. It is not a callback in the
  authored BigBlack delegate; no speculative extra knowledge-update call was added.

Spawn/unspawn use `Manager::SpawnBigBlackMsgHandler` `0x82719598` /
`UnspawnBigBlackMsgHandler` `0x8271f618`. The spawn searches newest breadcrumbs
with the original strict distance/collision checks, then the original rotated
offset candidates and vertical/horizontal line tests, water rejection and last
fallback. Cvars remain runtime references. The original entity
`big_black_regular01` uses `big_black_lw`, whose model ancestor is already in the
required preload seed set; no duplicate model/asset loader was added.

## Files and review limits

`.scratch/career-services-20261014/scoped-manifest.json` maps all 72 changed/new
Rust paths to their pre-task backups, including the two HoM follow-up paths.
It is scoped to this task in the heavily modified existing worktree.
The implementation spans challenge managers, live session/AI ownership,
pedestrian protection and PC APT rendering/input. No gameplay time scale,
skater force tuning, real save contents or Android files were edited.

Controller navigation, authored-screen rendering, paid-service completion/
persistence, Big Black's observed protection behavior, mission transitions and
menu exit still require in-game validation. Successful compilation and source
review are not substitutes for those checks.

## Release build follow-up

Compilation followed. The first release attempt found
three integration errors, repaired without changing reference calculations or
event order:

- Resolve the BigBlack distance condition through the existing
  `generated_action_load::conditions::compare` module.
- Include the nine new tuning entries (indices 1273 through 1281) in the cvar
  registration list. Their reference-backed definitions and values are unchanged.
- Reborrow the input owner after paused replay clothing restoration, ending the
  earlier mutable borrow before restoring the retained presentation owners.

`cargo build --release -p skate2-pc -j 1 --timings` then passed, exit 0, in
12m43s. Normal optimization settings retained; 438 core and 19 PC warnings.
No tests or game launch. The executable is
`target\release\skate2-pc.exe`, 43,935,232 bytes,
modified `2026-10-14T04:00:00.2395713+03:00`, SHA-256
`336FD047CC8F030F0296B3DB1E955C728DF9214A6D3CFB2FBD9B1E473C40A60C`.

Build logs `release-build-1.log` and `release-build-2.log`, `.before-build`
backups, and all 72 scoped source hashes in `release-source-snapshot.json` are
retained under `.scratch/career-services-20261014/`. Only the three files above
changed from the preceding source handoff during compilation. No real save or
Android file was modified.
