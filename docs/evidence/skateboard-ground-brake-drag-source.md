# Grounded braking and linear drag source packet

2026-09-08. Read-only original debug XEX/PDB and original
Vault inspection. No production implementation or Cargo execution in this task.

## Identity and callers

| Decorated original symbol | Half-open address span | Original bytes SHA256 |
|---|---|---|
| `?Toolkit_CalcBraking@Skateboard@Physics@Sk8@@QAA?AUSkateboardForce@123@XZ` | `82b26df0..82b27074` | `575d187f1e6260491cb380ea72b3dbf645b320293abbbf1d6c895e234337f44b` |
| `?Toolkit_CalcLinearDrag@Skateboard@Physics@Sk8@@QAAMXZ` | `82b27078..82b272f8` | `2d5e649750d7fb39acde642f7d283fedfeeacd909b9c741d16eacc370fe588f9` |

Exactly two direct callers each: GroundAnimation::UpdateSkateboard `82d8a7b0`
calls braking `82d8a8d8`, drag `82d8a8f4`; PhysicsGround::UpdateSkateboard
`82d9a018` calls braking `82d9a6bc`, drag `82d9a77c`.
The ordinary GroundAnimation path calculates front, rear, braking, slide,
drag, collision before choosing collision-only versus ordinary publication.
Ordinary queue order is front4, rear5, slide1, brake2, then SetLinearDrag;
collision-only publishes type15 and suppresses the drag write. Thus these
leaves must execute even if collision later discards their results. See the
[front/rear packet](skateboard-ground-foot-forces-source.md) for wrapper scope.

Braking returns a 32-byte hidden result: actual r3 is output, r4 Skateboard;
IDA's this/result labels are reversed. Drag returns scalar f1. Neither leaf
enqueues a force, writes a body, or changes processed input.

## Owners and original values

Both use freshly acquired GlobalAttributeCollection singleton and member
`+ac=physics_brakes_default` at every lookup, never Skateboard own `+534`.
Global constructor `828b1608` builds r28=`d7edbd362d7d2152` at `1630..1644`;
`24b4..24c8` builds class `191bce2c9e145f51`, FindCollection `24cc`,
Instance construction `24dc`, retained pointer store `24e8`. Missing singleton
asserts/DebugBreak before continuation; this is not a source default owner.
Every scalar lookup uses index zero; absent pointer calls DefaultDataArea(4).
No cvar read occurs in either leaf. Do not introduce a cvar gate/cache.

Existing read-only inspect_vault.exe inspected original extracted skater.vlt/bin.
Class physics_brakes export offset376504; selected default has parent zero and
exactly these six inline entries (type index0, flags40/00). Definitions all
have float type `e22228fbb3c209d8`, size4/count1/flags4/alignment2.

| Role (descriptive, not recovered authored name) | Raw 64-bit key | Original word | Node offset |
|---|---|---|---|
| Foot braking coefficient | `d71f120752432c0a` | `42480000` (50) | `5bf20` |
| Manual braking coefficient | `c19a40df5e7b2662` | `420c0000` (35) | `5bf70` |
| Brake stop speed | `92d0c41a3315b63d` | `3c23d70a` (approximately .01) | `5bf60` |
| Balance drag speed ceiling | `65468a36a6e32f03` | `3f800000` (1) | `5bf50` |
| Ground normal Y threshold | `343f67237b0bf1bc` | `3f6b851f` (approximately .92) | `5bf40` |
| Balance drag coefficient | `75a86a1f8ff8ab6d` | `3c23d70a` | `5bf30` |

Inline word is node+8; independent byte scanning verified keys/words. Values
are test oracles, not runtime hardcoding. Extracted VLT SHA256 is
`496215b210c3ce8bda3f7a64b3cc3d59c56546e8d3026dbf1da4a72f93fa7dde`.

