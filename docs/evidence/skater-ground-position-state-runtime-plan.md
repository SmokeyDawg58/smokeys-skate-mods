# Ground-position state and jump-counter runtime closure

Read-only audit, 2026-09-08. Confidence **exact** for cited raw fields/stores and **high** for the integration plan. No Rust edits or Cargo. Companion skater-calculate-ground-position-runtime-plan.md contains the implemented/reviewed CalculateGroundPos leaf graph; this packet identifies its remaining HiLOD scheduling authorities.

## Current code versus actual state owner

The actual pair retains `PhysicalPlayerAssemblyOwner`, SAME `PhysicalPlayerReckoningOwner`, SAME `SkeletonProcessedContact`, prepared-velocity history, actual board body/ground basis and shared `PhysicalPlayerPhysOut`. These are reusable authorities. `physical_player_ground_position_session.rs` now computes a selected native board result from that same owner, with an actual completed Reckoning phase.

No retained native `PhysicalPlayerHiLOD::mPhysicsState` (+2a8), state-object collection, `SetPhysicsState` or `CheckForStateChange` implementation was found. Existing `physics.rs::PhysicsCategory` is a semantic enum (not repr/native integer values). `GroundStateInput::suggested_state` is a detached reconstruction of one state-changer case with caller-supplied flags/tuning. `GroundAnimationState` is a separate selected calculation state, not the native virtual state owner. Neither authorizes publishing a chosen Ground category as live current state.

`PhysicalPlayerPhysOut` currently retains selected motion/ground/SystemReckoning fields and latch/occurrence. State and Air remain omitted lifecycle components routed through `PhysOutRemainderStage`; Ground wallRideExit also has no retained field. Extend this SAME aggregate for any new selected fields, retaining existing lifecycle/latch and `PhysOutValue` origins. Do not create a second state output or derive category from contacts.

## Category source and early snapshot

Native `PhysicalPlayerHiLOD::Update5_ProcessOutput`82e04ea8 resets actual PhysOut, runs numerous ordered subsystem FillPhysOut calls, then reads the actual current state more than once. The category source is the later read:

- 82e055e4 loads HiLOD+2a8 actual state pointer;55e8 its vtable;55ec virtual+14 `GetType`;55f4 call;55f8 retain returned state ID.
- 55fc calls `PhysicsState::GetPhysicsCategory`82d83b80;5600 retains category.
- Intervening airborne work is real: only state ID in[200,300) reads guarded Collision.numWheels; then independently guarded Air.mIsSkateboardAirborne is stored568c.
- Fresh PhysOut pointer5690, latch sample5694/assert569c..56b0; fresh State pointer+1c56b4; store category to State **+0c at56b8**.
- The category is not produced by `PhysicalPlayerStateChanger::FillPhysOut`82dd3bc8. That routine publishes revert/teleport flags only. Nor does `PhysState_GroundAnimation::FillPhysOut` produce it; that publishes actual state-triggered jump and trajectory when mJumpedThisFrame is true.

`GetPhysicsCategory`82d83b80 first asserts unsigned state<=702 (negative IDs also fail this guard), then descending signed ranges: >=700?Other700;>=600?GroundTrick600;>=500?OffBoard500;>=400?Grind400;>=300?Wipeout300;>=200?Air200;>=100?Ground100;otherwise Invalid0. Existing semantic enum requires explicit mapping; casting its ordinal is wrong. Safe-host invalid-ID refusal should be explicit.

`PhysOut_State::SetResetValues`82e25550 writes category **0/Invalid at2556c**, state0 at25570 and other source fields separately. This is legitimate Reset provenance, not a live Ground state. Native constructor invokes selected state initialization through its actual aggregate construction; exact constructor call site/lifetime must be retained when extending the component.

Next Update1 runs omitted Tick/line-test stages, then captures category EARLY: aggregate+2ac1fdc, latch+4c1fe0..1ffc, State+1c2000, category+0c**2008**. Only then ProcessedPhysIn::Reset200c. The ground-position OFFBOARD and AIR/GROUNDTRICK branches35xx use this retained r18 snapshot. Re-reading category from current output or processed flags at the ground-position stage changes source timing.

## Native current-state lifecycle is not a numeric setter

HiLOD constructor stores current-state pointer null at3e94, constructs retained state objects and collaborators, then loads **mPhysState_Sleeping+240 at4d00**, stores it as current +2a8 at4d0c, and calls **SetPhysicsState(100/PhysicsGround) at4d10**. Do not jump directly from allocation to a fabricated Ground enum.

`SetPhysicsState`82dff158 checks old GetType differs; resets retained state timers/counts and publishes previous/next/current processed state fields; calls old state's Exit; performs Actor/debug/offboard-related intervening effects; selects the already constructed matching state object; calls its Enter. These effects require source-owned state objects and actual collaborator dependencies, not only `GetPhysicsCategory(100)`.

`CheckForStateChange`82e005a8 reads actual state GetType at005d0, calls SAME StateChanger(+300)::CalcSuggestedState005e0. If suggestion differs, it stores **HiLOD+128=100 at005f8 BEFORE SetPhysicsState005fc**. A failed state transition preserves this counter store. This additional producer must not be omitted from a purported live jump-counter implementation.

## HiLOD framesSinceJump and allow-jump state

Constructor: r30=100 at82e03dc4?+128 store3dfc. Allow-jump is known true from the source flag operation: r7=7 at3dbc; load old word+1203de4; `rlwimi r8,r7,29,0,9`3de8 replaces high10 bits with e0000000 while preserving low22; store3e3c. Native allow-jump is bit30 (mask40000000), known true; do not zero the entire flags word. +d0 and +e0 ground positions are distinct source full-zero constructor stores3db0 and3dc0.

