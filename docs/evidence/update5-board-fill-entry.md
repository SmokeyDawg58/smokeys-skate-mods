# Ordered board FillPhysOut entry

Registered, pending gates, September 12, 2026. Confidence: high for recovered control and stores;
host VMX estimates remain an explicit numeric collaborator. No full Update5 or
physical-state publication is claimed. This packet covers
`Sk8::Physics::Skateboard::FillPhysOut` 0x82b2f3e8..0x82b2f7f8, called by HiLOD
Update5 at 0x82e04ef8, stopping before Pumping at 0x82b2f7fc.

## Retained owners and source order

The caller consumes the existing `Update5BeforeBoardFill` token and verifies its
actual physical-owner Arc against the same board/skeleton pair. It writes the
same retained PhysOut. It does not construct a replacement output or select a
physical state. Native entry captures the Part6 rigid body at f43c, before the
first position call. That body is retained for the early linear and late angular
reads, even though intervening transform calls are fresh.

1. Body.GetPosition(6), 82de7b20, at f440 delegates to Part.GetTransform.
   After the write assertion, stores f490..f4ac copy four columns to
   SkateboardReckoning +0. The existing live Part getter includes the authored
   inverse-body transform; the raw rigid-body pose getter is not equivalent.
2. A fresh output guard precedes GetEffectiveTransform f4dc, then copies all four
   columns to +40. Another effective-transform call f510 precedes the next guard;
   its Z column is stored to deck_nose_dir +80 at f54c.
3. Body+20 linear velocity is sampled once at f550. Effective-transform calls
   f554 and f568 independently supply Z and X. Dot3 at f56c consumes Z and the
   retained linear XYZ. BLT f59c compares with bits b8d1b717: ordered less selects
   -1, every other result including unordered selects +1.
4. The late Processed+8cc word read f5b0 supplies bit8 for the independent look
   sign. The output latch is sampled at f5d8, before the two full-vector
   multiplications f610/f614, and checked afterwards. Skater look direction is
   stored at +b0/f638; another guarded store writes velocity direction +a0/f678.
5. Cvar velocity direction is read first, then look direction. Each enabled
   branch performs two fresh effective-transform calls and only then checks the
   actual DebugOverlay singleton. Fresh raw f68c..f714 contains no draw call.
6. Existing motion writer f714..f7f8 samples its latch, reads Body+30 angular
   velocity, asserts, and stores Motion+40. A fresh guard precedes dot3 of the
   retained early linear velocity, then Motion+50 receives that original vector.
   The final latch sample precedes unconditional rsqrt estimation and refinement;
   the resulting scalar is stored to Motion+a0 after its assertion.

GetEffectiveTransform 82b23778 first calls the actual Part getter, then reads
Processed+8c4 mask00100000 and XORs the sign bit of all X/Z lanes when set.
This is distinct from the +8cc bit8 direction read. The implementation extracts
the already reconstructed board-entry getter for shared use; it does not cache
one transform or flags snapshot across calls.

September 12 correction: the older host Part getter returns an XYZ/canonical-W
projection, not the original packed fourth lanes. Promoting that result with
`SkeletonRootMatrix::known` was an overclaim and is removed from both the
effective helper and direct BoardFill read. New `try_live_part_transform_typed`
preserves the actual detached typed cache, while attached bodies retain the
existing XYZ computation and mark all four W lanes unknown. Effective flips
map known lanes only; copies, nose extraction and direction multiplication
propagate unknown W. Dot3 consumes XYZ and needs no guessed W. Exact packed
address-image publication remains a separate producer; this change supplies no
canonical 0/1 or invented pointer bits. Tests discriminate attached inverse
composition from detached known/partial cache and verify output W propagation.

## Live tuning and debug owner

Both boolean cvars have default false and null callbacks, confirmed by raw
initializer r7=0/r8=0. They belong to `Physics_Debug`:

| Label | Title | Initializer | Ref | Table |
|---|---|---|---|---|
| kSkaterLookDirection | Show Skater Look Direction | 82f7af88 | 8308dd8c | 82fd4400 |
| kSkateboardVelDirection | Show Vel Direction | 82f7afd8 | 8308d94c | 82fd4404 |

Registration follows native table order, while runtime sampling follows velocity
then look. Existing world DebugOverlaySlot is required only in the selected
enabled branch after its transform calls.

## Numeric and provenance boundaries

Body velocity readers retain XYZ and unknown W because the native fourth lane
is an intrusive pool word. Copying that vector does not justify synthesizing a
numeric W. Direction vectors instead preserve all known/unknown transform lanes
through the native multiplication. The actual contiguous source has no
contact-spy frame prerequisite: f550 and f718 are direct reads of the captured
body. The sealed sample retains one body/output/linear payload, with optional
frame provenance. The existing selected sampling API still requires a real
current frame and stores Some(frame), preserving all its validation. The new
contiguous sampler stores None, validates the actual body's pool occurrence and
integrity, and directly reads velocity. Publication always validates body/output
identity; only Some(frame) invokes additional frame checks. It publishes no
contact-frame provenance for the contiguous route. No frame is fabricated and
no frame-less output is advertised to later consumers as a contact-spy result.

Raw f7b4=158f6971 and f7c4=154e6171 are VNMSUB, each computing
`-squared.mul_add(inverse_square, -1)`. The shared older helper instead fuses a
negated input, producing +0 rather than native -0 for an exact product one.
The board-specific length helper preserves final negation and leaves the shared
numeric helper untouched. Each refinement separately rounds inverse squared and
inverse*0.5, then fuses the residual correction. Final squared*inverse is rounded;
ordered squared==0 selects positive zero only after both refinements.

## Validation scope

Focused cases cover distinct effective transforms per call, late look flags,
full W and signed zero, unordered projection, early locked-output behavior,
missing frame/linear-source failure after transform stores, enabled overlay
failure after two additional transforms, and native negative-zero residuals.
Existing motion tests cover late angular reads, retained early linear data, and
late numeric failures. Tests await root's combined gates. Original-data and complete-frame
validation remain outstanding.

Root independently reviewed all transform guards, sign selection, debug calls,
and both raw VNMSUB words on September 12 and approved the local helper switch.
Root also reviewed the frame-provenance distinction and approved removal of that
host-only prerequisite from the actual contiguous direct-body reader.
