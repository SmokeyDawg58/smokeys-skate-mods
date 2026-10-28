# Input to Actor: next retained producer closure

Read-only source/repository audit, 2026-09-08. No code or Cargo. Confidence exact for directly checked callsites and ownership instructions; high for decompiled flow below. This is an implementation dependency map, not a claim that full gesture arithmetic or Actor execution is recovered.

## Critical ownership correction

The gameplay Actor does NOT consume the SIM group's InputPad. There are three distinct layers:

1. Device manager publishes raw count plus24 converted values into its current half.
2. SIM/PRES caches independently collapse samples into their own four pads; SIM LastInput also gates AppMan PreUpdate.
3. PlayerUI sends simulation commands carrying a DIRECT device-manager sample; GameInputManager::PlayerInput owns another embedded InputPad updated by its registered GameInputSK8 command receiver. Actor and gesture recognition consume this gameplay owner.

GetRawInputCacheLine826dfc90 reads InputManager's actual device manager, atomically samples active index+324, then copies0x64 bytes of the selected device/half. It does not read a group cache or group pad. Therefore copying the SIM pad's button history into Actor, or feeding Actor the same SIM pad reference, bypasses the native command path.

## Smallest next usable closure: published sample to gameplay pad

PlayerUI::Update828af9f8 takes canSimulate and hasFocus. If canSimulate, it handles focus change first: retain new focus; on gaining focus emit command type2, receiver playerIndex+10, through actual proxy DoCommand. Intervening session-marker and optional network-constraint/teleport work precedes ordinary input sampling and cannot silently become no-op native claims.

It constructs PlayerInputData, samples actual Simulation network status for allowSensor, initializes payload focus true, then calls GetRawInputCacheLine with actual controller index. When caller hasFocus is false it clears all24 payload button values. It changes payload focus to false only when actual SIM pause flag is active and proxy next-generation frame>1. It emits type1 to receiver playerIndex+10 (SetData call828afcc4) and calls actual proxy DoCommand. Later reset-player/spawner effects precede proxy Tick. If canSimulate false and hasFocus false, only retained UI focus becomes false. Command serialization/queue/frame dispatch is a real missing collaborator; no generic host cadence belongs here.

GameInputSK8 constructor826df1c8 retains supplied embedded pad, shared actual map, and index; four global arguments initialize+0, DPD flag1, focusfalse, four bounced-button bytes zero; it registers its ICommandReceiver subobject with actual Simulation CommandManager. This is not a free-standing mapped snapshot.

OnCommand826df568: type1 unpacks actual PlayerInputData, stores received focus, then processes channels12,13,14,15 in that order. Each retained bounce byte startsfalse; whilefalse it becomes true only when sampled channel equals0 (both signedzero qualify, NaN does not). Until true, that channel is replaced with+0; once true it passes subsequent values. Then it calls embedded Pad.Update with literal24 at826df710/71c, irrespective of incoming cache-line count. Thus even disconnected manager count0 is not equivalent to invoking gameplay Pad.Update with0. Source type2 clears allfour bounce bytes then sets focus true; it does not reset the pad. Unknown command type asserts. Multiple-inheritance decompiler expressions incorrectly label some receiver-relative bytes as mPad/mMap; use typed fields and constructor provenance, not those aliases.

Next implementation should construct one selected PlayerInput/GameInputSK8 owner, retain its SAME existing InputPad, actual loaded map and four globals, and implement this checked command receiver plus an explicit typed PlayerInputData command boundary. Reuse input_pad.rs and input_map.rs. Device-manager published sample production and real proxy scheduling must be supplied by actual owners or remain explicitly scoped trace inputs. The selected cache owner alone is not sufficient. Recover exact PlayerInputData Pack/Unpack82a8c070/82a8c330 and Command GetData826df340 before claiming byte-stream parity; typed payload consumption can be separately scoped.

## Recognizer ownership and delivery

