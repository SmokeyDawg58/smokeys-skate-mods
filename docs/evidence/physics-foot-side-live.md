# Actual Physics foot-side consumer

Confidence: high source reconstruction; reference numerical traces pending.
Author output_continuation; independent source review and root registration/gates
pending. This packet does not claim full Physics.Update or UpdateAll.

`Sk8::Physics::PhysOutConditioner_Physics::CalcFootSideOfDeck`82e25b28 is
the fifth call of Update82e2f970, invoked82e2f9a4 after DampedCOM82e2f99c.
The next call is CalcLandingQuality82e2aa88 at82e2f9ac, outside this change.
No new cvars or fields are needed.

## Actual producers and retained ownership

`physical_player_phys_out_board_fill.rs::try_fill_board_transform_directions`
already publishes the actual part transform to SkateboardReckoning.transform.
This calculation uses that transform, not the effective transform.
`physical_player_phys_out_skeleton_part_fill.rs` publishes right-foot look
from bone19 column0 (source82b58734) and left-foot look from bone15 column0
(source82b58778). Both fields are already retained by the actual Skeleton.Fill.

PDB PhysOut_SkateboardReckoning.mTransform+0 has xAxis+0.
PDB PhysOut_Skeleton.mSkeletonFootRightLook+100 and mSkeletonFootLeftLook+110
are look vectors, not positions. PDB PhysOut_SystemReckoning.mFootSideOfDeck+70
is the existing `PhysOutSystemVector::Right` slot, initialized to RIGHT by
the existing constructor/reset. The legacy Rust enum name does not imply
a distinct extra output field; no second copy is introduced.

The new Conditioners method consumes `PhysicsBeforeFootSideOfDeck`, checks
its output plus retained Conditioners and published Physics owner bindings,
executes this actual callback on the same output, then returns
`PhysicsBeforeLandingQuality`. The native method reads only this->mPhysOut;
Physics histories remain unchanged. Missing or foreign owners fail explicitly.

## Exact source sequence

1. Capture output5b48; guard5b4c/assert5b64; capture Board pointer5b70.
2. Fresh output5b6c; second guard5b74/assert5b8c; first Skeleton pointer5b98.
3. Fresh output5b94; third guard5b9c/assert5bb4; second Skeleton pointer5bbc.
4. Load captured Board.transform.xAxis5bc8, captured first Skeleton.leftLook
   (+110)5bdc, second Skeleton.rightLook(+100)5be0. Add right+left5be8.
5. Dot3(axis,sum)5bfc; ordered comparison greater-than +0 at5c08, branch5c18.
   Only positive chooses fresh axis load via original Board pointer5c1c/5c40.
   Negative, either zero sign, and NaN take the other branch: construct sign
   mask80000000 in every lane5c24..30, XOR saved axis5c34, store scratch5c38,
   load selected vector5c40. This flips W and signed zero without arithmetic
   NaN canonicalization. Unknown W remains unknown in the host carrier.
6. Fresh output5c3c; fourth guard5c44/assert5c5c; System pointer5c64; publish
   selected full vector to +70 at5c6c. Numeric/guard failures do not publish.

Raw VMX128 VA decoder uses base bits16..20, bit5->32 and bit10->64. Checked
words:5be8=179df035 => VA61(right),5bfc=177fe1b5 => VA63(axis),
5c08=1b1bc965 => VA59(dot),5c30=1ad7b8f5 => VA55(allones),
5c34=16bfb335 => VA63(axis). IDA printed high-register VA aliases are wrong.
vmsum3 dot uses the existing actual ReckoningNumeric service, preserving its
documented host reduction/exceptional-value boundary; no normalization or
substitute geometry calculation is added.

## Root integration

Register in physical_player_phys_out_conditioners.rs:

```rust
#[path = "physical_player_physics_foot_side.rs"]
pub(crate) mod foot_side;
```

Register in physical_player_phys_out_construct.rs:

```rust
#[path = "physical_player_physics_foot_side_output.rs"]
pub(crate) mod foot_side;
```

After the actual retained DampedCOM continuation, call:

```rust
conditioners.try_update_physics_foot_side(before, output, numeric)
```

This consumes `PhysicsBeforeFootSideOfDeck` and returns
`Result<PhysicsBeforeLandingQuality, construct::foot_side::FootSideError<N::Error>>`.
No shared field additions/cvar registrations are needed. Do not rerun earlier
conditioners to manufacture a token. Root mounts the actual session caller.

Six focused tests authored: actual numeric operands/positive full-axis copy;
negative/zero/NaN sign-bit selection; unknown W; numeric/locked-output failure;
actual preceding prefix+DampedCOM token success with preserved Physics history;
foreign continuation rejection before numeric work. Tests unexecuted by author;
direct rustfmt only, no Cargo or shared/registered file edits. Root independent
source review and fmt/test/clippy gates remain required.
Root independently verified raw5be8 right+left,5bfc axis dot sum,5c08 ordered comparison,5c30 sign-mask and5c34 XOR; registered with actual preceding DampedCOM token. Accepted workspace70/original60/Clippy122/fmt48/diff49.
