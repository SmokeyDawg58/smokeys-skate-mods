# Edit Skater shop and receipt (2026-10-09)

Clothing in the career's Edit Skater is bought with the career's cash: parts behind an unlock are
locked, parts that cost money go on a receipt (the shopping cart) when tried on, and the receipt
screen buys them. Before this the port ran the editor with `EditSkaterPolicy {
unlock_all_clothing: true }` (every part owned and free); the policy is now off for the game.

## Native behaviour

### Part status (`festate_cac.cpp`, `fe_cac_filters.h`)

- `GetPartStatus` (`0x82659be8`): `locked`, else `purchased` (on the receipt), else `owned`, else
  `dlc`, else `none` (for sale). createskater's `UpdateThumbnailIcon` plays the status as the
  thumbnail's `mIcon` frame; `UpdateInfoPanel` (`0x00f70c..0x00f7ce`) shows
  `ID_CAC_FILTER_LOCKED` for a locked part, nothing (`#`) for owned, DLC or receipt parts, else
  `CAC_GetItemCost` as currency.
- `LockedFilter::AcceptImpl` (`0x82649010`): a part with neither `pUnlock` (u16 attribute 41)
  nor `pSponsorUnlock` (37) is unlocked; otherwise it stays locked until
  `UnlockablesBackEndCore::IsUnlocked(15, unlock, true)` or `IsUnlocked(16, sponsor, true)`.
- `OwnedFilter::AcceptImpl` (`0x82652c90`): a cost of 0 or -1 is owned; a part on the receipt is
  not; else `CACRecord::IsOwnedPartID` (installed, or in the slot's owned list).
- `DLCFilter::AcceptImpl` (`0x82670190`): `pDlcID` (7) set and every comma-separated id non-zero.
- `NewFilter::AcceptImpl` (`0x82648e00`), behind `IsPartNew` (`0x82649590`, part thumbnails only)
  and the thumbnail's `mNewIcon`: `IsNewUnlock(15, unlock) || IsNewUnlock(16, sponsor)`.

### Trying a part on (`OnThumbnailSelect` `0x8268aee0`)

| Part | Sound | Then |
| --- | --- | --- |
| remove row, owned, already on the receipt | `core_a_button` | worn |
| locked | `core_error` | nothing |
| costs more than the balance | `core_error` | `ID_CAC_NOT_ENOUGH_CASH_TITLE/_DESC` popup, one option `ID_COMMON_OK` (`FrontEndState_Popup::OnEnter` `0x82658474` sounds `core_popup`) |
| any other | `cas_accept` | `AddPartToList` onto `mReceiptList`, `SetScreenState(52)`, worn |

The balance is `GetReceiptBalance` (`0x82649ec8`): `GetCash` less every receipt line (-1 reads
as 0). `NotEnoughCashCallback` (`0x82659e78`) opens the receipt in mode 1 when it holds a part.

### Menus (`FrontEndState_CaC::ReceiveInput` `0x8268d818`, `OnLoaded` `0x8268ae20`)

- `mCACState` is set by `SetScreenState` on Slappy's (menu 1), Back Alley Pete's (11) and the Edit
  Skater menu (9, which also empties the menu stack). `mSubState` 1/2 is the new career's skater.
- `OnLoaded`: the new career's skater opens at Back Alley Pete's, any other at the Edit Skater menu.
- B with more than one menu stacked: `core_b_button`, then at Slappy's with a part on the receipt
  the receipt (mode 0), else `PopScreenState`. B at the root: nothing for the new career's
  skater, else `core_b_button` and `Finish`.
- Start (`cas_finish`) by `mCACState`: Back Alley Pete's goes on to Slappy's (new career) or the
  Edit Skater menu; Slappy's opens the receipt for a part on it, else the Edit Skater menu (and
  `ClearPreview`); the Edit Skater menu leaves its style menu, else `Finish`.
- `Finish` (`0x8268b828`) opens `ID_CAC_EXIT_CAC_CONFIRMATION` (Yes/No);
  `LeavingCACConfirmationCallback` (`0x826899d0`) runs `UpdateStatsAndAchievements` and
  `Shutdown` (`0x82659d28`), which resets the new flags of unlockable categories 15 and 16
  (`ResetAllNewUnlocksFromCategory` `0x82a73330`). Nothing here saves SKATER.P.

### The receipt (`festate_receipt.cpp`, state 20, screen 122)

