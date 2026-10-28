# IdleDismount Motion Sustain 274

The original-data raw-Y replay completes frame 32 in Motion state 101
`IdleDismount`, then stops at frame 33 on
`PendingMaskedExpression { mask: Sustain, expression: 274 }`. Its generated
state declaration names expression 274. The existing Pre274 expression is
`c16 && !c17 && !c18`; this packet covers only its separate Sustain mask.

In the Skate 2 debug executable, Motion
`HCDelegate_MotionGraph_OnBoard::EvaluateExpression<2>` begins at
`0x8291e218`. Its jump table begins at `0x8291e254`. Entry 274 is at
`0x8291e69c` and contains big-endian target `0x829221c8`, the common return-one
arm. It samples no conditions. The decompiler likewise has no explicit
`case 0x112` and returns one in its default arm. The Rust Sustain evaluator now
accepts 274 as constant true; Pre274 and all condition implementations remain
unchanged. The existing `native_sustain_true_arms_do_not_sample_pre_conditions`
test includes 274 and uses a condition closure that panics on any read; an
extra focused test was removed after independent review as duplicate coverage.

The new raw-Y replay and workspace build are pending the root agent's shared
Cargo run. The evidence supports this one expression only; it does not yet
show a completed off-board animation or physical dismount.
