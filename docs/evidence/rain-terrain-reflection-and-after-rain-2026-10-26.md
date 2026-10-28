# Local PC weather refinement after public Update28

The owner prioritised Medium/Heavy Rain performance and explicitly authorised
removing live player reflections in favour of subtle terrain reflections. The
new water pass samples only the original static environment cube at a softer
mip; it never marches rays through the live scene image. This removes reflected
character silhouettes and their missing-screen-data artefacts. It is an authored
PC effect, not a claim of recovered retail weather behavior.

The receiver field is continuous in world coordinates across mesh/material
groups instead of using different per-mesh distribution ranks for the visible
edge. Wider soft shores, lower reflection strength and weaker whitening preserve
the original floor texture without an opaque water-mask appearance. Depth and
the native character stencil still reject sky/models and dissimilar-depth taps.
Static receiver and roof admission remains over the real decoded geometry.

Rain ribbons preserve the existing Light/Medium/Heavy look, density, wind and
fall speed. A drop caches its exact unobstructed segment along that same wind
line. It reuses the segment only while position is on that line and before the
next geometry intersection; volume wrapping, line changes and new geometry
invalidate the result. This reduces repeated city BVH queries, especially for
the higher preset counts, without a coarse shelter grid.

Board spray is bounded to192 visual droplets. Published native deck positions
provide displacement; a downward eligible wet-ground query and an exposed-sky
query gate the four wheel areas. Motion emits short side spray; downward contact
adds a small burst. Airborne/held boards outside the contact distance do not
emit, teleports reset displacement, stale/cutscene/replay poses do not drive
emission, and clock jumps clear particles. No physics, timing or scoring changes.
The effect uses the rain pass with soft depth-tested droplets and no new texture.

The appended AfterRain enum retains all existing serialized enum/key meanings.
The native weather cycle puts "30 min after rain" after Heavy Rain: Light Rain
wetness, no falling streaks, no rain ambience/lightning, partly clearing clouds,
and a faint world-oriented rainbow. It is a selectable look, not a real-time
30-minute weather scheduler. Light Rain's density and sound level stay as before.
Medium/Heavy use stronger existing balanced recordings; the native Rain Volume
and SFX values still multiply the effect. Master/Windows levels are never changed.

The controller preview panel moves up72 native-stage pixels. Its contents move
together; original menu rows, stick icons and bottom-right hints stay in place.

Initial focused Naga and actual WGPU submission checks pass at1x/4x MSAA, and15
geometry/ribbon/splash unit checks pass. Moving-board fixtures verify wet contact,
finite bounded output, and no airborne contact. Actual game performance and
visual review are required before local delivery.120 FPS is the target, not a
universal guarantee, and no full online gameplay regression is claimed.

Because the visible field no longer uses per-mesh CDFs, runtime preparation
builds only the exact receiver/cover BVH. It skips millions of subdivision
patches and per-mesh rank sampling on each streamed generation. The original
CDF builder remains as reference/test code, rather than doing unused runtime
work. GPU admission now uses the wettable flag and exact normal directly.
