# Section-change stutter: avoid rebuilding resident GPU resources

Playtesting shows visible section-change stutters even near spawn in quiet mode.
Source audit found CityStreaming already moves CPU scene/texture loading onto
one worker. However, every completion called Inspection.initialize on the
gameplay thread, reuploading every submitted mesh and all selected textures
and recreating all alpha-test pipelines. Multiple instances of one mesh were
also uploaded separately within the same generation. Shader archive loading,
texture binding, and render-queue construction were synchronous after receipt.

Changes:
- Retain GPU meshes by (archive collection ID, arena ordinal, mesh ordinal).
  The mounted source is immutable during this Inspection lifetime. Moving
  resident collection indices are intentionally not cache keys.
- Retain GPU textures by (high/Presentation tier, archive collection ID,
  arena ordinal, texture ordinal). Low/high representations cannot alias even
  when they have the same GUID. PROXY keeps its existing separate owner.
- Mesh buffer and texture handles use Arc ownership; cloned mesh descriptions
  share immutable GPU storage. Character's mutable vertex replacement assigns
  a new handle and does not modify cached map buffers.
- Cache only successful texture uploads; retain only keys used in the latest
  generation. No unbounded travel history cache. Wgpu and object references
  retain resources needed by submitted work and the old draw generation.
- Reuse alpha pipelines by exact ALPHAREF; there are at most256 values.
- Move resident texture binding, shader manifest reads, render queue building
  and alpha-reference preparation into the existing single city loader. Publish
  scene, textures, and prepared queue together through its existing channel.
- Existing selection, XML caps, sort order, transforms, UVs, textures and alpha
  references are unchanged. These are host renderer optimizations, not claims
  that the incomplete renderer duplicates original GPU implementation.

Nonquiet diagnostic output reports uploads and retained resources each swap;
quiet play remains silent. Rustfmt parse-only passed for the four changed Rust
files; source audit checked all GPU resource constructors and the character
vertex replacement. No compilation or runtime tests run.

Remaining possible hitch sources: uploads of newly entering assets, per-draw
object/bind-group and uniform-batch rebuilds, native collision registration,
and driver synchronization. This change removes established redundant work;
stutter elimination and frame-time gains require a route comparison.