- `OnEnter` (`0x826784f0`) copies `mReceiptList` to `mNewReceiptList` (the kept lines) and opens
  screen 122; `OnLoaded` (`0x82669688`) sets `mState` and `SetScreenState`: mode 0 the menu picker
  (1), mode 1 the list (0).
- `ReceiveInput` (`0x82680e08`): B in mode 1 sounds `core_b_button` and `UpdateReceiptList`
  (the cart is kept); B in mode 0 on the list sounds `core_b_button` and returns to the picker.
- `OnMenuSelect` (`0x82680ef0`): on the list it toggles the line in `mNewReceiptList` and
  `SetScreenState(50)` (the script redraws the line and `UpdateCash`); on the picker it records
  `mLastOptionSelected` and: 0 Purchase (`cas_puchase_item`, `UpdateReceiptList`, then for each
  line `SubtractCash(cost, 0)`, stat 9, `MakePurchaseSFXDirty`, `AddPartToInventory` of each
  `GetWearStyleAlternates(part, false)`; both lists empty), 1 Edit Cart (the list), 2 Back to
  Store (`UpdateReceiptList`).
- `UpdateReceiptList` (`0x826785a0`): the lines no longer kept, with their wear style alternates,
  go through `CASSimManager::RemoveParts(.., replace)` (each slot left empty takes the undo
  point's part); the kept lines become `mReceiptList`.
- `OnCompleteImpl` (`0x82659fa8`) pops the state; `FrontEndState_CaC::OnGainFocus` (`0x8268ad88`)
  then `SetScreenState(52)` and, after Purchase, pushes the Edit Skater menu, after Back to Store
  re-enters the top menu.
- `MakePurchaseSFXDirty` is speech, not saving: `FrontEndState_StartHub::OnOutroDone`
  (`0x8265e63c`) plays the camera man's mission speech 841 on the way back into the game.

### cas/receipt (asset 74, template entry `0x82fdbaf8` `Receipt`, placement 0)

InitAction (character 47, `0x1b68`) defines `receipt`; root action `0x1b40` constructs it with the
title, the three labels and `mCash`; `OnLoaded` builds the menu picker (`Menu_Picker_Component`,
no wrap, the three `mReceiptOptionText` labels `ID_RECEIPT_PURCHASE`, `ID_RECEIPT_EDIT_CART`,
`ID_RECEIPT_BACK_TO_STORE`) and the eight-line list (`mItem0..7`, `MAX_NUM_RECEIPT_ITEMS`, with the
`VerticalSliderComponent` at length 245). The list's Up and Down move the line (and the window, only
when the line leaves it) with `core_nav_up/down`; X sounds `core_a_button` and calls
`OnMenuSelect`. `EnableReceiptList` swaps the button help (A Select / B Back + X
`ID_CAC_RECEIPT_ITEMTOGGLE`) and the highlighted line's text colour (0x221C0A on, 0xD8C279 off).
The `Receipt_Get*` game calls (`GameCmdTranslator` `0x82620b40..0x82620d78`) return strings:
`GetItemPrice`, `GetSubtotal` and `GetBalance` are `FormatCurrency`; a line is
`FormatString("ID_CAC_RECEIPT_LINEITEM", packed ID_CAC_COMPONENT_TYPE_<slot>, packed part
description)`, whose `{n:s}` tokens are `Localizer::Initialize`'s (`0x82980058`) type 0 (`d` 1,
`f` 2, `h` 3, `~` 4, `$` 5, `s` 0: the string copied).

### Cash

