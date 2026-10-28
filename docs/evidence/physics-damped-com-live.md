# Retained Physics DampedCOM consumer

Confidence: high source reconstruction; native numerical trace parity unclaimed.
Draft author output_continuation; independent review and root integration/gates pending.
No Cargo commands or registered/shared edits by this author.

`Sk8::Physics::PhysOutConditioner_Physics::UpdateDampedComPos`82e2a720
is called by Update82e2f970 at82e2f99c, after the existing three-call prefix.
It returns before CalcFootSideOfDeck82e25b28 (call82e2f9a4). The new
`Conditioners::try_update_physics_damped_com` consumes the actual
`PhysicsBeforeDampedCom`, checks its output identity and both retained owner
bindings, mutates the existing published `simple::Physics`, and returns
`PhysicsBeforeFootSideOfDeck`. This is not full Physics.Update or UpdateAll.

PDB SystemReckoning: damped velocity+0, actual COM velocity+10, ball offset+20,
COM+30, damped COM+40, up+50, heading+60, foot side+70. Existing concrete output
and vectors own these values. Physics histories are existing last position+60,
heavily damped velocity+70, accumulated error+80 and valid byte+bf.

## Source ordering and arithmetic

Initial guard748/assert750..764 precedes System capture768, valid-byte770 and
COM vector778. Invalid history: store COM to last positionaa30, valid=trueaa34,
zero heavy velocityaa3c. Accumulated error is not cleared. No cvars/numeric call.

Valid history: sample guard79c, capture targets VelBlend7a4, VelBlendY7ac,
Blend7bc, BlendY7c4; load floats7c8,7d0,7f0,7f4. Form coefficients
`[xz,y,xz,0]` and complement against `[1,1,1,0]`, then assert sampled guard834.
Multiply current COM by position coefficients858 before actual velocity load868.
Multiply velocity coefficients870; load old heavy velocity874; fused complement
times old plus rounded current product8b0; store heavy velocity8b4.
Load last position8bc. Predict with fixed f32 bits3c888889 (1/60), not session DT:
fused heavyVelocity*step+lastPosition8c8. Load accumulated error8d0, fused
position complement*prediction+weighted current8d4; subtract error8d8; dot3
with old accumulated8dc. Ordered dot>0 alone reads ErrorBlend908/90c and fuses
old accumulated*blend+position928. Zero and NaN do not enter this branch.

Read Draw934/938. If true require actual DebugOverlay singleton948; native
assertions968/988 are represented by checked missing-service failure. No draw
method is called here. Store last position994, fresh accumulated load99c,
fresh error subtraction9a4, capture/read ErrorIter9b4/9b8, complement9bc,
rounded weighted error9f0, fused old accumulated*complement+weighted error9f4,
store accumulated9f8. Final guardaa44 precedes retained last-position loadaa6c
and output damped COM storeaa70. Source prefixes survive subsequent failure.

Raw VMX128 VA extension decoded from word bits5 and10; IDA printed aliases
for these high registers are not trustworthy. Standard vmaddfp operand order
is D,A,B,C, arithmetic A*C+B. Scalar products stay separately rounded where
source vmul precedes vmadd. Dot3 uses actual ReckoningNumeric service; host
denormal/NaN payload/VMX reduction parity retains its existing declared limit.
Unknown persistent W propagates through arithmetic; dot3 ignores W only.

## Seven live cvars and authoritative registration positions

All section `Physics_Physout_DampedComBlend`; floats native min0/max1, no
callback (raw r10=0 before float ctor; decompiler extra argument guesses are
incorrect). Defaults are initial values, never runtime constants in consumer.

| index | label | title | default bits/value | initializer | ctor call | table slot | reference |
|---|---|---|---|---|---|---|---|
|195|kDampedComBlend|Damped Com Blend|3d4ccccd|82fa00d8|82fa0118|82fd6430|8311e6b0|
|196|kDampedComVelBlend|Damped ComVel Blend|3d4ccccd|82fa0138|82fa0178|82fd6434|8311e6dc|
|197|kDampedComBlendY|Damped Com Blend Y|3cf5c28f|82fa0198|82fa01d8|82fd6438|8311e964|
|198|kDampedComVelBlendY|Damped ComVel Blend Y|3cf5c28f|82fa01f8|82fa0238|82fd643c|8311e998|
|199|kDampedComErrorBlend|Damped Com Error Blend|3da3d70a|82fa0258|82fa0298|82fd6440|8311e914|
|200|kDampedComErrorIterBlend|Error Iteration Blend|3d4ccccd|82fa02b8|82fa02f8|82fd6444|8311e63c|
|201|kDampedComDraw|Damped Com Draw|false|82fa0318|82fa0344|82fd6448|8311e940|

IDA byte reads confirm all seven table dwords point to the listed initializer.
In current selected REGISTRATION_ORDER, insert `195,196,197,198,199,200,201`
after index50(table62bc) and before index80(table6454). Native omitted
intervening initializers are not silently claimed reconstructed.
Append seven `Definition` entries at these reserved indices using table strings
and `Value::Float(f32::from_bits(...))` / `Value::Boolean(false)`.
Existing float Value carries no metadata range; retain native0..1 evidence here
without inventing clamping (startup native float deserialization is unclamped).

## Root shared integration only

In physical_player_phys_out_conditioners.rs:

```rust
#[path = "physical_player_physics_damped_com.rs"]
pub(crate) mod damped_com;
```

In physical_player_phys_out_construct.rs:

```rust
#[path = "physical_player_physics_damped_com_output.rs"]
pub(crate) mod damped_com;
```

In input_cvar_runtime.rs (plus definitions/order above):

```rust
#[path = "physical_player_physics_damped_com_cvars.rs"]
pub(crate) mod damped_com;
```

At actual selected_ground_conditioning continuation, retain the returned
PhysicsBeforeDampedCom and pass it, same mutable PhysOut and Conditioners,
actual session InputCvarReferences, DebugOverlaySlot and numeric service to
`try_update_physics_damped_com`. Do not rerun the first three calls to fabricate
a replacement token. Return the new token as the next checkpoint.
No shared field additions are required.

Six authored focused tests cover first-update lazy inputs/error preservation,
live axis coefficients/fixed step, numeric-failure partial history, missing
overlay before position publication, positive error correction/unknown W, and
foreign actual continuation rejection. Tests are unexecuted by author; root
registration, fmt/test/clippy and independent source review are required.
Remaining three Physics calls and outer UpdateAll collaborators remain pending.
Root independent decompile/raw review accepted VMX operand/capture/store order. Registered actual selected path reaches PhysicsBeforeLandingQuality after next foot-side call. Accepted workspace70/original60/Clippy122/fmt48/diff49.
