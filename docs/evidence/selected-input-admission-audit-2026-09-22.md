# Selected device input admission

Fresh IDA and existing caller audit, September22. The device event packet reuses
the existing device manager, two group caches and shared map. It does not add a
controller mapping, debounce history, platform timer or gameplay input owner.

`GetRawInputCacheLine826dfc90` reads the actual manager active half atomically
at `826dfc9c..826dfcb4`, then copies exactly0x64 bytes from the selected device
at `826dfcd8`. The serialized PC method reads active index once and copies the
whole existing count/24-channel record. It neither drains the SIM cache nor
clears disconnected stale values. Checked initialization/index failures are
explicit host admission. The selected runtime moves the supplied existing
manager, separate initialized SIM/PRES caches, shared map and numeric/remap
services, without claiming completed native InputManager construction.

`TimerCallback82729770` calls `PollHardwareForInput` at `82729798`, then caches
group0 SIM and group1 PRES through the loop `8272979c..827297b4`.
`PreUpdate8272c478` first does network Idle if the app is started, then
`LastInput(SIM)` and returns false if absent/not advanced before asset, Flow and
vibration work. `GameModule::Render827320f0` separately drains PRES at
`827321c4/cc`; debug menu gates the later presentation Input, not this drain.
Existing cache helpers preserve multiple-publication collapse and independent
pad history. No timer interval, OS thread rate or one-poll-per-simulation-tick
claim is made by the exposed explicit events.

The narrow Windows adapter supplies the existing `WindowsPadPlatform` to this
event target and preserves poll/SIM-cache/PRES-cache order. The public event
trait is an application boundary, not a substitute callback implementation.
The concrete retained implementation exists in core; application session
construction remains pending. `InputManager::PollHardwareForInput826e0470`
also sends actual FE insertion/removal messages after device polling when the
FE singleton exists; these outer notifications remain outside the admitted
serialized device-only event. Network Idle, Flow and vibration also remain
outside the narrow SIM consume method.

Device constructor leaves guitar capability unknown. Existing actuator reset
must run against actual OS capabilities before a successful connected sample
can consume that flag. The runtime does not assume reset success. Tests reuse
the actual existing reset/conversion/platform boundary fixture; a second test
omits reset and verifies typed unknown-guitar failure without publishing the
partial inactive half.

External remapping must also remain real: fresh InitBoot calls InputManager
Init at `82740db8`, publishes it at `82740dbc`, starts Juice at `82740e38`, then
installs `JuiceBase::Instance` as external remap at `82740e54`. Native remap is
not an unconditional null. `UpdateGamePad82a74c70` gates actual `sJuiceReady`
then calls `GamePadPlugin::Update82ed8768`; that update invokes its device only
when index<mDeviceNum. Constructor `82ed7d48` sets mDeviceNum0, but full startup
device registration has not been established by this packet. The selected
runtime retains the supplied remap handle/service and invents no no-op reply.

# Next gameplay payload producer

Fresh `PlayerUI::Update828af9f8` confirms gameplay consumes a DIRECT manager
cache line at `828afc48`, not the SIM pad. `GameInputFrameRequest` documentation
is corrected accordingly: conversion to24 channels precedes serialization;
the gameplay map evaluates later against the command-owned player pad.

If canSimulate, source stores focus changes and emits type2 gain-focus before
ordinary input. Session marker and optional collision/teleport work intervene
before input sampling. The ordinary payload initializes focus true, obtains
actual Simulation network state, sets allowSensor=!isNetworkGame, then copies
the raw current manager record using its retained controller index. When caller
focus is false, all24 values are cleared; payload focus becomes false only if
SIM pause is active and actual proxy next-generation frame>1. Receiver is the
retained player index+10, command type1. Spawner/reset and proxy Tick follow.

The next bounded producer must take genuine supplied player/controller identity,
network and pause/proxy state, preserve this lazy sampling, and reuse existing
codec/command/session delivery. No hardcoded receiver10, offline allowSensorfalse,
synthetic focus or full PlayerUI/session-marker/teleport/spawner completion is
permitted. Whole command generation/batch/RNG scheduling remains separate.

Draft rustfmt and shared-patch apply-check passed. Focused tests distinguish
current manager data from stale SIM pad, independent PRES drain and preserved
disconnected half bytes. Independent review requested; root owns Cargo and
actual OS/original-data acceptance.

Registered readable-tail-input-1 validation passed fmt, strict clippy and
4,636 unit plus six doc tests. Original-data 18/20: neutral frame 2 now
passes parameterization/Complete/GetAttributes and reaches BlendSpace Eval.
