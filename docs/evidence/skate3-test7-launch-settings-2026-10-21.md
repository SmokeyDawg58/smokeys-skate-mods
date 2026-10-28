# Test7 launch settings correction

The initial Test7 distribution failed at BootPrefix because its packaging script
enabled `all_skate3_physics` but retained `off` for `darkcatch`, `footplants`, and
`offboard`. `GameplayConfig::parse` intentionally rejects that inconsistent
combination in `crates/skate2-core/src/ui/gameplay.rs`. The existing parser
regression already covers a master toggle without its required features.

This was a packaging error, not a physics or animation failure. The prior
component tests supplied `Skate3Features::ALL` directly and therefore did not
validate the serialized distribution settings. The original package had not
received an actual executable startup check.

The installed Test7 `gameplay.json` now enables all four flags and preserves the
existing difficulty (`skate2`) and automatic airtime slow motion (`off`). The
package generator now writes all four flags together. No Rust source, executable,
skater profile, or game assets were changed for this correction.

The corrected packaged executable was launched with an exact copy of the
corrected packaged gameplay settings and a copied Test7 profile under a scratch
`SKATE2_SAVE_ROOT`. The `perf-skate` diagnostic used an offscreen background
window and stopped automatically after 600 completed gameplay frames. It exited
with code 0 in 22.51 seconds including startup; the log confirms 600 ticks and
10 seconds of simulation. A final rendered capture contains the actual skater,
city, and HUD. Both the original external profile and the installed Test7
profile retained their original hashes. No other game process was running.

Executable SHA256 remains
`cb363a082e1aa0b790bbf81d0c3db29e49957fff73ada585c9af9864e5b3bfda`.
Corrected settings SHA256 is
`2ff9b89b8485547707e6aeb83b290603d9b75320e9109a6d2ab6b85e6c911fee`.
The matching settings hash was checked in both the installed package and scratch
save root. The error report, original settings, test logs, rendered capture, and
result are retained outside the repository under
`work/gesture131-test7-launch-fix`. The distribution ZIP is refreshed with the
corrected settings, updated validation record, and matching file hashes; live
runtime logs are excluded from the archive.

This validates the reported launch failure and a bounded gameplay smoke test.
It does not add whole-career, network challenge, or console parity certification.
