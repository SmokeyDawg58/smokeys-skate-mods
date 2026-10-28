# Rain-only replacement from supplied source

User source: skate2sourcecode.rar, rainaudio.rar and rain_loop.wav, supplied2026-10-26.
Rain runtime is an authored PC visual extension, not recovered Xbox rain behavior.
Public sequence25 remains unchanged; this work is local for review.

## Imported rain design

weather_world.rs and weather_puddles.rs retain the supplied geometry index and
continuous per-mesh area distribution. Opaque decoded map triangles classify the
actual depth receiver; decals, cutouts, water, sky and proxy geometry are not puddles.
Wind-facing cover queries prevent rain/puddles beneath roofs. The actual triangle
normal admits slopes through15 degrees. Water normals are level plus animated
ripples, rather than inheriting the faceted/depth-reconstructed ground normal.
That removes the old one-degree binary normal cutoff as a source of dark seams.

rain.wgsl retains the supplied hash/gradient puddles, shoreline, nine-neighbour
ripples,12 expanding reflection steps plus3 refinements, edge/range confidence,
and neutralized environment-cube/sky fallback. Hidden/offscreen geometry remains
unavailable to this screen-space method: complete planar skater mirrors are not
claimed. Unlike the supplied fork, the host snapshot includes characters before
weather so visible player pixels can participate. Reflections never sample their
own output. The runtime light editor and unrelated fork changes are not imported.
Its local-light-only highlights are omitted where this host has no such owner.

Rain ribbons use the supplied weather_rain.rs integer seeds, anchored wrapping,
18m/s wind,40m radius,23.2m height,.56m length,.032m width,.28 opacity and
geometry shelter/hit clipping. Generation is a GPU adapter to this backend, not
a copy of the unrelated particle system. Native Mods selections set intensity
0.30/0.65/1.0, producing1871/4054/6236 nominal drops. The random weather scheduler
is not imported, so the selected rain mode remains selected. This manual mode
adapter starts with the selected wetness/coverage instead of forcing a45s dry spell.
Storm desaturation50% and brightness90% are from the supplied final-pass shader.
The supersampling lane remains independent. Snow uses its existing separate path.
The former rain profiles, surface shader functions and recordings are removed.

## Audio and requested lightning exception

The supplied converter packetizes rain_loop.wav losslessly to EAAC v1 codec6.
Native decoding, source-rate resampling,50ms loop join,2.5s rise/4s fall and roof
muffling come from the supplied rain audio source. Converted assets stay external
to Git and are embedded during building, so friends do not replace BIG archives.
The WAV has a3.765dB persistent right-channel bias: attenuate only its louder
channel to match full-loop energy, retaining stereo detail. Native Rain Volume,
SFX, pause and visibility gates remain. No game/OS volume is changed.

At the user's explicit exception, only lightning bolt/jag/hash and1+.45flash
gain are retained from Skate3Remaster build10/038ff331. The bounded local flash
schedule is a PC adapter, not a recovered Daytryan timer. Medium/Heavy flash;
Clear,Light and Snow do not. No thunder asset or gain increase is added.

## Validation

Focused checks and receipts reside outside Git in local-gameplay-review-r26.
Shader parsing/validation passes at single sample and MSAA; the production Rust
rain module compiles against its WGPU API. Five production audio/lightning tests
pass, including native decode of the supplied WAV, fades, looping and roof muffling.
The separate semantic HoM fixture now selects fields from the real S2 schema and
checks48 actual S3 curve endpoints, replacing the previous self-repeating mapping.
Full game compilation and short silent scene captures are required before install.

## City-scale correction after the first actual game capture

The full city supplied729717 receiver/cover triangles and1756725 subdivided
patches. The first import delayed resident map publication and cannot be shipped
as validated merely because its component tests passed. Keep the supplied CPU
rain ribbon generator directly: each drop performs cover/hit queries once rather
than repeating the BVH walk for all six GPU vertices. The vertex buffer is reused.

The CDF now subdivides only puddle-eligible flat faces; steep cover triangles stay
in the original BVH. CPU per-patch cover rays are redundant with the supplied
fragment shader's exact sky query, so the CDF budgets the whole flat mesh and the
shader excludes sheltered water. This preserves the whole-mesh area cap; it does
not promise the same percentage of only exposed subareas. These are explicit
large-map host adaptations, not changes to the source ripple/reflection formulas.

Rain BVH/CDF completion is independent of native map publication. A scoped map
preparation worker publishes its immutable surface through OnceLock; dropped map
generations discard their unused result. Native terrain/player rendering does not
wait for rain. Before the matching geometry is ready, puddles/streaks are withheld
rather than querying the wrong map or raining through missing cover. The source
CPU ribbon generator is used directly, with one shelter/hit test per drop and a
reused vertex buffer. No periodic PC monitor or external background service exists.

The actual-city admission probe subsequently showed green (outside puddle contour)
across the visible pavement. Receiver/depth/normal admission was correct. Large
patches span many noise cells, so their conservative minima collapsed the source
CDF below the actual field. Fine meshes retain the original conservative method;
coarse meshes use area-weighted centre samples of the identical source field.
Coverage on coarse meshes is an estimate, not a strict70-percent pixel-area bound.
The dedicated large-floor fixture includes the original minima as a failing
negative control. Shoreline, world field, ripple and reflection formulas remain.
