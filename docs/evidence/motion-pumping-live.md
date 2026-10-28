# Pumping1925 retained Motion callback

Author draft, fresh IDA `sk82_na_m.xex` raw/decompile review 2026-09-19.
No registered file changed, no Cargo command or test execution. Four new Rust
files parsed with rustfmt; six production-kernel tests authored.

## Lifecycle / mounting

Vtable82300058 slots +2c Create82c9f388, +30 Begin825187c0 (rawBLR),
+34 Update82c9ecc0, +38 End82c9f2b0. Create allocates0x18/alignment0x10/name
`unknown`; failed allocation remains absent. Successful state has channel+10=-1,
influence+8=0, filtered+c=0, last-published+14=0. Begin does not reset these.
End does not need an instance; resolves IChannelAnimatable then ends PUMP0,
PUMP1,PUMP2,PUMP3,PUMP4 in that order through +2c EndChannelPrematurely.
Reuse actual `try_end_gesture_channel`, preserving missing channel success and
source channel flags28000000 when blend-out time equals zero.

Mount `actor_motion_pumping.rs` as Actor `motion_runtime_session::pumping`;
it owns `motion_pumping.rs` and nested `motion_pumping_tests.rs`.
Mount `actor_motion_pumping_attributes.rs` as
`input_cvar_runtime::motion_pumping`. Retain `PumpingAttributes` in actual shared
ActionRuntimeServices global bundle and pass live references to the callback.
Extend generated controller instance enum with PumpingInstance, using actual
allocator size0x18 before `after_allocation`. Begin is source BLR; Update uses
`motion_runtime_pumping(...,instance,false)`; End uses `(...,None,true)` and
must retain native reverse callback order. No selected-state or neutral bypass.

## Original Update order

Resolve IChannelAnimatable, IPhysical, ISkaterMotionGraph in that order.
ISkaterMotionGraph vtable82069688+7c is AllowPumping825fe018: byte+ca4 lowbit,
host flags mask01000000. Actual constructor flags03000000 include this bit;
therefore neutral uses the real flag, not an assumed false gate. A false flag
returns only after all three interfaces resolve and before instance dereference.
Adapter checks actual published animation occurrence and actor entity issuance,
actual physical allocation, and published Motion occurrence. End only resolves
IChannelAnimatable; it does not require physics or shared attributes.

With AllowPumping true:

1. Capture previous influence+8; read actual PhysOut.Ground+fc pumping_acceleration.
2. Live scalar57bcb5251cc1ed5c divides acceleration. fsel ef68..ef74 clamps
   ratio0..1, preserving unordered selection and signed-zero operand choices.
3. Scalarc7e4895c512a8550 is the smoothing weight. efd0 rounds weight*ratio;
   efd4 FMA computes `(1-weight).mul_add(previous_filtered,weight*ratio)`;
   store+c precedes the next graph read. Do not regroup those products.
4. Cached graph8 base3d0, x3e0/y400 evaluates filtered value. Store+8 atf01c
   precedes threshold scalarc47d82b1fb49e9d7.
5. Crossing is `!(previous>threshold) && current>threshold`. Raw f07c41990014
   rejects orderedGT previous; f08c41990008 requires orderedGT current.
   PreviousNaN can cross; currentNaN cannot.
6. On crossing, HasChannel queries PUMP0..PUMP4 in order until first missing.
   If all exist, skip both blend-length scalar reads and cvar238; retain the
   prior channel index and continue the final influence gate.
7. On first missing channel, read blend-in7e08405145cc275d THEN
   blend-out0ff834961814e5f9, THEN live kUsePumping238. The blend-in value is
   actually loaded/stored atf158/f15c despite being omitted by pseudocode.
   Enabled calls NewChannel with exact contract below. Native ignores its
   return status. Successful return OR cvar disabled stores selected index+10
   then current influence into peak+14. A modeled service failure preserves
   earlier filter/influence stores without pretending the NewChannel returned.
