# Gameplay controller input reconnaissance

Date: 2026-09-07. Read-only source recovery; no controller-input Rust
implementation or hardware parity is claimed. Confidence: `high` for the
inspected raw-control update, pending independent implementation review and
deterministic reference comparisons. Matched image SHA-256:
`3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`.

## Actual gameplay path

TimerCallback `0x82729770` calls InputManager::PollHardwareForInput
`0x826E0470`. cXenonPadDevice::SampleButtons `0x8298D0D0` calls XInputGetState
then MapStateToButtons; failure clears active status and returns zero buttons.
cInputPad::Update `0x829863B0` retains hardware state; cInputMap is a separate
mapping-expression layer.

Simulation::SimUpdate1 `0x8289F410` invokes GameInputManager::Update
`0x826E2578` before animation passes and physics. UpdateRecognizer `0x826E2008`
feeds right stick then left stick and negates each Y. This sign operation is
not part of RawControllerInput::Update.

Actor::GenerateActionGraphIntents `0x825FDA28` clears intents, dispatches AI or
player input, then clears listener events. Player input uses
ActionGraphInputListener::Fill `0x825FB4C8`, which calls
DerivedControllerInput::Update `0x825F1C30`. That update copies all seven words
of current raw state to previous raw state before updating current raw state.
The later animation-to-physics path includes Actor::SetUpPhysics `0x825FF6C0`
and SkaterManager::UpdatePhysics `0x825FFD98`.

## RawControllerInput::Update

Decorated symbol:
`?Update@RawControllerInput@DerivedControllerInput@Skater@Sk8@@QAAXPAVInputSK8@Input@4@PBU1234@_N22@Z`.
Address `0x825EE088`, exclusive end `0x825EE43C`; 237 instructions,
`0x3B4` bytes; MAP owner `entity_lib.obj`. Full PPC and PDB inspection supplied
the following contract.

PDB size is `0x1C`: six floats at `+0/+4/+8/+C/+10/+14`, followed by twelve
button bits in the high twelve bits of word `+0x18`. Other twenty bits are
unused by this function. InputSK8 is `0x20` bytes: cInput base `0x10` plus four
global arguments at `+0x10..+0x1C` used on every mapping query.

| Native query order | Map ID | Destination |
|---|---|---|
| GetValue 1–6 | 74, 75, 77, 78, 80, 81 | left X/Y, right X/Y, left/right trigger |
| GetState 7–10 | 76, 79, 82, 83 | left/right stick press, left/right bumper |
| Bumper arbitration | — | selection below |
| GetState 11–18 | 84, 85, 86, 87, 88, 89, 90, 91 | D-pad up/down/left/right, X/Y/A/B |

IDs are from PDB `Sk8::Input::eInputStates`. Floats are stored without clamps,
dead zones, normalization, sign changes, or range checks. The signature takes
previous raw state, useSensors, actorispushing, actorisskitching; useSensors is
unused in the PPC body.

If both new bumpers are set, retain the previous pair only when exactly one
previous bumper was set and pushing or skitching is true. Otherwise choose
left=true/right=false. When the new pair is not both set, retain the samples.
The previous pair is read only in the both-new branch. Arbitration precedes
all D-pad and face-button queries (`0x825EE240..0x825EE2B0`).

cInputMap::GetState `0x8298C8C8` evaluates the mapped expression and compares
`!= 0.0`: NaN is true, both signed zeros false. The lower cInputPad update's
`>0.5` threshold, debounce and repeat timing are different behavior.
GetValue `0x8298C718` asserts invalid map ID/offset. The raw update itself has
no null check, disconnected sentinel, sensor fallback, or error return.

## Implementation and validation boundary

A future typed raw-state implementation can consume real mapped results and
perform the complete evidenced selection/arbitration. Precomputed mapping
results are a valid explicit test boundary, not a substitute implementation of
InputSK8 or hardware polling. Preserve the eighteen query positions if mapping
evaluation is supplied as a collaborator.

Test distinct raw float bits, all map IDs/order, exhaustive current/previous
bumper and pushing/skitching combinations, inert sensor input, non-conflict
samples, and mapped nonzero semantics. Full input parity additionally needs
the mapping evaluator and real input.cfg, hardware conversion, sampling cadence,
derived hold/edge timers, intent curves, gesture recognition and per-tick traces.

Gesture entry points identified for later bounded recovery:
Recognizer::Matcher::FindMatch `0x826E1408`, FindMatchPattern `0x826E1038`,
UpdatePatterns `0x826E0B58`, PickWinner `0x826DFB00`. No behavior of those
functions is claimed recovered by this note.
