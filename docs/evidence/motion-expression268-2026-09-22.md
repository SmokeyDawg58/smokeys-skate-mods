# Motion expression268 ? 2026-09-22

Source independently inspected by reviewer and root. Pre829119e8 and
Sustain829204b0 both return negated condition402. Native caches the original
PhysFilteredState result at value64/mask00040000 (presence30), then negates
for the expression return. Condition402 is existing PhysFilteredState with
attribute2430 state teleport; reuse the actual filtered physical state owner.
The occurrence is reached after Action Wipeout23 on controlled bail frame2.

Two narrow expression arms delegate to the existing cached condition adapter.
No new condition or forced bail value. Fresh root disassembly retained in
artifacts/motion-expression268-2026-09-22-raw.json. Draft registration and
original-data execution pending. Confidence high for source order/identity.

Registered validation: fmt and strict clippy-motion268-player-sample-1 pass;
workspace 4,639 unit plus six doc tests pass. Original-data 18/20 (5.00s),
bail frame 2 now PendingExpression269; neutral frame 2 BlendSpace Eval.