8. Raw f234/f24840980008 are !LT branches: channel>=0 AND
   `!(current<previous) && !(current<last_published)` accepts unordered.
   Store peak+14 atf274 BEFORE SetChannelInfluence+3c atf298. Existing actual
   `try_set_body_spin_influence` preserves source missing-channel=false and
   direct influence assignment without clamp or NaN rejection; return ignored.

## Shared live attributes

Original Global+134 anim_motion_anim_pump uses generated anim_motion
class90c3decf0105631e, collectiond40d1dc0353679d7. Constructor828b2de8 and
publication2df4 follow actual allocation2dc4. Retain wrapper lifetime in the
same global bundle as other anim_motion collections. Generic scalar access
reruns Vault lookup/inheritance each call with index0; missing data defaults0.
Generated cached layout5c0 uses explicit DefaultDataArea5c0 fallback. Graph
route bypasses generic inheritance and uses eight x/y words at3e0/400.
The adapter checks global Vault identity against the actual physical owner.

## Exact NewChannel contract and scoped remaining service

Original pump names initializer82f85e90/table82fd4ed0 constructs five
FastString36 names PUMP0..PUMP4. PumpingSkaterAnim initializer82f84540 uses
literal `B_PUMP`822fa268 (global830a4684). Source NewChannel at82c9f218:

* selected pump name, animation B_PUMP;
* priority0, cyclefalse, mirrorfalse;
* speed1;
* blend-in length from7e08405145cc275d, blend-in-first-framefalse;
* blend-out length from0ff834961814e5f9, blend-out-last-frametrue;
* use-channel-attributesfalse (r10=0).

`PumpingChannel` records selected index and both live lengths. The draft actual
owner adapter explicitly returns PendingAnimationOperation only at this concrete
NewChannel call. Root approved this scoped boundary: existing
`actor_animation_channel_runtime::try_new_gesture_channel` owns the real channel
allocation/tree construction/BindPose/dictionary/priority pipeline but hardcodes
.3/.3 and last-framefalse. Parameterize that existing helper in root-owned work
to accept both lengths and last-frame flag; keep existing gesture defaults,
priority0/noncycle/nonmirror/speed1/first-framefalse/useAttributesfalse unchanged.
No duplicate allocator or post-publication parameter mutation is supplied.

When generalizing, apply live lengths at ChannelBlend Init's original stores,
set runtime.blend_out_last_frame=true and corresponding flag40000000 before
dictionary publication. Derive blending_out/flag08000000 from ordered
`blend_out > 0.0` (Init82d6d548/d568/d574/d590), including false for zero,
negative or NaN; existing gesture .3 happens to produce true. Then
then preserve all existing free-list/dictionary/max-channel/priority ordering.
Ignore returned NewGestureChannelStatus on successful Result, as native does.
This pending branch is not a claim of full Pumping animation-channel support.

## Cvar238 registration

Root reserved238 kUsePumping. Reference830a3414, initializer82f85f98,
constructor call82f85fc4, table82fd4ed4. Section `Animation_Skater_Pumping`
(822fafe4), title `Use Pumping`(822fafcc), label at822fafd8, bool defaulttrue,
null update callback. Late sampling only after both blend lengths.
Table neighbors:4ec8 kEnableDebugGestures,4ecc kEnableLandingOnBoardBodySpin,
4ed0 kPumpNames (non-cvar),4ed4 kUsePumping,4ed8 kCapDvAtTopSpeed,
4edc kAllowHolding,4ee0 kUseFineKickturn. Insert source table order rather
than numeric index order.

## Tests / review boundary

Six tests exercise actual kernel: disallowed missing-instance return timing;
disabled creation still recording first free index and publishing influence;
NewChannel failure retaining filter but not index/peak; old/currentNaN branch
semantics; influence failure retaining peak plus partial End order; full pump
channel set skipping lengths/cvar without resetting retained index.
Author ran only rustfmt. Source review, mounting, builds, tests and sustained
actual-owner neutral continuation remain root/reviewer responsibilities.
