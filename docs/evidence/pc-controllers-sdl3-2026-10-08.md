# PlayStation controllers without DS4Windows (SDL3) — October 8, 2026

Goal: a DualShock 4 working on PC without DS4Windows running, through a
library that can also be compiled for Android. The controls had to feel exactly as they did
through DS4Windows.

## Library

| Library | PS4 pad on Windows | Rumble on a PS4 pad | Android |
|---|---|---|---|
| SDL3 (`sdl3` 0.20.0, SDL 3.4.18) | USB and Bluetooth (HIDAPI driver) | Yes | Yes, through SDL's Java layer |
| gilrs 0.11.2 | Through Windows.Gaming.Input | No | Not supported |

SDL3 was chosen. It also reads PS3, PS5, Switch and generic HID pads.
- **Build:** the `build-from-source-static` feature compiles SDL with CMake and links it into the
  exe, so no DLL ships. The first build takes about 4 minutes.
- **Dependency:** it sits under `cfg(windows)` next to thindx-xinput until an Android port needs
  it.

## How pads are read

`WindowsPadPlatform` still asks XInput first, so Xbox pads are read exactly as before.
- **SDL pads:** `sdl_pads.rs` puts every other pad in the slots XInput leaves empty.
- **Thread:** SDL's objects stay on the thread that initialised SDL, so a thread of its own
  (`sk8 pads`) owns SDL. Every millisecond it calls `SDL_UpdateGamepads` and publishes each pad's
  XInput-shaped sample. A tick reads the latest one, within a DualShock 4's 4 ms USB report period.
- **Xbox-type pads:** SDL lists them too. They are skipped, because XInput reads them.
- **Focus:** `SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS` keeps the XInput rule: pads read whatever has
  focus, and the game applies its own `has_focus`.
- **Scripted demos:** these runs (`demo_sample`) do not start SDL. Benchmarks and parity runs
  therefore see the same slots as before, whatever pad is paired.

## The DS4Windows conversion

The reference DS4Windows is 1.4 (DS4Windows.exe, 2016). Its Default profile sets:
- `LSDeadZone`/`RSDeadZone` 0, no curves, sensitivity 1.
- Trigger dead zones 0.

So the stick values reaching XInput came from `X360Device.Scale` alone. The exe contains that
method's only `ldc.r4 258.00787f`, and its IL decodes to:

```
value -= 0x80; if (value == -128) value = -127; if (flip) value = -value;
return (int)((float)value * 258.00787f);   // 32767 / 127
```

- **Bit-exact maths:** the exe is AnyCPU (CLR flags ILONLY only), so it ran as a 64-bit process.
  Its float multiply is single precision, and full deflection gives ±32767. `ds4windows_axis`
  repeats this bit for bit.
- **Y axis:** Y is flipped to up-positive.
- **Triggers:** the trigger bytes pass through unchanged.

SDL hands over the same bytes:
- **Sticks:** `HIDAPI_DriverPS4_HandleStatePacket` sends each stick byte as `byte * 257 - 32768`.
  The PS3 and PS5 drivers do too. Gamepad sticks pass through unchanged.
- **Triggers:** `SDL_GetGamepadAxis` normalises the trigger to 0..32767 in single precision.

`stick_byte` and `trigger_byte` invert both. A test runs all 256 byte values through SDL's
arithmetic and gets each byte back.

Buttons take DS4Windows' XInput bits:
- Cross A, Circle B, Square X, Triangle Y.
- L1 LB, R1 RB, L3/R3 thumbs.
- Options Start, Share Back.
- The D-pad hat gives the D-pad bits, both of them on a diagonal.
- `XInputGetState` never reports Guide, so the PS button has no bit, as before.

Pads that are not PlayStation ones keep SDL's 16-bit sticks, with Y negated to up-positive.

**Capabilities.** The PC answers as DS4Windows' virtual pad did: a wired Xbox 360 gamepad, with
type 1, subtype 1 and flags 0. `SetActuator` 0x8298d178 therefore treats it as not wireless and
not a guitar.

**Rumble.** XInput's left/right motors map to SDL's low/high-frequency rumble, whose PS4 driver
sends each motor's top byte, as DS4Windows did. SDL ends a rumble after at most 65,535 ms, so a
held vibration is sent again every 30 s. The port does not drive vibration yet (below).

## Startup and pads connected later

The original learns each slot's guitar subtype in `cXenonPadDevice::SetActuator` 0x8298d178,
which reads `XInputGetCapabilities`. The game calls it every frame:
`Sk8AppMan::PreUpdate` 0x8272c478 → `InputManager::UpdateVibration` 0x826e22b0 (call
0x8272c5a4) → `Vibration::Update` 0x826e1650 for each of the four pads.

The port has not ported that update. It reads capabilities only in `InputManager::Init`, and a
sample from a slot with no known subtype fails with `UnknownGuitar`. This failed the first live
run: SDL had not yet listed the pad when `InputManager::Init` asked. Two host rules now cover it:

- **Startup wait.** The first slot query waits up to 2 s for SDL's first look for pads. The log
  prints it, for example `diagnostic SDL gamepads: ["Some(\"PS4 Controller\") PS4"]`.
- **Unknown slots stay empty.** A pad found in a slot whose last capabilities query found none
  reads as `ERROR_DEVICE_NOT_CONNECTED` instead of crashing the tick. This also covers an Xbox
  pad plugged in mid-game, which crashed before.

A pad switched on after the game started is therefore ignored until the native per-frame
`UpdateVibration` is ported. Turn the controller on before launching. Running DS4Windows at the
same time shows the pad twice: its virtual Xbox pad and the physical one.

## Verification

- **Tests:**
  - `sdl_pads` checks the ScpVBus scale, its symmetry, and the exact SDL round trip of all 256
    stick and trigger bytes.
  - `skate2-pc` release: 130 pass.
- **Live, DualShock 4 v2 over Bluetooth, DS4Windows closed:** SDL listed the "PS4 Controller" at
  startup. The game booted and read slot 0 every tick for 30 s (1,659 samples). The sticks idle
  at a raw 126–130, which comes out as −516..516 inside the game's own dead zone.
- **`parity.sh`:** IDENTICAL (6000 lines).
- **Playtest, October 8:** the PS4 controller works correctly.
- **Menu, uncapped:** 1,025 FPS with the pad thread running, against 1,044 for the previous build,
  within run-to-run noise.
