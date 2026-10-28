# Audio engine: RWAC, sample player, AEMS, Csis

## 1. RenderWare Audio Core (RWAC) setup — verified

`Sk8::Audio::AudioCore::InitializeAudioCore` 0x82a7f008:

1. `InitRWAudioCore` 0x82a7e5a0:
   * `Csis::System::SetAllocator(audio allocator)`, `Csis::System::Init()`.
   * `rw::audio::core::System::CreateInstance(alloc, 256000)`; `VectorToCsisMutex` (Csis shares the
     RWAC mutex); `Lock`; `mCommandBufferHighWater = 100.0`; `RegisterStandardRunTime`;
     `CreatePlugInRegistry` if absent.
   * `RegisterPlugInRunTime`, in order: GainFader, Pan2D1, SampleCapture, PeakingIir2, HighPassIir2,
     LowPassIir2, Delay, HighShelfIir2, VuMeter, UserMusicArbiter. (Others come from the standard
     runtime and `SkateSamplePlayerFactory::RegisterAemsPlugIns`.)
   * `DecoderRegistry::RegisterAllDecoders`; `SetAssertHandler`; `Unlock`.
2. `GetPluginHandles` 0x82a7d4d0 looks plugins up by FourCC: `Sub0` SubMix, `Scp0` SampleCapture,
   `Dac0` Dac, `VuM0` VuMeter, `Sen0` Send.
3. `CreateMasteringVoice` 0x82a7d570: `Voice::CreateInstance(priority 255, 3 plugins)` =
   `[SubMix("Master", 6 ch), SampleCapture(6), Dac(6)]`; Dac Event 2 = 48000.0 (sample rate);
   InitializeAudioCore then sends Dac Event 4 with 0.0. Name "AudioCore::mpMasteringVoice".
4. `CreateMusicMasterVoice` 0x82a7d658 / `CreateSpeechMasterVoice` 0x82a7d820: priority 200,
   `[SubMix("Music Master"|"Speech Master", 6), VuMeter(6), Send(6)]`; Send Event 0 connects to the
   master submix.
5. `SpliceManager` (0x1098 bytes) constructed and initialised.
6. `Snd9::Aems::SetSamplePlayerFactory(AudioCore.mpSamplePlayerFactory)` (= `gSkateSamplePlayerFactory`),
   `Snd9::Aems::SetMinimumUpdateRate(30.0)`.

### 1.1 Mixer thread and frame size — verified

`rw::audio::core::Dac::CreateInstance` 0x82bf5588 creates an XAudio source voice at 48000 Hz and starts
`Dac::XenonThreadFunc`. `Dac::XenonThread` 0x82beb8d8 loops: wait for the packet event →
`System::ExecuteCommands` (drains the command buffer) → `Dac::Mix` (`Mixer::Execute`, runs the RWAC
timers) → copy 0x1800 bytes from `gDacInterleaveOutBuffer` into the free one of two packets (or zeros if
no buffer is available) → submit. `Dac::XenonDownMix` 0x82bebb98: `ReOrderRwAudioCoreToWave(..., 0x100)`
(**256-sample frames**, RWAC channel order → WAVE order), a 128-sample `RampOutput` on start, and
`ClipFloats(-1, 1)`. A frame is therefore 256 samples × 6 ch × f32 = 6144 bytes = 5.333 ms.

### 1.2 Timers — verified

`TimerManager::ExecuteTimers(stage)` 0x82be37a8 calls each registered timer with `mTimerPeriod`
(seconds per mix frame), measures CPU cycles, and supports deferred removal. AEMS registers
"Aems Modules" at `TIMERSTAGE_BEFORECOMMANDS`. `Snd9::CalculateAemsTimerFrequency` 0x82c19118
recomputes whenever the period changes: `n` = number of iterations of
`sum += tick while sum + tick < 1/minRate` (minRate 30), `gAemsTimerSkip = gAemsTimerMultiplier = n`,
`gAemsTimerPeriod = n * tick * 1000` (milliseconds). With a 5.333 ms tick, **n = 6 and the AEMS period
is 32 ms (31.25 Hz)**. `SNDAEMSI_timerupdate` 0x82c1b3f0 runs every `n`-th tick and interprets every
module instance in timer-list order (head insertion, so newest first).

