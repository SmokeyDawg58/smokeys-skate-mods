# Retained selected PauseMan

Unregistered owner and shared integration patch; root-owned Cargo acceptance
pending. No OS focus or FE policy is supplied. Independent source/code reviewer
approved audited state domain0..25 and actual shared input adapter.

Fresh constructor82738d68 initializes native54entry buffer, flags0, maskFFFFFFFF,
then Push(INIT=0). kFlagsTable820d8548 entry0 is0, hence activeSIM is earned.
Push827381c8 asserts oldlen<53, appends, publishes ONLY last-state table flags.
Pull82733df8 removes ALL matches, preserving survivor order; new top orzero.
IsActive82603b30 locks and returns (mask & flags & suppliedFlag)==0. SIMflag1.
All26 literal tablewords were freshly read with IDA get_bytes. Unknown indices
are refused BEFORE lock/append as an explicit checked host admission boundary;
accepted-domain native ordering is preserved. No native invalid-memory behavior
or assertion continuation is claimed.

One Arc belongs to SelectedPlayerStartupOwners and is shared into queued input;
existing PlayerInputPause receives the same owner. Constructor INIT is retained,
not popped or replaced. Shared tests distinguish top-only from accumulated flags,
duplicate removal, empty stack, capacityfailure prefix and actual unfocused
sample proxy-read laziness after mutation by another Arc. No automatic window
pause producer or full PlayerUI/SimController admission completion claim.

Drafts: selected_pause_manager.rs and tests. Shared integration patch:
artifacts/selected-pause-owner-2026-09-22.patch. Registered code is unchanged until
current Cargo gate closes and root applies that reviewed patch.

Root integrated sharedpatch after action42-wheel-2 gate closed, using
--ignore-space-change for normalized line-ending context. Independently reviewed
same-Arc startup/input ownership and actual unfocused sample test. Pending next
strictgate; no native FE/pauseproducer or complete input admission claim.

Next focus-prefix audit (not implemented): fresh PlayerUI.Update828af9f8 matches
reviewer artifact. canSimulate=true compares and stores changed mHasFocus before
sending gain-only kind2 to playerIndex+10 atfa34..fa74. Fresh
Command.SetData<PlayerInputGainFocusData>826c2ff0 sets type2, resets and attaches
ByteStream255 capacity, performs NO writes, then publishes mPos0. Payload is
therefore empty despite caller initializing a zero data object. Native no-sim
unfocused clears mHasFocus; no-sim focused retains it. Existing queue.try_queue
and GameInputManager kind2/GainFocus consumer already implement transport and
bounce rearming. Planned retained focus prefix must reuse these, preserve store
before sendfailure, and stop before actual optional session-marker/network
teleport boundaries. It must not claim FE focus computation or spawner Tick.
