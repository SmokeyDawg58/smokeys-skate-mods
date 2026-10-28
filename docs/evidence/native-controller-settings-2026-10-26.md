# PC Controller Settings (local preview)

This is an explicit PC feature requested from an RPCS3 pad-settings screenshot,
not reconstructed Xbox behavior. Native Options screen120 gains a Controller
Settings entry in both front-end and pause menus. Existing entries keep their
indices. Separate Left Stick and Right Stick pages use the original APT selector
controls/fonts; a fitted-stage live preview is rendered alongside them.

Preferences: squircle0..10000 in100 steps (0 disables); multiplier0..200 percent
in5 steps; radial deadzone and anti-deadzone0..100 percent in1 steps. The pictured
preset sets squircle4000 each, multiplier1.00 each, deadzones9/5 percent and zero
anti-deadzones. Neutral defaults preserve all original signed16-bit values exactly.
The preview switch chooses raw hardware or the draft calibrated result. Drafts
are applied only after Save succeeds. B/Cancel discard the root draft; B from a
stick subpage returns to the controller root retaining the draft. Start/closing
does not apply it. Defaults and pictured preset are also drafts until Save.

The single app boundary processes XInput and SDL hardware samples before the
shared SIM/PRES cache. Buttons/triggers, native input maps, native gesture
thresholds and physics timing are unchanged. This is an additional deliberate
user calibration layer. Existing external remappers may already alter the input;
the game cannot recover the original device values before those applications.
D-pad-only navigation on these pages allows free stick motion without changing
the selected row/value. Disconnected pads clear their preview sample.

The formulas are independently implemented at16-bit precision: scalar gain,
rescaled radial deadzone, anti-deadzone minimum outside the deadzone, then polar
squircle radius expansion using4*x*x*y*y/(x*x+y*y)^2. The UI terminology and
4000 squircle scale were checked in the primary RPCS3 source:
https://github.com/RPCS3/rpcs3/blob/master/rpcs3/Emu/Io/PadHandler.cpp
No RPCS3 implementation code is transplanted. This is not a claim of identical
RPCS3 processing: RPCS3 uses an8-bit intermediary and blends its anti-deadzone
inside the deadzone; this PC layer deliberately keeps the inner deadzone silent.

controller-settings.json lives with the existing PC preferences (or the isolated
SKATE2_SAVE_ROOT during diagnostics). It uses the existing strict JSON parser,
range/version/duplicate checks and a temporary-file rename on Save. Persistence
failure leaves the active calibration unchanged and the menu open for retry.
Malformed preferences fall back to neutral with a visible menu error; they are
overwritten only by an explicit Save. No polling process or external controller
tool is installed.

Focused tests cover every signed16-bit neutral input, radial and extreme bounds,
independent stick gains, squircle symmetry, strict parsing, main/pause navigation,
draft cancellation, successful Save/reload, and raw/calibrated preview separation.
Full-game checks must also verify APT layout and D-pad navigation while the
diagnostic sticks are held off-centre. Diagnostic windows use isolated profiles
and SKATE2_AUDIO_DUMP; user/game/Windows volumes are never changed.

The separate Snow uniform ABI is repaired alongside this feature: its WGSL
parameter block now matches the288-byte host buffer after old rain fields were
removed. Actual Snow pipeline/draw validation is required before installation.
