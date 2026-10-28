# Selected Actor hands/face session extraction

2026-09-20. New unregistered `actor_animation_hands_face_session.rs`, intended
`actor_construction::hands_face_session`. This is a narrow host composition of
existing reconstructed runtime calls, not newly claimed global scheduler parity.
No Cargo/shared edits. Source inventory: actor_animation_hands_face_corpus.rs,
actor_animation_hands_face_runtime.rs, actor_animation_body_session.rs.

`ActorConstruction::try_hands_face_session(lod, body_services, token_pool, selection)`
uses existing checked_body_animation to verify actual published Actor occurrence.
Actual left/right commands come from their retained playback owners; face commands,
playback manager and output refs come from the retained face owner. Existing startup
body services provide live use-jobs reads and actual job-manager ownership.
No duplicate playback, pose allocation, token evaluator or command implementation.

Caller supplies actual LOD and a HandsFaceSelection capability. Its local-player
and hand-clamp methods are invoked by the original runtime at source points, not
sampled eagerly. DispatchHandsFaceAnim825f87b8 reads local after beginning face ACS
and clearing initialized flags; clamp is conditional on localLOD5/nonlocalLOD2.
There is no local=true, lod0 or fabricated missing-cvar fallback in this adapter.

Exact inherited selected-owner order: Begin left, Begin right, runtime dispatch
(internally owns face Begin/End and token resets), End right, End left, Sync left,
Sync right, Sync face. Animation mutex is not held across outer End/Sync calls.
Any error preserves accepted earlier operation prefixes; no abort/rollback is
invented. Typed errors retain BodySessionError, selection source errors, Begin
BindPoseError and generic HandFaceRuntimeError.

Returns original left/right/face OwnedAnimationSqtBuffer references after successful
Sync. It does not require generations to advance or manufacture output at omitted
LOD; optional diagnostics can read these references separately. Corpus assertions
and unconditional output readback remain outside production.

Mount validation: replace selected corpus hand session composition with caller-owned
explicit selection capability and compare retained outputs/generations under existing
controlled lod0 case; also preserve the existing corpus LOD0 no-clamp assertion. The manager's
broader multi-Actor schedule and production LOD/local/clamp producers remain callers'
responsibility rather than being implicitly supplied by this session.
