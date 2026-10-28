# Four-mode landing spring

2026-09-08, implementation following accepted68261. The new retained owner and
lazy Vault provider are integrated into the current-animation producer. Accepted
checkpoint17769/60e4f5 EXIT0 passed strict workspace and original-data validation.
Separate source/code/tests/shared/corpus review passed. This is a
source-backed component integration, not a native per-tick trace claim.

`Skeleton::UpdateLandingAdjust`82b401d8..82b41100, SHA256
759a922e5e77b10fd99f106c2b09bd5bfbecec2433e29edd8f5c8fe9b19230ad.
All four modes share a small scalar spring/offset producer and existing graph4
evaluation. Smallest complete scope replaces current LandingStart/LandingSpring
stops, keeping the actual offset and current-animation owners.

## Retained data and actual inputs

PDB LandingAdjustData atSkeleton+3e48 has nine fields: adjusting+0, type+4,
time+8, YPos+c, YVel+10, secondLastComVel+14, lastAnimPos+18,
lastFilteredState+1c, desiredComForGrind+20. Constructor574b4..574d4 stores zero
except lastAnimPos=MAXFLOAT7f7fffff. TeleportReset54228..542bc leaves it intact.
Extend/replace existing three-field LandingHistory; do not add another history.

Current processed velocity+260 dotted with processed Up+220 is this helper's
velocity. Retained SkeletonCOM+3d20 belongs to LandingOnBoard and is different.
Initial height is dot3(PhysicsRecordCOM29d0 - PhysicsRecord.positions[0]2550,Up).
PhysicsRecord base1f50 means2550 is its position array, not transform0.

Continuing normal animation height is PREVIOUS animation-record COM.Y at2a00
minus CURRENT mapped0.P.Y. The new animation record is produced later. Optional
physical-COM cvar is sampled on EVERY active call, before testing offboard->ground.
Its value affects only that transition: processedVelocity.Y*dt fused
into(PhysicsCOM.Y - processedDeck.P.Y). No WorldToAnim/root dependency is needed.

## Ordered stages to preserve

1. Maximum-velocity lookup always precedes transition tests. Disable clears
   adjusting immediately. Processed velocity dot then debug cvar follow.
2. Starts prioritize coffin, manual, then ground/grind. Store time0, adjusting1,
   type before initial velocity/position. Grind or coffin uses second-last
   velocity; other starts use current processed dot. Store lastAnimPosMAX before
   initial YPos from physical COM/position.
   A second debug read follows the start stores; it must not be hoisted before them.
3. Every active mode reads coffin duration and asserts only if it equals zero,
   even non-coffin modes. NaN passes the assertion.
4. Ground0/manual2/grind1 share spring math; coffin3 evaluates bounce graph.
   Desired grind height moves within retained desired +/- speed; speed is not
   multiplied by dt. Preserve source attribute acquisition/load timing.
5. Store YVel before spring debug read. Sample minimum-position attribute
   afterward and before YPos store. Apply first-frame ground correction,
   further debug read and termination conditions in source order.
6. Count service is read before prepare-jump override. Processed flags8c4 bit
   200000 clears adjusting and replaces the already-read count with6.0.
   Then write offset P.Y/YJustSet/YCount, followed by time/lastAnimPos/history.
   Other offset P lanes and full-transform count/flag remain separate.
   Active Time/LastAnimPos still commit if this call just deactivated the spring.
   Final common history stores secondLastCOMVel before lastFilteredState.

Fixed dt is3c888889 at82261f04. Spring displacement=rounded(YPos-desired);
elastic=rounded(displacement*K1). Damping/elastic acceleration uses original
fnmsubs ordering. New velocity=FMA(acceleration,dt,oldVel), then source fsel
clamps (ground upper0, other spring modes upper maximum velocity). Position
uses rounded((newVel+oldVel)*0.5), fused*dt+displacement, then separately+desired.
Do not substitute Rust clamp/min/max or algebraically reassociate.

Late blend gate41030=40980018 is BO4 !CR6.LT: NaN blend is eligible when
termination is false. Do not encode that gate as blend>=0.05. Ground recovery,
duration and grind-target-time tests use ordered strict comparisons.

## Runtime attributes and cvars

All attributes here use global physics_animation default+CC. Scalar missing
pointer fallback is DefaultDataArea4 zero. Graph4 fallback is DefaultDataArea30
zero; metadata is ignored, X at10 and Y at20. Keep lookups lazy.

| Attribute | Key |
|---|---|
| Maximum velocity | 5199e2e749e7220b |
| Coffin duration, mandatory active read | 1d4c65d8fd179fe2 |
| Coffin velocity denominator | 4f4e3f4566ec34e3 |
| Coffin base | 2f996144e5ef4eea |
| Coffin bounce graph4 | 25e6e31bc2841e1f |
| Spring desired COM | 9091e734f21a004d |
| Minimum Y position | 2a690d7e8c1edcf4 |
| Ground minimum elapsed | 01758b419109fdae |
| Coffin frames | 9abaff270d53f138 |
| Grind blend graph4 | 804ce67ea03946fa |
| Grind target speed | 74e3fc7cdd2f8c97 |
| Grind K1 | 10989560be00f690 |
| Grind K2 | bf5036b2311179e8 |
| Grind target after time | 8dac44f903262ddb |
| Ground K1 | ac532ffcdeba62d8 |
| Ground K2 | 2471dd04ee7ee637 |
| Manual K1 | 01ba609bb65cadb5 |
| Manual K2 | 475a3cb859bdb0a6 |

