# Actual input-map value dependency

2026-09-08, read-only debug XEX/PDB audit. This identifies the next original
controls dependency; it is not a claim that the evaluator, pad polling, input
history or gestures are integrated. No Cargo or input implementation performed.

## Actual selected map owner

`InputManager::Init` loads literal `config/input.cfg` at `826e27d8..27e0`,
then stores the returned map at manager `+14` (`27ec`). The literal is at
`820b9324`. `InputManager::LoadInputMap` (`826ddc18`) preprocesses the file at
`dc3c`, allocates the map, initializes capacity for **208 identities** at
`dc5c`, loads parser input at `dc70`, compiles the named mappings in explicit
order, and finally purges the parser at `826de980`.

| Gameplay ID | Actual expression name | Compile call |
| --- | --- | --- |
| 74 | `GP_LStickX` | `826de128` |
| 75 | `GP_LStickY` | `826de138` |
| 78 | `GP_RStickY` | `826de168` |

These are expression identities, not hardware button/axis indices. Actual
Skeleton preparation uses 74/75, while its later current adjustment path uses
74/78. Each call evaluates the expression again, with the current captured
InputSK8 map/pad and its four argument fields. The existing jump service must
not route those reads through the already processed RawControllerInput state.

PDB `cInputMap`: `+00 mMaxMaps`, `+04 mMapOffset`, `+08 mParser`,
`+0c mOpCodeLen`, `+10 mOpCodeTop`, `+14 mOpCodeBuf`.
PDB `cInput`: vtable at zero, `+04 mIsDPDPositionUpdated`, `+08 mPad`,
`+0c mMap`; `InputSK8` adds four floats at `+10..+1c`.

The actual Xbox archive file was subsequently recovered by the root agent via
the existing `MountedBig`/`MountSet` library under `SKATE2_ARCHIVES`, logical
path `data\config\input.cfg`. The native relative request is `config/input.cfg`;
the actual data-prefix/current mount resolver remains an explicit integration
dependency. No PS3 file was substituted. The extracted original is 10,121 bytes,
SHA-256 `172c59f603e9c67e84523053ded15586efba027952dd8a68f9c8c071eb094f07`.
It remains external at `%TEMP%\skate2-original-input.cfg`; the read helper
`%TEMP%\skate2_read_input_config.rs` used the already compiled format library.

The selected original definitions are simple, with no config arguments,
preprocessor directives or included files in this extracted input:

| ID | Original definition line | Resolved retained pad reads |
| --- | --- | --- |
| 74 | 175: `GP_LStickX = LStickR - LStickL;` | Button16 Value minus Button17 Value |
| 75 | 176: `GP_LStickY = LStickU - LStickD;` | Button18 Value minus Button19 Value |
| 78 | 180: `GP_RStickY = RStickU - RStickD;` | Button22 Value minus Button23 Value |

The aliases are authored near the file's beginning. They have no modifier;
GetValue's default Value policy applies. The corresponding source SUB case
reads its first register at `8298bef8`, second at `bf0c`, performs single
`fsubs` at `bf18`, obtains destination at `bf20`, stores at `bf24`. Do not add
another deadzone/normalization layer here. These three mappings require only
actual alias/reference resolution, retained pad Value reads and subtraction;
the wider opcode inventory below is not a reason to implement unused trig or
power before closing this selected original control path. A map reader must
still validate the actual selected definitions and refuse changed/unsupported
expressions instead of hardcoding ID outputs.

The same original gameplay block (lines175..197) establishes the other
`RawControllerInput::Update` mappings without extending the required grammar:

| ID(s) | Definition(s) | Retained pad index |
| --- | --- | --- |
| 76 / 79 | `GP_LStickIn = LStick` / `GP_RStickIn = RStick` | 6 / 7 |
| 77 | `GP_RStickX = RStickR - RStickL` | 20 minus21 |
| 80 / 81 | `GP_LTrigger = LTrigger` / `GP_RTrigger = RTrigger` | 10 / 11 |
| 82 / 83 | `GP_LBumper = LBumper` / `GP_RBumper = RBumper` | 8 / 9 |
| 84 / 85 / 86 / 87 | `GP_UDPad/DDPad/LDPad/RDPad = DPadU/D/L/R` respectively | 0 / 1 / 2 / 3 |
| 88 / 89 / 90 / 91 | `GP_XFace/YFace/AFace/BFace = X/Y/A/B` respectively | 14 / 15 / 12 / 13 |

