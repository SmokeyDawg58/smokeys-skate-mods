# Local rain optimization after preview26

The user rejected rolling back the new visuals: retain the supplied friend's
rain/puddles/ripples/reflection implementation and the requested lightning. Only
audio returns to the previous working Light/Heavy recordings and mixer, including
its louder-channel attenuation. Normal game/Windows volume preferences are not
changed. These recordings remain embedded; no separate audio pack is needed.

The earlier full-resolution fragment queried the whole static city BVH for each
visible pixel and again for exposed puddles, before tracing scene reflections.
At4K this dispatched that work over8.3million pixels. The new effect grid is
bounded near1.05million pixels (1280x720 at3840x2160); full-resolution scene colour,
reflection source, native model stencil and rain ribbons are retained. A
depth-aware four-tap reconstruction rejects depth discontinuities, while the
original character/vehicle stencil prevents water from painting onto models.
The water-effect resolution is a deliberate PC performance/quality adaptation,
not recovered console behavior or a claim of planar/offscreen reflections.

Rain used to return zero ribbons whenever a newly streamed map's asynchronous
surface was not ready. Ribbons now remain active; the last ready surface is reused
only for the same PresentationScene.location_name while a replacement prepares.
Different locations never share the retained surface. Newly exposed geometry
still waits for its correct data before it can receive puddles. Retired map jobs
check their target lifetime during patch/CDF work, and completed GPU uploads
release CPU-only patch scratch instead of retaining millions of build patches.

The clear-day sky is replaced with a local overcast treatment using the supplied
field-noise helper; the scene and screen-space reflection samples receive the
same cool tone. Sky replacement respects the native character stencil and fully
covered sky pixels. Mixed MSAA silhouette pixels keep their native colour
coverage instead of becoming binary sky cutouts. No character mask/debug image
is used as a display output. The friend's rain appearance and reflection work
remain, rather than reverting to the earlier rain renderer.

Focused shader validation and actual WGPU recording pass at1x and4x MSAA on the
RTX3080Ti. Geometry tests retain material/roof/slope/continuous-field checks and
add a resolution-budget check. Audio checks use the actual old recordings and
verify equal channel energy by attenuation only. Short silent real-game moving
captures and before/after timing are required before delivery; no future FPS
guarantee is inferred from component tests.
