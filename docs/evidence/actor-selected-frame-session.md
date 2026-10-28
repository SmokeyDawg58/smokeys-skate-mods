# Selected Actor prefix composition

2026-09-20. Confidence high for extraction of the existing selected callback
schedule. New unregistered `actor_selected_frame_session.rs`, intended child
`actor_construction::selected_frame_session`. Exact file paths were checked
absent before creation. This is a selected host driver, not recovery of the
whole native multi-Actor manager or all Actor callbacks.

## Reused implementation and source identities

The production sequence comes from `actor_action_runtime_corpus.rs`; all fixture
assertions, repeat-frame proof, constructor auxiliary creation and default settings
construction are excluded. Existing implementations remain the sole producers:

1. GenerateActionGraphIntents825fda28 through retained input services and numeric
   capability; Actor Phase1 prefix825f41a8 through retained cvars.
2. Generated Action Update; Motion attribute clear (native call825f43a0);
   generated Motion Update using the caller's actual ForcePhysicsWriter.
3. Animation Phase2 with explicit caller wait policy; Phase3 with retained body
   services; body ACS synchronization (UpdatePhysics825ffea8).
4. Existing `try_hands_face_session` using explicit caller LOD, retained token
   pool and live lazy HandsFaceSelection. Its source packet is
   `animation-hands-face-session-live.md`; the
   implementation references DispatchHandsFaceAnim825f87b8 and existing ACS
   Begin/End/Sync producers. No local-player/clamp query is sampled early here.
5. SetUpPhysics825ff6c0 (the SkaterAnim virtual+88 is the existing blr825187c0,
   not another body Sync); selected Actor.Update825f10f0..1140 after physics
   publication, including the actual one-shot flag clear and current explicit
   AI/nonempty Chasee boundaries.

Existing supporting packets include `actor-action-motion-intent-bridge.md`,
`actor-animation-frame-phase.md`, `actor-animation-physics-publication.md` and
`actor-update-selected-input-flags.md`. This extraction adds no arithmetic,
conditions, native field mapping or duplicate callback implementation.

## Retained capability and scheduling contract

SelectedActorFrameRequest carries dt, explicit actual LOD, the phase2 wait
policy, borrowed cvars and borrowed settings. No LOD0/local-player=true or
constructor settings are assumed. SelectedActorFrameServices borrows the
existing body/input owners, actual Motion writer, BlendPoseTokenPool, live
HandsFaceSelection and ReckoningNumeric. All are caller retained; this function
does not allocate a second owner or reset a token generation between ticks.

The returned HandsFaceSessionOutputs are the existing original initialized SQT
buffer owners. They do not promise a generation advance at every LOD. Body and
hand/face writes can already have committed when later setup fails; distinct
typed boxed errors preserve the exact failed stage and underlying error type.
Re-running the whole prefix after failure is not a transactional retry.

The caller must have mounted the Actor and constructed its physics auxiliaries.
The following actual physical Update1 performs ProcessBoundAnimationPhysicsInput
exactly once. This helper deliberately ends before that consumer. No second
Actor tick, appended Motion evaluation, physics solve, post-solve callbacks,
quality selection heuristic, or whole-manager completion token is introduced.
The caller owns the unimplemented manager policies and excluded Actor callbacks.

## Validation

Standalone rustfmt parsed the new Rust file. Author ran no Cargo and edited no
registered files. Root integration should retain existing original-owner corpus
observations, replace the corresponding production calls, and run the original
asset path plus repository gates. The wrapper has no new numeric/state kernel
requiring mirrored synthetic tests. Independent review remains required.