IDs92..95 (`GP_LStickRawX`, `GP_LStickRawY`, `GP_SensorZ`, `GP_SensorX`)
are authored literal `0.0f` for six-axis interoperability, not absent hardware
fallbacks. The selected gameplay mapping grammar needs named definitions,
alias references, ButtonN, subtraction and these optional scalar literals;
comments/whitespace/semicolons are syntax. It does not need generic expression
operators from the unrelated menu/editor entries merely to resolve these
definitions. Full-file parsing must preserve or skip unrelated expression
bodies structurally, never misparse their punctuation as gameplay definitions.

## GetValue and ephemeral expression state

`cInputMap::GetValue` is `8298c718`. It asserts for signed ID below zero or
ID at/above `mMaxMaps` (`c760..c774`), then checks that the selected offset is
nonnegative and below `mOpCodeTop-1` (`c780..c7ac`). A typed host wrapper should
refuse at these boundaries; it must not return a default zero on malformed map.

The local `cInputExec` initializes only these selected fields:

- config[5]=g0 at `c7b4`, config[6]=g1 at `c7b8`;
- default button modifier **Value (0)** at `c7c0`;
- config[7]=g2 at `c7c4`, config[8]=g3 at `c7c8`.

It calls `Execute` at `c7e4`, using the retained buffer and current selected
offset. PDB executor: default modifier `+00`, pad `+04`, temporary float[64]
at `+08`, config float[32] at `+108`. Execute stores the pad at `8298ba2c`;
there is **no temporary/config-array zeroing** in the inspected entry. Do not
initialize every register to zero in a future expression evaluator. Track
unwritten temporary/config reads as an explicit host uncertainty/refusal, with
config register zero's special constant semantics below.

`Execute` (`8298ba00..8298c710`) begins at program word zero, loads/increments
the instruction index at `baa0..baa8`, dispatches on word bits31..26 at
`bacc`, and returns temporary register zero at `c704` after the END opcode.
This is the game's input-expression language; implement meaningful typed
expression operations, not a guest PPC interpreter or machine context.

The inspected dispatch establishes the dependencies below. It is not yet a
complete raw numerical review of every expression operation:

| Operation family | Source region / contract |
| --- | --- |
| literal/copy | `bb6c`, `bb84..bb94`; literal consumes the following word |
| conditional/unconditional skip | `bbac..bbc4`; conditional reads its operand and skips for zero |
| OR/AND | `bbf8..bc84`; short-circuit right operand access |
| comparisons | `bca0..beac`; audit raw unordered predicates before porting all variants |
| add/subtract/multiply/divide | `bec8..bf9c`; source operand reads and single-precision operations |
| min/max | `bfb8..c024`; reads low source field before upper source field; subtraction plus fsel, not generic min/max |
| power/trig/log/exp/floor/ceil | `c034..c55c`; substantial shared numeric kernels, not interchangeable host libm claims |
| absolute/square root | `c578..c614`; square root uses estimate/refinement/zero selection |
| angular wrap/negation/not | `c628..c6d4`; preserve source rounding/zero predicates |
| END | `c6dc..c704`; returns existing temp0, does not compute an implicit result |

Destination temp index is bits25..20; ordinary source fields are ten-bit
indices in bits19..10 and bits9..0. Their source order is operation-specific.
`GetTempReg` (`82988f80`) checks index below64 before returning the actual
temporary slot. Arithmetic result publication follows the source operand
reads and destination selection; refusal must retain preceding expression
writes without inventing output completion. Invalid opcode reaches the native
assert at `c6ec`, rather than selecting a successful zero result.

## Register and retained pad resolution

`GetRegister` (`8298b918`) checks unsigned register below1024, then:

| Register | Actual source |
| --- | --- |
| 0..63 | ephemeral temp[reg] (`b968`) |
| 64..95 | `GetConfig(reg-64)` (`b9d0`) |
| 96..1019 | `GetButton((reg-96)/6, (reg-96)%6)` (`b9bc`) |
| 1020..1023 | native assert (`b9e8`), then zero only after that assertion path |

`GetConfig` (`82989148`) gives config0 the source constant +0 (`91a4`),
without reading the array. Nonzero config must be below32 and reads retained
executor config[reg] at `9198`. Only config5..8 were initialized by this
GetValue overload; the others are not proven readable defaults.

`GetButton` (`82988fe8`) resolves PDB `eButtonModifiers`: Value0, Held1,
Pressed2, Released3, Repeat4, Default5. Default reads the executor's current
default modifier, asserts if still Default, and re-enters the selection at
`89010` (`89040..89068`). GetValue sets this to Value. The relevant four-argument
GetState overload (`8298c8c8`) independently confirms the **same Value policy**
at `c970`, config5..8 writes at `c964/c968/c974/c978`, and final expression
`!= 0.0`. It does not choose Held for an unmodified button name. Both signed
zeros are false and NaN is true in this final predicate; explicit `.held`
expressions remain a different source operation.
The selected calls are value `89098`, held `890b0`, pressed `890d8`, released
`89100`, repeated `89128`. Boolean results convert to scalar 0/1.

