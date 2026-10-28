# Grounded foot producers through the retained deck force queue

2026-09-08, implementation and separate independent review.
Accepted checkpoint **44200/74ab79**, following IK checkpoint 96455/672297.
Workspace48531/ffa3c8: 3,183 unit tests and six doc tests pass. Originalffe51c:
all ten pass. Strict all-target Clippy44200/74ab79 and fmt/diff8902b1 pass.
Subsequent brake/drag/slide/collision additions are outside this checkpoint.

## Implemented path

The actual physical-player factory now retains a bounded Skateboard force
owner separately from SkateboardBodyConstruction. Queue initialization and
the own physicsdeck binding precede body construction. The same boxed owner
moves into the actual board candidate and survives failed admission/removal;
legacy body-only candidates have no implicit replacement runtime.

Front/rear foot calculations use the retained global physics_feet.default
reader, preserving pointer/value timing, balance NaN branches, fused arithmetic,
normalization and partial output writes. The actual original values are read
through Vault, never substituted as runtime literals. Unknown vector W remains
unknown in queue records.

Add retains at most 21 full force/local-position records in original order.
Apply skips attributes and body callbacks for an empty queue; otherwise it reads
the own offset, captures the entry range, resolves the actual attached Part 6
body for each entry, and applies cached inverse-mass force and full rotated
inverse-inertia torque to that same rigid-body pool. Force stores precede the
late tensor/torque reads; torque stores Y/X/Z precede cooldown zero. The final
attribute operation is lookup-only and occurs after all body effects. Errors
preserve completed prefixes. Applying does not clear the queue.

The own physicsdeck class 9ed291a274453fc0 binds collection
550740b00aa8d4ac, not global default d7edbd362d7d2152. In original data its
offset key is absent and resolves to positive zero. The global default's
approximately 0.05 value must not be substituted. See the independent raw
constructor and archive proof in [queued Apply source](skateboard-queued-apply-forces-source.md)
and the [foot source](skateboard-ground-foot-forces-source.md).

## Validation and limits

- Foot-focused compilation/tests: 13 pass; mounted original foot attributes:
  one pass, exact five archive words independently verified by reviewer.
- Broader skateboard-focused run 82828/6bbba0: 137 pass, three ignored. Subsequent
  diagnostic Display changes are awaiting the complete validation below.
- Independent source/code review passes for foot kernels/providers, own
  attribute binding, retained queue/owner, actual body consumer and bridges.
- New mounted component proof uses the same original physical pair, retained
  Reckoning and Vault with explicitly controlled processed balance/speed/bodyY/
  velocity inputs. It checks all body slots against an independent numerical
  oracle after two applications of the same pending queue, then explicit reset.
  It passes in the ten-test mounted run ffe51c. Initial e7534b run had nine
  pass/one failure because the paired fixture's selected-class load omitted
  physics_feet. Adding that original class to the same retained Vault fixed
  the fixture; no force math, runtime tuning or missing-pointer default changed.

This completes neither all six GroundAnimation producers nor pre/postphysics
scheduling. Braking, drag, slide and collision producers remain next. The
preceding wheel-line stage is not skipped by a claimed full PrePhysics entry:
the new entry is explicitly ApplyForces only. Reset is explicitly callable at
the postphysics boundary and is not called by Apply. Live Actor/current
processed-input production, controller history/gestures, animation feedback and
the complete playable loop remain unfinished. Immutable Vault bindings do not
model database/editor reload or alias-driven queue mutation during lookup.
Host floating-point services retain previously documented Xenon trace limits.
