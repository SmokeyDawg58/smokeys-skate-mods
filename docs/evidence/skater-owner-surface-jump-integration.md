# Retained skater state, surface motion and jump input continuation

## Accepted behavior and source

The selected HiLOD Reckoning constructor (82e03ff8) now executes on the same
physical-player owner before outer Skateboard construction. That owner retains
one processed-input allocation and one Reckoning allocation through board and
Skeleton use. The prior diagnostic Reckoning/processed copies were removed.
Successful ground updates issue an occurrence token; refusal/reset invalidates
it. A Reckoning phase precedes physics and is not a contact-frame certificate.
See physical-player-reckoning-owner-runtime-plan.md for ordering and exclusions.

Selected Skateboard Fill surface motion (82e2fdd8..82e3004c) now consumes that
retained Reckoning, the same processed raw deck transform, and actual Part6
motion published into the pair PhysOut for the current completed solve. Ground
projection, the deliberately discarded NormalizeSafe result, sign selection,
late ground transform read, angle wrapping and separate publication guards
follow the recovered source. W stays unknown where its producer is unknown.
The next prior-output copy reads these same publications into the same processed
owner. See skateboard-physout-surface-runtime-plan.md for raw-source details.

The original Rob's Park fixture relocates the registered board using actual
terrain query geometry and uses the existing real seven-Part linear-drag setter
with zero as an explicit diagnostic input. It injects neither contacts nor body
velocity. The resulting actual wheel contact is checked. In checkpoint
81139/075420, actual DeckVelocity was [-0.120835714, 2.1052477, -0.42021823],
SurfaceVelocity [-0.62931615, 0.16491818, -0.42021823], SurfaceSpeed 0.77448064,
and SurfaceAngle 1.1943473. Projection and magnitude agree with an independent
f64 oracle within the documented tolerance. Controlled Reckoning inputs,
BoardFlipped=false and drag remain fixture inputs, not a complete Actor tick.

Raw type-zero jump/prepare dispatch and late runtime Vault reads now belong to
the same processed owner (82b59de8). Seven focused tests cover source ordering,
partial stores, nullable input and floating-point branches. Actual Mg/Andale
raw-event production and input ownership are not yet connected. The selected
jump flag projection does not claim the other processed flags are established.
See skater-jump-event-runtime-plan.md.

The latest Skeleton adjustment addition retains one non-Clone pair of fields
through construction and admission, initializes positive zero after successful
actual IK, and preserves both fields across ResetSkeleton. Source ctor stores
82b57478/82b57480 follow IK82b5743c and omitted wobble82b57468. Original globals
bind anim_motion.jumping and physics_jump.default; binding default instead of
jumping for the former is incorrect. Independent raw-source review
accepted these changes. See skeleton-jump-bridge-runtime-plan.md.

## Validation boundary

Owner/surface/jump-dispatch checkpoint: workspace85967 passed 3,367 units
(3,124 core, 202 formats, 38 app, three symbols) and six docs. After style fixes,
strict all-target Clippy and all twelve original-data checks passed
81139/075420, including the moving surface fixture. The new Skeleton adjustment
constructor and fallible controller adapter are undergoing a fresh full gate;
these earlier counts do not certify later additions. Three adjustment tests
passed in56973/f26e78.

Confidence: high for recovered selected source paths and controlled actual-data
execution. No native per-tick trace or Xenon-estimate bit-exact claim. Full input
history/gestures, Actor phase scheduling, all processed flags, full Fill,
launch/trajectory integration and portable presentation remain required.

## Accepted input/jump integration checkpoint

90366/fe4900 passes strict Clippy and all12 original-data checks; workspace
92865/9ff6fa passes3395 units (3152core+202formats+38app+3symbols), six docs.
Fmt/diff690e9c pass. This supersedes the pending constructor/adapter gate above.
The actual original-pair corpus now passes retained jump globals, same Skeleton
Prepare74/75 reads/stores, actual processed strength/adjustment copies, reset
before missing-human refusal proving PREPARE clear-to-set and both Skeleton
adjustments preserved, plus actual Hippy event override (.125 versusminimum.25).
The selected wrapper handles only recovered jump/contact records; other native
attribute effects and Actor raw stream generation remain unimplemented.

Input modules parse original Xbox config, convert raw integer channels with
source radial math, retain actual pad Value/held/pressed/repeat state, and feed
original mapped RawControllerInput. Six cache tests cover constructor/Init,
collapse, 30-slot overflow alias, separate SIM/PRES owners and partial failures.
The original-data input fixture explicitly schedules four pad updates and
keeps device/remap assumptions diagnostic. The separate command-produced Actor
input owner discovered in input-actor-next-closure.md is required next; do not
wire the SIM cache's pad into Actor or claim its history is interchangeable.

Future prepared-history files currently have source review only. Remaining
input matchers/history/listener, generated graph execution, launch/trajectory,
full physics input/output stages and renderer/camera still block hands-on parity.
