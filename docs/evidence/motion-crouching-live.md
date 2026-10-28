# Crouching723 lifecycle and live callback draft

Fresh IDA `sk82_na_m.xex` inspection 2026-09-19. Draft only; no controller,
constructor bundle, cvar registry or crate mount changed by this author.
No Cargo commands run. `rustfmt --edition 2024` parsed the five Rust files.
Seven kernel tests are authored, not executed by this author.

## Mount packet

* `actor_motion_crouching.rs`: child of
  `actor_construction::motion_runtime_session::crouching`.
  Owns nested `motion_crouching.rs` and its `motion_crouching_tests.rs` child.
* `actor_motion_crouching_attributes.rs`: child
  `input_cvar_runtime::crouching`. Retain one `CrouchingAttributes` in the
  actual shared ActionRuntimeServices global bundle, bound to its actual Vault.
* `physical_player_phys_out_crouching.rs`: child
  `physical_player_phys_out::construct`; adds const Motion+bc pump-potential
  getter using the same component presence guard as existing speed getters.
* Preserve generated `NamedMotionBehaviour::Crouching { crouch }` payload.
  This name feeds the actual mounted IAnimatable setter; do not substitute an
  invented name or a physics output write.
* Extend controller instance enum with this kernel's `CrouchingInstance`.
  Native allocation size0x38/alignment0x10/name `unknown`; call
  `motion_runtime_crouching_create` with actual allocation success. On failure
  no cvar is sampled. Begin and Update use `motion_runtime_crouching` with the
  retained instance, live cvars/globals, original name and actual controller dt.
  The bool `begin` selects only the source lifecycle method, not a state bypass.
* End is source BLR and drops no extra native-owned collaborators. Keep native
  controller instance disposal and repeated Begin/Update ordering.

Vtable82302838 slots: +2c Create82cb3e88, +30 Begin82cb4240,
+34 Update82cca698, +38 End825187c0 (`4e800020`). Begin tail-calls
Initialize82cb3f80. Create initializes all meaningful data fields and samples
233 only after successful allocation. Initialize zeroes +8,+c,+10,+14,+18,
+1c,+20,+24,+28,+2c,+30 BEFORE it samples233 at3fe0..3fe8; earlier stores
survive a late failure. It then resolves IPhysical and IMotionGraph, reads
Animation+48, Motion+a4, crouching cached graph8+190, scalar74ea95392d415a5d,
and stores `1 - clamp((animation-baseline)/(curve-baseline),0,1)` to +10.
All clamps preserve source fsel unordered and signed-zero operand choices.

## Retained global wrappers and source timing

Both use generated `anim_motion` class90c3decf0105631e and cached layout5c0:

| Shared global | Collection | Alloc / generated ctor / store |
| --- | --- | --- |
| anim_motion_crouching +12c | 2d5ba8be3ccba8fb | 828b2d34 / 2d58 / 2d64 |
| anim_motion_auto_pump +15c | 5e223c914f2f5db3 | 828b3094 / 30b8 / 30c4 |

Runtime generic scalars call `try_get_data` afresh with index0 and default
missing data to zero4. Captures retain actual AttributeRuntimeData ranges;
reading a captured scalar happens separately. Cached graphs bypass generic
inheritance: crouching graph8 bases190,390,450, x+10/y+30; auto-pump graph4
base590, x+10/y+20. Missing cached layout uses source zero5c0 fallback.

Update interface order is IPhysical, IAnimatable, IMotionGraph,
ISkaterMotionGraph. Adapter checks actual occurrences/entity and physical Vault.
The final Skeleton component is captured before the deck-compression scalar:
the adapter retains its actual physical owner and validates the component,
then reads deck_angle_compression after the scalar. It relies on the existing
serialized Actor/physics tick's stable constructed component ownership; no
copied deck-compression value is sampled early and no synthetic physics owner
is supplied. Root should preserve that serialization when mounting.

## Update sequence and source branch proof

1. Crouching graph450(1-crouch) negated; Skeleton+204 absorption divided by
   live d0c31cb8445c940a and fsel-clamped[-1,1]. Motion+54 velocityY is added
   and STORED ataa18 before lookup4d4bac56989f5f2b; multiply/storeaa78.
   Ordered negative velocity flips the absorption term. Skeleton+208 normal
   compression uses6bde80babf0eb9d5; combine by original negative/positive
   branches and fsel, then combine bias. Minimum crouch is Skeleton+210.
2. Capture previous angle/magnitude, read/store AutoPumpAngle then AutoPumpMag.
   Thresholds132b67ff08fd39c0 / dba1abca593a9f9b. New pulse requires ordered
   abs(old/new angle)<angle limit, old magnitude<limit, new magnitude>limit.
   Raw ac90/acac `40980010` reject unordered; ac9c LT/acb8 GT are ordered.
