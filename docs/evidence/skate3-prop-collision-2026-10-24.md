# Skate 3 city-prop collision mesh integration

Integrates the owner-supplied `2-skate3-props-no-lip.patch` after the on-foot
marker-return fix. The earlier marker change is retained. The contributor
reports kicker ascent of 0.326 m before and 0.723 m after this patch, compared
with 0.639 m in the Skate 2 reference mode. These are supplied measurements;
they were not independently reproduced for this delivery.

The patch selects DMOs whose enabled aggregate volumes resolve to authored
collision meshes on one rigid body. It retains their finite-mass solver body,
uses the authored mesh instead of overlapping primitive contacts, and retains
the native triangle order, contact flags and edge features. Model-space mesh
decodes are cached; moving bodies re-place the triangles at their current
transform. Wheel, foot and camera queries use the same triangles. Other bodies
retain the existing solid-shape path, and unsupported mesh plans fall back to
the authored volume set.

This is a PC integration policy, not a new claim of recovered console parity.
Behavioral confidence for this specific release remains a hypothesis pending
runtime evaluation. At the owner's explicit request, this delivery is compiled
and published without gameplay or regression tests. No skipped check is marked
as passed in the release report.

The contributor's `skate3_city_prop_kicker_has_no_lip` corpus remains in source
for later use, and is included in the normal future automatic validation gate.
It spawns a large kicker through the living-world DMO path and measures ascent.
Its presence does not mean it was executed for this release.
