# Local water-contact and grazing-reflection preview,2026-10-26

The owner disliked large stacked bright rings during skating and walking, and
water turning black at a low camera angle. This PC-authored presentation is a
visual approximation, not a recovered retail physics or animation system.

Water contacts remain gated by actual solved water depth and native actor/deck
pose. Approximate left/right foot offsets alternate only after enough horizontal
travel; low-speed animation jitter cannot emit repeated steps. Board contacts emit bounded directional, time-varying disturbances at the two
wet wheels of alternating axles and fine short-lived droplets. Walking steps
emit irregular local footprint movement and fine droplets, without ring waves. Airborne, dry, replay and
teleport paths do not create trailing effects. Foot position is approximated from
hips/bounds and velocity; native animated shoe contact has not been recovered.

For grazing views, the shallow/deep reflection contribution is bounded so the
underlying terrain remains visible; very dark legacy cube samples fade to a
subtle floor/overcast fallback. Native shadows, accepted weather lighting,
water-plane geometry, rain particles, clouds, bird/rain audio and volume controls
remain intact. This does not promise physical optical accuracy on every map.

Focused tests cover real water/dry/air contacts, idle jitter, carried-board
walking, and actual GPU pipelines; short silent local map captures examine
several starting locations and angles. No public publication.

A real-GPU fixture with a deliberately black environment cube and a low
camera produced a 10th-percentile water/floor brightness ratio of0.258 with
the previous shader, versus0.831 with the revised shader. It also exercises
1x/4x depth/stencil and a submerged camera. This controlled fixture is not a
substitute for native gameplay views across the full map.


## Contact visibility follow-up

The previous authored PC water interaction emitted wakes but its output was too subtle in ordinary gameplay, as reported by the player. This revision enlarges the localized directional wheel trails and irregular foot disturbances, and increases fine contact spray. It does not change the underlying rain, water material, lighting, audio, or native physics. These visual contact effects are an approximation, not recovered Skate 3 fluid simulation.

A contact-only sampling fallback handles a narrow visible interpolation seam when at least 75% of the interpolation weight belongs to native ground and the actor is close to that ground. Inferred render cells do not become collision geometry. Dry ground, missing terrain, another floor, replay, and teleports remain excluded by the contact gates. The player's review in the real game remains necessary to judge visibility and appearance.


## Submerged skater and contact-height follow-up

The prior presentation skipped all skater-stencil pixels, making submerged legs appear in front of water. Character pixels below a locally supported water surface now receive an undistorted partial water overlay; above-water character pixels remain protected. The contact solver interpolates the local surface height instead of selecting the highest neighboring water level. When available, local physical skeleton foot positions place the authored walking effect; bounded hips/bounds fallback remains available. This is visual rendering and does not apply fluid forces.

Controlled GPU checks cover both above- and below-water stencil pixels and preserve the silhouette. Synthetic adjacent closed basins at different heights both collect water independently. These checks do not establish every bowl in the streamed game world; investigating dry or apparently floating water in additional map geometry remains necessary.
