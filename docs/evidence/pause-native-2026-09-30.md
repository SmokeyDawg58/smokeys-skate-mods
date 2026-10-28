# StartHub native base menu

Source inspection only, 2026-09-30.
Confidence high for native tables/control flow; no runtime or pixel parity claim.

`PauseMenuCmdTranslator::sMethod_GetOptionText`82623580 obtains frontend state7
and calls StartHub::GetOptionText826444c8. GetNumOptions82623550 follows the same
owner and virtual interface. StartHub OnEnter8265e4d0 opens screen11; OnIntroDone
826536e0 picks a mode and sends SetScreenState0 except single-player challenge.
`eStartHubMode_SinglePlayer` is enum1; offline freeskate is0.

SinglePlayer's table8208e248 contains nine option IDs `[8,0,1,3,2,4,5,6,7]`.
Its GetOption82644370 indexes this table; virtual GetNumOptions resolves to the
merged constant-return function82a6b248 returning9. The text table82fdfc54 maps
IDs to Debug, Replay Editor, Map, Profile, Blog, Edit Skater, Trick Guide,
Game Settings and Quit localization keys. Exact keys live in pause_menu.rs.
The reference build explicitly includes Debug; neither native code nor the
authored starthub/menu_picker scripts filter it. Do not claim retail filtering.

Offline freeskate table8208e230 contains `[8,0,1,5,6,7]`, omitting Edit Skater.
The PC menu keeps Map/Edit Skater, so this
base deliberately selects the original SinglePlayer **presentation** list.
It does not construct career/challenge state or claim the current skating
session is career mode. All destination dispatch is intentionally inactive.
This includes Quit and Debug: they remain navigable menu rows without actions.

SinglePlayer IsItemEnabled82644600 enables every listed item except Replay,
which uses mbReplayAvailable. OnEnter8265e4d0 sets that flag only when the
presentation replay recorder holds at least150 frames. That recorder is absent
in the selected runtime, so Replay is disabled. GetOptionText8264c788 substitutes
new-content counts for Map/Blog only when career counts exist and are positive;
the base uses the ordinary localized strings without fabricated counts.

ReceiveInput8265e778 closes on keycode0(AptStart) or3(AptPrevious), marks
mbBackToGame, blocks loading, then requests PopState. StartHub OnLeave8264c4d8
closes the screen; OnOutroDone8265e628 clears the load block. State-manager
UpdateOnLeave82d66300 calls OnLeave then UnpauseGame immediately, not after
the movie's outro finishes. StateManagerFrontEnd::PauseGame82647910 pushes
FRONTEND3; UnpauseGame82647758 pulls FRONTEND. AddMenuPauseState827382a8
instead pushes MENUSYS2 and is not this path. Existing SelectedPauseManager
is the pause owner; the new menu controller does not duplicate it.

gFEStateInfo82fded80 uses20-byte records. StartHub record7 at82fdee0c is
`0100000000000002000000000000000301000000`: pause1, transition0, dimmer0,
HUD visibility2(HideExceptOnline), HUD-stack0, music3, ActionScript-input1.
UpdateOnEnter82d663f0 applies HUD visibility, pauses, resets rumble, then enters
the screen. Thus the offline game HUD hides and no extra dimmer is authored.

Authored `menu_picker` sets mEnableWrapAround=true, starts index-1, selects first
enabled on Update, and the starthub SetState0 explicitly selects0. Prev/Next
wrap and skip native-disabled rows. AptNext forwards selected row; this base
consumes selection without a destination. Movie agent retains visual timelines.

AptInputIntegration::mPlatformButtonMap82fdace8 has20-byte records. First field
is InputSK8 state ordinal:134 AptStart,136 AptNext,137 AptPrevious,146 AptUp,
147 AptDown. AptUp precedes AptDown in table order. The third field is the
APT input identifier, **not** an InputSK8 state:301 Start,302 Next,303 Previous,
14 Up,15 Down. Existing input-map/pad owner must produce events and repeats;
the controller receives pulses and never polls hardware. Focus suppression is
host integration policy and leaves an existing menu open.

Unimplemented scope: destination states, career counters, replay availability,
frontend audio backend, full FE input exclusivity/transition scheduler. Original
native state manager additionally validates allowed pause owner/FE mode/NIS
(CheckForPauseState826477b0); the selected freeroam integration is narrower.

Independent static review of the runtime agent's integration: one poll caches
SIM and PRES; both groups drain, but no SIM command is enqueued while open or
on either boundary. The frontend return precedes terrain, Flow, marker, physics,
animation, camera and completed-frame advancement. The app uses the retained
scene and pause-only UI update, clears host timer debt at both boundaries and
discards paused backlog. FRONTEND3 push/pull occurs only on a visibility edge;
opening accepts native INIT0/MANUALCAM6 owners. Trait implementations and
registered outcome consumers were inspected; no concrete issue found. This is
source review, not a compiler or runtime validation result. Native OnEnter's
SetSimRate(1/60,1 tick) message is outside the current base controller.
