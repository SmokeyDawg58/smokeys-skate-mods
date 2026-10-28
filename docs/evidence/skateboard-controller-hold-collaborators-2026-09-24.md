# HoldSkateboard collaborators at the raw-Y OffBoard500 entry

The mounted raw-Y replay reaches `StartController` at frame 59 as described in
`skateboard-controller-start-offboard-2026-09-24.md`. This packet covers only
the newly staged board animation and hand-drive leaves. It does not claim the
offboard transition executes them yet.

## Inventory and source

- `Skateboard::SetStandard` `0x82b355a0` and Body collision-volume enabling
  `0x82b241a8` already have actual-board Rust owners (`try_set_board_standard`
  and the retained volume flags). Do not duplicate them for Hold.
- `SkateboardBody::HookEnableAngularDriveSoft` `0x82b24ad8` already has a
  selected retained-hook writer in `dynamic_assembly_skater_air_drive.rs`.
  Its current `EnableAngularDriveOnly` caller also clears the linear drive,
  so Hold must invoke the angular leaf without that caller's trailing write.
- `Skateboard::EnableBoardAnimation` `0x82b2a140..0x82b2a184` captures Body,
  stores animated=1, writes the retained Body linear drive as raw HARD=2,
  spring `0x426fffff`, damping +0, strength `0x4560fffe`, then calls angular
  soft with strength 1.0. `EnableBoardAnimationSoft` `0x82b2a188..0x82b2a1c8`
  stores animated=1, then calls linear soft and angular soft with the same
  incoming strength; disassembly shows `fmr f9,f1` and no intervening overwrite
  of `f1`. The linear soft leaf is `0x82b24bb8` and uses its own linked
  `kSkateboardLinSoftDriveDisp/Str` cvars and reciprocal-square-root sequence.
- `SkateboardController::EnableHandDrive` `0x82dcc618..0x82dcc734`
  rereads `kSKC_UseHandDrives` twice. The first false exits. After the sentinel
  assertion, a second false skips dynamics yet still calls
  `UpdateDriveFrames` `0x82dcc71c`. A true second read samples HandDriveStr
  then HandDriveDisp for angular spring/type/damping/strength stores, then
  freshly samples both for linear stores, then calls `UpdateDriveFrames`.
  Hard type is 2, spring scale `0x426fffff`, strength scale `0x4560fffe`.
  The selected dynamics Arcs were already retained by controller construction.
- `kSKC_UseSoftHook` constructor `0x82f986a0` gives default true under
  `Physics_Skateboard_Controller`. `kSKC_HandDriveDisp` constructor `0x82f985f0`
  and `kSKC_HandDriveStr` `0x82f98640` each give default 5.0 and range 0..10.
  These are registration facts, not permission to hardcode live tuning.

## Rust and validation status

`skateboard_entry_animation_enable.rs` stages both board animation call orders
on the existing `SkateboardEntryState`. `skateboard_controller_hand_drive.rs`
stages retained selected-drive writes and the late frame callback on the
existing controller. Four focused unit tests exercise call order, numeric
payloads, the first/second cvar gates, and failure prefixes. `rustfmt` ran;
Cargo validation and mounted replay belong to root integration.

`HoldSkateboard` still needs the actual `UpdateDriveFrames` animated-part
transforms (`0x82dcc408`), a source-backed LetGo path for the other Start
branch, and the live same-owner callback wiring. These callbacks must not be
replaced by successful no-ops. Confidence: **high** for the staged call orders
and stores, **pending** for live OffBoard500 entry and Xenon numeric-bit parity.

## Follow-on linked cvars and linear soft hook

IDA constructors `0x82f7bb40`/`0x82f7bba0` register
`kSkateboardLinSoftDriveDisp` default 5000.0 and Str default 0.5, in
`Physics_Skateboard_Drives` at initializer table `0x82fd4488/8c` after the
existing angular soft cvars. Controller constructors
`0x82f985f0`/`0x82f98640`/`0x82f986a0` register HandDriveDisp 5.0,
HandDriveStr 5.0, and UseSoftHook true in table `0x82fd5dd0/d4/d8` after
UseHandDrives. The five definitions were added once to the existing linked
`InputCvarReferences` runtime, with reads in `input_cvar_skateboard_hold.rs`.
Startup INI changes are intentionally not clamped to GUI ranges, and a new
manager link changes subsequent reads.

`HookEnableLinearDriveSoft` `0x82b24bb8..0x82b24c98` reads the Disp target,
rounds `Disp * strength`, runs the numeric reciprocal-square-root estimate,
then reads Str and writes the retained Body hook linear type SOFT=1, strength
`Str * 0x4560fffe`, spring, and source refinement damping. The implementation
uses the existing angular leaf's damping graph, shared as one function; the
existing `EnableAngularDriveOnly` still performs its trailing linear-hard-zero
stores. A separate angular-soft method omits that caller-specific tail for
board animation. The linear focused test checks linked override values,
numeric payload and preservation of the independent angular drive.

This follow-on is staged only. The live `StartController` still needs the
same-board callback wiring, frame transform and LetGo completion. Cargo and
mounted replay have not run for this batch yet.

Independent IDA review approved the staged valid-hand branches and noted that
the first disabled hand-drive cvar branch lacked a test. A focused test now
asserts that this first false read returns before hand and frame services;
the next Cargo build will validate it. The invalid-hand refusal is host policy.
