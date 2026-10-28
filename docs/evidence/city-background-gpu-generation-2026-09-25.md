# City section-change frame stalls: background GPU generation

## Identified synchronous work

CityStreaming already loads/reconciles CPU resources and builds native render submissions on its loader. However, completion called Inspection.initialize on the frame thread. That rebuilt the texture selection table, uploaded newly required mesh/texture resources, created alpha pipelines when absent, created all per-draw bindings, and built the map uniform batch. Replacing the object prefix also destroyed thousands of old bindings on the frame thread. Previous GPU caches reduced repeat uploads but did not remove this synchronous burst.

## Change

- Extracted existing GPU creation method bodies into a cloneable GpuResourceBuilder. GpuRenderer forwards to the same implementations. It shares device/queue/layouts/sampler and copies the surface format; it has no surface, window, resize, present, or simulation access. Existing renderer layouts/sampler now use Arc.
- MapBuild owns CPU scene/textures, the replacement native queue/material metadata and GPU caches while a single city-gpu-builder constructs the complete replacement. It uses the same diffuse/proxy/high texture lookup, shaders, alpha references, transforms, colors, mesh validation, texture upload and uniform construction as before.
- Active queue/colors/uniforms/objects remain usable while either loader or GPU builder works. Only complete success replaces the map prefix. Character objects remain after that prefix. LiveSkatingView updates current-camera uniforms immediately after publication and before render, so worker-time camera matrices are not displayed.
- Old map objects, queue/material strings, colors and uniform batch are returned to the same worker for destruction. Completion polling is nonblocking. Retirement channel closes with its Inspection owner. Load/build failures remain errors; no partial replacement is displayed.
- Caches move between owners instead of cloning scene data, and existing per-generation cache pruning runs on the worker. At most one replacement GPU build is pending; next focus selection uses the latest position after publication.
- Native focus selection/radii and collection dependency logic are unchanged. No physics, animation, input, timestep, or quiet-logging changes.

## Verification and limits

Rustfmt parse PASS for both changed Rust files. Static review: original creation method bodies reused, no worker surface access, no frame-thread initialize on streaming completion, map-prefix/character-tail ordering preserved, latest camera update precedes render. Local pinned wgpu0.19.4 source confirms native Device/BindGroupLayout/BindGroup/RenderPipeline Send+Sync; CPU scene owners already cross the existing loader channel.

Not compiled or runtime tested. Check with build-release.bat and play-quiet.bat, testing spawn-to-city-and-back, turning across section boundaries, textures/foliage and character/camera continuity.

This removes an identified CPU-frame burst; it is not evidence of zero hitches. Shared GPU upload bandwidth/driver contention and synchronous simulation terrain registration can still contribute. A generation remains visible until its replacement finishes, extending visual load latency instead of blocking gameplay. Initial scene construction remains synchronous before play.
