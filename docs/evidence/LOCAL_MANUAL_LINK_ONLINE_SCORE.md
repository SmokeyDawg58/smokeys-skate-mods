# Local manual links and online score cancellation

Preview13 stopped crashing, but the owner reported that manual-to-manual trick
links lost their scores and online challenge points did not count. The owner's
run1792935482647662500-8672 (EXE6c58c16a...) recorded repeated
skate3_darkcatch_landing_rejected and skate3_gameplay_reset events. At source
tick219, controller buttons were0 (RB released) and the solved board up-Y was
0.8777, yet the rejection reported held=true and cleared1267.5 live points.

The admission adapter physics/frame.rs treated deck-up dot physical-Hips-up<0
as an additional held button. The mirrored native Hips basis is not world up;
this also corrupted earlier sound contact admission and was already removed
from that separate audio path. Here it interpreted upright regular trick
landings and manual connectors as retained darkcatches. Orientation also cannot
establish that a player is still holding the catch control.

The online rule now reads only the actual RB sample or explicit DarkCatch
intent as held. The confirmed-in-air catch history, touchdown ordering,
release-after-invalid-landing protection, teleport reset and offline exemption
are unchanged. World board up-Y is recorded only as diagnostic context. No
points, multipliers, animation timing or challenge/network scoring rules are
altered. The previous untyped-revert crash guard remains in place.

Confidence: high for the observed false cancellation. Challenge-score recovery
must be checked through the production local-player score publication and the
online challenge aggregator. Public online play remains an owner review step.
Validation is restricted to focused real-data and short silent integration
checks with copied saves. Keep this repair local until owner approval.
