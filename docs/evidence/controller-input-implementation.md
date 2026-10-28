# Mapped player controller input

Confidence: **high** for the reconstructed mapped-input update and bumper
arbitration. Addresses, map IDs, field identities, and branch order were
independently inspected in the live matched debug IDB on 2026-09-07. This is
source-derived behavior with synthetic validation, not a recorded gamepad or
native per-tick differential, and not playable input integration.

## Source and position on the player path

`Sk8::Skater::DerivedControllerInput::RawControllerInput::Update` is
`0x825EE088..0x825EE43C`, size `0x3B4`, 237 PPC instructions. Its decorated
symbol is
`?Update@RawControllerInput@DerivedControllerInput@Skater@Sk8@@QAAXPAVInputSK8@Input@4@PBU1234@_N22@Z`.
The matched linker map assigns it and `DerivedControllerInput::Update` to
`entity_lib.obj`.

Its sole direct caller is `DerivedControllerInput::Update` at `0x825F1C30`:
the loop at `0x825F1C6C..0x825F1C7C` copies all seven words of current raw
input to previous raw input, then the call at `0x825F1C88` updates current.
That caller is reached from `ActionGraphInputListener::Fill` at `0x825FB4C8`
(call `0x825FB5B0`), reached from `Actor::GenerateActionGraphIntents` at
`0x825FDA28` (call `0x825FDAA8`). The latter selects AI/player input and clears
queued listener events afterward. Its production callers include
`Actor::StartAnimUpdatePhase1` and `SkaterAnimPhase1Job`.

The wider source path starts with `Sk8AppMan::TimerCallback` `0x82729770`
calling `InputManager::PollHardwareForInput` `0x826E0470`.
`cXenonPadDevice::SampleButtons` `0x8298D0D0` calls `XInputGetState`, then
`MapStateToButtons` on success. `Simulation::SimUpdate1` `0x8289F410` calls
`GameInputManager::Update` `0x826E2578` before animation phases 1/2/3 and
`SkaterManager::UpdatePhysics` `0x825FFD98`. Gesture recognition and raw
control interpretation therefore precede animation-to-physics handoff.
None of those producer/caller implementations is supplied by this module.

## Types and exact mapping

The PDB raw type is `0x1C` bytes. `LStickX`, `LStickY`, `RStickX`, `RStickY`,
`LTrigger`, and `RTrigger` are floats at `+0,+4,+8,+C,+10,+14`.
Word `+0x18` holds twelve named bits, from high to low: `LStickIn`,
`RStickIn`, `LBumper`, `RBumper`, `UDPad`, `DDPad`, `LDPad`, `RDPad`,
`XFace`, `YFace`, `AFace`, `BFace`. Its low twenty bits are unused by this
function. Rust uses named fields and does not reproduce packing or padding.
The host zero-state `Default` is not a claim about the native constructor.

`Sk8::Input::InputSK8` is `0x20` bytes, with `cInput` base `0x10` and four
float global arguments at `+0x10..+0x1C`. Each native query supplies map `+C`,
pad `+8`, and all four globals. PDB `Sk8::Input::eInputStates` independently
names every ID; they are mapping identities, not physical button indices.

| Native query order | Map IDs and destination fields |
|---|---|
| Six GetValue calls | 74 LStickX, 75 LStickY, 77 RStickX, 78 RStickY, 80 LTrigger, 81 RTrigger |
| Four GetState calls | 76 LStickIn, 79 RStickIn, 82 LBumper, 83 RBumper |
| Bumper conflict resolution | reads previous bumper bits only if both new bits are set |
| Eight GetState calls | 84 UDPad, 85 DDPad, 86 LDPad, 87 RDPad, 88 XFace, 89 YFace, 90 AFace, 91 BFace |

Each float is stored without clamping, deadzone, sign changes, magnitude
normalization, or validation. This code consumes already evaluated `f32`
results and preserves their storage bits; it does not model arbitrary Xenon
floating-register intermediates or signaling-NaN exception behavior.

`cInputMap::GetValue` `0x8298C718` and the four-global-argument overload of
`GetState` `0x8298C8C8` evaluate the mapped expression through `cInputExec`
with `kButtonModifierValue`. The latter compares the result with positive
zero at `0x8298C998..0x8298C9B0` (`fcmpu` then `bne`). Thus negative values,
nonzero subnormals, infinities, and NaNs are true; either signed zero is false.
Do not replace this with the separate lower-level `cInputPad::Update`
threshold of `>0.5`.

The source bumper branches at `0x825EE240..0x825EE2B0` implement:

```text
if both current bumpers:
    if exactly one previous bumper and (pushing or skitching):
        retain that previous bumper
    else:
        retain left only
```

If both current bumpers are not set, the sampled pair is unchanged. Previous
both and previous neither always select left in a conflict. The three boolean
parameters are `useSensors`, pushing, skitching; this Xbox 360 body never
reads `useSensors` (incoming r6). It retains pushing/skitching from r7/r8.

## Implementation boundary and failures

`controller_input.rs` implements these field updates and conflict branches.
`MappedControllerInput` supplies the completed expression result for each
requested map identity; state conversion is performed locally. A provider
must preserve the original pad/map/globals binding. `MappedControllerSnapshot`
is explicitly a precomputed set of eighteen results in ascending ID order.
It is not a hardware packet, polling implementation, or expression evaluator.

The source body returns void and has no disconnected sentinel, allocation,
assertion, null check, recovery, or failure result. It requires valid source
pointers. Rust references supply valid input/previous objects; no fallible
provider policy is introduced. Native map evaluation itself asserts IDs
outside `[0,mMaxMaps)` or offsets outside `[0,mOpCodeTop-1)`; that expression
engine and its failure paths remain outside this implementation. The typed
control enum represents only the eighteen proven valid identities.

The remaining player path includes hardware normalization, real `input.cfg`
expressions, pad debounce/repeat, polling and simulation scheduling, derived
hold/edge timers, ActionGraph input curves, gesture recognition, ActionGraph
execution, animation, and physics. This module does not claim any of them.
It has no application caller yet. `GameModule::DoInputUpdate` is mainly
debug/menu/replay shortcuts, not the missing gameplay control implementation.

## Focused validation

Five source-derived tests cover all eighteen query identities/order, every
button destination, raw scalar bits including signed zero/NaNs/infinities/
subnormals/out-of-range magnitudes, nonzero mapped-state semantics, exhaustive
current/previous bumper and pushing/skitching combinations with both sensor
values (128 cases), and a repeated-sample sequence retaining the previous
arbitrated bumper while replacing unrelated fields.

These tests are deterministic host checks derived from PPC and PDB evidence.
A real mapped-input corpus and native recorded control trace have not yet
been captured; they remain required before a full input parity claim.

The isolated module passed all five tests using `rustc --edition 2024 --test`
and passed `clippy-driver --edition 2024 --test -W clippy::all -W
clippy::pedantic -D warnings`, plus `rustfmt --edition 2024 --check`. The root
integration owns the library export and full workspace validation; an initial
Cargo filter before that export selected zero tests and is not counted as
validation of this module.
