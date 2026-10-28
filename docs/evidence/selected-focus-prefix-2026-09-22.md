# Selected PlayerUI focus prefix

Source confidence high. PlayerUICtor828ae0f0 stores actual player index and
mHasFocus=false. Update828af9f8 through828afa74 preserves changed-focus store
before emission; only gain emits kind2 to playerIndex+10 with u32 wrapping.
Fresh SetData<PlayerInputGainFocusData>826c2ff0 attaches/reset stream255 but
writes no payload bytes (mPos remains0). The caller's zero-initialized data
object is not a four-byte wire payload. Existing kind2 consumer is reused.

Without simulation admission, false focus clears the retained field; true
focus leaves it unchanged. Native focus is gameplay/FE admission, not simply
OS window focus. Full caller audit: artifacts/player-pause-focus-source-2026-09-22.md.

SelectedPlayerStartupOwners retains one Arc<Mutex<SelectedPlayerFocus>> from
its actual loader/player index. QueuedInputCorpus clones the same owner and
executes the prefix before ordinary kind1 input in the same queue. Poll/cache
and consumption precede it; sync command generation follows. The fixture
supplies explicit can-simulate/focus=true; optional session-marker/network,
spawner and enclosing GameModule/SimController are not claimed implemented.

Tests cover first gain, unchanged focus, loss and regain, wrapping receiver,
no-simulation preservation/clearing, and focus store before queue failure.
Independent source/code review approved drafts and registered shared ownership.
Action47-focus initial gate found missing Mutex import plus unrelated match/
field-order lints; corrected. Action47-focus-2 validation is in progress.

Action47-focus-3: fmt/Clippy31.85s pass,4710 units+6 docs pass. Original20/22
in7.14s: both queued variants reach old delivered1 assertion with actual2.
Corrected fixture only: initial gain kind2 + ordinary kind1 =2, subsequent
unchanged-focus frames =1. Existing consume advances RNG once per command;
expected index is frame+2. Focus behavior is not suppressed to satisfy fixture.
Focused original rerun pending. Earlier Juice test INIs used textual true;
existing native ConvertAToI treats that as0. Tests now use numeric1 unchanged
production converter; both pass workspace.
