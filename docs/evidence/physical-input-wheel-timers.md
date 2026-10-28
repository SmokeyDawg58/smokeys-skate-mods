# Update1 wheel flags and skitch timers

Scope: `Sk8::PhysicalPlayer::HiLOD::Update1_ProcessInput`, native
`82e03130..82e03500`, immediately after the prior Skeleton/Filtered/OffBoard
copies and before GroundPosition at `82e03504`. Confidence **high**: raw control
flow, offsets, stores and float predicates reviewed; no native deterministic
per-tick trace yet. This does not complete the entire physical frame.

The implementation uses the same bound `PhysicalPlayerPhysOut` occurrence and
the existing `PhysicalHiLodPrefix`; it adds only the processed destinations
vehicle ID `+930`, reskitch delay `+a1c`, and repush delay `+a20`. Reset
`82b39540/82b39590/82b39594` writes zero to these fields. A generic uninitialized
processed owner retains unknown values. No duplicate AI flags, state counters,
vehicle owner, or prefix timers are introduced.

`3130..32f8` independently guards Collision for each reached wheel operand:
`w2 || w3`, `w0 || w1`, `w2 || w0`, `w3 || w1`. Each pair short-circuits;
later pairs read again. All four results are computed before flags are stored.
`332c/3358/3380/33a8` independently reread processed board-flip bit `8c4/100000`
and store `8c8/10000000,08000000,04000000,02000000`. Without flip these are the
four results in order; with flip they are swapped pairwise. Neighbor flags
remain unchanged.

`33ac..33dc` guards State and compares its actual raw state to SKITCHING 104,
not the category or the separate current physics-state owner. On this branch
the live cvar is sampled at `33ec..33f4` into prefix `+168`, then a new State
guard precedes reading `grab_spline_desc.object_id` into prefix `+164` at3420.
The descriptor stores this field as `i32`, while the retained vehicle ID uses
`u32`. Native `lwz341c/stw3420` copies the raw word without numeric conversion;
the Rust handoff reinterprets all 32 bits, including negative descriptor IDs.
The cvar is `kTimeToAvoidVehicleAfterSkitching`, reference `830e0bc4`,
initializer `82f9e908`, registration table `82fd62b0`, section
`Physics_States_Skitching`, title `TimeToAvoidVehicle`. Raw default is
`40000000` (2.0); editor range 0..10, null callback. Startup serialization is
unclamped and the runtime reads the linked target only on the reached branch.
The graph agent independently verified this initializer and registration.

All five floating branches are **NOTGT**, not ordered less-or-equal:
`342c=4099001c`, `3478=40990008`, `3488=40990024`, `34dc=40990008`,
`34ec=40990018`. Their bodies therefore require ordered `value > 0.0`.
The avoidance timer decrements by a fresh processed `+93c` dt and publishes
the retained vehicle ID even if subtraction crosses zero or produces NaN.
The independently guarded Ground `+114` input replaces the retained reskitch
timer only when positive. A positive retained timer is decremented, then copied
to processed `+a1c`, then the retained vehicle ID is copied to `+930`.
The independently guarded Ground `+118` input and prefix repush timer use the
same ordered-positive gates, ending with the processed `+a20` store at3500.
No subtraction is clamped. NaN/zero/negative incoming values preserve retained
timers; NaN/zero/negative retained timers skip decrement/publication.

Integration API is
`SkeletonProcessedContact::try_copy_prior_output_wheel_timers(&owner)`, matching
the immediately preceding body-copy API. The root caller retains the original
Update1 continuation and invokes these stages on its same processed/physical
owner pair. Errors preserve preceding writes; a local completion marker only
covers this consecutive segment.

Focused tests exercise actual constructed PhysOut and processed owners for
board-flip mappings, neighboring flag preservation, default/non-skitch cvar
nonaccess, missing skitch cvar failure, live unclamped tuning, all timer edge
classes, expired negative values, and positive timers with NaN dt. A separate
fallible reader test proves fresh wheel operand order and short-circuiting.
Tests are implemented; the root integrator owns Cargo and original-corpus
validation. Host floating arithmetic is not a native numeric-parity claim.
