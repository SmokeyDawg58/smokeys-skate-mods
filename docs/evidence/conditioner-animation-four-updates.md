# Animation conditioner: four bounded callback drafts

Fresh source/decompile/raw review, 2026-09-19. New unregistered source file
`physical_player_conditioner_animation_updates.rs` is intended as a child of
`conditioners::animation`. It operates on the existing retained `Animation`;
no parallel histories or default-filled output owner are introduced.

## Source order and boundary

Animation.Update82e31970 calls CurrentTrick82e300c8 at31984,
CompletedTrick82e2ff18 at3198c, AnimationTurn82e2b470 at31994,
BodyTilt82e2ce60 at3199c, DeckGroundAcceleration82e299b8 at319a4.
The draft implements four independent methods, deliberately no wrapper that
would silently omit the intervening AnimationTurn.

CurrentTrick reads actual Trick.current_animation.name, compares packed names
ridingidle/wipeout/land/tailmanual/nosemanual/switching, then independently
checks the output accessor and stores only current_name. Invalid is the existing
empty FastString, not Andale nullString. Static source initialization has no
runtime service effects; existing FastString36::encode preserves comparison.

CompletedTrick only performs its first current-name read when retained previous
name is valid (ff88). It unconditionally reads full current data at3000c and
retains its complete clone at3003c. Separate checked flag store30068 precedes
separate checked completed-name store300a0..b8. A failure in the last store must
leave history and flag committed. This is tested using the production method.

BodyTilt captures actual Animation+0 atceb4, writes previous history to
Animation+10 atceec, then retains the captured vector atcef0. Live kDrawTiltInfo
is read only afterwards atcefc. True requires actual DebugOverlay singleton;
the final draw branch callcf68 targets825187c0, a genuine BLR.

DeckGroundAcceleration first independently checks SkateboardReckoning and
SkateboardMotion and captures actual transform/+70 acceleration. At29a7c it
samples the ground write guard before vector math, checks it atac8, then reads
Ground transform. At29b00 it samples the destination guard before final math,
checks it atb40 and stores Animation+70 atb64. The typed trait retains separate
sample/check boundaries instead of resampling later.

Raw VMX128 register correction is essential: decompiler v94/v92/v91/v86 are
actual v62/v60/v59/v54, and final transpose v81/v79/v77/v74 are49/47/45/42.
Each transpose interleaves I,K and J,P, producing columns
[Ix,Jx,Kx,Px], [Iy,Jy,Ky,Py], [Iz,Jz,Kz,Pz]. Product X then fused Y then fused Z
therefore produces W=dot(P.xyz, value.xyz), including when matrix basis W is
unknown. It must not become zero. After deck transpose local Y is explicitly
positive zero (29aa4); the forward direction transform still executes all three
terms, including the Y*0 term for NaNs. Final ground transpose has its Y replaced
with positive zero at29b3c. Native multiplication/FMA graphs are retained;
no inverse helper that subtracts translation is suitable here.

## Actual-owner integration plan

Mount child beneath conditioner animation; expose only the methods needed to
the parent Conditioners wrapper. Wrapper verifies Conditioners and its published
animation Bound output identity against PhysicalPlayerPhysOut before execution,
as existing Physics children do. Use the same retained animation.payload.

Implement AnimationUpdateOutput using that mutable actual PhysOut and live cvar/
overlay services. Every trait read/store maps to a fresh native write-latch
check, not existing const `try_animation_output`, which intentionally does not
sample the latch. Trick accesses use concrete.trick; tilt and acceleration stores
use concrete.animation. Bind deck and ground transforms to actual published
PhysOut transform fields, and +70 to SkateboardMotion acceleration (not empirical
acceleration). The guard associated type is existing PhysOutWriteGuard;
sample_guard uses sample_write_guard and the guarded operations use
check_write_guard with the captured guard. Avoid a constructed test snapshot in
the native caller. Trait failures remain caller-visible after preceding writes.

## AnimationTurn remains the next required implementation

82e2b470 is not an empty neutral-ground branch. It reads actual mirrored flag,
deck speed, target lean, ground turn, speed wobble twice and carving twice.
Use the retained global physics_animation AttributeRuntimeInstance and original
copied/updated three ScalarPid owners. It refreshes PID control data from live
layout and mutates slow/fast lean and direction histories before final output.
PointGraphEval uses original arrays (8-point graph X/Y offsetsd0/f0,10/30,90/b0;
4-point wobble graph120/130). Existing graph evaluator must be checked for exact
endpoints, NaNs and interpolation order before reuse. Direct scalar attribute
lookups have native DefaultDataArea fallbacks, distinct from layout reads.

Final independently guarded stores: animation_turn c038, target_lean c064,
input_lean c090, lean_quickness c0bc, lean_direction c0e8, holding c118,
camera_turn c148, camera_input_lean c17c. Only then sample kDrawCarveInfo.
Preserve raw FSEL graphs rather than Rust min/max/clamp (NaN and signed-zero
semantics differ). This draft does not yet discharge that method or Animation.Update.

Validation: standalone rustfmt passed. Three authored tests cover packed name
filtering, lazy completed-trick access and partial failure retention, native W
and NaN propagation. No Cargo or registered file edits were performed.