## 2. Skate's AEMS sample player (per-voice DSP chain) — verified

`Sk8::Audio::SkateSamplePlayerFactory::CreateInstance` 0x824f91d0 is called by `Snd9::PlayerPlay` for
each AEMS PLAYER start. `SkateSamplePlayer` (136 bytes) owns one RWAC `Voice` whose plugin chain is:

```
SndPlayer1 → Rechannel → Resample → HighPassIir2 → LowPassIir2
  → [Send "wet" → GlobalEffects env-send submix]   (only if GlobalEffects.mpEnvSendSubPlugInPtrs[0])
  → Gain
  → [Send → flange submix]                         (patch 4096/0x2000/0x4000 with input8 == 1)
  → [PeakingIir2, PeakingIir2]                     (patch 1024 with input8 == 1 and 15 inputs: notch pair)
  → Pan2D (6 out; front/rear angle constructor params) → Send (6) → dry submix
```

The final Send targets `Snd9::Aems::GetDrySubMix()`. If `mSkaterType` is set (patch 512/2048, DMO
moveable-object voices from `CSTATEMGR_DynamicObject::GetMoveableObjectVoice[2]`), it targets that voice
instead and the azimuth is stored in `gSkateSamplePlayerFactory.mSendInfo[]` rather than on Pan2D.
At most 14 plugins (assert). Voice name `"sk8sampleplayerfactory.cpp"`-derived;
priority = `samplePriority × 0.01`.

* Sample header via `SndPlayer1::GetFileInfo`; channel count must be 1..6 (assert).
* Multi-channel azimuth checks (assert if adjacent azimuth spacing ≥ 910 units):
  2 ch uses azimuth indices {0, 1}; 4 ch {0, 1, 3}; otherwise {1, 2, 4} with azimuth[1] = 0.
* Mono: `Pan2D attr0 = azimuth[0] × 0.0054931641` (= 360/65536 degrees).
  Multi-channel: `Pan2D attr1 = 0`, `attr7 = front azimuth`, `attr8 = rear azimuth` (degrees).
* SndPlayer1 Event 5 (play) parameters: RAM sample pointer, stream path, stream offset clamped ≥ 0,
  rate 1.0.
* `mSampleLength = numSamples / sampleRate` (double seconds).

### 2.1 Input mapping — `SkateSamplePlayer::SetInput` 0x824ea058

| Input | Effect |
| --- | --- |
| `PLAYER_INPUT_PITCHMULT` | `Resample.attr0 = value / 4096` |
| `PLAYER_INPUT_VOL` | `mVol = value / 32767`; `Gain.attr0 = mDryLevel × mVol`; `SendWet.attr0 = mWetLevel × mVol` |
| `PLAYER_INPUT_FXWET0` | `mWetLevel = value / 32767`; `SendWet.attr0 = mVol × mWetLevel` |
| `PLAYER_INPUT_DRYLEVEL` | `mDryLevel = value / 32767`; `Gain.attr0 = mVol × mDryLevel` |
| `PLAYER_INPUT_LOWPASS` | `LowPassIir2.attr0 = (float)value` (Hz) |
| `PLAYER_INPUT_HIGHPASS` | `HighPassIir2.attr0 = (float)value` (Hz) |
| `PLAYER_INPUT_AZIMUTH` | assert (azimuth must go through `SetAzimuth`) |
| user `VOL` with patch | flange send level = value / 32767 |

`SetAzimuth` 0x824ea270 restores the default front/rear angles once for multi-channel samples, then sets
`Pan2D.attr0 = azimuth × 0.0054931641` (or `mSendInfo` for moveable-object voices).

`Snd9::PlayerSetInput` 0x82c17358 (AEMS input index → player): 0 → PITCHMULT, clamped to
[0, 0xFFFF]; 3 → `SetAzimuth(value, gLegacyAzimuths)`; 1, 2, 4..8 and 9..136 → `SetInput(index, value)`.

### 2.2 Constants worth noting

`1/4096` pitch, `1/32767` gain, `360/65536` azimuth degrees, voice priority `× 0.01`,
`Pan2D1::mDefaultConstructorFrontAngle` / `mDefaultConstructorRearAngle` (values UNVERIFIED, R-E2).

## 3. AEMS runtime — verified unless marked

