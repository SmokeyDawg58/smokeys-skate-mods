# Actual wheel material and retained ground-query integration

2026-09-08, implementation and separate independent review.
This packet records bounded runtime integration, not a playable Actor tick.
Final validation results are recorded below once all gates finish.

## Material producer to actual contact consumer

The actual physical pair retains one SurfacePhysics construction, using the
existing Vault-backed source constructor. Its five stable material owners issue
sealed occurrence/slot/Arc tokens only after construction completes. Dispatch
extends the same processed owner already bound to that pair's PhysOut occurrence.
Equal coefficients from a foreign SurfacePhysics owner do not authorize writes.

SetWheelMaterialForSurface (82b29fb8) captures the selected pointer once after
actual Assembly resolution, then performs four separate actual Part stores in
source order. DynamicAssemblyPartRuntime owns the optional binding; immutable
template coefficients and Parts4..6 stay unchanged. Generic material M need not
implement Clone. Partial failures preserve preceding stores, and detach/reattach
preserves the Part's selected material.

Board collision Create/Update resolves current Part material and copies its three
coefficients at serialization time. The setter does not rewrite old records.
A shared material revision guards stale collision use until actual Update.
The host definition guard retains its previous precedence over the added material
owner guard; an original corpus exposed and corrected that diagnostic regression.

physical_player_wheel_material_corpus follows actual selected tokens into four
wheel bindings, serialization, and a real next Rob's Park ground solve. Test-only
observations come from the actual prepared/solved ContactJacobian batch, matched
to exact wheel body occurrences. Static/dynamic friction words are checked there;
restitution is checked in serialization, not claimed as a surviving Jacobian field.
Foreign equal-value owner rejection and unchanged publication records are tested.
The explicit diagnostic replacement helper restores the real selected token before
the solve. The same processed owner is returned by the earlier PhysOut corpus.

## Five queries across physics

simulation_world_line_query now exposes staged acquisition, Begin, Add, End,
Sync, indexed result and Release on its existing actual selected World owner.
The existing single-line route reuses those stages. Leases are occurrence-bound,
noncloneable and never implicitly released on Drop. Failed acquisitions retain
their earlier children. Zero flags trigger the source assertion boundary before
acquisition; unknown nonzero bits can produce an empty aggregate. A sole child
uses direct indexed-result semantics; aggregates guard each child's own count.
Add visits all children even after a capacity false; nearest ties preserve order.
Ground/DMO/optional vehicle order and nonempty-unpublished DMO refusal remain.

SkateboardBodyConstruction retains one wheel-line state with its actual board
lifetime. Start (82b24530) keeps the lease before Begin, samples actual Part
transforms0..3, adds four thin position-to-position-up*0.2 rays, then samples
Part6 and adds the world-down100 ray with fatness word3e19999a. The first four
capacity bools are ignored; the fifth is asserted. Its actual Part getter's host
affine W is not native evidence: published ray/deck-start W remains unknown.

End (82b24700) Syncs before resetting wheel height and each wheel's normal/distance,
preserves fsel NaN/signed-zero behavior, retains deck hit fields on a miss, and
clears the batch only after Release succeeds. Native surface IDs are full32 bits;
the current clustered-mesh source's u16 is explicitly widened at that boundary.
Review caught the initial narrow retained type; a high-bit regression covers it.

The original wheel corpus uses the same original-data Reckoning fixture and
selected World owner, validates the five actual Part-derived requests, observes
real batch results, applies the actual queue, runs a real ground solve, checks
that the same retained batch/results survived, then validates End publications.
The shared constructor fixture's explicitly diagnostic tests-per-batch was raised
from1 to5: the prior isolated IK capacity correctly rejected the deck addition.
This changes no production cvar provider and is not claimed as source-default
capacity recovery. Entity flag production and complete constructor chronology
remain outside this bounded schedule.

## Required fat geometry

collision_fat_triangle_line implements 82adeb48 and its source sphere/nearest-region
helpers, through the original 82adf658 wrapper. It preserves initial plane and
barycentric gates, native fraction comparisons, bounded five-region transitions,
overlap normalization, negative/NaN radii, partial stores, and numeric refusals.
The successful iteration-budget exit can lack fields needed by the wrapper;
that becomes an explicit unready-success refusal, never a fabricated hit.
The wrapper offsets the result by original line fatness, not summed radius.

Independent raw review corrected the nearest-face use of the sign-adjusted
determinant and shifted-U dot ordering before parameter publication. The numeric
refusal sweep distinguishes that early failure from the later V failure. Source
packets: skateboard-wheel-line-source.md, world-fat-triangle-line-source.md,
skateboard-wheel-material-runtime-plan.md. Existing host numeric estimate/reduction
limits remain explicit; these are not Xenon per-tick reference traces.

## Remaining integration

Complete the SAME board contact producer's last-valid ground normal, interleaved
BodyPostPhysics effects and motion output. Then finish GroundAnimation/jump state
and Actor/input history/gestures with portable presentation. Neither wheel queries
nor material selection alone completes those source schedules. Broader DMO work
remains parked; all current geometry uses actual retained owners.

## Accepted validation

Checkpoint **49472/bb2165**: workspace3,291 units (3,048 core), six docs; all11 original checks PASS, including actual wheel friction and positive fat deck query retained across solve. Strict Clippy96171/45bf85 and fmt/diff55b343 PASS. Independent source/code/actual-owner and corpus review PASS. Test-only completed-job memory readback preserves Running state and does not call native GetResult before Sync. Full playable goal remains active.
