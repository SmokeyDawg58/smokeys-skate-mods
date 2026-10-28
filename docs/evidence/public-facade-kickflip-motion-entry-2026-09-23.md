# Public facade scripted Kickflip: Motion entry checkpoint

The original-data `original_selected_live_facade_reaches_one_queued_visual_frame`
test now feeds the retained right-stick Kickflip points through the same
`SelectedLiveSession` facade used by the PC application. It begins with a
neutral visual frame and then advances the public queue per frame; the
authored stick path comes from the loaded right-pattern corpus and is encoded
through the existing Xbox pad radial fixture. This is a regression probe for
the full queue/Actor/Action/Motion/physics/presentation boundary, not a
completed playable trick.

At frame 7, original input recognizes Kickflip, Action reaches state 52, and
Motion selects the ground Kickflip entry path. The integration only mounted
previously missing, source-confirmed conditions and callbacks:

- Pre434 is the source-ordered lazy conditions 139, 140, 141, (89 or 13),
  then not 138. Conditions 140/141/138 read retained Motion flags,
  `PhysOut.Animation.should_disallow_tricks_due_to_stairs`, and
  `PhysOut.OffBoard.is_grabbing_object` respectively.
- Pre86/87/91/92 read the already parsed pro-skater/intent conditions.
- Pre446 (`0x8291ddcc`) lazily checks `IsGrinding` 144 through 155 in
  native order. The shared callback reads real `Filtered.is_grinding` before
  optionally reading `Filtered.skating_grind_name`; the Motion and Action
  wrappers validate their respective controller contexts.
- Pre294 (`0x82912f58`) lazily evaluates LastState161 then
  ManualOutTimer162. LastState already uses parent-relative substate lookup
  and ancestor traversal. The timer reads the retained Motion graph and
  compares strictly greater than zero. Pre90 checks LastState163.
- `IsDoingTrick` behaviour 1126 creates no instance, sets Motion flag
  `0x08000000` on Begin, does nothing on Update, and clears it on End.
  `ScoringTrick` behaviour 1128 creates no instance, has no Begin/End work,
  and on Update ORs the `mIsTrick` bit `0x01000000` while copying its
  five-word packed name into both six-word trick fields with a zero final
  word. The existing Motion-to-physics payload transfer consumes these
  fields.

IDA audits checked expression 294 at `0x82912f58..0x82913018`,
`LastState` `0x82c94a00`, timer callback `0x82c9deb8`, `IsDoingTrick`
Begin/End `0x82c9fae0`/`0x82c9fbd8`, `ScoringTrick` Update
`0x82cbec18`, and concrete setter `0x825fe338`. The source requires the
specific owner reads, condition order, null-instance behavior, and packed
string conversion above.

`SetTrickHeight` behaviour 2020 on leaf 1078 `FromAntic` is now integrated.
Its Begin resolves Animatable then Motion, uses present `TrickHeight` intent
directly, or falls back to `GestureSpeed`, tree `AnticStrength`, the linked
debug CVar and two booleans on the existing `anim_motion.jumping` Vault
collection. The clamp uses the native two `fsel` comparisons; it writes a
normalized `TrickHeight` settable attribute. The selected leaf update thunk
`0x828f32b0` admits its exact 29 active slots.

On held frame 8, the Trick ancestor checks Pre387
`Runout87 && (FilteredGround89 || FilteredGrind13)` and Pre293
`Runout87 && FilteredAir85`. Both reuse the existing real physical-output
conditions and native condition cache. TakeOff Pre88 is the existing
`WillExpire156` with `InTime=0.05`; its shared kernel was corrected so a
NaN time comparison returns false as the original `fcmpu`/`ble` does.
Kickflip In Sustain87 and Trick Sustain434 both go to the native constant
true case and do not reread Pre conditions.

The complete scripted Kickflip match, hold and release now passes through
the public facade, including a completed queued visual on every tick:
`artifacts/original-facade-scripted-kickflip-sustain434-2026-09-23.log`.
`cargo test --workspace` passes after adding the source-ordered
`Animation.kbDrawTrickHeightDebugInfo` label to its frozen CVar oracle:
`artifacts/workspace-public-facade-kickflip-final-2026-09-23.log`.
This proves the selected path completes, not that the rendered trick pose or
physical trajectories match the debug build tick by tick. A live controller
and visual runtime test remain necessary.