GameInputManager constructor826e3308 initializes its player vector, actual GameInputBroadcaster (queue6), two known-pattern lists, then Load826e3260. Load orders right-stick files skater.pat, skater90.pat, skaterN90.pat, skater_air.pat, skater_fingerflip.pat, then left-stick skaterls.pat and skaterstep.pat. LoadDataFile826e3180 and actual .pat parsing/ownership remain missing in Rust.

Reset826e2c20 destroys previous PlayerInput owners, then constructs indices0..8; after each insertion it adds known-pattern matchers for left group0 then right group1. PlayerInput ctor826e2440 initializes two empty matcher lists, its embedded InputPad, null input pointer, flags=(index==8)+1, then obtains map from actual AppMan SIM input0. It allocates two history containers with201 vector slots and initial begin/end at storage begin, size0, then constructs GameInputSK8 around its own pad. Do not infer zero-valued historical samples from the uninitialized vector fill temporary, or expose full capacity as actual history. AddPatterns826e2b18 binds each matcher to that player's same history container and actual known patterns; constructor/matcher details need separate raw recovery.

GameInputManager::Update826e2578 requires debug overlay, iterates actual player entries, skips null pad/count0 or flags&updateMask==0, calls UpdateRecognizer then GenerateGameInputEvents, and finally flushes broadcaster queue. SimUpdate1 calls it8289f4b8 before later animation/physics stages.

UpdateRecognizer826e2008 maps IDs78 then77, negates Y, then calls right UpdateInput at826e2090 with type1. It maps75 then74, negates Y, then calls left UpdateInput at826e20f0 with type0. Four globals/map/pad are sampled at each map call. Later enabled display branches perform additional attribute/map/overlay reads; do not hoist those or assert they are always absent.

PlayerInput::UpdateInput826e18d8 applies actual kInputHistoryDeadZone independently to absolute X/Y, inserts newest vector at the front of the same retained ring (evict tail on collision), then makes TWO ordered matcher passes. First pass IsHoldingLastMatchedPattern may enqueue held-pattern messages. Second pass FindMatch starts an explicit empty match record, performs matching and optional display branches, and enqueues match messages with actual input pointer, normalized speed/error/time and player index. Held messages therefore precede newly found matches for that stick. Exact VMX deadzone predicates, ring layout, matcher updates/hold reset and cvar capture timing still require a bounded arithmetic audit before implementation.

Known missing matcher functions: UpdatePatterns826e0b58, FindMatchPattern826e1038, FindMatch826e1408, PickWinner826dfb00, plus IsHoldingLastMatchedPattern and actual pattern loader. Do not replace these with generic flick thresholds. Required outcomes include held/released flick, combined grabs, multi-flip and left/right recognizer history.

GenerateGameInputEvents826dfdc0 evaluates map GetState IDs59,60,61,62,63 and emits DebugPrev/DebugNext/DebugPause/DebugReplaceSkateboard/DebugResetWorld respectively. This routine is NOT the gameplay ollie/flick producer. Gameplay gesture events originate from UpdateInput matcher results.

ActionGraphInputListener ctor825f9f28 retains Actor/player identity, constructs derived current/previous raw states and hold timers, empty events, null candidate and heldfalse, then registers three message handlers on SAME GameInputManager broadcaster port. ReceiveInputEvent825f9408 and ReceivePattern825f9490 reject other player indices and append ordered events. Pattern event retains actual pattern name and normalized speed. Kickflip/Heelflip/N_Kickflip/N_Heelflip update candidate-for-holding. ReceivePatternHeld825ee440 checks player index, nonnull candidate, and case-insensitive equal pattern name before setting held-intent flag. Missing listener must not be replaced by directly firing Skeleton jump.

## Actor and animation continuation

