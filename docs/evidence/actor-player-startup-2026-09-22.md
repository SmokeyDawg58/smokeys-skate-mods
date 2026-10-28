# Same-player animation consumer and fresh selected startup

Scope: connect the previous same-loader ApplyPlayerAttributes producer to the
existing Actor animation selection/start and constructor bind-pose handoff.
The actual repeated-frame original corpus now starts fresh; isolated recognizer
and body-session diagnostics remain separate. No second input, physics,
animation, job, player-settings or player-loader state machine is introduced.
Confidence: high for selected source/ownership/order; execution awaits root.

## Actual owned inputs

`ActorAnimationPlayerServices` borrows the same PlayerAttributeServices used by
PlayerLoaderSlot.try_apply_loaded_attributes. Recipe/full64 character ID,
stance, sex, gestures and style read that retained PlayersBackendSettings.
Local index, network and ghost read the same SkaterManagerLocalIdentity and its
LocalBackendOwner/GameInfo. No default ID/regular=true/skater0/ghostfalse replies
are hidden inside an adapter.

Native consumer references: Actor animation selection825feccc..feefc;
GetCACSettings825f0770; GetPlayerCharacter8255b0b8 full64; GetPlayerStyle8255b5d0;
GetPlayerGestures8255b6a0. Fresh raw6f8..714 copies the four stored gesture words
in order. Existing source-backed PlayersData.Reset82561e68 owns regular0,
male1, style0 and GestureSet.Reset82561d70 results[0,1,2,3]. Marquee Apply changes
character/stance/hat, preserving the other fields. A custom consumer can read
these real owned values; this does not complete the missing CAC profile loader.
The existing full64 six-pro/CAC-sentinel branch stays unchanged and lazy.

Global FaceAnimationManagers, SimulationAnimationStartup and the original
AnimationJobManager are reused. Cvars are sampled at existing caller sites.
Sprint keeps capture-boolean then read semantics. `try_bind` resolves the actual
optional job Arc BEFORE entering buffered animation locks, as existing
ActorBodyAnimationServices does. It never creates a substitute manager.
Missing jobs remain an error only when require_job_manager is reached.

The pre-entry binding is necessary: GetBindPose acquires buffered lock at
actor_animation_bind_pose.rs504, and evaluation calls require_job_manager while
holding it. SimulationAnimationStartup.retained_job_manager itself locks that
buffered owner. Resolving the capability inside that callback would deadlock.
Independent reviewer verified both the hazard and the final pre-entry fix.

## Production constructor composition

`ActorConstruction::try_complete_player_animation` invokes the existing prefix,
Vault/sprint suffix, loaded-face completion, publication, initialization,
animation physics-input construction and constructor bind-pose handoff. It
operates on the caller's retained Actor; errors leave completed Actor prefixes
present. Existing global startup/faces/source are arguments, not reconstructed
inside this method.

`ActorPhysicalAnimationInputs` owns the same input/body services, database
hierarchy, AnimationPhysicsRef and temporary OwnedAnimationPoseBuffer. Its
fallible handoff checks hierarchy identity and copies the original pose and
parameter snapshots into existing PhysicalPlayerAssemblyInputs. Native
constructor sites825fef00..ff16c remain composed from existing components.
No body-session diagnostic dispatch or synchronization probe is called.

The old OriginalPhysicalAnimationInputs corpus name becomes an alias of this
production handoff type, so isolated diagnostics keep their existing behavior
without introducing a second representation. The temporary pose still lives
through the selected physical constructor scope; full host lifetime extraction
remains future application bootstrap work.

## Connected fresh original-data path

Only repeated_frames selects actor_player_startup_corpus::construct. It:

1. Uses the actual local manager already constructed/reset by world startup.
2. Constructs one backend settings owner and one existing PlayerLoaderSlot.
   The native GetDefaultMarquee82cfdbe8 ID is an explicit first-test selection,
   not a claimed profile-absence or CAC-validity result. Cached Vault layout text
   selects the actual recipe/face name; actual same-loader Apply establishes
   settings before the animation adapter reads them.
3. Retains loader phase Registering. Its callback asset identity is a fixture
   token, and no fake registration-completion/presentation publication occurs.
4. Reuses existing original animation startup and actual face manager load,
   creates original GameplayInputUpdateManager and GameInputCommandManager,
   resets input once, constructs Actor/listener/graph prefix, and calls the new
   production animation composition. No DoCommand, recognizer update, Fill,
   Action input mutation, or diagnostic body frame precedes gameplay.
5. Passes the SAME settings to physical hat selection and the three-frame tick.
   The former second PlayersBackendSettings construction in three_frame is gone.

Existing original terrain, explicit parked-grind policy and numerical boundary
remain unchanged. Authored spawn, simulation flag1, callback test identity and
the existing corpus animation thread identity1 remain explicit test policies.
The public mounted host facade, actual presentation registration/spawn command,
native profile owner and actual host-thread adapter are not claimed complete.

## Review and checks

Three focused tests cover same-owner post-binding writes, full64 IDs/CAC sentinel,
native owned custom defaults, local reset -1->0, live ghost changes, unchanged
manager identities, live constructor cvars, and missing job capability.
The existing original repeated-frame test now exercises the fresh integration.

Five new Rust files formatted; artifacts/actor-player-startup-2026-09-22.patch
passes git apply --check. Independent source/code review approved the connected
packet and the final job-lock correction. No registered edits or agent/reviewer
Cargo. Root owns compile/Clippy/unit/original-corpus validation and any newly
exposed native producer dependencies.
