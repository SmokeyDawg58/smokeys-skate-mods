# Retained body tree Eval and buffered SQT execution

Source review: 2026-09-12, Andale executable/PDB through IDA. This closes the
nonnull branch of the existing body command producer for trees that fit the
selected original buffer and job capacities. It is not a claim of a complete
Actor animation frame scheduler or bit-identical animated VBR decoding.

## Producer and retained state

`Animatable::EvalCurrAnim82d6e658` samples its live blend-cull threshold, builds
`tAnimationEvalParams` (ACS+0, FetchSystem+4, UpdateOldTime+8=true,
BlendFrames+C=2, threshold+10), begins commands at82d6e6a4, calls the actual
current node (or the bind node only when NULL) at82d6e6e4, then emits SQTtoLocal,
local ExtractMatrix, LocaltoGlobal, global ExtractMatrix, EndCommands through
82d6e71c. The Eval boolean is not used to substitute another pose.

`actor_animation_body_dispatch.rs` now selects the actual retained body/channel
reference and dispatches it into the same named ACS, scheduler and local/global
buffer allocations used by the prior bind-pose branch. Sync remains separate.

* Clip Eval82d6ef78 requires the real header even when culled. It scales old and
  current times by base-speed times speed, clamps with FSEL, repairs previous
  time only when no loops and previous>current, and calls Fetch82d6f088 only for
  doEval and not trimmed. UpdateOldTime stores old_time=time and loops=0 at
  82d6f0a0/a4 even when culled. The evaluated bit is stored after acceptance.
* Pose Eval825852d0 requires its original header, resolves the GUID and emits
  FetchPose only when requested; evaluated=doEval.
* PhaseBlend82d6e9d8 caches the threshold, uses FSEL clamp and the original fused
  endpoint smoothing polynomial (82d6ea34..ead4). Raw BLT82d6eaf4 and
  BGT82d6eb60 gate children with ordered coefficient<1 and coefficient>0;
  unordered gates both off. Both true outputs emit Blend, then store their OR.
* AddSQT Eval82588308 evaluates the motion operand first and skips the other
  operand entirely if it returns false. Its ACS calls are AddSQT(true,true) or
  AddSQT(true,false). PDB names these parameters ChannelFromFirstOperand and
  LeftToRight. The buffered producer82a2d6d4..6fc reverses both stack operands
  and inverts ChannelFromFirstOperand for LeftToRight=false; this also changes
  the reused output buffer. This correction is present in command acceptance.
* Mirror82c92818 always visits its child, then calls ACS.Mirror only if requested
  and successful. It does not store its own evaluated bit.
* ChannelBlend82d6f178 visits base first and channel second, with ordered
  doEval && coefficient>0 for the latter. It emits ChannelBlend(coef,false)
  only when both produced output. It returns their OR, without a separate bit.

The root integrator independently reviewed PhaseBlend raw scalar/FMA/gates,
AddSQT's motion-first traversal and buffered operand reversal, and the
TrajectoryCalculation composition order. Implementation and review were split
between agents; test success alone was not used as source proof.

## Commands, storage and jobs

`actor_animation_clip_fetch.rs` retains the actual database occurrence, header
record, GUID resolution through the same manager, destination hierarchy, and
per-part current/previous frame descriptors. Fetch timing is encoded and then
decoded through the existing source timing implementation. Clip plans are
consumed by the real FetchVbr work entry, not by a constant-pose substitute.

Blend82a25dbc..de4 packs floor(coef*8191)&0x1fff. SQTProcessEngine::EnQueueCmd
82a2b228..24c decodes this with exact float constant39000400. Frame blending
uses the 16-bit descriptor coefficient multiplied by37800080. These packed
values, rather than the unquantized tree coefficient, drive execution.

