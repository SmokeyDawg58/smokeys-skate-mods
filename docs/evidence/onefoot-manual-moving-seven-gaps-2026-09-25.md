# Seven dispatch gaps: one-foot and moving manual, September 25

Source checked solo in IDA; no build or runtime executed here.

- Motion Pre4 8290b2fc: c15 (WillExpire). Pre116 8290dd18: c196 (HasIntent).
- Motion Pre398 8291949c: c207 || (!c89 && !c85). Native ComVelCompare first, then two filtered-state exclusions. Shared true branch829122f8; false829195ac. All three Sustain cases independently map to native true default829221c8.
- Action Sustain84 82908ca8: c38 || c39. Pre84 remains distinct and unchanged.
- Motion conditions238/239 are shared HasAGIntent objects. Activation82cc1748 resolves IActionGraph, queries presence, then returns true for NOP or evaluates comparison. Added shared payload dispatch using the existing Action intent owner/check_action lookup, after validating Motion context. Extracted the existing lookup body so Action and Motion reuse it; did not pass a Motion controller through an Action context validator. Missing intent remains false, including NOP; present zero remains present. Existing comparison semantics retained.
- Action update828ed5d8 OneFootRight41, shared authored aliases42 OneFootLeft and44 TransToOffBoardAir: OnBoard6 parent then instance19/20 CreateMGIntentFromAGIntent and21 CreateConstMGIntent. Exact authored inherited arrays reuse existing callbacks.
- Motion update828f5138 Moving1352: Holding1273 (828f4490) parent, then35 CreateAttribute,36/37 AttachIntent,38 PlayAnimation. Holding invokes Manual1027,32 SetScoreAugmentation and33/34 native empty. Exact ancestry flattened once, including Brake1335 slots.

Validation: static authored ancestry and unique dispatch/behavior audit PASS for Action41/42/44 and Motion1352. Rustfmt parse-only PASS on eight touched Rust files. Extended Action ancestry and Motion Sustain regression coverage; added expression398 short-circuit cases. Tests are not executed. Quiet mode unchanged. Run build-release.bat then play-quiet.bat.
