# Captured four-tick lean excursion: SetTurning polar conversion

Capture: artifacts/lean-trace-19086-23931.csv (131072 events, ticks10349..10862).
A playtest saw a severe visual lean for a few frames and then exited. The recorded
excursion at10823..10826 ends35 ticks before exit. The source fix below still
requires a compilation/playtest; no Cargo/rustc/game was run.

## Located the first discontinuity

Action94 / Motion1170 / Physics100 stay unchanged around the event. Body tilt
is zero and fakie remains false. Turning input smoothly decays toward zero.
The first recorded jump is the Reshape result consumed by SetTurning, before
animation tree selection, pose compression, or renderer palette construction.

| Tick | Turn input bits | Direction input | Published angle | Published direction |
| --- | --- | --- | --- | --- |
|10822|008e5d69|0.02760189|0.000014767886|-0.03035803|
|10823|00041c32|0.030407459|-0.99962664|-1|
|10824|00001b30|0.033368602|-0.9996371|-1|
|10825|0000009c|0.035930257|-0.9996479|-1|
|10826|00000003|0.037871692|-0.9996588|-1|
|10827|00000000|0.039141364|0.00001801702|-0.043050554|

The corresponding PhaseBlend nodes3/54 choose an endpoint for those four
ticks, and the local body matrices jump. Fast wheel rotations elsewhere in
the capture are not evidence of this body-lean issue.

## IDA and processor semantics

Existing implementation: animation/graphs/motion/callbacks/
motion_set_turning_reshape.rs, polar(), called twice by Reshape82c954e8.

- Native82c955dc vrefp128 ->82c955f4 negative multiply/subtract ->82c955f8
  refinement ->82c955fc Y*inverse ->XMVectorATan.
- Native82c95610/14 compare zero against X, and82c9563c selects signed half-pi
  for the zero-axis case. The second conversion repeats at82c9584c..58c4.
- Raw instructions captured in visual-lean-subnormal-ida-2026-09-29.txt.
  The VA register labels printed by IDA can be wrong; raw decoder confirms
  the same X register is used by reciprocal and zero/negative comparisons.
- Non-Java VMX comparisons treat denormal operands as zero, and arithmetic
  uses sign-preserving zero for them. See the vendor's
  [AltiVec Programming Environments Manual](https://www.nxp.com/docs/en/reference-manual/ALTIVECPOWERISAPEM.pdf),
  floating-point compare/reciprocal sections (vcmpeqfp and vrefp).
- Xbox default NJ mode is also reflected in
  [Xenia Canary thread initialization](https://raw.githubusercontent.com/xenia-canary/xenia-canary/canary_experimental/src/xenia/cpu/thread_state.cc)
  (VSCR00010000). An aligned instruction scan of the currently loaded XEX's
  executable segments found no standard mtvscr instruction. This is supporting
  evidence, not a hardware register capture of Skate2.

Host kept the four subnormal values nonzero, while 1/X overflowed to infinity.
The existing refinement then generated NaN. The native-style scalar fsel clamps
in cartesian() selected their upper limits on the resulting unordered values.
At actual zero, the explicit half-pi selection masked the invalid intermediate,
which is why the glitch disappeared at10827.

## Change and limits

Normalize subnormal operands to SIGN-PRESERVING zero in this existing polar
helper, so its vector arithmetic and zero-axis selection agree at that
boundary. The exponent test is exact: magnitude bits below00800000, not a
gameplay tolerance. Normal inputs, NaNs/infinities and graph/cvar/tuning values
are not rewritten. Both polar conversions use the same helper. No global
floating-point mode change, new animation algorithm, stance change or renderer
workaround. Broader host estimate/rounding parity remains outside this fix.

Regressions added alongside the existing implementation: all four captured
input bit patterns, both signs, both Y directions, both quick/slow paths;
smallest/largest subnormals, signed zeros, smallest normals and NaN/inf
preservation. Rustfmt syntax check passes. Rust tests are added but UNRUN.
An independent Python/CRT-fmaf calculation verifies the old reciprocal prefix
produces NaN exactly for the four captured nonzero subnormal inputs. This is
numeric evidence, not execution of the changed Rust code.

Next: run build-release.bat and play-lean-trace.bat, repeat high-speed
steering then coasting. Escape promptly if another snap occurs. Do not claim
all possible visual lean bugs are eliminated before that playtest.
