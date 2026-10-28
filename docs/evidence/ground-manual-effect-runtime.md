# Ground CalcManualEffect source and unregistered runtime draft

Source: actual Ground82d9a774 calls Skateboard.CalcManualEffect82b32130 with
r3=hidden 0x30-byte result, r4=the SAME board and r5=false. Both boolean arms
are implemented in the draft; the decompiler's object/result types are swapped.
PDB SkateboardManualEffect is angular displacement+0 and corrective force+10
(world force+10, local position+20). This is not a new board force owner.

## Owner and commit order

The new Entry child `skateboard_manual_effect.rs` uses existing board fields:
e8 manual_error_integral, ec manual_last_target_angle, f0 manual_last_actual_angle,
f4 manual_torque, f8 manual_time_since_reset. It NEVER touches fc manual_spin_angle.

1. 321bc/c4 zero full corrective world/local vectors; angular output untouched.
2. 321d0/1d4 capture Processed94c flipped-controls and9a0 balance.
3. Three boolean-selected Manual attribute lookups/loads, each missing pointer
   using source DefaultDataArea4. Then capture actual Reckoning+20 matrix.
4. 3240c/24 debug reference read, optional actual overlay requirement, BEFORE
   balance equality at32454. Balance zero cannot bypass any previous read.
5. Zero balance32a4c..5c clears e8/ec/f0/f8/f4 in that order. Nonzero includes
   NaN and follows the active numerical path, with explicit numeric errors.
6. 32a64/6c read f4 and multiply final scalar; angular vector32a94 is the
   captured ground X XOR-negated in all known lanes, times this scalar.
   Corrective local32a98 then world32a9c are full zero in BOTH active/zero arms.
   Unknown source X.W stays unknown, even when multiplying by zero.

## Active path

3245c..98: directed=balance*flipped. NOTGT(directed,0) includes unordered and
chooses -1; otherwise +1. Magnitude is abs(directed).mul_add(.75,.25)*sign.
Capture derivative-active from existing f8==0 before first target attribute.
3250c/548: degrees=attribute*directed-magnitude, then degrees*3c8efa35;
reuse Math.Angle.Normalize (source callee and existing numeric boundary).

32560 gates actual8c4/20000000. If clear, read deckZYDelta9cc, stateTimer974,
three Manual attributes and source XMVectorSin824b0060. Reuse the existing
`reckoning_sin_cos::sine` exact polynomial/reduction, including explicit nearest
rounding collaborator. Capture graph8 X60/Y80 from actual Manual layout0xa0
BEFORE the source absolute-speed948 read32714. Two successive Normalize calls
combine curve*(wave+deck-term) and existing target.

32744 checks f8 again. First use normalizes target*initial-scale and writes f4
at327b0. Always publish target ec at327b4 BEFORE actual Body.GetPosition(6)
at327c4. Reuse typed Part6 transform; signed AngleBetweenVectors receives
Part6.Z, captured ground.Z, captured ground.X (wind). Normalize result, read old
f0 then publish new actual angle327e8, subtract old actual, normalize target-error.

32850..90: late max-error load; old integral read; derivative delta is multiplied
by captured derivative-active then derivative coefficient. Scalar fsel symmetric
clamp retains native NaN behavior. Proportional term uses separately rounded
multiplication. Integral is old.mul_add(3f733333, bounded*3d4ccccd) and publishes
e8 BEFORE the later maximum-derivative attribute lookup.

328fc first flags8c8 capture; derivative clamp; ((flags>>1)^flags)&08000000
controls live kWrongWheelScalar read. Then source32934 reloads flags8c8.
Wrong wheels: front bit10000000 clear with balance>0 OR rear08000000 clear
with balance<0. Unordered balance takes neither wrong-direction test.
Wrong branch329ac publishes f4=(old*decay)*wheelScalar BEFORE a final attribute
lookup. At329e8 final output scalar becomes wheelScalar*finalAttribute.
Correct wheels329f0..a00 update f4 with three ordered adds: old+derivative,
then integral, then proportional. Second live debug check32a04 precedes fresh
DT93c and f8=DT+old-f8 publication32a44. A late error preserves earlier stores.

## Attribute keys in source order

All keys are reads through the existing Body `SkateboardAttributes` Manual
projection, native global+c8 classa0d7351efa176078/defaultd7edbd362d7d2152.
No new instance is constructed by the adapter.

| Purpose/site | false / true key |
|---|---|
|proportional322fc/32218|a054d2d54eda3c5c / 7ed1825985b2679a|
|integral32354/32270|4f0e182cfd45a690 / ee157c8b8459aeac|
|derivative323ac|72288fb0cee6eda8 / b1fbfff98936b84a|
|target324ec|1bd81d97e88e2232|
|deck delta325b0|89ed27f8244dfa61|
|wave frequency32618|7c52f92bc4a258b6|
|wave amplitude3269c|47faac614476f338|
|initial torque32790|710fc1fef4c3ca3f|
|error clamp3283c|01372180f38867cc|
|derivative clamp328d8|966a83ee602b0876|
|wrong-wheel history decay32984|e8c9db1077d1d889|
|wrong-wheel final output329d0|c9906592101163bd|

Generated Manual ctor828b0ed8 requests0xa0 DefaultDataArea. New child
`skateboard_manual_effect_attributes.rs` captures X60/Y80 from SAME instance.

## Cvars and live adapter

Reserved144: kShowManualInfo, reference8308d8d8, initializer82f7b438,
table82fd4438, Physics_Manual / Draw Manual Info, boolean false, null callback.
Reserved145: kWrongWheelScalar, reference8308ddb0, initializer82f7b5e8,
table82fd444c, Physics_Manual / Wrong Wheel Scalar, float .5, GUI[0,1], null
callback. Fresh raw initializer instructionsb45c/460 andb60c..624 establish these.
New `input_cvar_manual_effect.rs` getters require actual linked references.

`physical_player_manual_effect_source.rs` retains references to actual player,
Processed Arc, board session/backend, constructor attributes and overlay slot.
Reckoning lock spans only the source transform capture; callers must not hold
that same lock around this operation. The actual Part6 typed read occurs late,
after target-history publication. No host canonical W is promoted.

## Integration and validation boundary

All five Rust files remain unregistered. Required registrations: Entry child
manual_effect; Body attributes child manual_effect_attributes; constraint-session
manual_effect_source; cvar child/manual definitions144/145. Root owns caller
mount after torque output and before LinearDrag. No shared field migration is
needed beyond the already planned separate Heading spin field used by tests.

Five new focused kernel tests cover zero balance's mandatory prefix and unknownW,
early output initialization, active wrong-wheel history/output distinction,
late failure partial writes and powerSliding/nonmanual graph ordering. Tests
are NOT run; no Cargo was invoked. Original corpus/live first-frame admission,
full source independent review and compilation remain root integration gates.
Numeric services retain explicit host estimate/reduction/rounding boundaries;
this is not a claim of bitwise Xenon floating-point equivalence.
