# Grounded Motion brake / kickturn selection

2026-09-19. Confidence **high**, freshly checked against connected
`sk82_na_m.xex` IDA pseudocode and raw generated switch cases. Draft only;
no Cargo execution, actual-frame parity claim, or lifecycle admission.

## Actual next path

After OnGround24, failed EnterSkitching87/Into270 and Skitching88, the active
selector reaches RidingIdle89/Riding277 (earlier Random275/Bumped276 inactive).
Brake714 has no expression and visits Moving1014 then Stationary1015.
Moving expression215 returns NOT cached condition310; Stationary218 returns
the SAME cached condition310. Each has one initially active Into child,
1160/1163, expression216 condition311 HasIntent("Brake"). Other brake children
are inactive. Thus absent Brake intent rejects both branches while still
consuming the speed predicate. No brake shortcut is valid.

Next is KickTurn715 expression413. Pre evaluator8290aa50 absolute jump table
8290aa90 supplies these case ranges:

| Expression | Native range | Ordered expression |
| --- | --- | --- |
|215|829104a4..8291051c|!c310|
|216|8291051c..8291057c|c311|
|218|829105f4..82910654|c310|
|413|8291a5ac..8291a6d4|c290 OR (!c312 AND !c313)|

413 HasIntent call8291a5d0 tests290 (NewKickTurn, attrs2633). Success branches
to shared true829122f8. Otherwise LastState312 call8291a62c rejects a true
Turning.Idle match; then LastState313 call8291a684 rejects a true KickTurn
match. These names are authored attrs2648/2649. Condition314 is distinct.
Default6 last-state and absent NewKickTurn therefore ADMIT parent715; absence
of a kickturn intent does not by itself reject its parent expression.

Its active children are FromFakie1016(expr346), Right1017(expr347),
Left1018(expr348), all leaves. Raw continuation now included in the draft:
346 at829159b4 = c14 IsRidingFakie AND c314 HasIntent;
347 at82915a7c = c316 HasIntent AND c291 HasIntent;
348 at82915b48 = c317 HasIntent AND c291 HasIntent.
Their shared c314/316/317/291 condition payloads must be used, not inferred
direction/button constants. Condition314 and291 both ask KickTurn presence;
316 is authored greater=0 KickTurn,317 less=0 KickTurn. No duplicate synthetic
thresholds are introduced. These callbacks consume existing HasIntent payloads.

When all three reject, inactive Sliding716/HandPlantGround717/Revert718 are
skipped and Turning719/Idle1025 is reached. Its first active child1169
AggressiveOrLoose tests437 at8291cc38..8291ce0c: ordered lazy OR of331,332,333,
334,335, all IsProSkater. Native direct calls arecc5c,ccb4,cd0c,cd64,cdbc.
Authored names are Loose, Aggressive, MarkGonzales, DannyWay, JerryHsu.
With an actual Default pro profile each fails; next child1170 Default has
no expression and is selected. Still1171/Moving1172 are not consumed.

IsRidingFakie82c9a308 checks actual cached ISkaterAnim/UCOM equality before
virtual getter82cb7530, whose raw lwz+e4c/extrwi selects flag20000000.
The draft reuses PlayAnimationActor.resolve_interface(SkaterAnim) for the
same owner/context/publication checks, then reads the retained player's
known flag. Unknown flags remain an explicit error, never assumed false.
IsProSkater82caac80 was freshly checked: same owner check, virtual+30 atad58,
packed FastString30 equality. Reuse existing actor_motion_attach_intent.rs
motion_runtime_is_pro_skater; do not add a second implementation.

## Speed producer and actual-owner adapter

PhysicsSpeedCompare.ActivationFunc82c99808 verifies cached IPhysical/UCOM.
AlongSkateZ branch82c998cc chooses GetPhysOut82c998e8 then Motion pointer+4,
scalar+a8 ZSpeed and EvalCompare82c998f8. Otherwise GetPhysOut82c99904,
Motion+a4 SurfaceSpeed and EvalCompare82c99914. The PDB labels DeckSpeed at
Motion+a0 separately. Do not recompute magnitude.
Shared payload Speed {comparison, along_skate_z} already captures authored
attrs2646 abs=true/2647 less=0.15 for condition310. Runtime must consume that
payload and existing EvalCompare helper; no threshold constant belongs in
the adapter. No new cvar or global provider is needed for these conditions.

