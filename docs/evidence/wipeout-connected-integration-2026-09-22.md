# Connected Wipeout integration, September 22

Root integrated the independently source/code-approved control, connected Update prefix, and impulse/recovery packets. Detailed native addresses, constants, cvar metadata and failure/publication order are preserved in:

- artifacts/wipeout-control-review-2026-09-22/
- artifacts/wipeout-connected-update-review-2026-09-22/README.md
- artifacts/wipeout-tail-review-2026-09-22/README.md

Actual selected pre-solve dispatch now calls try_update_pair_wipeout_until_prediction only for the same physical owner's WipeoutGround state after prediction-before-state. It executes actual control/drag/weights, steering and hook softness, skeleton roots/hooks/first-frame IK/ragdoll, then direct deck impulse and recovery/teleport request decisions. It returns typed PendingPrediction, never a completed Update3 token or a Ground fallback. Prediction, controller continuation and state-specific post-solve remain required.

No alternative body, COM, drive, attribute, or query owner was introduced. Control lives below physical_grounded so it can use existing private world owners; visibility was not widened. Ragdoll's previously deferred same-volume wrapper is now registered and called. New cvars269..277 preserve source registration order, defaults and null callbacks. Direct force math shares existing inverse-inertia tensor multiplication without replacing native store order. Actual reference traces remain required for numerical parity.

Connected-prefix focused tests passed in workspace-connected-startup-1 (4,474 total unit tests plus six docs), with fmt and strict Clippy. No actual original Wipeout Update tick was exercised by that checkpoint: existing original test proves only Ground.Exit/Wipeout.Enter. Recovery packet and subsequent production startup/slide changes are under a new gate. A review approval is not runtime acceptance.

Root lint-only integration changes preserve dt-first addition and native multiply-by-minus-one via documented expectations, move test modules after implementation items, and remove an unused error variant. Host numeric estimate/trigonometric limitations are unchanged.

Subsequent integration (new gate pending): reviewed prediction and post-physics
packets now complete their selected state calls. See artifacts/wipeout-prediction-review-2026-09-22/README.md
and artifacts/wipeout-post-review-2026-09-22/README.md. Same queue selects actual
retained Wipeout future, preserving optional constructor fields and unknown W.
Post-physics reads actual history26d0, parts23 then1, and stores over then slow.
Cvar278 SlowSpeed is linked with source default1 and null callback.

Root issues private-owner WipeoutUpdateComplete only after prediction submission.
Shared controller and PrePhysics consume that same owner. Ground/Air retain
completed-phase validation; Update5ReckoningRead::RetainedWipeout carries the
successful Wipeout token and validates owner/current state before reading the
constructed retained Reckoning. Native82e04f40/44/48 always reads that retained
object after trajectory output and before state-changer output; Wipeout does
not call Reckoning.Update. No synthetic phase, reset or update is used.
Independent code/source review approved this integration. Existing test-only
manager wrapper still uses completed-phase validation for isolated Ground probes.

Post dispatch is after Skeleton.Post and before common PostWipeout. The post
module lives under physical_grounded::wipeout for existing private world access;
no world-field visibility widening. Root corrected an additional old predictor
queue fixture initializer to PredictionTarget::Onboard(0) found by compilation.
Actual independent original Wipeout runtime probe is in development; neither
review nor compilation establishes a completed live bail or reference parity.