PDB current Skateboard processed pointer is `+110`; relevant ProcessedPhysIn
members: bodyZDirection+170 (full Vector3 storage), flags word+8c4,
surfacePhysicsCollection+904, zSpeed+944, absBodySpeed+948, balance+9a0,
push+9a4, footBrake+9a8. Flags are raw mask40000000 foot braking and
20000000 manual braking. The surface access is actual processed+904 then
Attrib::Instance layout pointer+4 then physics_surfaces::_LayoutStruct
`BrakingScalar+108`. It is not gravity, mass, or global brakes tuning.
Drag separately reads Skateboard Reckoning pointer+104, groundNormal+1f0.Y;
do not substitute ProcessedPhysIn groundNormal, contact output or world up.

Runtime producer boundary: PhysicalPlayerHiLOD::Update1_ProcessInput
`82e01f98` resets processed input at `0200c`, processes animation at `02134`,
updates SurfacePhysics at `02268`, writes zSpeed at `026e0`, calls actual
PrepareToolkitInput at `036cc`, then Skeleton::ProcessData at `03718`.
PrepareToolkitInput `82b26910` derives bodyZDirection from actual deck Z,
flip scalar, zeroed Y and source NormalizeSafe sequence (`26b20..26ba0`),
and derives absBodySpeed from retained zSpeed with source sign multiply
`26c68..26c9c`. The earlier mass/deck/effective prefix is already implemented
at bounded retained scope; this packet does not claim the rest or native
Actor/controller/animation scheduling is implemented. Complete flag/control
and surface selection writers still require independent producer integration.

## Braking exact order and failure prefix

1. Read processed pointer and flags (`26e18/24`). If foot mask set, fetch
   foot coefficient (`26e74`, default `26e84`), reload processed pointer
   `26e88`, read coefficient then footBrake `26e90`; single-precision
   multiply **footBrake * coefficient** at `26e94`. Otherwise select +0.
2. Read flags again at `26ea0` using the current retained pointer from step1.
   If manual mask set, fetch manual coefficient `26ef0` (default `26f00`),
   read `26f04` and **replace**, not add to, foot result. Both lookups occur
   when both flags set; a malformed foot value cannot be skipped.
3. Reload processed `26f08`, capture surface pointer `26f10`, read zSpeed
   `26f14`, compare against +0; read layout `26f20` then BrakingScalar
   `26f24`. Multiply **surfaceScalar * coefficient** `26f28` regardless of
   either braking flag. Ordered zSpeed<0 selects +1, all other values
   (both signed zeros and NaN) select -1. Multiply **sign * magnitude**
   `26f58` separately, without reassociation/FMA.
4. Unconditionally fetch stop threshold `26f98` (default `26fa8`), reload
   processed `26fac`, read attr `26fb0`, absBodySpeed `26fb4`. Source
   `26fc0=40980008` is BO4 BI24: branch on **not LT**, including unordered.
   Therefore choose +0 only for ordered absSpeed<threshold; otherwise keep
   signed magnitude, including NaN in either comparison operand.
5. After all above reads, write full +0 world force at `2703c`, full +0
   local position at `27044`, duplicate full zero local write `27048`.
   Reload processed pointer `2704c`, read full bodyZDirection `27058`,
   multiply every lane by selected scalar `2705c`, overwrite full world
   force `27060`. Raw `173ad8b5` decodes VA58, not IDA's printed v90.

Native checked failures before step5 preserve the entire output. A direction
read refusal after zero initialization preserves two zero vectors. All-lane
multiplication is required even for zero magnitude: NaN*0 is not force zero,
and unknown source W must not be filled in or silently accepted as known.
Preserve captured surface owner while allowing later processed reads at the
source sites; a blanket upfront snapshot has a narrower alias/reload contract.

## Drag exact order and exceptional comparisons

1. Unconditionally fetch/read stop threshold `270d8/270f0`, reload processed
   `270f4`, multiply **threshold * 2.0f** (`40000000`) at `27104`, read
   absBodySpeed `27108`. `27114=4098003c` branches on not LT, including
   unordered, directly to stopped=false without reading flags/push/balance
   in this first test.
