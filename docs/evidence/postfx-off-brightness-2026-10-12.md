# PostFX-off brightness increase - October 12, 2026

Adds another 10% on top of the existing brightness adjustment when
PostFX is off. PC only; this is an explicit presentation preference, not an
original-game parity correction.

`FisheyeParameters::for_display` in `gpu_renderer/final_pass.rs` now multiplies
brightness by `1.22 * 1.15 * 1.10` (1.5433 total, previously 1.403), only when
`settings.postfx` is false. The existing world/sky final blit applies the gain
before HUD/fade/FPS drawing. PostFX-on uses the existing base brightness.

Updated the existing expectation for this gain. No tests added or run. Reviewed
the scoped diff and shader consumer; Rust syntax parsed successfully using
rustfmt's stdout output. No compile or gameplay run. The current executable
needs rebuilding for both this change and the preceding lens-reflection removal.

Backups and diff: `.scratch/postfx-off-brightness-20261012/`.
Android, gameplay/physics and the previous lens-reflection removal are unchanged.
