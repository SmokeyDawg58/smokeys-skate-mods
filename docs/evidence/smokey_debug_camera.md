# Smokey debug camera (local experiment)

The reference is the bundled Skate 3 port's `skate-game/src/debug_cam.rs`,
which uses a rising R3 + D-pad Right chord to toggle the manual camera and B
to spawn at its matrix. `skate-core/src/camera/manual_cam.rs` owns camera
movement, trigger elevation, the L3 speed modifier and the spawn matrix.
This is a PC mod port, not a claim about original Skate 2 behavior.

The experiment connects that manual camera to the Skate 2 session, holds the
skater's input idle while it is in use, and queues the normal player teleport.
The default toggle is R3 + D-pad Right and the default teleport is B. Mods >
Keybinds offers a single Debug Camera row; selecting it records the next button
or two-button chord after release. The teleport button remains B. The selected
buttons use the game's button atlas. The object dropper continues to use LB +
D-pad Left and Contacts uses LB + Y; conflicting shortcuts are rejected.
Current admission is limited to free roam.

The source lives in an isolated working copy while physics work continues. The
final executable built successfully. Silent offline checks verified the default
and rebound chords, B teleport after camera movement, exit by the chord without
teleporting, and returning control to the skater. The in-game Keybinds screen
was checked at its default and rebound settings; a conflicting chord was
rejected, cancel left the saved binding untouched, and the selected icons
matched the saved controls. Online gameplay has not been validated.

The first preview moved the camera and teleported correctly, but the game's
presentation restorer replaced the fly camera during rendering. The fix makes
the debug camera authoritative for rendering while preserving the restored
skater animation. Silent screenshot and render-position checks covered Skate 2
and Skate 3 physics at the waterfront and Community Pool. These verified camera
movement on screen, a stationary camera without drift, B teleport, a rebound
shortcut, and the normal rendered camera after teleport and toggle exit.

The next local revision reverses the FOV direction: D-pad Left increases FOV
and D-pad Right decreases it. A fully held RT targets 20x speed with the same
proportional trigger response and smooth transition. Focused control tests and silent rendered gameplay checks passed at the
waterfront and Community Pool. Normal forward movement measured 4.71 m over
the test interval; half RT measured 45.19 m and full RT 85.35 m, including
the smooth acceleration ramp. Normal vertical travel was 9 m and full RT
162.9 m. Left widened the FOV and right narrowed it; holding the opening
chord did not change FOV.

The local controls preview maps debug-camera height to D-pad Up/Down, FOV to
D-pad Left/Right, and proportional acceleration to RT. A fully pressed trigger
targets 10x speed for forward, sideways, and vertical movement, using a smooth
transition. The opening chord must be released before its buttons can adjust
the camera. Silent rendered-camera checks covered Skate 2 at the waterfront and
Skate 3 at Community Pool. Over the measured 90-tick movement interval, normal
forward travel was about 4.7 m and full RT was about 42.9 m; normal vertical
travel was 9 m and full RT was 81.9 m. The ratio reflects the smooth ramp to
10x. FOV changed in both directions without moving the camera, the held opening
chord did not change FOV, and the normal forward speed matched the earlier
preview. Separate checks confirmed B teleport, rebound activation, exiting,
and restoration of the gameplay camera.
