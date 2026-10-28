# Remaining playable runtime links — 2026-09-22

Independent bounded read-only code audit, with this evidence file explicitly
authorized by root. Exact Test-Path returned false before creation. No Cargo,
registered-code edits, or new source implementation. Line anchors reflect the
files at review time; concurrent source integration can move them.

The goal remains a runnable skating build, including input, onboard motion,
bail and presentation. Passing the retained three-frame corpus does not itself
provide that build. Older remaining-work notes (a September 8 inventory) are not an
up-to-date inventory of the September 22 implementation. Do not redo systems solely
because that older inventory describes them as missing.

## Actual executable boundary

`apps/skate2-pc/src/main.rs:198` dispatches inspection/data/probe commands.
At line 247 the default path explicitly reports that the playable runtime is
not implemented and returns failure. `render_inspection.rs:195` receives keys;
its frame method at line 211 changes the inspection camera and updates static
map submission transforms. It neither owns nor ticks the physical player.

`crates/skate2-core/src/lib.rs:9` and the private Actor/physical assembly modules
show the current application-facing gap. The retained owners and selected
stage methods are implemented, but there is no public mounted player bootstrap
and frame facade connecting them to this executable. Reuse those owners and
methods behind a narrow facade; do not create a second physics implementation.
Startup asset/attribute/animation/input mounting currently lives substantially
in test-only original-data corpus code.

## Input and animation: existing chain versus missing callers

`physical_player_three_frame_corpus.rs:106` supplies a neutral 24-value sample;
line 128 loops three times, line 131 calls the retained input session, line 156
calls the Actor prefix, line 177 physical pre-solve, line 199 collision/solver,
and line 208 post-solve/output. This file is a test-only child and is not called
by the window. Its input session is imported from the recognizer corpus.

The native raw-pad conversion already exists in `input_pad.rs:14`
(`XboxPadSample`, integer buttons/triggers/sticks). Existing mapping, command
codec, gameplay pad, recognition and Actor delivery must be reused. The app
has no gamepad polling dependency or gamepad event path; its only keyboard
consumer outside window closing is the inspection camera. Needed link:
platform sample/focus/disconnection -> existing native pad conversion and
command/input ownership -> the retained frame session. Do not substitute the
SIM/PRES cache pad for the Actor's command-owned gameplay pad.

**Do not duplicate the animation tick prefix.**
`actor_selected_frame_session.rs:98` already composes GenerateActionGraphIntents,
animation phase 1, generated Action and Motion updates, animation phases 2/3,
body synchronization, hands/face, SetUpPhysics and the selected Actor suffix.
It uses retained body/input services; it is not an empty animation placeholder.
`actor_animation_mount_corpus.rs:152` defines the current retained fixture
services; initialization at line 188 mounts the actual startup owners and
obtains constructor physics parameters using `phys_Tpose` at line 249.
That constructor lookup is not evidence that every live frame remains T-pose.

Nonneutral graph coverage is still bounded. In
`generated_motion_runtime_lifecycle.rs:341`, live state-update dispatch accepts
states 6 and 1170 with exact behavior sets; line 347 rejects other updates.
`generated_action_runtime.rs:441` supports selected trick states 52..54,
Ground Idle 94, Antic 83/88 and its accepted default thunk; line 451 rejects
other state updates. Expression/condition/callback boundaries also remain.
Next concrete validation is real steering, push, ollie/flatground and grab
input through the same chain, recording the first actual unsupported thunk
or missing producer. Implement those native branches instead of broadening
dispatch to fabricated success. Neutral multi-frame success cannot certify
the full controls or animation repertoire.

## Bail and landing

Wipeout requests, the retained WipeoutGround constructor, Ground-to-Wipeout
transition and WipeoutGround.Enter are already implemented.
`physical_player_ground_to_air.rs:205` selects the actual WipeoutGround owner;
the same module invokes its entry after Ground.Exit. The entry in
`physical_player_wipeout_state_session.rs` performs real board/skeleton velocity
operations and ragdoll collision. Do not recreate these as another bail system.

The repeated pre-solve composition currently dispatches Air explicitly and
otherwise follows the Ground update sequence. It does not implement a live
WipeoutGround.Update tick. The selected PostPhysics, Fill and GetType dispatch
extensions being integrated cover Ground/Air, not WipeoutGround. Those are
concrete missing continuations for sustained bail and recovery. Air-to-Ground
landing/recovery needs corresponding real transition coverage; existing
`physical_player_physics_state_session.rs:384` and `:394` retain explicit
landing-wobble and landing-force dependencies. LivingWorld-specific entry
branches remain separately excluded from the first-build scope.

## Existing renderer and missing pose publication

The inspection renderer already loads original map geometry/textures through
the existing native render submissions. `gpu_renderer/character.rs:146` and
`:201` provide character creation/update using actual skin weights, bone
palette rows and source-generation checks. Search found no callers outside
their definitions. Their comments explicitly retain the missing Actor pose
publication/draw mount. Reuse these APIs; do not build another renderer.

Needed link: completed retained Actor animation/physical pose -> checked
character palette publication -> retained GPU character object, together with
the actual board transform and a player-following camera. Current character
shading is explicitly diagnostic unlit diffuse (line 190); original material
lighting and hair alpha are not implemented by that draw. A camera-only map
inspection window is not the requested skating test.

## Scheduling and acceptance scope

The approved first predictor harvest belongs to the explicit flags-3/no-visual
serial profile. Its packet documents native pair-pipeline, early StartFrame,
StepSelector/locomotion and second-harvest scope separately. Connecting the
first harvest exposes actual completed-query scoring; it does not close these
remaining application links or certify a native full-frame scheduler.

After source-reviewed Air/scoring continuations, the next reviewable product
milestone is a retained player startup/tick facade consumed by the existing
window, real host input feeding the existing command path, and actual character
and board pose rendering. Input-triggered graph and bail/landing dependencies
must remain explicit until implemented and tested. Provide runnable runtime
diagnostics from that path; do not present inspection-only or corpus-only
success as the completed first playable build.
