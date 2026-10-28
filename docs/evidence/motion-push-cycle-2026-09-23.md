# Motion PushCycle source, 2026-09-23

The 120-frame raw-A Motion replay reached occurrence 2358 CreateInstance at frame 74 after the PushContact parent update was corrected. This note covers the callback source and focused component tests; the independent full replay result is recorded by the integrator.

## Reference ownership

- `Sk8::Behaviours::PushCycle::CreateInstance` 0x82caf9b0 allocates 0x10 bytes, stores stage 1 at instance +8 and float 0 at +0xc. The outer behaviour is allocated separately (0x34) by factory helper 0x82cb68b8, with `mNewPushName` at +0x1c.
- `Begin` 0x82cd6328 resolves the MotionGraph and SkaterMotionGraph interfaces, gets the graph's PushState, evaluates the `anim_motion_pushing` cached graph at +0x1e0 with input 0 and writes the instance +0xc. It copies intents, reads live `kAllowHolding`, then tests `PushingIntent` and the configured name. All true sets stage 0 and PushState +0x20 high bit. Otherwise it sets stage 1 and clears that bit.
- `Update` 0x82cd6590 also resolves SkaterAnim and IPhysical, reads the physical output z speed (+0xa8), clamps it with PPC `fsel` to nonnegative, and copies the MotionGraph intents. Stage 0 with `kAllowHolding` and Pushing maps press time to change in velocity using 0x82c95328, caps at the generated `max_holding_acc` accessor 0x82c8ad60, then raises PushState +4 as needed. Stage 0 without Pushing sets stage 1, resets instance change in velocity from graph +0x1e0 at 0, and clears the flag. Stage 1 with `NewPushIntent` and configured name becomes stage 2. Stage 2 without configured name becomes stage 3 and writes the retained change in velocity to PushState +4. Stage 2 otherwise reads the top speed scalar, live `kCapDvAtTopSpeed`, conditionally maps current Pushing value, raises PushState +4 as needed, then sets its high flag.
- The bound `End` callback 0x825187c0 is a `blr`; it writes nothing. The generated Motion binding records all four callbacks.

## Constants and existing owners

- `MapButtonPressToChangeVel` 0x82c95328 reads the duration graph at cached Pushing layout +0x230, divides the press time, clamps through two `fsel`s, then evaluates the change graph at +0x1e0. `PushingMotionAttributes::first_push_graph` already owns these cached graphs and the Vault identity check.
- Correction September24: generated `max_holding_acc` 0x82c8ad60 uses key
  `07a7a486d4c75e46`, not the previously recorded reversed halves.
  `insrdi r4,r9,32,0` at82c8ad80 puts r9 in the high word. The original
  Vault resolves this key to4.0; the reversed key is absent and returned0.
  `string_hash64("max_holding_acc")` independently agrees with the corrected key.
  The stage2 top-speed key is likewise `1a4ca1769dccb3a8` (82cd6a08..24),
  shared with first-push Update82cc579c..57b8. The first-push teleport threshold
  is `4c4b9cdf5860f565` (82ca0c20..3c). Repush End keys were checked at
  82ca10fc..1118 and82ca1154..1170 and were already correct.
- `kAllowHolding` initializer 0x82f86038 calls `cVarBoolRef` at 0x82f86064 with section `Animation_Skater`, title `Allow Push Holding`, true initial value, and null callback. Its registration table entry is 0x82fd4edc, directly after `kCapDvAtTopSpeed` at 0x82fd4ed8. Begin reads its target at 0x82cd6534, Update at 0x82cd6880. The existing `kCapDvAtTopSpeed` CVar is reused.
- PushState's retained `push[1]` and `push_bits` already represent offsets +4 and +0x20. Intent names `Pushing` and `NewPush` match the Action input producer. The configured new-push name comes from the generated behaviour's parsed payload. No separate animation-property buffer is introduced.
- Raw `fcmpu` at 0x82cd68ec followed by `ble` skips the holding cap on ordered `<=` **and unordered** comparisons: `ble` tests the cleared GT bit. The `fcmpu`/`bge` pairs at 0x82cd6904/0x82cd6908 and 0x82cd6a9c/0x82cd6aa0 likewise skip the PushState +4 write on unordered comparisons because `bge` tests the cleared LT bit. The top-speed `fcmpu`/`bgt` at 0x82cd6a40/0x82cd6a44 maps the button value for unordered speeds/thresholds. The corrected Rust comparisons reflect these raw branches rather than the decompiler's simplified greater-than/less-than notation. This correction followed a separate camera SphereTest PPC review and supersedes the first focused NaN test; finite behavior and the 111-frame raw-A progress are unaffected.

## Validation and limits

The original three focused `skate2-core` tests passed for finite branches. The NaN test was corrected after the PowerPC condition-register review and requires rerun before the corrected edge branch is called validated. The full raw-A replay now advances through PushCycle and Pre444 to frame111, where a distinct PushEnd callback remains pending. The tests exercise callback order and retained fields with an ordered source spy; they are not a reference runtime trace. Confidence is **high** for the checked finite callbacks and fields, and **medium** for full-session integration until the later boundary passes. No unrelated callbacks were inferred.
