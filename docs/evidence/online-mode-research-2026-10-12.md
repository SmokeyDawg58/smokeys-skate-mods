# Online modes: original menu reachability (2026-10-12)

## Scope and conclusion

Read-only research: inspect the Xbox debug build in live
IDA and compare names with the original menus to identify potentially unused
online modes. No production code changes, compilation, tests, or game launch.
PC repository only; Android remains parked.

**1-Up is the strongest finding.** The normal ranked, unranked, and ranked
leaderboard selectors explicitly exclude it. The separate developer Online
menu does not exclude it and passes a selected OneUp type/key to the online
launch flow. That developer menu exposes its controls only when the online
connection reports `CONNECTION_READY`. This establishes a retained launch path,
not a successful present-day online match or a fully working hidden mode.

The supplied Xbox and PS3 assets both retain the 1-Up challenge records and
the normal-menu exclusion. This is not evidence of Xbox-debug-exclusive content.

## Original menu baseline

The main online menu dispatches ranked/unranked selection to Quick Match or
Custom Match (`FrontEndState_MainMenuOnlineOptions::OnMenuSelect`, `0x82674b58`;
`InitOnlineMenuPopup`, `0x826713e8`). The hub has corresponding routes in
`SetupRankedOnlineMenu`, `0x82688d98`, and `SetupUnrankedOnlineMenu`, `0x82688e48`.

The native nine-entry network catalog is `Sk8::Net::kOnlineChallengeInfo` at
`0x83003da0`, stride 268 bytes, confirmed from the PDB `ChallengeInfo` layout.
`GetSk8ChallengeType`, `0x827840b0`, maps it into the broader challenge enum.
The normal menu titles come from `m_iaChallengeNameHALID` at `0x82fe06b0`,
resolved through the original English localization data in `miscboot.big`.

| Network ID | Internal name | Challenge type | Normal match selector |
| --- | --- | --- | --- |
| 0 | OwnTheSpot | 9 | Spot Battle |
| 1 | Slap | 12 (Lap) | No qualifying challenge records in either supplied base catalog |
| 2 | BestTrick | 8 | Best Trick |
| 3 | Jam | 15 | Jam |
| 4 | OneUp | 32 (1up) | Explicitly excluded |
| 5 | SKATE | 4 | S.K.A.T.E. |
| 6 | DeathRace | 19 | Deathrace |
| 7 | HallOfMeat | 31 | Hall of Meat |
| 8 | OnlineFreeSkate | 22 | Separate Freeskate route, outside the competitive selector loop |

