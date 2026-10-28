# ACS NBlend and BlendSpace evaluation

Registered after independent root source/code review of the animation agent's
packet. Evidence and draft snapshots are in
`artifacts/acs-nblend-attributes-independent-2026-09-22` and
`artifacts/acs-nblend-implementation-2026-09-22`. Do not rerun the draft generator
over registered changes. Root freshly read NBlend82a26348, BlendSpace Eval82d772f0,
EvalBlendPose82d6ecd8, raw worker82a2b8ec..ba54 and constructor82a326fc..32778.

The existing BufferedBodyCommands owns actual NBlend list quota, OtherSQT usage
and latched error. Setup uses the supplied original descriptor, including the
distinct rounded average-adjusted OtherSQT limit. BeginCommands clears error;
EndCommandSet clears usage after submission; Sync clears list quota after wait.
The list equality boundary increments its count before a checked missing-storage
failure. Existing command/error rewind behavior remains a documented host boundary.

Commands retain copied coefficients and ordered source IDs. The worker reads
each bone during each ascending source pass, preserving writes visible through
aliased later operands. It uses fused accumulation without hemisphere correction
and normalizes quaternions only after every source. Existing host horizontal-dot
and reciprocal-square-root numerical policy is reused and is not instruction-level
exceptional-value parity. Eight focused tests distinguish buffer aliasing, fused
rounding, quaternion cancellation, partial writes, coefficient validation and
the separate reset lifetimes.

BlendSpace Eval rejects trimmed nodes first, evaluates actual active-volume
children with doEval=true, ignores their boolean returns, emits NBlend, then reads
the live print cvar. EvalBlendPose visits children, emits all coefficients across
three streams, then each stream's blend command. Debug overlay execution remains
explicitly unsupported when enabled; the existing global debug-line debt remains
recorded. No binary-blend substitute or new scheduling owner is introduced.

Initial fmt passed. Initial strict check found only documentation, method closure,
explicit default, diagnostic payload and operand-order lint issues; root corrected
them while retaining native addition order. Second fmt and strict clippy pass
(27.36s); full workspace and original-data validation are running. Confidence high
for admitted source order and owner identity; no whole-frame/playable claim.

Completed validation: workspace-nblend-2 passed 4,647 unit plus six doc tests.
Original-nblend-2 18/20 (5.68s): neutral frame 2 completes Actor animation
and now reaches pre-solve SystemLogic UnsupportedCurrentState(200).
Bail remains PendingExpression269.