### 3.1 Bank lifecycle
See [formats.md §6](formats.md#6-aems-module-bank-abk-magic-abkc--verified). Bank removal:
`Snd9::Aems::BeginRemoveModuleBank` / `IsModuleBankRemoved` / `RemoveModuleBankHandler` (R-A4).

### 3.2 Instance creation — `SNDAEMSI_CreateModuleInstance` 0x82c18c20
Runs under `System::Lock` when a Csis class is instantiated (one call per module subscribed to the
class):

1. If `curinstances >= maxinstances` do nothing (the class exists but this module does not play).
2. Allocate `datasize` bytes ("AEMS Module Instance") and copy the module's data template `pdata`.
3. Instance header (first 24 bytes): instance-list node (head-inserted into `module.moduleInstance`),
   timer-client node (head-inserted into `sndaems.timerclient`), `{pcode, pdata = instance + 24}`.
4. At `instance + destroydataoffset`: `DESTROYSTATE.settings = {pModule, pModuleInstance, pClass}`.
5. State subscriptions, walking forward from `instance + 24`:
   * ClassDestructorState (20 bytes) if `classDestructorPresent` → `SubscribeDestructorFast`
     (callback `SNDAEMSI_SetClassDestructor` sets `triggered`).
   * `numGlobals × GlobalVariableState` (28 bytes) → `GlobalVariable::SubscribeFast`
     (`SNDAEMSI_SetGlobalVariable` writes `globalValue`).
   * ClassDataState (20 + 4 × numOutputs) if `classDataPresent` → `SubscribeMemberDataFast`
     (`SNDAEMSI_SetClassData` copies `numOutputs` parameters into `outputs.value[]`).
   * `numFunctions × FunctionState` (28 + 4 × numParameters) → `Function::SubscribeFast`
     (`CsisFunctionCallback` copies parameters into `outputs.value[]` and sets `triggered = 1`).
6. `++curinstances`.

`SNDAEMSI_updatedestroy` 0x82c174f8 (opcode 4, when its `triggered` input is set) reverses all of this:
unlink, unsubscribe in the same order, `Release` every player's sample player, `Release` every
class-controller class, `--curinstances`, free.

### 3.3 Bytecode — `Snd9::InterpretModule(pcode, pdata)` 0x82c19c20

```
loop:
  op      = u8  pcode[0]          ; 0xFF terminates
  nroutes = u8  pcode[1]
  v = OP[op](pdata)               ; opcode semantics below
  for r in 0..nroutes:            ; routes start at pcode+4, 8 bytes each
      src = s32, dst = s32        ; byte offsets relative to the CURRENT module's pdata
      *(s32*)(pdata+dst) = (src == -1) ? v : *(s32*)(pdata+src)
  adv  = s32 after the routes ; pdata += adv
  pcode = address after adv
```

Values are 32-bit integers. "round" below means `trunc(x + 0.5)` for x ≥ 0 and `trunc(x − 0.5)`
for x < 0 (round half away from zero), computed in **f32**. `period` = `gAemsTimerPeriod` (ms).
`rand()` = `iSNDrandom()` (unsigned; generator is open item R-A5).

| Op | Name | Semantics (state layout from the PDB) |
| --- | --- | --- |
| 00 | ClassDestructor | out = triggered; triggered = 0 |
| 01 | ClassData | out = outputs.value[0] (+0x14) |
| 02 | GlobalVariable | out = globalValue (+0x18) |
| 03 | CREATE | out = triggered; triggered = 0 (fires once per instance) |
| 04 | DESTROY | if triggered: destroy instance (§3.2); out = 0 |
| 05 | CallFunction | optional clamp of each parameter to its {min,max} pair (when clampParameters); `Function::CallFast(handle, params)` if `inputs.triggered`; out = 0 |
| 06 | COUNTER {min, max, value, s8 direction, triggered, override} | if min ≤ override ≤ max: out = override (value unchanged). Else if triggered > 0: value += direction; if value > max → value = min; else if value < min → value = max; out = value. Else out = value |
| 07 | RANDOM {min, range, outputcurrent, triggered} | if triggered: outputcurrent = rand() % range + min (unsigned modulo); out = outputcurrent |
| 08 | RANDOMSHUFFLE {u16 inputoffset, u8 entrysize, u8 avoidrepeat, min, u16 index, u16 range, outputcurrent, numberset[]} | if *(pdata+inputoffset) ≠ 0: j = index + rand() % (range − avoidrepeat − index); swap numberset[j] ↔ numberset[index] (u8 or u16 entries); outputcurrent = min + chosen; ++index; if index < range: avoidrepeat = 0 else index = 0, avoidrepeat = 1. out = outputcurrent (a shuffle bag that never repeats across a pass boundary) |
| 09 | RANDOMWEIGHTED {TABLE* ptable, min, range, outputcurrent, triggered} | if triggered: r = rand() % 100; acc = 0; for i in 0..range: acc += (s8) ptable.entry8[i]; if acc > r: outputcurrent = min + i, stop. out = outputcurrent |
| 0A | RANGETRIG {tripgte, triplte, resetgte, resetlte, hastripped, output, input} | if tripgte ≤ in ≤ triplte: if hastripped: out = 0 else hastripped = 1, out = 1. Else if resetgte ≤ in ≤ resetlte: hastripped = 0, out = 0. Else out = 0 |
| 0B | DELAYTRIG {f32 curtime, output, triggered, delaytime} | if triggered: curtime = 0 else if curtime < 0: out = 0. Then if curtime ≥ (f32)delaytime: curtime = −1, out = 1; else curtime += period, out = 0 |
| 0C | STATEGEN {u16 inputoffset, u8 numinputs, outputcurrent, outputvalue[]} | first i with triggered[i] ≠ 0 (inputs at pdata+inputoffset): outputcurrent = outputvalue[i]. out = outputcurrent |
| 0D | MERGETRIG {u8 n, triggered[]} | out = any(triggered[i] ≠ 0) |
| 0E | ENVELOPE {u16 inputoffset, s8 prevcontrol, u8 cursegment, f32 remaining, f32 delta, f32 output, u8 numsegments, s16 releasesegment, f32 initial, {f32 duration, f32 target}[]} | control = *(pdata+inputoffset). Start (control 1, prev 0): seg 0 from `initial`. Release (control 3, prev ≠ 3, cur < release): jump to the release segment from the current output. Running (control 1 or 3, cur < num): remaining −= period; if > 0 output += delta, else output = target, advance and set up the next segment (`delta = (target − output)/duration × period`). Control 2 holds. **End-of-last-segment and other paths set output = 0 per the decompile — confirm in raw PPC (R-A3).** prevcontrol = control; out = round(output) |
| 0F | TABLE {TABLE* ptable, previnput, outputvalue, inputvalue} | if input == previnput: out = outputvalue. Else x = clamp(input, tmin, tmax) − tmin. Resolution 1.0: direct entry (u8 / s16 / s32). Otherwise linear interpolation between floor(x·res) and the next entry (clamped to n−1): out = round(a + (b − a)·frac). (Mixed u8/s8 reads in the 8-bit path — confirm, R-A3) |
| 10 | DELAYLINE {u16 inputoffset, u16 size, u16 write, u16 read, prevdelay, buffer[]} | inputs {value, delaytime ms}. On delay change: clamp < 0 to 0, d = trunc(delay/period + 0.5), min(d, size−1), write = read + d. Wrap indices; buffer[write] = value; out = buffer[read]; ++write; ++read |
| 11 | MUX {u8 n, control, in[]} | 1 ≤ control ≤ n ? in[control−1] : 0 |
| 12 | DEMUX {u8 n, s16 prevcontrol, control, value, outputs[]} | outputs[prev−1] = 0 (via value slot indexing); if 1 ≤ control ≤ n: outputs[control−1] = value, prevcontrol = control; out = outputs[0] |
| 13 / 14 | MIN / MAX {u8 n, in[]} | min / max over inputs |
| 15 | SCALE {u8 n, f32 recip, in[]} | p = (f32)in[0]; p *= (f32)in[i] for each following input; out = round(recip × p) |
| 16 | ADD {u8 n, in[]} | sum (wrapping i32) |
| 17 / 18 | SUBTRACT / MULTIPLY {a, b} | a − b / a × b |
| 19 / 1A | DIVIDE / MODULO {a, b} | b == 0 → 0, else C `/` and `%` (truncating) |
| 1B | PLAYER {pModuleBank, psamplegroup, pSamplePlayer, s8 prev[2], u8 numinputs, updateoutputs, sampletype, numuseroutputs, sampleselect, playcontrol} | c = clamp(playcontrol, 0, 2). On change from prev[0]: 0 → `Release` player and reset outputs; 2 → `Pause`; 1 → `Unpause` if a player exists, else (unless resuming from pause after play) select `entry = group.entries[clamp(sampleselect)]` and `PlayerPlay`, or reset if `sampleNum == 0xFFFF`. prev[1] = prev[0], prev[0] = c. If c == 1 and a player exists: c = `PlayerUpdate` (1 playing / 0 finished and released). out = player ? c : 0 |
| 1C | OSCILLATOR {u8 waveform, f32 phase, period ms, amplitude} | period ≤ 0 → 0. Wrap phase to [0,1). 0 sine = `iSNDsin(round(phase·1024))`·amp/65536 (scaling R-A3); 1 square = phase < .5 ? 0 : amp; 2 saw = phase·amp; 3 triangle = 2·amp·(phase < .5 ? phase : 1−phase). phase += period_ms_timer/period. out = round |
| 1D | RAMP {f32 current, f32 delta, prevtarget, prevduration, duration, scale, target} | if (f32)target == current: out = target. On target/duration change: duration ≤ 0 → current = target; else delta = (target − current)/duration × period / 4096. current += scale × delta, clamped at target; out = round(current) (confirm, R-A3) |
| 1E | ADDMAX {u8 n, max, in[]} | min(sum, max) |
| 1F | SUBTRACTMIN {min, a, b} | max(a − b, min) |
| 20 | MULTIPLYMAX {max, a, b} | min(a × b, max) |
| 21 / 22 | MIN2 / MAX2 {a, b} | min / max (ties → b) |
| 23 | SCALE2 {f32 recip, a, b} | round((f32)a × (f32)b × recip) |
| 24 | ADD2 {a, b} | a + b |
| 25 | Function | out = triggered; triggered = 0 |
| 26 | ControlClass {ClassHandle, Class* pClass, clamp, n, params…} | inputs {create, destroy, params[]}: destroy → `Release`; create && !pClass → (clamp) `CreateInstanceFast`; !create && pClass → (clamp) `SetMemberDataFast`. out = class refcount or 0 |
| 27 | SetGlobalVariable {handle, min, max, prev, value} | on change: clamp to [min, max], `GlobalVariable::SetFast` |

## 4. Csis — verified

`csiscore` (49 functions). Handles are `{void* mpPrivate; int mKey}` and validated against the
descriptor's key (−3 = stale, −6 = unbound).

* `Class::CreateInstanceFast(handle, Parameter* memberData, ClassData** out)` 0x829b2ad0: allocate
  `ClassData {updateClients, destroyClients, mpClassDesc, refCount = 1}` (16 bytes, "CsisAlloc"); call
  every subscribed constructor client (AEMS modules); `SendParameters(memberData)` to update clients.
* `Class::SetMemberDataFast` → `SendParameters`. `Class::Release` → destructor clients, free on 0.
* `Function::CallFast(handle, params)` → every subscribed `FunctionClient`.
* `GlobalVariable::SetFast(handle, &value)` → every subscribed client.
* `Parameter` is a 32-bit int/float union. Game code constructs classes through generated handles
  such as `Csis::gClass_grindHandle` with typed parameter blocks (`Csis::C_foley_utility`,
  `Csis::Class_Flips`, …).

## 5. Remaining engine work

DSP plugin maths (Pan2D1 law, IIR coefficient formulas, Resample interpolation, GainFader, SubMix/Send
routing, ReverbModel1, Delay, TimeStretch, VuMeter, channel reorder), SndPlayer1 request/loop/stream
behaviour, StreamPool and seek tables, and `iSNDrandom`/`iSNDsin` are listed in
[open-research.md](open-research.md).

<a id="aems-structs"></a>
## 6. AEMS struct reference

The authoritative layouts of all ~100 `AemsDef`/`Csis`/`CsisDef` types (PDB, llvm-pdbutil) should be
committed alongside the implementation as `docs/audio/aems-types.txt` when coding starts (generated, not
hand-written). Key ones are quoted inline above.
