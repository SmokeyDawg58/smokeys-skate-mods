# Private rain and cinematic composition repair

Local preview only. Public update remains held.

The supplied rainpreview.rar is a standalone OpenGL/Python look-development
application. Its code was read as source material, not executed. Heavy Rain uses
the supplied rain-settings.json; Light Rain uses its class defaults. RainField
motion, wind, world width/length, edge fades, colour and streak profile are
ported to WGSL with GPU hash seeds. The saved look is retained as rain-look.json.
The wet-film, warped puddle mask, one-degree puddle slope limit, Fresnel blend
and animated impact rings follow rain_surfaces.py. Lighting stays with the map;
we do not copy the preview courtyard's fixed sunlight or global brightness.
Its two fixed-height planar mirrors are adapted to bounded screen-space depth
reflections with the authored environment fallback. Offscreen geometry is not
reflected. This is an explicit PC adaptation, not identical planar reflections.

The owner screenshot showing white world/player with dark clouds identifies
an ordering failure: weather replaced the sky after native cinematic effects.
Weather now reads the raw scene and renders to a separate texture, then copies
that result into the raw scene before native bloom, grading, HoM composition
and final blit. There is no same-pass texture read/write alias. Clear skips the
weather pass. Native character colour restoration masks are suppressed under
cloud cover so invisible/cutscene player stencil does not restore sunny colour.
All stencil samples contribute to coverage when MSAA is enabled. Rain's wet
coating does not leave a dry ghost outline on horizontal world depth.

The directional shelter map follows rain wind. Its world-anchored 2048px/96m
coverage uses linear comparison sampling and a 3x3 tent rather than binary
texels. Snow retains vertical shelter; fractional coverage softens the large
sawtooth boundary reported by the owner. Depth normals choose same-surface
neighbours to avoid stair risers contaminating tread shading.

Rain audio, native menus, weather persistence and user volumes are unchanged.
Shader validation and short silent game captures are recorded separately.
