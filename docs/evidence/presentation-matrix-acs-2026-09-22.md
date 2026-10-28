# Selected final-matrix ACS pass

Scope: first connected dependency of final presentation palette publication.
The existing retained presentation replay and interpolated allocations remain
the source and destination. This packet extends the existing buffered ACS
collector, work object, job manager, scheduler chain and Sync implementation.
It does not infer model membership or construct an independent pose owner.

## Native evidence

- `Sk8::SkaterPresentation::UpdateBodies` 0x827de988: after the first body
  command-set Sync, MoveRoot 0x827deb24 and Expressions 0x827deb2c run. The
  second command set is model-gated. Its selected stream is BeginCommands
  0x827deb70, SetSQT 0x827deb7c, SetMatrix 0x827deb8c, PoseWithMatMult
  0x827deb94, ExtractMatrix 0x827deba0, EndCommands 0x827deba8. EndSet/Sync
  occur at 0x827debc8/d0. The later saved-hips slot-zero overwrite is outside
  this ACS packet and belongs to the caller, not the worker.
- `AnimCmdSys::AnimCommandSystem::SetMatrix` 0x82a2f428 validates current
  command state and pointer then calls `SetData` 0x82a2c828 with MATRIXID.
  GetNextLoc 0x82a2c84c precedes usage accounting. Matrix increments
  NumSetMatrixCmds at 0x82a2c87c, but the common capacity condition reads
  NumSetReplaceSQTCmds and its maximum at 0x82a2c8e0/e8. Do not invent a
  different matrix-count capacity check. The selected stream emits one of
  each input, after the existing SQT collector's admission.
- `PoseWithMatMult` command 0x82a282d8 increments other-matrix usage before
  capacity checking. The top matrix and preceding SQT are consumed; a dynamic
  matrix output reuses its slot. A fixed matrix would allocate another slot,
  which is outside this selected external-input stream.
- `PoseEngine::EnQueueCmd` 0x82a31520 case 5 lazily allocates work storage and
  the separate pose-matmul job pool at 0x82a31a18..31c34. Pool count is 14,
  dependencies are the existing blend-pool count plus one (19 for admitted
  presentation), work units are MaxPoseOps / 6 + 1. Existing category2
  affinity and original manager identity are retained. This is distinct from
  the existing ordinary Pose job pool.
- `ACSposeMatMult_job` 0x82a23e78 reads count u16, bone count u8, trajectory
  node u8, and operation rows [SQT*, inverse-bind*, matrix-output*, extra*].
  It calls `AnimCore::Anim_Tools::PoseWithMatMult` 0x82a23528, then copies to
  optional external extraction only after the full calculation, at
  0x82a23ed0..3ee4. Therefore a host unsupported hierarchy failure does not
  publish a partial external pose.
- `PoseWithMatMult` local and hierarchy arithmetic matches the existing
  `animation.rs` helpers: rounded quaternion products before doubles, fused
  diagonal expressions, scales, parent traversal with trajectory-parent
  exclusion. Final bind composition 0x82a23a5c..3afc uses X product, Y fused
  add, Z fused add; translation starts with X fused into global translation.
  Input inverse-bind W coefficients are ignored. The existing generic
  four-term `multiply_matrices` / `apply_inverse_bind` is deliberately not
  used. Native internal final-axis store order is Z,W,X,Y; the selected host
  internal temporary has no external alias, and extraction follows full work.

## Ownership and limits

The supplied immutable `Arc<[AnimationMatrix]>` is the model inverse-bind
allocation retained through command execution. SetData copies it when the job
runs. External SQT is likewise sampled by the existing live SetData reader.
The typed matrix destination is the same `OwnedAnimationPoseBuffer` storage,
made unreadable on Extract emission and readable after the existing root wait.
No neutral pose, identity inverse-bind fallback, or corpus response exists in
the production path. Existing single-stream ACS admission remains explicit.

This collector admits exactly one dynamic matrix on the initialized full
presentation descriptor. It does not generalize native usage/stack counters,
fixed-matrix allocation, multiple matrix commands, or native malformed-hierarchy
trap behavior. Raw constructor 0x82a32348/354 computes
`(cap[4] + 1) >> 1`; Data 0x82a302d4 multiplies this by the character count.
Pool dependency capacity 19 is limited to the admitted existing blend pool's
18 jobs, matching native saved `mMaxDepJobs + 1` at 0x82a31b20/2c.

The next packet must supply actual model inverse-bind ownership, call this
stream on corrected replay, overwrite the saved hips matrix after Sync, then
reuse actual model remapping and GPU row publication. Current code does not
claim complete UpdateBodies, frame-map/model admission, cloth, or rendering.

Numerics retain the existing host signed-zero/exceptional-value boundary of
the local diagonal negative fused expression; no unconditional Xenon bit-parity
claim is made. Confidence: high for recovered graph/owner order; actual mounted
execution awaits root integration and the supplied-model caller.

## Validation

Focused drafted tests cover late source mutation before and after SetData,
trajectory-parent exclusion, synchronization visibility, fused cancellation,
ignored inverse-bind W, and invalid late hierarchy leaving external extraction
unwritten. Shared patch is generated against live files, rustfmt-formatted,
and `git apply --check` passes. Root owns all Cargo/original-data execution.
Independent reviewer source/code approval received for this bounded ACS scope;
no reviewer Cargo run. The selected entry has a temporary dead-code expectation
until the following real supplied-model caller is integrated.