Manual blend is different: raw4096c loads cached instance layout+4 then field140,
PDB LandingAdjustManualBlend:PointNegGraphData4. Full layout size220; X at150,
Y at160. No generic hash lookup occurs at this stage. Source generated ctor
828b0f00..828b0f78 SHA965b5302e371771e69a9788925af21049eced965cdca2bceb75d8c500d8bc434
loads layout+4 at0f44, compares null0f48, skips fallback with bne0f4c, requests
DefaultDataArea220 at0f50/54 and stores+4 at0f58. Root independently read raw
instructions and first220 bytes83059d30: all zero, SHA256
44ddd2f478477ebd1c1cd5b99400af48cd46033c59173195f48870e608cec810.
Use actual cached-layout route with proven generated null fallback, preserving
the global/Skeleton instance distinction introduced in checkpoint68261.

| Cvar | Reference | Constructor default |
|---|---|---|
| DebugLandingAdjust | 8308ec08 | false |
| DebugUsephyscogland | 8308e714 | false |
| LandingAdjustFirstFrameHack | 8308eca0 | bits3c75c28f |
| FramesToBlendAwayOffset | 8308e9d4 | 15.0 |

Independent review confirmed all ten debug draw calls resolve to825187c0=BLR
and all four cvar constructor defaults. Retain actual singleton availability/
assertions. Do not invent renderer work as a dependency for proven no-op calls.

Suggested API: non-Clone SkeletonLandingAdjust with new/try_update, dedicated
lazy attribute/cvar traits, actual physics and previous animation-record borrows,
current mapped0 and existing mutable offset. Preserve typed attribute, cvar,
debug-singleton and duration-assert failures with all completed source stores.

Validation must cover all four start priorities, processed-vs-retained velocity,
previous animation COM, multiframe spring history, first-frame correction,
grind timing, coffin graph/division, all lazy failure prefixes, count-before-jump,
NaN blend eligibility, W preservation and existing mapped-deck/four-IK consumers.
Original active values/manual layout and runtime traces remain to validate.

## Implementation and accepted validation

`skeleton_landing_adjust.rs` now owns all nine retained fields on the existing
`SkeletonCurrentAnimation`. There is no second landing history or production
state-seeding API. The current-animation call borrows the same physics record
and previous animation record, runs the spring, applies the shared offset to
mapped volume0 and four IK targets, then updates the animation record and root
input. Teleport's existing projection preserves the spring state.

`skeleton_landing_attributes.rs` retains a distinct global generated instance.
Its16 scalar requests and two keyed graphs remain lazy; the manual graph uses
the cached layout directly. Missing storage uses proven zero defaults, while
malformed or unsupported found storage remains an error. Immutable allocations
still exclude editor mutation and native alias races.

Independent review passed for production code,21 kernel tests,
eight provider tests and the shared integration. Tests discriminate start
priority, actual record ownership, all service failure prefixes, clamp/NaN
branches, grind timing, coffin graph/count behavior, and prepare-jump ordering.
The cancellation fixture yields YVel bits458007de; defusing its operation gives
45800800. A separate late-debug failure preserves source outer-negation's
negative zero80000000. The two-frame current-animation test covers all four
modes while changing the new pose COM, proving that the spring consumed the
previous COM and published the adjusted deck/IK/record afterward.

The ten debug draws are no-ops, but the source performs eight singleton
acquisitions: one shared by the initial three draws, two at start, three in the
spring and two before termination. The checked host retains those failure
boundaries without introducing ten invented singleton acquisitions.

New physical_player_landing_spring_corpus runs after the existing active
LandingOnBoard helper and before second postRecords. It uses actual disable/
history transitions into all four modes, continuation and prepare-jump count6;
no test state seeding, replacement owner or extra physics solve is introduced.
All16 scalars and three graphs are compared with original storage, including
cached manual layout. Independent finite f64 equations check retained spring
fields and Y output; that verified Y plus the actual retained full transform
feeds exact mapped0/four-IK/24-row/COM/delta/root-input checks. Full rotation
decay has prior coverage, not a new independent oracle here. Retained physical
postRecords velocity remains unchanged by these controlled animation ticks.

Original run97475/89d39f passed all nine tests. Initial strict Clippy14343 found
documentation/test lint issues and the intentional source sum-half operation;
fixed documentation and test style, retained production arithmetic with a
localized explanation. Rerun83576/0a515a passed. Final strict17769/60e4f5 EXIT0:
fmt,2843 units (2615core+187formats+38app+3symbols), six docs, all-target Clippy
-Dwarnings, all nine original core and diffcheck PASS. Nine core/one app are
ignored in ordinary units; all nine core originals ran separately. No source
numerical behavior fix was required during validation. These are host checks,
not Xenon per-tick captures or completed GeneralUpdate/IK.
