# Camera obstacle CVar registration, 2026-09-23

Scope: seven static reference definitions and linked live reads in the existing
`InputCvarReferences`/`InputCvarManager` owner. This does not implement
`CameraObstacles::FindObstacles` or camera collision queries.

IDA's debug-symbol initializers and `.CRT$XCU` table entries establish the
following exact section, label, title, source default, and registration order:

| Index | Initializer | Table | Section | Label | Title | Default |
| ---: | --- | --- | --- | --- | --- | --- |
| 349 | `82fa21a8` | `82fd65fc` | `Camera_Avoidance` | `kbPedestrianAvoidance` | Pedestrian avoidance | true |
| 350 | `82fa21f8` | `82fd6600` | `Camera_Avoidance` | `kbSkaterAvoidance` | Skater avoidance | true |
| 351 | `82fa2248` | `82fd6604` | `Camera_Avoidance` | `kActorSearchRadius` | Actor search radius | `10.0f` |
| 352 | `82fa29f8` | `82fd6664` | `Camera_Avoidance` | `kbInterleaveAvoidance` | Interleave avoidance | true |
| 353 | `82fa2a48` | `82fd6668` | `Camera_Avoidance` | `kbEnableCollisionAvoidance` | Enable collision avoidance | true |
| 354 | `82fa2c78` | `82fd6684` | `Camera_Debug` | `kbShowAvoidanceDebug` | Show avoidance debug | false |
| 355 | `82fa31c8` | `82fd66c8` | `Camera_Dynamics_Settings` | `kInsignificantSpeed` | Insignificant speed | `0x3e19999a` (`~0.15f`) |

Raw PPC for `kActorSearchRadius` loads `0x41200000` for its default and
`0x00000000..0x42c80000` for its editor range. `kInsignificantSpeed` loads
`0x3e19999a` and range `0x00000000..0x3f000000`. Both pass a null callback.
The five boolean initializers likewise pass null callbacks. The existing
manager's startup float handling deliberately does not clamp to editor ranges.

The cvar registry appends indices while placing each new initializer at its
original table position in `REGISTRATION_ORDER`. Each typed getter reads the
current linked target, so startup config and later target changes are observed.
Confidence: **exact** for registration metadata and defaults, **high** for the
Rust manager's live-target behavior based on the previously recovered manager
implementation. The camera consumers remain to be integrated separately.

Validation: `cargo test -q -p skate2-core input_cvar_runtime::tests --lib`
passed 34/34 tests. The focused test checks unlinked errors, all seven source
defaults, and live linked reads after startup config changes. The existing
section/title ordering oracle was extended for these seven entries and the
concurrent `kAllowHolding` and `kOB_IK_MaxFeetYDist` additions.