3. Motion+bc pump potential; read0908ab96fbbeee9a and fmadds filter/store+30.
   Read2b35f6aec83da8f4: rawadd0 `40980008` is !LT, accepting unordered.
   New pulse stores mode_time0/mode1 before fresh durationddcb6014e96d27e5.
   Mode1 ordered timeout becomes2. Threshold1056991e714f6fa3, mode1/2,
   ordered crouch>threshold and sufficient potential store mode3 before actual
   Motion `NewAutoPump=1` publication. Finish9b4f552598bb121a, mode2/3 and
   rawafa4 !GT(crouch,min+finish) return mode0, including unordered.
4. Mode0 auto_crouch0. Mode1 reads actual speed, auto graph590, fresh
   crouch-speed383e92759fc0115c, fresh durationddcb6014e96d27e5; min(target,
   old+speed), then dt+time/store and cap duration+1/store. Mode2 subtracts
   f05644bfb1ef25c6, mode3 subtracts8df4151accbad2ed; source fsel max with0.
5. Crouch intent, actual **IsPowerSliding** twice around HardTurnCrouch and
   Manual intent reads. Vtable82069688+6c=825fe0e0; bit04000000. This is NOT
   IsManualing. Manual present<0 during power slide imposes .4 floor.
   Then live232 selects auto crouch or0; no earlier autopump work is skipped.
6. Blend454689d21d4f7436 clamped0..1, source fmadds and ordered .005 deadzone,
   store+c. Compression scale0f76bbc45524d46b. Active player/auto/continuing
   crouch sets byte18 BEFORE late3176b692cfd4664e; after read reset+34 and
   choose delta floor. Inactive clears18 and adds literal bits3c888889 to+34,
   NOT controller dt. Live233 ordered time comparison publishes
   `PlayerControlledPump=1` (b3e0 raw40980024 skips !LT incl unordered).
7. Capture29b4474528c0a68c, graph390(1-crouch), then READ captured scalar;
   negate product. Capture same key anew, graph390(crouch), then READ again.
   Clamp desired delta. Read51593962368ffdd4 twice separately around current
   delta to get lower/upper acceleration bounds. Store delta, add/store crouch,
   clamp/store crouch[min,1].
8. Capture actual Skeleton, re-read scalar d2099fe03d6fafbb, then read
   Skeleton+214 deck angle compression; ordered product>crouch raises crouch
   without a subsequent max1 clamp. Scale dc5b93554f2faebf; fnmsubs b740 means
   `-(crouch.mul_add(scale,-1))`; clamp0..1. Actual Motion+a4 speed,
   graph190(speed), fresh74ea95392d415a5d; final fmadds b824 produces
   `(curve-baseline)*value+baseline`. Publish loaded crouch FastString30 through
   actual mounted IAnimatable +48, false,-1. No animation channels are fabricated.

String initializers82f82468 / 82f82570 read literals822f93dc `NewAutoPump`
and822f946c `PlayerControlledPump` respectively.

## Cvar registration

| Index | Label | Reference | Initializer / table | Section / title | Default / range |
| --- | --- | --- | --- | --- | --- |
| 232 | kUseAutoPump | 830a4f2c | 82f86950 / 82fd4f48 | Controller / Enable AutoPump | bool true |
| 233 | kTimeToConsiderPumpingMode | 830a36a8 | 82f869a0 / 82fd4f4c | Physics_Pumping / Player Pumping Extra Time | float .5 / 0..1 |

Native initializer table neighbors:4f38 kMOAngleMaxDelta;4f3c kMOSpeedMaxDelta;
4f40 kMORawSpeedMax;4f44 kMOCrouchMaxDelta;4f48/4f4c above;4f50 kSmoothTest;
4f54 kShowManualOutTimer;4f58 kLandingVelocityTwoFootThreshold;
4f5c kLandingImpactMaxBlendDelta. Previously mounted next item is
kAnimUpGesture82fd5168. Register source order, not append-only index order.

## Validation boundary

Seven tests exercise actual kernel: failed allocation avoiding233; Begin late
cvar reset persistence; Begin graph/baseline ordering; late velocity scalar
failure preserving first store; graph failure preventing premature captured
scalar read; active player flag surviving late scalar failure; unordered
pump potential publishing NewAutoPump even when232 disables its input blend.
No registered production path, corpus or sustained tick has been run for this
draft. Root integration and independent source review remain required.

Integration correction: Initialize82cb3f80 raw4138 loads outer PhysOut+38,
4140 loads float+48: PDB PhysOut_Animation::mCentreOfMassToDeck.
The initial adapter incorrectly named this anim_crouch; corrected to actual
centre_of_mass_to_deck with kernel name CentreOfMassToDeck. Raw4154/4160
separately loads Motion+4/+a4 deck speed. Independent recheck confirmed both.
