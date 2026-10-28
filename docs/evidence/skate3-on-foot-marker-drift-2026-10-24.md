# On-foot marker return: discard the reset tick's board velocity

Contribution: Tally's `1-marker-on-foot-no-momentum.patch`, supplied by the
owner on 2026-10-24 (original patch commit `20e6f82db8acedc7b2cdce664c704168da9f6fcf`).
The contributor reported approximately 1.469 m of unintended movement before
the change. That number is supplied evidence, not an independently measured
baseline for this integration.

The PC host's explicit off-board reset passes through `PhysicsGround100`
before entering the biped ground state. `biped_ground::enter` supplies the
processed velocity at +608 to the recovered ground placement. The reset tick's
board velocity must not become the returned on-foot character's momentum.
The existing `player_state::resume_after_climb` already clears the world-motion
velocity and processed +608 lane before re-entry.

The fix applies the same clearing only while
`teleport_state.manual_on_board() == Some(false)`. The target survives the
intermediate ground reset and is cleared when the off-board family is reached
(`finish_manual_reset`). Normal dismounts, airborne landings and on-board marker
returns retain their existing velocity policy. This is a PC host integration
policy with high confidence from its state ownership and regression coverage;
it does not establish new console parity for the reset bridge.

Validation requires `skate3_offboard_marker_return_does_not_drift`, using the
real map and embedded Skate 3 player assets. It covers both an idle character
and walking immediately before returning, then checks neutral-input movement
after the reset settles. The release gate also retains the walking-after-return
test, overlapping online-reset test, existing physics features, actual-game
marker checks, audio checks and public online sign-in. All diagnostic saves are
scratch copies. Results belong to the matching release's `VALIDATION.json`.

The separate prop-collision patch mentioned in the contributor's README was
not supplied and is outside this change.
