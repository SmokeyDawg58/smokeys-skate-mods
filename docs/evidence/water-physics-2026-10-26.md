# Optional water physics: PC-authored gameplay

The Water Physics selector is off by default and saves in a separate water-physics.json, so older EXEs do not have to parse a new field in their strict gameplay preferences. This is an authored mod, not a recovered retail Skate 2 or Skate 3 feature.

The visual rain solver publishes local board-wheel water-depth samples to the local Skate 3 simulation. Samples must be recent, for the current weather mode, and near the current board. A sampled wheel touching at least 3 mm of water retains 15% of its ordinary static-world contact friction. Other skaters, props, dry wheel contacts, replay and airborne board physics are not deliberately modified. Uneven wheel contact can produce natural instability through the existing solver; deeper water adds bounded drag and, after a short period of moving through at least 14 cm of water, requests an existing native wipeout.

Footprints and water depth are from the local visual height field; this is not a hydrodynamic simulation and does not establish physical parity or network-wide synchronized weather. The current implementation is limited to the Skate 3 simulation. It does not work in Skate 2 physics, and a missing or stale field deliberately produces no effect.

Validation must include enabling and disabling the setting, actual contact and wipeout traces, an unaffected dry baseline, and user review before this feature is shared.
