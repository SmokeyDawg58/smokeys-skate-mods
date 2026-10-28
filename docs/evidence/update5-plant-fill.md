# Actual HandPlant and FootPlant output callbacks

2026-09-13. Confidence high from fresh raw82db8580..86a0 and82db6890..6a50.
Four new unregistered Rust children and four tests; rustfmt passed. Independent
review and Cargo/corpus validation remain root gates, not author claims.

HandPlant call82e04f60 guards first, loads animation position5c0, captures output
flags134 before vector store85d0, copies time5d0, then separately rereads source
flags5d4 four times to replace bits31/30/29/28 in order. Output low28 survives.
One guard covers this structure copy. Fresh fifth flags read8610 precedes guard
8614 and publishes found flag1ae. IK flags9f0 load8640 precedes guard8644 and
publishes grabbing1b3. Final guard8670 precedes surface9c4 load8694/storeAir+d0.
Source floats/vectors preserve bits and W. No reset or hypothetical active gate
is introduced. Runtime reads actual HandPlant manager fields lazily per use.

FootPlant call82e04f6c reads resultsValid1d5 before any output guard and returns
if false. True guard/read/store order is time1b8->Air+c0, total1bc->c4,
landing1d6->1af; fresh landing||planting short-circuit before next guard/store1b0;
guard then surface1d0->cc; fresh planting&&(part1cc==15) before guard/store1b1;
fresh planting&&(part1cc==19) before guard/store1b2. All source fields already
exist. Missing optional surface fails at its late read after four publications.
Do not create a zero surface or missing prediction result. Existing constructor
and reset produce actual resultsValid=false; active Update producers remain
separate pending work, while the entire output callback handles active fields.

The private FootPlant child validates retained Weak owner and Skeleton identity
without constructing a substitute owner. Session validates world/pair identity,
executes actual HandPlant before actual FootPlant and returns
Update5BeforeControllerFill{owner,grind_policy}, preserving explicit upstream
grind policy. It never claims Controller.Fill or complete Update5 execution.

Root registrations:

```rust
// physical_player_phys_out_construct.rs
#[path = "physical_player_phys_out_plant_fill.rs"]
pub(crate) mod plant_fill;
// physical_player_footplant.rs
#[path = "physical_player_footplant_fill_access.rs"]
pub(crate) mod fill_access;
// simulation_assembly_constraint_session.rs
#[path = "physical_player_update5_plant_session.rs"]
pub(crate) mod update5_plants;
```

Integration: world.try_continue_update5_plants(board,skeleton,before_handplant)
where input is actual update5_managers::Update5BeforeHandPlant. Output import is
constraint_session::update5_plants::Update5BeforeControllerFill. Its two public
crate fields owner and grind_policy continue to the independently owned next
Controller.Fill adapter.

Tests discriminate separately reread hand flags vs whole-word snapshot, low28
preservation, vector W, foot invalid gate before locked-output guard, late missing
surface prefix and native left/right toe IDs15/19. No Cargo run by author.