Buffered GetNextLoc82a24c28 increments the dynamic SQT allocation monotonically;
the immediate Blend branch's release must not be applied to buffered mode.
Only exhaustion enters the separate ReuseBuffer path82a24d9c onward. The ACS
constructor82a321f4/21fc sums AverageFetchCommands+AverageSetReplaceCommands,
clamps to at least2, then constructs Data at82a32384. For the actual one-character
setup this gives20 dynamic SQTs. This capacity is retained in BufferedBodyCommands.

Within the selected no-reuse, single-batch capacity, fetches fill distinct SQT
allocations and ordered SQT operations reuse the source-selected output. A real
FetchVbr job precedes the real optional SqtBlend job, then SqtToLocal and
LocalToGlobal jobs. Those jobs use the existing fetch/blend/local/global pools,
same null root, backend and separate Sync. The previous broad rejection of all
nonnull trees is removed. Capacity exhaustion/ReuseBuffer and multiple full
fetch/SQT batches remain explicit unsupported branches.

## Numerical graph and boundaries

ACSsqtBlend_job82a2b528 case0 begins82a2b600. Scale/translation use fused
(second-first)*coef+first. Quaternion dot>0 chooses that branch; otherwise it
uses negative fused(sum*coef-first), including dot==0. Quaternion normalization
has two reciprocal-square-root refinements. VNMSUB is implemented as negation
after the fused result, preserving the source exact-zero sign distinction.
Host nested dot3/dot4 and sqrt-reciprocal estimate seeds remain numerical
boundaries rather than asserted Xenon bit parity.

Case1 weights the coefficient by the selected channel translation.W (false
selects operandB), then ordered upper/lower comparisons clamp to1/0 before
the same blend graph. Case8 multiplies scales, rotates operandA translation by
operandB quaternion, adds operandB translation, composes B quaternion with A,
and retains W from the encoded ChannelFromFirstOperand selection.

TrajectoryCalculation82d6e2c8 uses XYZ sign-bit XOR for inverse previous
quaternion. Translation is rotate(invPrevious, rotate(currentQ,deltaT)+currentT
-previousT). Rotation is compose(compose(deltaQ,currentQ),invPrevious).
Translation.W is explicitly channel weight or1. The actual previous bone0
descriptor and loop delta are retained and used.

Case3 mirrors ordinary bones by multiplying quaternionXY and translationZ by
-1. For trajectoryUse1, root or parent0 bones instead multiply quaternionYZ and
translationX by-1; parent0 first composes with [0,1,0,0]. Each mirror-table pair
is visited once, swapped, and the partner receives the ordinary reflection.
The source uses multiplication for these reflections; it uses XOR only for
the trajectory inverse quaternion.

Animated VBR sample production originally used the existing formats decoder.
Root review found its per-call cosine IDCT/quantization and missing source +0.5
were not valid parity evidence, and is correcting that separately. Until that
review/correction is complete, original animated tests prove owner/command/job
integration and changing retained output, not numeric parity. The packed-scale
W expansion in Vector_ExtractPackedNVBR also remains under explicit review;
the local matrix consumer does not consume scale.W.

## Validation and next caller

Focused tests cover failed command acceptance before evaluated-bit publication,
culled Clip old-time/loop stores, AddSQT motion-first culling, PhaseBlend curves,
orthogonal-quaternion negative selection, trajectory previous-frame/channel W,
mirror-pair order, and the optional SQT job dependency barrier.

The dedicated original body command-engine fixture uses its own original
startup, first validates the unchanged bind path, then constructs the actual
B_GSTR_SKY_BOTH tree and advances/evaluates/synchronizes two frames into the
same local/global allocations. It does not change the shared physical Actor
constructor fixture. Root owns Cargo and corpus execution and records results.

At this review point `try_add_animation_time` still has only the isolated
PlayAnim corpus caller; its PhaseBlend/Clip/Channel arithmetic was raw-reviewed.
The production body session calls the new Eval dispatcher, but a complete
source Actor phase1/phase2 driver, transition completion/removal, and the live
attribute updates that select blend values remain separate integration work.
