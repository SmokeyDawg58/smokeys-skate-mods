# Selected retained presentation runtime

The new `skater_presentation_selected_runtime.rs` extracts the already reviewed
selected path from the original-data frame probe. It owns the same retained
gather producer, numeric scratch and supplied-model `CharacterPalette`; it owns
no second animation, job manager, pose or inverse-bind allocation.

`try_new` takes the actual selected startup, supplied model scene and explicit
host LOD, submission/gather capacity, provider and opaque-W transport policy.
Model loading stays in `mounted_character_scene`; no environment, corpus helper,
assertion or neutral/default tuning enters the runtime. Existing host model
admission and once-zero retained backing policies remain as documented in the
palette and data-buffer evidence. This admits one explicit selected Actor, not
native roster filtering, interpolation, camera or whole-frame scheduling.

The shared patch registers the module and converts `SelectedFrameProbe` to a
driver of `try_update`. The order is unchanged: actual `WriteData827dbf98`,
retained backing read, existing `ReadSkater`/extract/end/sync, existing
`MoveRoot827deb24` then `Expressions827deb2c`, second ACS pass at
`827deb34..827debd0`, saved hips slot zero at `827dec00..827dec24`, model remap,
then the existing full-four-term upload packing from `828221e0`.

Numeric reduction/estimate, sin/cos, cvar and job services are supplied. The
test numeric wrapper delegates the existing host profile and records twist
length operands. Optional immutable observer calls preserve original checks
after production, decode, correction, final matrices and packed publication.
The `None` path performs no diagnostic reads. The authored source invariance,
channel finiteness, eight twist operations, saved-hips equality and every mesh's
finite published rows/generation checks remain in the corpus driver.

The runtime binds the actual hips Arc and exact base/replay allocation ranges;
another presentation is rejected before mutation. Its palette accessor admits
only a completed update; entering an admitted update hides the prior palette
until packing succeeds. Typed phase errors retain completed native stores.
Callbacks are host diagnostics, not recovered native behavior. No retry or
rollback semantics are invented for failed native phases.

Validation: draft rustfmt and shared-patch `git apply --check` passed. Independent
review requested; Cargo and mounted-data execution belong to the root integrator.

# Next producer audit (read-only)

Fresh IDA `GameInfo82559418` raw confirms `li r11,0` at `82559440` and
`stw r11,0x10(r31)` at `82559470`: signed sync level starts at zero. Actual
`CommonBackEnd` constructor writes zero again at `825623f8`. It also changes game
mode at `825623f0`; the existing selected static-field owner does not claim that
complete constructor. `CommandManager::DoCommand825e5610` reads the same
`sGameInfo83040500+0x10` lazily after `addToCRC`; signed level >=2 enables CRC.
This is not a cvar. Add the retained field/getter to existing `PhysicalGameInfo`
and adapt the same Arc, rather than carrying the corpus `OfflineSync(2)` into
production. Isolated CRC tests may keep their explicit level2 fixture.

Fresh freezing initializer `82f6a5d0` calls at `82f6a5fc`: reference `83052764`,
initializer table `82fd2248`, section `Physics_Simulation`, symbol
`kEnableFreezing`, title `Enable Freezing`, true, null callback. Raw
`Island::Step_Solver3` resets static RF at `827bae4c`, then reads the current
target byte +0xcd at `827bae5c`, conditionally calls freeze at `827bae6c`, then
contact spies at `827bae74`. Null target has no native fallback here.

`FreezeIfNecessary82e0be40` reads manager Params+0x24 (manager+0xcc), unsigned
threshold, separately from Simulation replacement cooldown. World Initialize
stores2 for both manager Params at `827b0f20`/`827b0f7c`; manager copies Params at
`82e0c364` and separately sets Simulation cooldown at `82e0c71c`. Existing pool
initialization's30 is not a substitute for this manager initialization.
The next bounded packet should retain that Params field on the existing
constraint/manager owner, initialize it through the actual selected World
producer, initialize the separate pool cooldown at the source constructor
boundary, and feed a live cvar/manager adapter to existing contact feedback.
No new camera, input or physics owner is needed. True default freezing must be
tested through the actual selected frame; the existing controlled-zero fixture
must not become production policy.
