# Retaining the real input producer into loaded Action

Read-only production audit plus unregistered corpus adapter. No execution claim.

The current game_input_recognizer_corpus::check constructs the actual
GameplayInputUpdateManager, original Input.cfg and all authored recognizers,
Reset-created nine players and command receivers. Actor retains player0 and its
listener registers with this manager's route. The original Ollie codec samples
are passed through Pack82a8c070/Unpack82a8c330, PlayerInput::UpdateRecognizer
826e2008, manager Update826e2578 and actual event Flush. Listener Fill produces
Trick/Ollie/GestureSpeed, but these checks run before generated graphs are loaded.

The concrete break is later in the SAME function: it explicitly unregisters the
listener, then returns only Actor and OriginalPhysicalAnimationInputs. Both the
command manager and gameplay manager (histories, matchers and actual event route)
drop. Retained player Arc alone cannot provide future recognition. Later Action
checks therefore insert controlled AG stimuli rather than consuming this route.

The new game_input_loaded_action_corpus.rs is an unregistered child proposal. It
owns these existing managers without reconstructing or resetting them. Its
step_loaded reuses the original codec step helper, shared runtime Vault/cvars and
overlay, then calls actual try_generate_runtime_action_intents825fda28 and loaded
Action Update. Identity check requires the same Reset-created player0 Arc. The
real Actor listener remains registered in the retained route. No direct AG/MG
insertion and no DiagnosticPhysics implementation is used here.

Root integration patch proposal:

1. Register loaded_action child in game_input_recognizer_corpus. Change its check
   result to include LoadedActionInputSession {commands: manager, gameplay}.
2. Remove only the explicit unregister at the old probe endpoint. Keep the
   existing listener registration identity assertions. Return the same managers
   after original animation/graph constructor stages complete.
3. Retain the session in physical_player_ground_startup_corpus and thread it to
   the selected later Actor check alongside existing input_services. Do not put
   the session inside Actor or create a second listener/player/route.
4. Select original_right_pattern("Kickflip") from the SAME loaded pattern Arcs.
   Feed its authored chronological points (stored points are newest-first),
   using codec-fed samples and source waiting/hold timing. Observe actual listener
   events before Fill if diagnosing; no manually synthesized Matched/Held events.
   Each selected frame must consume Fill before advancing the loaded Action graph.
5. Assert actual AG Trick+Kickflip on matched input, MG ordinary/mirrored name
   selected from the real animation mirror state, later genuine HoldPattern and
   same Gesture instance becoming its held intent, then source release cleanup.
   Do not inject DontMirrorTrick merely to make an expected name convenient.

Remaining explicit boundaries: original existing codec probe uses controlled
offline command sync level2; gesture-display enabled requires a real retained
SkaterManager local-index capability (currently optional None, late error).
No zero/default replacement is permitted. Actual full Update5/conditioner state
publication remains incomplete: the live Fill adapter reads real State output,
while Action selection reads filtered output; a constructed default filtered
state is not proof of grounded simulation. If this prevents naturally entering
Ground/Trick, report that real producer boundary rather than invoking the
controlled Ground setter in this adapter. Motion Update/animation/physical frame
execution after AG-to-MG transfer must be scheduled separately and must surface
its next executable condition/callback instead of claiming a complete trick.

Existing grabs/button-combination Fill, flip-Held listener matching, shared
Juice/multiflip state and matcher kernels are reused. Their component tests do
not prove a codec-fed Kickflip sequence on the loaded graph. The actual original
Kickflip sample timing and selected state trace require the next corpus gate;
the draft intentionally does not assert an unobserved universal point recipe.

Root acceptance: registered retained session and both callers; input probe runs after selected Update1 on same mounted Actor. Original43 passes matched Kickflip/held/release and logs sixteen Default3 selections. No physical trick claim. Replaced draft constructor-only authority_for_listener identity check with actual retained listener.player Arc; original42 exposed that phase-gate mismatch. workspace49/original43/Clippy77/fmt36/diff36 PASS.