PDB pad: `+04 mNumButtons`, 24 `tButton` entries at `+08`, stride16.
Each entry has Value+0, IsPressed+4, IsReleased+5, IsHeld+6, IsRepeated+7,
RepeatCount+8, DebounceCount+c. These are retained pad-update outputs, not raw
platform packet fields. `cInputPad::GetButtonValue` (`826df2a8`) returns +0
when actual `mNumButtons==0`, otherwise checks signed button range then reads
Value at `f324`. That genuine empty-pad branch is distinct from fabricating
an absent pad as a zero-valued device. `IsButtonPressed` (`82986138`) has the
same empty-pad/range ordering, then reads its retained boolean. Other modifier
accessors are `82986028` (Held), `829860b0` (Repeat), `829861c0` (Released).

## Reuse and minimum closure

`controller_input.rs` already defines IDs74..91 and performs source raw-field
sampling plus bumper arbitration. Its `MappedControllerInput` deliberately
starts at **completed expression results**. `MappedControllerSnapshot` is a
diagnostic result array; it supplies none of the map, pad, config, expression or
polling ownership above. Keep it for tests, not as a live controls substitute.

The minimum next closure is the original selected input.cfg/alias resolution ->
typed retained map expressions -> actual pad/config read adapter -> existing
mapped-controller and Skeleton jump input consumers. Recover the selected
expression subset first; refuse unsupported expressions rather than adding
generic deadzones, clamping, axis swaps or smoothing. Share the original
numeric helpers only after their operation graphs and exceptional behavior
match the selected input operations. In particular, destination temp storage,
min/max evaluation order, short-circuit reads and unwritten config readiness
must survive the port.

The pad producer remains a separate necessary dependency: existing evidence
places `TimerCallback82729770 -> PollHardwareForInput826e0470`, Xenon sample
`8298d0d0`, retained pad Update `829863b0`, then Simulation's GameInputManager
Update `826e2578` before animation/physics. The input history, gestures and
Actor handoff remain open, as documented in `controller-input.md` and
`skater-jump-event-runtime-plan.md`. This packet does not replace those stages
with a sampled host gamepad or claim frame-accurate controls.

## Raw fingerprints

SHA-256 over end-exclusive debug ranges:

| Range | SHA-256 |
| --- | --- |
| `8298c718..8298c7fc` | `9313532f25b8b893db185b0079741530b83833e8b6297e048e69de5847776d2a` |
| `8298b918..8298ba00` | `26baf85ec11e0b05ce1058d712d2adfd91e89938670ba934751d9e9ee5a0c0f9` |
| `82988fe8..82989148` | `9147eccac9af048bb41fc613f0fb9db54fcbc9bf6743688c0fff54eed3c16659` |
| `82989148..829891b0` | `fd41c70a5c4d0b23f07c86cbe4080fdb31e0a330c45230e46a8d287103787f50` |

## Implemented selected evaluator and independent review correction

2026-09-08: input_map.rs/parser/tests now parse the actual external Xbox config
and evaluate selected gameplay mappings. Original file FNV-1a64 is
9922ceaf891281d6 (length10121); SHA-256 above remains the asset fingerprint.
The actual retained InputPad implements the map Value service; MappedPadInput
connects it to RawControllerInput::try_update without precomputing expressions.

Independent raw review found that native ParseStatement publishes an identifier
only after compiling its expression/semicolon. Alias lookup is therefore now
limited to preceding definitions, rejecting forward/self references. It also
found native computed children execute before direct-register operands: a
naive eager AST changes read order for Button0-(Button1-Button2). The selected
runtime explicitly refuses such nested arithmetic; original gameplay mappings
need only direct operands. One subtraction or negation over direct literals/
Button values is supported. Raw unary fneg8298c680 confirms negation. No guest
CPU or input bytecode interpreter was introduced.

The existing RawControllerInput update body was extracted once into a fallible
try_update, with an Infallible adapter preserving the old API. Native sample
order and bumper arbitration are unchanged. A failed map read preserves earlier
writes and stops later reads; no missing input is converted into neutral values.
Independent review accepted both extraction and narrowed evaluator.

Original-data checkpoint30000/7100d0 passed all12 checks, including original
cfg parse plus raw integer packet -> exact radial/trigger converter -> same
retained Pad -> map -> RawControllerInput. The fixture distinguishes immediate
Value-based mapped GetState from suppressed pad Held/Pressed transitions. It
explicitly schedules four Pad updates; it does not prove poll/cache/Actor timing.
The new actual jump bridge/corpus follows this checkpoint and requires its own gate.
