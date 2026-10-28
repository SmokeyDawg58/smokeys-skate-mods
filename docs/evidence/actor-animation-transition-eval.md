# Retained TransitionBlend pose consumers

Confidence: high. Fresh IDA raw/decompile review of Eval82d6e198,
EvalBlendPose82d78f30 and GetBlend82d6e090, 2026-09-19. New unregistered
`actor_animation_transition_eval.rs`; no Cargo execution by author.

Mount as child `actor_animation_construction::transition_eval`. Root changes
only the Body arms of `initialization::body_dispatch::eval_reference` and
`hands_face_dispatch::eval_blend_reference` to call `eval_body_reference` and
`eval_body_blend_pose` respectively. Each Tree leaf invokes the existing actual
retained tree pool. TransitionBlend resolves the producer's retained store
membership/runtime, recursively preserving the actual FROM and TO references.

## Body commands

Eval calls GetBlend first (e1b4), FROM.Eval (e1d4), TO.Eval (e1f8); both child
calls receive the caller's unchanged doEval. No PhaseBlend endpoint culling.
Length is loaded and checked only AFTER both calls: e2104199001c accepts
ordered length>0, so zero/negative/NaN assert. Host typed refusal keeps commands
and leaf evaluated/old-time writes completed before that assertion.

When doEval and both child results are true, e250 reads flags. Bit80000000
chooses ACS ChannelBlend instead of ordinary Blend; e268 extracts bit40000000
as its boolean parameter. Reuse `AnimationEvalCommands::blend` or the existing
false-policy `channel_blend`, preserving real buffered acceptance/numeric rules.
For bit40000000=true in channel mode, the adapter explicitly refuses at this
command boundary. Root authorized that boundary because the existing interface
cannot carry this boolean; the selected BlendAnim producer clears both bits.
No ordinary BlendAnim branch is skipped, and flag40000000 alone is ignored as
the source does. No refusal occurs for a flag whose command is not reached.

After successful optional command emission, evaluated=FROM||TO commits at e29c.
Failed child, assertion or command leaves that transition field unchanged.
The child tree Eval implementations retain their own source partial writes.

## Hand/face token streams

Unlike body Eval, length>0 asserts before GetBlend and before either child
(78f644199001c). 78f9c40980060 tests !LT(coefficient,1), so TO-only includes
unordered. Otherwise FROM runs first; ordered coefficient>0 then constructs
the opposite pool buffer, evaluates TO and uses existing token-pool Blend.
78fc4/78fc8 compute (poolIndex-1)&1, retained exactly using wrapping subtraction.
There is no evaluated-field write in this method. Token/error prefixes remain
in the real BlendPoseTokenPool.

## Shared weight and verification

Calls runtime_audit's `try_transition_blend_weight(index)` from the mounted
transition_time module. Fresh raw confirms fdivs time/length, fsel lower/upper
clamps, true-LT length<0.05 linear return, then separately rounded square/cube/
twice-cube and fused square*3-twice-cube (e0f8 fmsubs). No positive-length check
was inserted into this shared helper: the two caller assertion timings differ.
Store/weight errors map to existing typed MissingOwner/Missing errors; neither
is converted into destination-only success.

Root validation should cover original retained FROM/TO pose trees at weight0,
interior and1, command failure preserving old transition evaluated, culled
body traversal advancing old-time state, and hand/face stream endpoint choice.
Existing animation tree/command/token tests exercise the reused primitives.
No synthetic Actor constructor or duplicate pose allocator was added solely
for tests. This draft requires the mounted original-owner composition gate.
