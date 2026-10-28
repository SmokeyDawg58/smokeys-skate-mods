# Big Black: mission dismissal and challenge-map travel

2026-10-14. PC only, solo. Source changes; **not compiled or run in-game**.

Question: should missions dismiss Big Black? In playtesting he stayed behind after
changing locations, with travel through the challenge map.

## Reference

Live IDA, with Hex-Rays and debug symbols available:

- Database: `sk82_na_m.xex.i64`.
- Input SHA-256: `3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.
- Original Xbox database archive:
  `<game folder>\data\db\db.big`.
- Archive SHA-256: `6332b45232c4fe42257aa1082522fa8b9d594e2885767e6bba4fd74141273fa3`.
- Native pseudocode, raw instructions, popup bytes, message definitions,
  original-data audit, backups and scoped source review:
  `.scratch/big-black-travel-lifecycle-20261014/`.

Confidence: **high** for these source mismatches. Runtime and visual parity
remain unconfirmed until rebuilding and exercising the changed paths.

## Mission acceptance

`Sk8::Service::cServiceManager::OnChallengeAccept`, `0x8289d428`, checks Career
game flag 4 and the running `service_security` global activity. When the actual
Big Black entity exists, it reads the challenge's **local data** attribute
`AllowSecurityServiceToBeCalled` at `0x8289d49c`:

| Attribute | Original behavior |
| --- | --- |
| True | Keep Big Black; unpause his main timer if its ID is valid. |
| False | Block acceptance, deactivate the service, and add a 1.5-second goodbye timer. Its expiry allows acceptance to continue. |

Without an active protector it requests service-prerequisite reevaluation and
waits for the authored service activity to allow acceptance.

The port's `services::on_accept` already follows this branch and reads the
inherited local-data attribute. `signup::low_priority_accept` invokes it before
the start state, and `low_priority_accept_done` waits for the service gate.
Contacts challenge selection reaches this same signup flow. Completed map
retries and direct map Own the Spot starts use a different native loading path;
the map's dismissal check described below applies before that travel begins.
No blanket dismissal was added to `RunMainActivity` (native `0x8289e050` only
calls `HideBigBlackIcon`).

The original archive audit resolved both true and false attribute values across
challenge collections, including inherited records: 242 true, 560 false and two
unresolved values. These are collection counts, **not counts of playable
missions**. Runtime reads remain in the existing Vault adapter; no mission list
or tuning was hardcoded.

## Map travel requires dismissal

The original does not carry the hired protector through this map teleport flow:

| Native function | Address | Behavior |
| --- | --- | --- |
| `FrontEndState_ChallengeMap::OnMenuSelect` | `0x826828f0` | Before the info option travels, query `IsBigBlackServiceActive`; open dismissal confirmation when true. |
| `OnMenuSelectChallengeComponentProperty` | `0x82679600` | Apply the same check to an owned property. |
| `ConfirmPurchaseCallback` | `0x826751f8` | Complete the purchase first, then apply the same check. |
| `DismissAndTeleportCallback` | `0x826750f0` | Only result 1 deactivates security, blocks map input and starts the existing fade/teleport. Result 0 does nothing. |

The port still had comments assuming there was no Big Black service and returned
the travel action directly. Added the missing confirmation to the shared map
flow, covering locations, active challenges, completed-challenge Retry, owned
properties and newly purchased properties.

Native popup template **217**, `gPopupTemplate + 217 * 56` at `0x82fdebd0`:

- Style 0; title `ID_COMMON_WARNING_TITLE`.
- Description `ID_BB_DISMISSAL_CONFIRM`.
- Two buttons: `ID_COMMON_NO`, then `ID_COMMON_YES`.
- Default selection 0; not cancellable with B.

The existing authored `sk8popup` renderer presents this template. A switch from
the property purchase confirmation to dismissal starts a new popup intro;
moving the selection within the same popup does not restart it. Declining
dismissal retains the map page, service and destination; it does not undo an
already completed property purchase. The existing purchase diagnostic's enum
match now recognizes the new popup and leaves it to manual input; no test was
added or run.

On confirmation, the host drains the queued living-world operations before
starting travel. This applies the real pedestrian cull and actual-entity status
readback while the current Actor, terrain and living-world owners still exist.
It does not wait for a simulation tick after the stream focus has moved.

## Shared shutdown sent the wrong event

`cServiceManager::DeactivateSecurityService`, `0x8289d020`, invalidates the two
timer IDs, sends **cMsgServiceEnded**, removes the map icon and unspawns the
entity. The source incorrectly sent **cMsgServiceAboutToEnd**.

The executable folds both `SendMsg` template instantiations into the same
function, which IDA labels `SendMsg<cMsgServiceAboutToEnd>`. The constructed
message kind proves which message is actually sent:

- Raw `0x8289d070..0x8289d0a8`: kind **0x82b365c4**, key 0.
- `cMsgServiceEnded::MessageDefinition`, `0x8216c9b0`: **0x82b365c4**.
- `cMsgServiceAboutToEnd::MessageDefinition`, `0x8216c974`: **0xbaccd82a**.

Corrected the shared shutdown event. The ten-second warning keeps its original
AboutToEnd event. Original `service_security` bytecode confirms the distinction:

- Ended handler at `ChallengeCore.bin + 0x13a8d8` plays the completion speech and
  changes to `service_ended` for key 0.
- AboutToEnd handler at `0x13a758` only plays the timeout warning speech.

This correction applies to map dismissal, Contacts dismissal, prohibited mission
entry, timeout and game unload through their existing shared shutdown method.

## Scope and validation

Compiler follow-up: the new map caller exposed E0624 because the helper was
visible only to `public_facade::career`. Changed `apply_career_living_world`
to `pub(in super::super)`, allowing the `public_facade` ancestor and its
descendants to call it. No method-body or gameplay changes. This correction
was not compiled or runtime-tested; backup in
`.scratch/big-black-travel-visibility-20261014/`.

- Five edited Rust files passed Rustfmt syntax parsing with `--emit stdout` and
  `skip_children=true`. File hashes were unchanged by parsing.
- Reviewed scoped diffs against pre-task backups and all `MapPopup` matches.
- No compilation, Cargo check, tests or game launch. The exact reported travel
  sequence still needs rebuilt runtime confirmation.
- No physics, spawn distances, navigation rules, Android or save-file changes.
- Prior HoM signup, multiplier-ring, navigation and Contacts fixes preserved.

Separate pre-existing follow-up: the session-marker adapter still reports no
security service and retains pending implementations for its load-screen query
and dismissal popup. IDA has a separate `PlayerUI::DismissAndTeleportCallback`
(`0x828af178`), used only for marker teleports requiring loading. Its evidence
was captured, but that adapter is not part of the challenge-map path repaired
here. The Create-a-Spot popup also has a separate callback (`0x826731d0`). No
complete parity claim is made for either separate UI path.