This yields the six ordinary competitive mode families. The original EA manual
also describes six ranked challenge modes and says unranked uses the same modes:
[Skate 2 Xbox 360 manual](https://www.gamesdatabase.org/Media/SYSTEM/Microsoft_Xbox_360/Manual/formated/Skate_2_-_2009_-_Electronic_Arts.pdf),
printed pages 6-7 (Xbox LIVE section). Menu interpretation above comes from code
and original data.

### Eligibility is data-driven

`OnlineProfileCmdTranslator::sMethod_IsChallengeValid`, `0x8263d320`, forwards
through `IsChallengeValid`, `0x8269e490`, to `IsChallengeTypeValid`, `0x826988b0`.
That converts the network type and requires a nonempty result from
`ChallengeOnlineManager::GetOnlineChallengesByType`, `0x825bf228`.

The latter collects leaf `challenges` records, follows GlobalData, requires the
boolean field `0xf9a9a1dded84a643`, checks `ChallengeType`
(`0x02b828ccdb4785e4`), and requires `Location` (`0xf1c6d5fa4a23951e`) to be
registered with the location manager. These attribute hashes were checked in
raw PPC instructions; the decompiler mishandles several 64-bit parameters.
`GetLeafCollections`, `0x82ab9710`, removes records that are parents of others.
An enum entry alone therefore does not put a mode into a menu.

Both supplied base databases contain 804 challenge records. Before the runtime
location-validity check, their online leaf counts agree: SKATE 10, BestTrick 10,
OwnTheSpot 33, Jam 11, DeathRace 18, OnlineFreeSkate 18, HallOfMeat 20, 1up 10,
OnlineFreeSkateActivity 162, and Property 1. These are record counts, not counts
of modes or confirmed playable locations.

## 1-Up: explicit UI exclusion and substantial retained content

### Three exclusions

- `ranked/ranked_custom.apt`, `PopulateListData`: the condition at
  `0x9ca4..0x9cc2` requires `IsChallengeValid(i)` and `i != mOneUpIndex`.
  `mOneUpIndex` is assigned 4 at `0xacec..0xacf2`.
- `unranked/unranked_custom_match.apt`: the corresponding condition at
  `0x7d38..0x7d56` uses `i != m_1upIndex`; the constant 4 is assigned at
  `0x8701..0x8707`. This movie also handles the host option state.
- `InitRankedLeaderboardChallengeTypePopup`, `0x826712a0`, explicitly requires
  validity and `v4 != Challenge_OneUp` before adding a leaderboard option.

The ranked, unranked, and main online menu APT/CONST payloads inspected here are
byte-identical between the supplied Xbox and PS3 folders (SHA-256 recorded).

### What survives

Original instructions describe players setting and beating scores on selected
obstacles, with round wins earning letters toward spelling `1-Up`.
Relevant labels include `ID_MISSION_TEMPLATE_1UP_DESCRIPTION`,
`ID_THROWDOWN_1UP_DESCRIPTION`, and `ID_PARTYPLAY_1UP_DESCRIPTION`.
The Party Play labels are leftovers; their presence does not establish a normal
Party Play selection route. The native Party Play challenge enum has SpotBattle,
Skate, and HOM.

Five specifically named online entries:

| Collection | Display title |
| --- | --- |
| `ol_1up_01_svm_mountainup` | Mountain Up |
| `ol_1up_02_dt_matrix` | 1-Up the Matrix |
| `ol_1up_03_ot_cathedral` | Cathedral 1-Up |
| `ol_1up_04_wf_gvr` | GvR 1-Up |
| `ol_1up_05_ur_lombardier` | Urban Ruins 1-Up |

Five additional `t_*_1up_*` leaves have the online boolean enabled. There are
18 type-32 records including parents and offline records. All ten online leaves'
location strings are present, NUL-terminated, in the global locator arenas;
this was a string-presence check, not a runtime location-manager capture.
Stream/boundary XML and mission assets also exist for these online locations.

`ol_1up_02_dt_matrix` inherits a 25-state graph including `setup_1up`,
`round_setup`, `wait_for_leader`, `wait_for_scoring`, `leader_success`,
`leader_failure`, `prepare_follower`, `follower_1up`, `wait_for_1up`,
`process_round`, `round_highlight`, `show_results`, and `complete`.
The Shingo introduction record references the same states.

Lua chunks in the original `ChallengeCore.bin` contain score comparison,
selected/used objects, turn handling, round wins, HUD updates, highlight creation,
and cleanup branches for SinglePlayer, PartyPlay, and OnlineChallenge.
Example chunk offsets: `0x148a18` (cleanup), `0x1775ac` (round processing),
`0x183560` (follower scoring). Native audio notification hooks include
`OneUpSuccessHandler`, `0x824ffc10`, and `OneUpFailureHandler`, `0x82500008`.
These establish surviving systems, not that every path is complete.

`StateOnlinePlayNow::BuildFilters`, `0x8276d2f8`, even retains the ranked OneUp
player limit of four alongside SKATE. Its Quick Match path does not perform the
normal menu's explicit OneUp exclusion. Consequently this report does not claim
that an invitation or externally supplied matchmaking result can never reach it.

### Actual developer menu route

`cDebugFEMenu` constructor, `0x82741018`, constructs an `Online` submenu and
adds it to `Debug Frontend Menu`.

`DebugOnlineMenu` constructor, `0x8273ff30`, loops all nine network types and
adds any with a nonempty `GetOnlineChallengesByType` result. Unlike the normal
APT menus, it has no special OneUp exclusion. It supplies selectors named
`Select Challenge Type` and `Select Challenge`, plus `Play Now!`.

`ResetItems`, `0x8272bf00`, adds those controls only after
`GetStatus() == CONNECTION_READY`. `onChange`, `0x82737100`, populates challenge
keys for the selected type. `onCommand`, `0x82736c90`, puts the selected type and
key into `StateMessage_EnterOnlineGame` and sends it to the flow manager.
Thus a developer route exists conditionally; successful service connection,
match creation, and completed play have not been demonstrated here.

## Other candidates and weaker remnants

### Spot Race

- Broad challenge enum value 20; no corresponding current network ChallengeType.
- English `ID_MISSION_TEMPLATE_SPOT_RACE_DESCRIPTION` describes racing between
  spots, completing trick objectives, and crossing the finish. The old pause
  hint `ID_PAUSEMENU_HINT_35` calls it an online-exclusive event.
- Stream and boundary files remain for financial/library, matrix, park, stadium,
  zen; oldtown upper/warehouse; parkade; community center; and a test course.
- **Zero type-20 challenge records** in both supplied base catalogs.
- `MissionCmdTranslator::sMethod_GetSpotRaceObjective`, `0x8260f7c0`, and
  `GetOldSpotRaceObjective`, `0x8260f898`, return literal placeholder names when
  a main challenge exists, rather than obtaining an actual objective. Raw
  instructions were saved alongside the decompilation.

Conclusion: evidence of an older online mode design and retained assets/UI, with
no normal selection route found. It is not established as a finished hidden mode;
whether parts became another race variant is unresolved.

### Slap / Lap

Network ID 1 (`Slap`) maps to broad challenge type 12 (`Lap`). The network catalog
and converters retain it, and localization contains Lap Race/Slap labels and
leaderboard text. The normal title/description entries are still
`Lap Challenge Title TODO` and `Lap Challenge Description TODO`.
Both catalogs have **zero type-12 challenge records**, so the valid-content gate
rejects it in the normal selector and the developer selector's collection loop.
An old `slap_test` stream/boundary pair also remains.

### Grind Race

PDB enums retain `Sk8::Xml::ChallengeType::ChallengeType_GRINDRACE = 8` and
`Sk8::Challenge::eRaceType_GrindRace = 5`. The current network enum has no
GrindRace entry, and `GetXmlChallengeType`, `0x82784148`, never returns the XML
GrindRace value. No checked challenge record has `RaceType == 5` in either base
catalog. Classification: enum/protocol/race-variant remnant, not demonstrated
playable content. No mechanics are inferred from its name.

### Last Man Standing, Slalom, Slide, Gate

These broad challenge enum values (21, 10, 11, 13) have zero challenge records in
both base catalogs. Their generic title/description labels are `TODO` placeholders.
They have no independent entry in the current network mode enum. Evidence is
insufficient to infer their intended rules or completeness.

### Last Man Standing follow-up: classification survives, rules not established

The original `gChallengeTemplate[21]` at `0x82fda720` provides a more specific
clue. Its `Sk8::Challenge::sChallengeTemplates` row is 32 bytes:

| Field | Original value |
| --- | --- |
| `mType` | 21, `eChallengeTypes_LastManStanding` |
| Icons | `unknown`, null, null |
| `mTitle` / `mDescription` | `ID_MISSION_TEMPLATE_LAST_MAN_STANDING_TITLE` / `_DESCRIPTION` |
| `mMode` | 2, `eChallengeInfo_SinglePlayer_Online` |
| `mKind` | 3, `eChallengeInfo_Trick_Race` |

The row's raw big-endian words are `00000015 82045cc0 00000000 00000000
82045cc8 82045cf4 00000002 00000003`. Field offsets and enum names were checked
against the PDB types in live IDA. It is the only one of the 51 template rows
with kind 3; the separate kinds are None=0, Trick=1, Race=2, Trick_Race=3.
This is exact classification metadata. A combined trick/race concept is a
possible interpretation, not established gameplay. These fields cannot tell us
whether eliminations depended on bailing, collisions, scores, or race position.

The follow-up checked:

- Named native references: front-end enum registration (`0x826111f4`), Lua enum
  export (`0x8287fc30`), the debug challenge filter list (`0x820d4950`, used in
  the debug front-end constructor), and title/description/template tables.
  No function or class named LastManStanding was found.
- The nine direct template-array consumers: challenge telemetry, title getters,
  poster titles, and icons. No mode-specific game rules were found in them.
- Eighteen direct callers of `Attrib::Gen::challenge_global_data::ChallengeType`
  (`0x824b2868`): no LastManStanding-named case in their decompilations.
- Both 804-record base challenge catalogs: zero type-21 records.
- 1,537 compiled Lua chunks per supplied game copy: no string constant matching
  LastManStanding / LastMan / Last_Man / bounded LMS. Raw database strings did
  retain the generic enum names in `progressioncore.bin`.
- 35,013 Xbox and 36,407 PS3 named BIG entries: no matching mode asset name,
  with no archive-header read errors. Also checked 396 Xbox APT/CONST payloads;
  no matching Last Man Standing or elimination text was found there.
- English localization retains the TODO title and description plus rank and
  leaderboard labels. It supplies no Last Man Standing instructions.

Generic elimination-related Lua was traced through message-handler blobs,
state records, and the challenges referencing those states. The SKATE
`process_round` / `follower_not_copy` handlers belong to type 4. The contest
`run_round` / `end_round` / results handlers are referenced by types 2, 6, 8,
and 15. No connection to type 21 was established; those scripts are not evidence
for Last Man Standing rules.

Confidence: exact for the template, enums, counted records and search results;
unresolved for intended mechanics and how much implementation once existed.
Negative name searches and decompilation inspection cannot prove that no
anonymous or shared implementation survives elsewhere. No playable flow or
1:1 reconstruction can be claimed from this evidence. No game code was changed,
and no builds, tests, or game launches were performed in this follow-up.

Tool-output packet: `.scratch/online-mode-research-20261012/last-man-standing-followup.json`.

## Names that should not be counted as hidden modes

- OwnTheSpot is the internal name behind the ordinary **Spot Battle** menu label.
- Jam is an ordinary competitive mode.
- `SetupFreeskateActivitiesMenu`, `0x8268a500`, explicitly exposes Gap,
  Accumulation, Group Trick List, Simultaneous Trick, Survival, and Tag categories,
  enabling each according to available activities. Tag's label is `Tag the Spots`.
  All six have authored activity records in both supplied catalogs.
- `KingOfTheMountain` is registered in the awards enum (`BindEnum<eAwards>`,
  `0x8287f260`) and appears in award-grant progression code; it does not by itself
  identify a separate online mode.
- `SetupTestMenu`, `0x82689290`, is a hub UI test with Singleplayer/Offline/Online
  menu links. Its existence is not evidence of another competitive game type.

## Evidence, confidence, and limits

Exact: catalog IDs, enum values, normal-menu exclusions, developer menu conditions,
placeholder returns, payload identity, and recorded counts from the checked data.
High: OneUp has substantial retained authored game flow and a conditional debug
launch route. Unresolved: end-to-end playability, original online service acceptance,
historical removal reasons, and whether weaker remnants were cancelled or reworked.

Research packet: `.scratch/online-mode-research-20261012/`:

- Six decoded original APT audits, Xbox/PS3 main/ranked/unranked.
- `native-menu-and-mode-reference.json`: 30 native functions; raw PPC for key gates.
- `native-debug-menu-reference.json`: ten more functions, five PDB enums, catalog.
- `xbox/ps3-challenge-inventory-v2.json`: corrected inline-pointer relocation reader.
  The earlier unversioned inventories are superseded: their text fields were null
  because inline pointers were not followed. Type/count findings are unaffected.
- `xbox-online-localized-labels.json`: original label/string pairs.
- `original-content-summary.json`: menu hashes, counts, asset names/hashes, locator
  string matches, 25 state names, and selected Lua call summaries. The call summaries
  are not a substitute for bytecode control-flow analysis.
- `inspect_original_catalog.py`: read-only helper using the existing Vault/BIG readers.

Sources stayed in the supplied original game folders. Only decoded research and
metadata were written under `.scratch`. No modes were enabled or reconstructed.
