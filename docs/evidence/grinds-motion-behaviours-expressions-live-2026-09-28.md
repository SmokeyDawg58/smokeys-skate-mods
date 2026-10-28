# Grinds: grinding Motion behaviours, grind graph expressions, live policy flip (2026-09-28)

Continues `grinds-air-adjust-footplant-motion-2026-09-28.md` with the same
scope (2026-09-27: finish the parked grinds 1:1 with IDA, real game values).

Status: source written and statically reviewed against IDA (debug
`sk82_na_m.xex`). rustfmt PASS on the new files and every own hunk; line
endings of every touched file unchanged. NOT compiled, NOT tested. The live
facade now runs the grind code (`RunRecoveredPrefix` / `RequireNative`), so this
build is the first live exercise of everything from batches 33..38.

## Motion behaviours (`motion_grinding.rs`, `actor_motion_grinding.rs`)

- `CreateGrindAttributes` (Create 82bb3da8 null, Begin/End 825187c0 BLR, Update
  82ca1710): IPhysical, ISkaterAnim and ISkaterMotionGraph asserted. Only while
  PhysOut Filtered+0x50 `mIsGrinding`: AddMgAttribute(Filtered+0x8
  `mSkatingGrindName`, 1.0); reversed = `GetIsMirrored` XOR SkateboardMotion+0x111;
  y = cross(SkateboardMotion+0x50 velocity, SkateboardReckoning+0x80 nose).y
  (vmulfp128 then fused vnmsubfp); `ble` sends an unordered y to the not-positive
  arm; y > 0 gives Backwards iff not reversed, otherwise iff reversed; then
  AddMgAttribute(`GrindFacingBackwards` / `GrindFacingForwards`, 1.0).
- `ControlGrindCrouch` (Create 82712148: 0x0c, float +8 = 0; Begin 82cb1d10:
  +8 = Animation+0x48 `mCentreOfMassToDeck`; Update 82cc6e30; End BLR): three
  `anim_motion` 0x3cce4d5548190472 scalars (GlobalAttributeCollection+0x154,
  ctor 828b1590 at 828b3028): speed 0x71cd7678af2a7a5a = 0.6, maximum
  0xa7c310ed53f89af6 = 0.95, minimum 0x804b5ecec1491fa2 = 0.6. Input
  `clamp(1 - HasIntent(Crouch))`, physical `clamp(1 - Filtered+0x38 mGrindCrouch)`
  (fsel pairs), `kUsePhysInducedGrindCrouch` takes `fsel(input - physical)`;
  +8 moves toward the target by `dt * speed`; AddSettableAtt(driveDistToCom, +8,
  false, -1). `kShowGrindCrouchText` only asserts the DebugOverlay singleton: its
  four prints call 825187c0 (BLR).
- `GrindControlFade` (Create 82cbce08: 0x24, +8 = 1, +c = +10 = 0, +1c = -HALF_PI,
  +20 = HALF_PI, +14/+18 unwritten; Begin 82cbcef0; Update 82cc67f8; End BLR).
  Begin: +c = 0, +8 = 1 before any lookup; `GetBaseAnim` (IAnimatable +0x2c,
  Animatable+0x4dbc) captured first; AddSettableAtt(distBoardToCog, COM-to-deck);
  twist written normalized 0 then 1 with `SkaterAnim::SetAttributes` (+0x4c) and a
  base `GetAttribute` (vtable +0x34, flags 0xf, asserted found) after each, giving
  the twist range +1c/+20; Skeleton+0x1f8 `mShoulderTwist` (mirrored folds to
  `PI - t` / `-PI - t`), clamped, written, +10 = +14 = twist, +18 = 0. Update skips
  once while +8 is set; else +c += dt, adjusted = clamp(rate*intent + adjusted),
  blended = (1 - blend)*twist + blend*adjusted, delta limited to +-change around
  the previous delta and to +-limit, twist = clamp(twist + delta), written, then
  +10/+18 and +8 = 0. `anim_motion` 0x861925f635314abd (GlobalAttributeCollection
  +0x138, bound at 828b2e30): rate 0x30ce1d85ce6b2b42 = 0.3, blend
  0x68a73621151265c0 = 0.15, change 0x3643621ccb9b2578 = 0.05, limit
  0x81248d4d1a96309a = 0.5. `kShowGrindControlFade` needs the overlay only.
- Base `GetAttribute` routing (new `motion_base_attribute`): Transition
  82d6db68 follows mToAnim; Sequence 82d6de68 follows mToAnim only while
  PlayingNewAnim (+0x14, vtable +0x40 folded to 826b65c0), else mFromAnim. The
  existing JumpInto always followed To for its query; it now uses the same routing,
  keeping To for SetTime 82d6db38.
- Cvars 824..826 (`Animation_Skater`): kUsePhysInducedGrindCrouch true,
  kShowGrindCrouchText false, kShowGrindControlFade false.

## Grind graph expressions and conditions

- Motion Pre 8290aa50 / Sustain 8291e218: 54 Pre and 55 Sustain cases reachable
  from the Grinding18 subtree were unported (grind-type selectors 44..85, grab
  Into/Cycle/Out 285..291 and 382..386). Written from the native decision paths
  of `tools/ida_graph_switch_paths.py`; `compare_graph_expressions.py motion`:
  733 matched, 0 mismatches.
- Action Pre 828ff990 / Sustain 82906048: 11 Pre and 10 Sustain cases around
  Grinding16 / TrickOutOfTipSlashGrind22 (DoubleGrab 41, Sideways 89/90,
  TrickFrom90* 94..97, At90 103, Straight 105/106) plus StartBrake14's Pre 0;
  `compare_graph_expressions.py action`: 200 matched, 0 mismatches.
- New Motion conditions: `CurrentGrabType` 82c950d0 (UCOM ISkaterAnim, false when
  absent, `IsCurrentGrabType` 82cb77a0: +0xee8 == grab) and
  `IsGrindBluntingBackslash` 82ca7d30 (PhysOut Grinds+0x88 `mGrindType` == 4,
  GRIND_TYPE_BACKSLASH).
- Transition hook 8 (channelblend 1.5 s, useChannelFromWeights,
  blendWithCurrentFrame) is used only by the 32 grind exits to InAir21.
  TransitionAnim 82d7aa68 passes blendWithCurrentFrame only to TransitionBlend::Init
  82d6b810 (bit 29); of the TransitionBlend virtuals only AddTime 82d6df98 reads it
  (Eval, EvalBlendPose, GetWeight, GetBlend and TransitionComplete do not), and the
  port already models both. The 32 transitions join the audited override list.

## Live policy flip

`selected_session_public_facade.rs`: `pre_grind: RunRecoveredPrefix`,
`post_grind: RequireNative`. `GrindManagerFillPolicy::ExcludeForFirstSkatingTest`
is now selected only by test corpora (dead-code expectation outside tests).
Prerequisites checked live: world line queries and the (empty) DMO pool from
`selected_session_terrain.rs`, rail splines registered by the normal-arena grind
adapter, GrindManager and children constructed with the physical assembly.

## Remaining and risks

- First live run of the whole grind stack; expect typed errors to point at the
  next gap rather than silent behaviour.
- Camera GrindDetector candidate search stays pending but unreachable by default:
  `mIsGrindDetectorActive` is the profile option `mGrindTransparency` (tSettings
  +0x2b), which `tSettings::Default` 825644d8 writes false.
- `GetAttribute` on a BlendSpace node is still pending in the port; if a grind
  twist tree contains one, GrindControlFade Begin reports it.
- DMO grind splines (dynamic provider) arrive with the DMO task.