`GetCash` is the `Cash` point pool; a fresh career starts it from the pool's layout word 5, which
reads 300 in the shipped data (seen live: the new career's wallet is $300).

## The port

| Where | What |
| --- | --- |
| `session/edit_skater_shop.rs` | statuses, the receipt list, `select`, `open_receipt`, `update_receipt_list`, `purchase` |
| `ui/edit_skater.rs` | `CacState`, the new career's Back Alley Pete's start, B and Start by state, `OpenReceipt`, `ReceiptView`, `RECEIPT_OPTIONS`; the part thumbnail's select sound moved to the frontend |
| `session/edit_skater_frontend.rs` | the shop's input (popup, receipt picker and list), row statuses, costs and new flags, the wallet |
| `session/edit_skater_session.rs`, `selected_session_edit_skater.rs` | the shop's cash and unlocks: the career's, or for the new career's skater the starting cash and default unlocks; the new career's skater starts from a fresh record |
| `session/selected_session_career.rs` | `close_editor_shop`: the new flags of categories 15 and 16 reset, the purchases subtracted (credit 0) and the career saved |
| `challenge/unlockables.rs`, `signpost_progress.rs` | `reset_all_new_unlocks_from_category`, `starting_cash` |
| PC `marker_ui/receipt_movie.rs` | cas/receipt translated; `edit_skater_movie.rs` wallet, price and locked text, the one-option popup; `localizer.rs` `format_string_labels` and the `s` type |

The native writes nothing at the editor's close; the next career save carries the record and the
cash. The port already saved the record as the editor closes, so a purchase saves the career at
the same moment, keeping the two consistent.

`Finish` opens the exit confirmation (`ID_CAC_EXIT_CAC_CONFIRMATION`, Yes/No, default Yes, not
cancellable, like every popup the editor opens: `sPopupInfo` `0x82d67a80` defaults
`mIsCancellable` to 0); Yes leaves through `Shutdown`, No re-enters the Edit Skater menu
(`LeavingCACConfirmationCallback` `0x826899d0`). Not ported: stat 9 and the purchase speech.

## Verified live (2026-10-09, scratch saves)

`career_run.sh` with `SKATE2_DIAGNOSTIC_SHOP=1` (T-Shirts):

- New career: the editor opens at Back Alley Pete's with $300; T-Shirts shows 87 locked, about
  104 for sale ($10-$30) and the remove row owned. The $10 stampable tee joins the cart (wallet
  $290); the receipt reads Cash $300, Subtotal $10, Balance $290; Purchase; the career starts
  with `shop bought [10], cash 290` and autosaves at 290.
- `SKATE2_DIAGNOSTIC_SHOP_REOPEN=4800`: the editor reopened from the pause hub reads the tee back
  from SKATER.P as owned (check mark), buys the $30 long-sleeve tee: cash 260, saved.
- `SKATE2_DIAGNOSTIC_SHOP_EDIT_CART=1`: Edit Cart, X off ($0 / $300), X on ($10 / $290), B to the
  picker, Purchase.
- `SKATE2_DIAGNOSTIC_SHOP_SPREE=1`: parts join the cart until the balance is $0, the next one
  raises `ID_CAC_NOT_ENOUGH_CASH` and A opens the cart (mode 1).

## Mission cash

The native's cash sources (callers of `cSignPostManager::AddCash` `0x825dd7a0` and
`IncrementPointsPoolPoints` `0x825d78d0`):

- `IssueChallengeAwards` (`0x825ddfc8`) at a challenge's completion: every `progression_awards`
  row whose `ChallengeID` (word 1) is the challenge's pays its amount (word 2) into its point pool
  (word 0) with credit 0; a `Cash` row also counts in `mCashEarnedResults[2]`. The port's
  `signpost_progress::issue_challenge_awards` is the same loop. 160 of the 228 award rows pay cash
  (about $292,000 over the career); `m_1a_01`..`m_1a_04` pay none, `m_1a_05_photo_rail` and
  `m_1a_06_photo_puzzle` $50, `m_2a_01` $150, `m_2a_02` $200, Rob Dyrdek's film missions
  $1000-$5000.
- `AwardPrizeCash` (`0x825de2d0`, Lua `ChallengeProgression.UpdateCashAward`): contest purse by
  rank; ported. `ProcessEntryFee` (`0x825de3f0`): ported. `DeductChallengeLosses`: betting only.
- The Lua natives `AddCash`, `AddCashWithCreditDefinition` and `AwardCalloutCash` exist but no
  shipped script calls them (their names appear in no archive; `UpdateCashAward` is the only cash
  native in the scripts).
- `IncrementPointsPoolPoints` on pool 43 (`Cash`) adds, for a gain with a board sponsor
  (`GetBoardSponsorKey` `0x825d1198` != -1), the `Sponsor Bonus` message setting's percent of it
  (credit 4). Ported 2026-10-09 in `signpost_progress::increment_points`.

Live (`career_run.sh` with the photo-rail diagnostics): `m_1a_01` complete, autosave $300;
`m_1a_03` complete, $300; `m_1a_05_photo_rail` complete, $350.

## Saved location

The native saves the skater's place only with a career save (`SaveProgressionOrientation`, once a
challenge is complete), not while moving. Continue Career on that save printed `SignPost loaded
(3 completed, 46 available)` and `continue at [1670.42, 102.41, -521.47] (saved orientation
true)`: the photo rail's spot, where the last autosave was made.
