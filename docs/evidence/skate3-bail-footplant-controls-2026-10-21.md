# Skate 3 bail and footplant input recovery, 2026-10-21

This change restores missing native input producers and the original camera
remap gate inside the Skate 2 selected player's compiled Skate 3 implementation.
The existing bail and footplant physics kernels and their tuning are retained.
The Skate 2 character, world, scoring, HUD and network identity are retained.

## Original evidence

Original TU3 physical image base `0x82000000`, SHA-256
`f4aa113eb541bfba03dbc108cf5ab43f58c965b20fa3b82f9c40938a0ad841c4`.
The original executable/image remains outside the source tree. Local static
research is recorded in `work/native-controls108-evidence.json`, the accompanying
disassembly and `native108-plant-{start,adjust}-vmx.txt` in the development workspace.
VMX128 operand layout was cross-checked with the repository's existing IDA
helpers and the maintained [Xenia opcode table](https://github.com/xenia-project/xenia/blob/master/src/xenia/cpu/ppc/ppc_opcode_table_gen.cc).
No interpreter, guest executable or assembly translation is shipped.

* `PlayerUI` `82898920`, gate `82898B80..82898BE0`: the offboard State75
  branch excludes object grabbing and state503. The separate onboard branch
  reads State83 at `82898BD0..82898BD8`; true also calls remap `82898D20`.
* Wipeout Fill `82D3EF30..82D3EF88` sets State83 at `82D3EF84`.
  Footplant Fill `82D710C0..82D71134` sets the same flag at `82D7112C`.
  Those publications already exist in the port. Only the offboard gate had
  been connected to the native camera remap.
* The remap uses camera-relative `[-leftX,0,leftY]` and publishes world X/Z.
  Wipeout `82D3C3F8` projects this world vector onto forward/right. Passing
  raw stick Y directly into world Z reverses perceived up/down for some
  camera orientations. An extra PC-only horizontal negation at the intent
  boundary was also removed; it cannot substitute for a camera transform.
* Input listener Fill `825999F0` calls the object-movement producer
  `8259C4B0` at `8259AAA0`. The Derived pointer is listener+8; listener+0x24
  is Derived+0x1c, the current left-stick X. This is not the right stick.
* Producer `8259C500..8259C768` gates curve evaluation on either nonzero left
  axis. It evaluates abs(stick angle)/pi against inherited
  `inputlistener/default` PointNegGraph payloads: rotation16
  `Hash_9ADFC2E222938C1E`, X8 `Hash_1A1A7AC37A72DF87`,
  Z8 `Hash_05BA8B52C23B3481`. The four header words precede the X/Y arrays.
  X/Z are scaled by their corresponding stick axes. Rotation includes the
  authored signed curve times stick magnitude plus right X and native clamps.
  Publication order is `OB_ObjectMvZ`, `OB_ObjectMvX`, `OB_ObjectMvRot`,
  including centered zero values. The original reciprocal pi is
  `822F8610`, bits `3ea2f983`.
* Original `ActionGraphIncludes/onboardgeneral.xml` maps the X/Z names into
  `BodyAdjustX/Z`. Attribute handlers `82BDC418..82BDC440` write the actual
  processed floats at +0xB74/+0xB70 without a sign change. This original
  graph/attribute path is reused, rather than directly writing footplant state.
* Existing FootPlantManager adjust `82D70CE0` samples those fields at
  `82D70D20/28`, applies `(1-t)*0.02 + t*0.01`, projects out positive up
  movement through `82C1E170`, and moves the Bezier release endpoint while
  retaining the last handle length. It updates release direction, retaining
  release speed. Constants are original `821E9580`=`3ca3d70a` and
  `820D71E8`=`3c23d70a`. Start `82D704B0` and Update `82D70E40` are unchanged.

## Implementation and ownership

`physics/controls.rs` now uses actual State83 in addition to the existing
offboard branch. One sampled camera mapping feeds both Derived input and
physical input; it is not applied twice. Offboard movement ownership is still
restricted to offboard states. Pin filtering still runs after intent generation.

`skate-core::input::object_intentions` supplies the missing original producer.
`controls/object_settings.rs` loads the three real curve payloads through the
existing embedded Collections service, with inheritance and errors retained.
There is no fitted boost, alternate stick profile or fallback curve. Original
Skate 2-only play does not acquire this selected controller.

The rotating diagnostic journal adds `skate3_control_path`, containing source
tick/state, sampled camera axes, Derived left axes, State83 and actual
BodyAdjust/bail attributes. It records plant frames and every sixth bail frame
inside the already enabled journal block; it does not change simulation time.

## Validation

The feature-enabled core test executable builds successfully (927 existing
warnings). Thirteen controls/history/remap tests pass; the new vendor object
producer unit test passes. Only the two new Rust files were formatted outright;
edits to existing mixed-ending files preserve matching original line endings.

Actual original Skate 2 road and the embedded original Skate 3 player payload:

* A raw gamepad-triggered bail enters physical state300. All four stick
  directions pass through State83 camera remapping into actual animation
  wipeout-control attributes, matching the sampled pair without an extra sign
  reversal or a second transform. Test passes, 16.64 seconds.
* Five moving footplants (neutral, timed back pull, held forward, held left,
  held right) enter real state601 and return to air. Every nonneutral sample
  reaches BodyAdjust, changes the release vector and uses the camera remap.
  A single-tick back pull increases release Y from `2.2367425` to `2.352709`;
  actual flight apex rises from `-58.25202` to `-58.222588` in this fixture.
  Test passes, 68.94 seconds. Both original saves remain unchanged; scratch
  copies and separate journals are used. Embedded asset disk reads do not grow.

An earlier stationary-plant test correctly showed that directional changes
can lower an already almost vertical launch. That failed fixture is retained
as `controls108-footplant-corpus`; the moving test models a timed pullback,
not an assertion that every downward hold always raises every plant. Timing,
approach and camera direction affect the original mechanic.

Confidence: **high** for recovered gate, input ownership, curve fields and
release steering semantics. Native bit equality is not asserted: the existing
PC vector reciprocal-square-root estimate differs from the console seed.
No deterministic console trace was available for complete HoM/footplant parity.
The headless bail test is not a full Hall of Meat challenge completion test.
The final release executable and bounded rendered smoke results are recorded
in the Test 6 package's `VALIDATION.json` and `VALIDATED-BUILD.txt`.

## Frozen Test 6 result

Release build passes, 8m10s, 18 existing PC warnings. EXE SHA-256
`8054ddc82191ecb9b6a4f1f96d72d3823dd50375e3e69cb77aea2d3a0034edaa`.
Source fingerprint `94848dc9b5166bdfd9c280883cc3f3179d355a70780d62e42a4ede82d199507f`
matches the build script and the captured runtime build metadata (4899 files).

Bounded optimized Easy Motorized skating passes: 900 source ticks, 27.32 seconds
including startup/capture, normal process exit, no fatal/partial journal events,
original save unchanged. Capture visually retains the Skate 2 skater, board,
map and HUD. Our test process is closed. Simulation median after frame60 is
5.297ms, p95 6.459ms; three remaining spikes exceed33ms, maximum130.717ms.
This control fix does not claim to resolve the previously reported stutters.
Test 5's existing user save and preferences are copied into a fresh Test 6
folder; the original and previous test saves are preserved.