Ground-position gate at35d8 reads **old** +128 and tests signed>0 before later counter update. Ground position update is skipped for early captured AIR200 or GROUNDTRICK600, otherwise allowed when old count>0. Even processed frames-since-teleport<=1 only suppresses later displacement/filtered-velocity work; +d0 was already stored3608. Final processed+1e0/+1f0 copies remain independent source stores36ac/36b8.

After prepared-velocity work, Update1 counter stage82e038e0..39e8 is:

1. Capture actual PhysOut and check its latch before Ground pointer3914 and wallRideExit byte+1203918. If true store+128=0 at3924.
2. Capture current processed pointer392c. Read old +1283938, wrapping raw addi+1 at393c, store count3940. Native uses the 32-bit register result; Rust debug overflow must not introduce a refusal.
3. Read processed **halfword+8d0 at3944**, processed flagsword+8c4 at3948. Test halfword&1 at394c/3954 and extract JUMP bit22 at3950. PDB proves halfword bit0 is **mIsHippyJumping**, the SAME actual field already retained by `SkeletonProcessedJumpEvents::hippy`, not an unknown offboard flag.
4. If Hippy true, accept without reading allow-jump. Otherwise if JUMP false, reject without reading allow-jump for condition; if JUMP true, sample HiLOD+120 allow-jump bit30 at3964..3970; accept only true.
5. Reject: read retained +120 again3974; use complement of the EARLIER captured processed flagsword3978 to set allow-jump=!requestedJump397c/store3980; fresh read processed+8c4 at3984, clear JUMP bit22/store398c. Thus the earlier captured request controls allow-jump while a later word read preserves unrelated current bits during the clear. Do not call begin/reset or replace all known flags.
6. Accept: capture kbDrawJumpDebugInfo target399c, read bool39a0. If enabled, load actual debug-overlay singleton39ac; only if null assert/debugbreak39b8..39c8. No drawing call occurs in this selected branch. Only AFTER those reads store count0 at39d0 and clear allow-jump bit30 at39d8. Debug failure retains earlier optional wallride reset plus increment, but does not perform these final resets.
7. Fresh processed pointer39dc, current counter read39e0, publish processed **+91c at39e4**. Counter storage and output publication are distinct owners/stages.

No NaN comparison exists in this counter stage; inputs are known bits, bytes and signed/wrapping integers. Zero/negative old counters matter to the earlier ground-position gate. Subsequent +12c jump-correction counter has a separate producer from PhysOut_Air::mStateTriggeredJump and is not interchangeable with +128.

## Remaining actual inputs and reusable producers

`SkeletonProcessedJumpEvents` already owns source Reset/type0 Hippy and JUMP. It needs a narrow read-only known-Hippy accessor and a source clear-JUMP mutation preserving PREPARE/other known bits; do not duplicate these fields in a HiLOD projection. Selected raw jump/contact processing does not imply full Actor event production, but it is a valid existing same-owner source for this bounded counter stage.

WallRideExit source: `PhysOut_Ground::SetResetValues`82e250c8 writes false at**25194**. Actual `PhysState_PhysicsGround::FillPhysOut`82d8ddf0 stores its retained **mExitingFromWallRide** to Ground+120 at**82d8df90**, after a source latch guard. This production state owner is absent. Extending the aggregate with genuine constructor/Reset-backed false is valid selected provenance; silently returning false from an absent Ground component is not. Later runtime wallride publication remains a real state dependency.

Category snapshot similarly can consume a genuine same-aggregate Reset Invalid value once the selected field's actual Reset is implemented. That allows a explicitly Reset-backed controlled first-step path; it does not prove active native state or justify default Ground. To claim actual next-frame category, integrate retained state object construction/Enter/Exit and source GetType publication.

Existing actual CalculateGroundPos bridge, same Skeleton effective-animation-transform APIs, prepared-history adapter and PhysOut latch infrastructure should be reused. Remaining +e0/+d0 projection needs:

- actual early category snapshot capability bound to SAME PhysOut occurrence, with sampled latch/source origin;
- same retained HiLOD ground/history fields initialized at source ctor execution, including old framesSinceJump100 and known allow-jump bit;
- staged OFFBOARD actual Skeleton anim-to-world translation or actual board ground-position computation;
- actual intervening effective Skeleton transform and processed stores (no generic success callback pretending it happened);
- actual processed timeDelta+93c capture, framesSinceTeleport+928 and estimate-port filtered-velocity suffix, or explicit stop before omitted work;
- SAME processed Hippy/JUMP reads/late clear, actual Ground wallride byte and debug cvar/singleton callbacks for subsequent counter stage;
- explicit later StateChanger transition producer for the counter100 store.

Recommendation: first extend SAME PhysOut selected category/wallride storage and provenance plus actual HiLOD field projection/constructor. Implement the source counter stage against existing processed jump fields and staged Ground/debug reads. Keep state-change100 as a separately authorized actual transition stage, not a caller-set convenience. Then connect selected ground-position stages through actual effective-transform and filtered-velocity services. Full native Update1/Update5 and a live Ground category still require the actual current-state owner; existing force-leaf completion alone cannot supply it.

Fresh raw SHA-256 anchors (end exclusive):

- Update1 counter82e038e0..82e039e8: 0f1b8c72e920d2e05f05f7d9cc0288fcba1aaf6b5bfffd77de4b74b0fccd4e60
- Category publication82e055e4..82e056bc: 68cfdec79d7e3c371fe58752ec973382e0b56a7dc6759e12584ce3182a027e75
- CheckForStateChange82e005a8..82e00618: 2f793be9e35abbfb7694fce6e34ebd85fec005996a113534a59f6df03e09c221

