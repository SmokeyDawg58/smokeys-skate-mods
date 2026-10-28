# Air relaunch timer: 32-bit CR0, the KnownAir recovery the port never tried (2026-09-26)

**Status:** rustfmt parse PASS; both touched files are rustfmt-neutral. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

## Problem

- A manual into a 360 inward (worst at low speed) sometimes stays in plain Air (200) instead of KnownAir (201): slow spin, and the camera breaks.
- Earlier traces showed the mechanism:
  - one frame after the pop, the toe touches the ground under `mAnimationDangerZone`, so KnownAir drops to Air;
  - Air's trajectory relaunch is blocked by the same contact (flags8c4 0x8000);
  - the port never tried the relaunch again, so the jump stays in Air.
- A/B check: the same xex in Xenia does not show the bug. One theory was that native also drops briefly, but recovers.

## Native gate (`PhysState_PhysicsAir::Update` 82d9f4b0)

```
82d9f4d0 lwz   r11,0xB0(r31)     ; mTrajectoryTimer (int), Air.Enter stores 0
82d9f4d4 addic. r11,r11,-1
82d9f4d8 stw   r11,0xB0(r31)
82d9f4dc bgt   skip              ; CR0 from the record form
82d9f4e4 flags8c4 & 0x8000 -> skip (skeleton contact)
82d9f4f4 Processed+8e4 mPhysicsState == 0 -> skip
82d9f504 TrajectorySelector+2392 mJustStartedQuery -> skip
82d9f514 LaunchTrajectorySelector; 82d9f51c timer = 7
```

The source is `if (--mTrajectoryTimer <= 0)`, and Hex-Rays renders it that way as well.

**The port's old reading (wrong).** It treated CR0 as a 64-bit compare of the zero-extended `lwz` value. After one blocked attempt the stored -1 reloads as 0x00000000FFFFFFFF, which is positive, so the gate never ran again.

**Evidence for the 32-bit compare:**
- A survey of the whole `.text` found 6389 record-form add/sub instructions:
  - 81 of them feed a `lwz`-loaded 32-bit int straight into a record form and a signed branch;
  - 0 sign-extend (`extsw`) first.
- The MSVC Xenon compiler therefore relies on CR0 comparing the low word. Titles run in 32-bit mode, where record forms compare the low 32 bits; otherwise all that code would misbehave on negative ints.
- This matches the decompiler, and the Xenia A/B where the bug is absent.

## Fix

Both implementations of the gate now compute `timer = timer.wrapping_sub(1)` and test `timer <= 0` as i32:
- the live Air update, `physical_player_air_update_prefix.rs`;
- the reference model, `physics.rs::update_physics_air`.

A blocked relaunch therefore retries on every following frame. Once the transient DangerZone contact clears, it relaunches (timer = 7). The selector reports ready (flags8c4 0x400), and `CalcSuggestedState`'s Air case (the 0x400 check at c4fc) moves Air back to KnownAir, which the port already admits (`(Air, 201)`).

**Tests.**
- `physics_air_trajectory_gate_compares_low_word_and_short_circuits`: renamed. Its timer -1 case is now a blocked retry that still reads skeleton contact.
- New: `physics_air_blocked_relaunch_retries_every_frame_until_contact_clears`, with contact on frames 1-2 and a launch on frame 3.

**Doc corrections.** `physics-state-machine.md` and `known-air-manual-audit-2026-09-26.md` stated the 64-bit reading; both now carry a dated correction.

## Expected effect

- After a manual pop, a DangerZone toe graze still drops KnownAir to Air for about 2-3 frames, as in native.
- Air then recovers to KnownAir: normal spin speed and camera, and the jump is no longer stuck.
- Pops that never graze (higher speed) are unchanged.