Actor::GenerateActionGraphIntents825fda28 clears actual ActionGraph intents, chooses AI generation or (unless source force-brake bit) listener Fill825fb4c8, then clears listener events. Later wipeout request and force-brake intent handling remain ordered. Fill calls DerivedControllerInput::Update825f1c30; that copies seven current raw words to previous before RawControllerInput::Update825ee088. The latter leaf already exists in controller_input.rs; retained derived timers, magnitude/edges, gesture-to-intent mappings and full Fill do not. Existing RawControllerInput::Default is explicitly host convenience, not proof of a native retained owner constructor.

SkaterAnimPhase1Job825fe800 calls GenerateActionGraphIntents at825fe824 then Actor::UpdatePhase1. StartAnimUpdatePhase1825ff988 also calls it in its direct route at825ffc50. UpdatePhase1825f41a8 includes actual skater locomotion/body animation and ActionGraph then MotionGraph controller updates. Generated controller selection cannot be replaced by a dynamic XML graph merely because dynamic semantic pieces exist.

Existing reuse boundaries:

- input_pad.rs: source conversion and retained per-pad history; input_pad_cache.rs: selected serialized group ring; input_map.rs/parser: actual selected expressions and fallible mapped-provider bridge.
- controller_input.rs: exact raw mapped fields/arbitration; no derived/controller/listener owner.
- generated_onboard_state_graph.rs: selected generated metadata only (Action224 states, Motion1354); explicitly no generated ticks.
- state_graph_runtime.rs, dynamic_state_graph_controller.rs and motion_graph_* modules: dynamic construction/conditions/authority/selected playback services; not a completed native generated Actor graph.
- play_animation.rs, play_animation_lifecycle.rs, play_remapped_animation.rs and animation.rs: reusable selected playback/hierarchy services. Andale formats implement actual assets/codec/attribute collection; they do not produce Actor intents or schedule the native graph.
- physics.rs MotionGraphOutput::drain_into and AnimationPhysicsInput: existing one-shot output and animation-to-physics seam.
- physical_player_current_animation.rs: actual pair checked selected pose preparation; API still takes pose and adjustment input explicitly.
- physical_player_jump_events.rs and skeleton_processed_jump_events.rs: selected actual jump consumer now exists; does not establish Actor ownership or full ProcessAnimAttributes production.

Repository search found no retained GameInputManager, PlayerInputData command receiver, matcher/history runtime, ActionGraphInputListener or DerivedControllerInput owner beyond the raw leaf. Living-world behaviour_graph_runtime.rs recognizer-count fields refer to pedestrian behaviour, not this input recognizer.

Recommended order: close command-produced gameplay pad first; bind actual Actor-input capability to that same owner; recover/implement retained derived-input constructor/update around the existing raw leaf alongside actual .pat matcher/history producer; then listener ordered delivery and grounded intent slice through the SOURCE-SELECTED generated graph, playback output and existing jump consumer. This is multiple concrete dependencies, not one missing wrapper. No direct button-to-jump injection or forced named animation can stand in for this path.

Fresh SHA-256 anchors (end exclusive):

- PlayerUI828af9f8..828afd70: ad2655e96080445c2c06473b490cbb2b18ff30d38d1674893b5ef6ab3e3bb97f
- OnCommand826df568..826df734: 2cc9fec7ed3431f5c7d3ba0b15c012745fe8bcb883bf5e35f86cf89fcc238e24
- PlayerInput.UpdateInput826e18d8..826e2008: 1c296f96356124a3370ae75b9535b2c357de2c5dce0d03627ffa85fdc7e71387
- UpdateRecognizer826e2008..826e22ac: 35b54f0fae83db1bed2ce4654e0e590a9b45978fbaeaa16ec93ff974d6374849
- GenerateIntents825fda28..825fdb50: c44a38a1c930774317360d3dae5cce37490c90e127bd691ee99a7debcd67e288
- Actor.UpdatePhase1825f41a8..825f43dc: 6cbab089b4199378c82dbd614ed6c6869ed7ea6df7ab5abdeff952ee2cc12a74