2. Otherwise read flags `27118`; require either brake bit. If present read
   push `27134`; ordered push>0 rejects (NaN does not reject here). Otherwise
   read balance `27140`; equality to +0 yields stopped=true. Both signed
   zeros equal; balance NaN yields false. This is short-circuit evaluation.
3. Independently read balance again `27154` using the same processed pointer.
   Equal zero skips balance-drag tests. Nonzero or NaN fetches balance speed
   ceiling `271a0`, reloads processed `271b4`, reads ceiling `271b8` and
   absBodySpeed `271bc`. `271c4=409800a4` is again not LT: unordered rejects.
4. Only if that speed comparison is ordered LT, fetch/read normal threshold
   `27208/27220`; then load actual Reckoning pointer `2722c` and groundNormal
   vector `27240`, splat Y `27244`, compare Y>threshold `27250` and test the
   all-true CR6 bit. Raw `1b7ee165` decodes VA62, not printed v94. All lanes
   are splatted Y, so ordered scalar > is equivalent; NaN rejects.
5. Only on the successful balance/low-speed/normal condition fetch drag
   coefficient `272b8`, read f1 `272cc`; otherwise f1=+0 `272d4`.
6. Only now inspect previously captured stopped flag `272d8`; true replaces
   result with literal **1.0f** at `272e8`. Do not return1 early before the
   balance reads/possible lookups; source collaborators could change balance
   between reads. Return scalar, no board linear-drag mutation in this leaf.

All selected lookup failures occur before a drag result returns. Typed
services should preserve deferred lookup/value/input reads, not eagerly
decode all six keys or all input fields. Immutable snapshots are acceptable
only with their explicit existing scoped ownership limitation; they do not
prove callback mutation/native schedule equivalence.

## Validation and next implementation requirements

Confidence **exact** for audited leaf raw branch/read/write order, key
assembly and PDB offsets; **high** for original Vault provenance via existing
parser plus raw-byte cross-check. Independent source review and implementation
tests remain pending. No per-tick reference trace or live input parity claim.

Discriminating tests: both brake flags with foot failure and manual override;
surface read despite no flags; threshold/direction failure output prefixes;
signed-zero/NaN branch table; nonzero/unknown W and zero*NaN; drag early
threshold short circuits; balance read twice; late coefficient failure before
stopped override using mutable services; actual retained global inheritance
distinct from own deck collection; original six inline words as test oracles.
The original surface BrakingScalar must come from the selected runtime surface
instance/layout. Its original collection selection/value has not been replaced
with a guessed constant by this packet.

## Bounded implementation follow-up

After independent source review PASS, root assigned the four new files
`skateboard_brake_drag.rs`, `skateboard_brake_drag_tests.rs`,
`skateboard_brake_attributes.rs`, `skateboard_brake_attributes_tests.rs`.
The leaves reuse the existing full-vector SkateboardFootForce record and
provide staged input reads, a captured surface token, separate attribute
lookup/value stages, and owner-bound retained global Vault data tokens.
Known W multiplies normally; unknown W remains unknown. The redundant native
local zero store has no intervening observable collaborator and is represented
by one zero assignment. No cvars or normalization were added.

Five leaf tests cover ordered collaborators/manual override, failures before
and after output initialization, exceptional scalar comparisons and W,
repeated balance with late coefficient failure before override, and NaN drag
short circuit. Three provider tests cover default/hash, retained inheritance
and foreign tokens, and deferred malformed scalar reads. A cfg(test)
`check_original` helper checks the six original values for root's existing
corpus; no additional ignored test wrapper was added. Standalone rustfmt
passed; author did not run Cargo. Root integration/compilation and independent
implementation review are pending at this entry. These remain leaves, not a
claim that the actual grounded producer path or native Actor scheduling runs.
