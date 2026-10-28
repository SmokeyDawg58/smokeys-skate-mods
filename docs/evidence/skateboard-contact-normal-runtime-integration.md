# Actual board contact to ground-normal output

2026-09-08. Root integration; authors and separate source
review. Confidence: high at the selected semantic scope, not Xenon bit parity.
This extends the activation/compression checkpoint48385/ec0b04.

## One producer and actual owners

`physical_player_board_postphysics.rs` operates on the existing
`SkateboardContactSurface` Arc owned by the actual board. It validates the
completed solver contact list, runs actual Part activation and compression,
obtains the native Island collision-list entry, and traverses that same list
once. Its selected endpoint is the ground-normal commit at82b37498.

The stages preserve the seven interleaved validity/debug initializations,
pre-filter contact-speed and opposing-normal effects, actual other/this Entity
and Assembly group reads, group4 deck-body geometry, first/strict-Y winner
stores, missing-wheel line fallback, seven current-body velocity updates,
stair/grip flags, surface vote, and normal accumulation with per-wheel actual
inertia angular-drag stores. Relative velocity remains unknown after Reset and
is written only by a winning contact. Native XYZ-only contact/body producers
retain unknown W in full-vector stores and arithmetic.

Four validity bytes and seven BE physics words share one backing. Validity
indices4..6 alias the first physics word; these are not independent booleans.
Legacy surface projection values are synchronized with these writes, so the
existing vote does not silently consume an obsolete physics word.

Debug colors bind the exact immutable authored collision-definition Arc and
actual wheel/truck/variable deck ranges. The 32 white constructor words come
from initializer82f75a08..20, then ctor38418/41c; static target83073a80 alone is
not the initialized evidence. Native idle/contact color stores need no renderer.
The temporary debug Instance collaborator is still outside this projection.

Selected CollisionInfo Reset executes after one-shot body entry and before
the island lookup, matching call82b382dc before82b382e8. A routing refusal keeps
those writes. Remaining constructor grip/free-wheel fields are retained before
Init execution. Both bits40/80 clear at387c4; grip vector/cosine are zero from
383a4/a8. Later controller mutation remains a separate required producer.

## Runtime data, publication and provenance

Normal angle comes from the actual global `physics_reckoning.default` provider:
capture, full-key0e508708cc7f5249 lookup, and scalar decode remain separate.
The original80-degree word appears only in corpus assertions. Foreign equal-
value provider tokens refuse. Existing standalone cosine, unsigned-angle and
estimate/refinement implementations are reused. Mutable singleton replacement
and Xenon seed/dot/NaN-payload behavior remain existing explicit boundaries.

Actual cvar targets are staged caller capabilities. The corpus supplies explicit
source-default diagnostic cvars; it does not implement live console registration
or imply those values are hardcoded production tuning.

Successful Start, a new actual solved frame, and successful End are associated
by a same-board host provenance record. It adds no native Sync/GetResult calls.
The normal endpoint rejects stale wheel results and repeated frame attempts.
It issues its normal-frame marker only after every required stage succeeds.

The selected pair FillPhysOut bridge copies last-valid twice: GroundUp30334 and
LastValidGroundUp30370. Each copy separately samples its latch, reads the source,
then checks that sampled latch and stores. It preserves the first store on a
second-read failure and does not reuse the collision-surface helper's different
guard-before-read order. Pair/current-frame provenance is checked; DynamicUp
and preceding/following Fill stages remain outside this bridge.

## Validation and remaining scope

Focused tests cover Reset/alias semantics, failure prefixes, unordered gates,
unknown W, group/debug ordering, zero-length fallback estimates, motion updates,
angular-scalar-only mutation, and separate output reads. The original corpus
runs the actual solved board path, checks compression with independent f64
Cramer's-rule projection, verifies actual angular drag values and both output
copies, and exercises replay and subsequent-frame refusal.

Strict checkpoint33750/cd8fdc passes all-target Clippy and original12;
workspace45647/0dd2ba passes3,333 units (3,090 core) and six docs; fmt/diffaa80dd
passes. Independent source/code/integration review passes. Logs:
`%TEMP%/skate2-contact-normal-workspace.txt`, `skate2-contact-normal-clippy2.txt`,
`skate2-contact-normal-original-final.txt`.

The final original fixture reports one contacted Part, zero contacted wheels,
and updated_last_valid=false. It exercises the real fallback path and retained
last-valid output copies; it does not independently demonstrate a newly changed
last-valid normal. Focused source tests cover successful wheel-normal commits.

This work
does not finish time-since-wheel/truck continuation, complete FillPhysOut,
GroundAnimation jump orchestration, Actor/input-history scheduling or portable
presentation. The playable goal remains active; broader DMO remains parked.

See the raw/source ordering in
[the BodyPost plan](skateboard-body-postphysics-runtime-plan.md),
[contact-state audit](skateboard-contact-state-extension-plan.md), and
[body-transform evidence](skateboard-body-transform-source.md).