Draft `actor_motion_runtime_grounded_selection.rs` is intended as child
`grounded_selection` of generated_motion_load::runtime. Its Actor method uses
the existing bound Motion context check, actual cached physical allocation,
and actual retained output. REQUIRED SMALL INTEGRATOR EDIT:
add `try_motion_deck_speed` beside `try_motion_z_speed` in
physical_player_phys_out_construct.rs, identical component-complete check,
then motion_scalars[PhysOutMotionScalar::DeckSpeed as usize], map value,
ConcretePhysOutError::NotConstructed if absent. Native performs no output
write-latch sampling; existing generic motion_scalar would introduce one.
Draft intentionally references this not-yet-mounted accessor.

## LastState owner and name resolution

LastState.ActivationFunc82c94a00 captures name4a0c/controller4a10, reads actual
condition parent4a20, invokes GetSubState4a64, GetLastState4a9c, then
StateHasDescendent(last, target). No Actor controller re-lock is permitted
inside the already-held generated controller update.

GetSubState wrapper82955eb0 passes searchparent=true at82955f78. Index helper
8292a890 maps NULL parent to root0; splits at the final dot and recursively
resolves prefixes; searches immediate children in declaration order using
case-sensitive full-length equality; on miss climbs the current parent's
ancestors. The suffix search retains searchparent=true. NULL target is a
normal miss. StateHasDescendent8292a680 walks FIRST argument's ancestry,
including equality, with NULL first argument false. This is not a comparison
of full diagnostic state paths or a target==last leaf test.

Mount ConditionPayload::LastState {state} by passing the retained controller
last_state, current condition parent, generated STATES and authored state
string into last_state_matches. Keep existing successful condition cache
publication order. The helper includes original-table tests for sibling
KickTurn resolution, Turning.Idle dotted resolution, descendant matches,
Default6 nonmatch, NULL last state and exact-length name matching.

## Minimal integration and remaining boundaries

1. Register the unique child under generated_motion_runtime.rs.
2. Add component-only deck scalar accessor as above.
3. Dispatch shared PhysicsSpeedCompare/Speed to the Actor adapter; dispatch
   Motion LastState to the retained-controller helper.
4. Route Pre indices215/216/218/346/347/348/413/437 to draft expression helper while retaining
   one shared condition cache. Do not route Sustain through Pre speculatively.
5. Preserve all existing PendingExpression/PendingCondition/PendingCallback
   failures. This draft explicitly rejects228 and every unowned expression.
6. Dispatch PlainCondition::IsRidingFakie to this draft's Actor method;
   dispatch IsProSkater payloads to the existing attached-intent Actor helper.

Selection success is not lifecycle completion: OnBoard entry occurrence17
AttachIntent remains an independently required callback. Brake Into1160/1163
use update thunk828f3958; KickTurn FromFakie1016 uses828f2aa0, Right/Left use
828f2b18. Do not bypass current callback gates or claim sustained ticks.

Selected neutral leaf1170 ancestry is0,3,24,89,277,719,1025,1170. Behavior
ranges respectively0..4,17..33,90..93,177..179,722..725,1413..1414,
1923..1928,2172..2173. Turning1413 is SetTurning; Idle1923..1927 are
SetScoreAugmentation, AttachIntent, Pumping, ApplyingBodyTilt, CreateAttribute;
leaf2172 is PlayAnimation. Update thunks in ancestry order are828edd40,
828edf40,828ee770,828f03b8,828f04c8,828f1d88,828f2d28,828f39b8.
These are integration blockers to recover, not admitted empty callbacks.

Validation performed: rustfmt on unique draft only. Added focused tests for
413 lazy order and reached-error propagation, explicit next-expression
failure, original generated-table LastState resolution, kickturn children
operand order, and actual Idle child table Default selection after437's
five failures. Root owns Cargo,
corpus, registration and independent source review. Only these two new files
were authored for this bounded task; existing skitch draft/shared modules
were not edited.
