# Native diagnostic window foundation

2026-09-12. This is a host GPU implementation, not recovered Skate 2 shader,
camera, residency or frame-scheduling behavior. Root owns executable integration
and Cargo validation. No GPU run had occurred when this packet was written.

The pinned baseline is wgpu 0.19.4, winit 0.29.15 and pollster 0.3.0.
The [official wgpu versioned workspace manifest](https://raw.githubusercontent.com/gfx-rs/wgpu/v0.19.4/Cargo.toml)
declares Rust 1.70 and uses winit 0.29, raw-window-handle 0.6 and pollster 0.3
together. The [winit versioned manifest](https://docs.rs/crate/winit/0.29.15/source/Cargo.toml)
declares Rust 1.65 and enables its 0.6 window-handle implementation by default.
This is a deliberately compatible pinned baseline, not a latest-release claim.
The repository declares Rust 1.85; resolved transitive dependencies and that
minimum toolchain still need a separate gate. Current host is Rust 1.98 with
only x86_64-pc-windows-msvc installed, on an NVIDIA GPU.

wgpu keeps the backend boundary separate from decoded assets and simulation.
The initial default instance selects a supported native backend and reports
the actual adapter; no assertion that DX12, Vulkan or texture compression is
available replaces device creation. Native defaults are not a tested Android,
Linux or browser port. In particular this window runner uses native blocking
startup, while mobile resume/surface lifetime and browser asynchronous startup
need their own integration. No second OpenGL backend is introduced.

`gpu_renderer.rs` borrows `NativeMeshGeometry`, validates each original indexed
triangle-list draw including signed base vertex, then owns uploaded position
and index bytes. CPU scene compaction cannot invalidate those uploaded buffers.
It preserves draw ranges and exposes an explicit column-major clip-from-local
matrix (depth zero to one) and diagnostic color per object. Depth testing is
LESS_EQUAL. Culling is deliberately disabled in this inspection pipeline;
this is not a claim about native material winding. The solid shader does not
sample invented fallback textures, lighting, fog or shadow values.

Linear RGBA8 upload is an explicit validated interface. BC1/BC3 decoding or
feature admission, mip/face upload, original sampler selection and material
binding remain separate work. Uploaded textures are not yet consumed by this
solid diagnostic shader. Original texture formats must not be relabelled RGBA8.

`render_window.rs` accepts a host client for initialization, window input and
per-redraw work. The host owns mapped inputs and fixed-step simulation. The
window callback does not equate display cadence to simulation ticks or invent
a skating controller. Zero-sized surfaces are suspended, resize recreates depth,
lost/outdated surfaces are reconfigured, timeout retries, and fatal errors exit.

Finite-frame options count successful presentations. BMP capture copies the
same rendered surface through an aligned GPU readback buffer, waits for mapping,
and writes its actual RGBA8/BGRA8 pixels with top-down rows. Unsupported surface
copy/format admission fails explicitly. This is real-window capture, not a
separate headless or synthetic scene pipeline. Screenshot inspection, device
validation, resize/minimize and actual original-mesh submission are pending root
integration. Source material/camera limitations remain in the preceding evidence
packets; no original assets were added to the repository.

## Original diffuse texture follow-up

Root reported a successful Vulkan original-map solid capture at
`artifacts/robs-park-gpu-first.bmp`. This is the first real GPU validation;
the following texture extension still awaits its own integrated capture.

The renderer now consumes actual `NativeTexture.linear_levels`. Existing
`material.rs` format constants and `texture_block_layout` distinguish DXT1
(4x4, 8 bytes), DXT5 (4x4, 16 bytes), and A8R8G8B8 (1x1, 4 bytes). The existing
decoder already untiled and normalized byte/halfword endian ordering; upload
does not swap again. DXT1 maps to BC1 RGBA, DXT5 to BC3 RGBA, and normalized
packed A8R8G8B8 to BGRA8. R5G6B5 and non-2D resources remain explicit admission
errors. Exact original format IDs include the fetch descriptor's channel data;
the renderer does not identify formats from approximate names.

BC support is requested only when advertised by the adapter. All authored mip
levels are checked for unique ordered indices, face zero, expected dimensions,
block size, exact row pitch and complete payload before allocation/upload.
Base compressed dimensions must satisfy GPU block alignment, with an explicit
error instead of changing normalized UV coverage. Sub-block tail mips copy their
full physical blocks. The fetched official wgpu-core 0.19.4 source
`command/transfer.rs` lines 293, 368 and 426 independently confirms block-row
units, physical mip extents and block-aligned copy sizes.

`upload_mesh` carries decoded TEXCOORD0 Float2/Float3/Float4 XY values; unsupported
or missing UV data cannot enter `create_textured_object`. The solid diagnostic
branch has unused UV padding, never a fabricated sampled coordinate. Root
resolves the exact `i_diffuse` material channel and counts unresolved draws;
the GPU layer never chooses the first available texture.

The diagnostic sampler is repeat/linear including mips. The shader samples
Unorm texture data and squares RGB into linear space, then hardware sRGB surface
encoding displays it. An sRGB surface is required; no double texture sRGB decode
occurs. This reuses only the original diffuse helper's gamma-two step. Original
anisotropy, lightmaps, CSM, fog, brightness, exposure, tone mapping and alpha
semantics are not claimed. The previous unused raw-RGBA upload and default-run
wrapper were removed; actual original-texture upload is the retained API.

Two CPU admission tests cover physical BC tail blocks and rejection of cube,
wrong-face, truncated and missing mip data. Root owns their execution and the
subsequent actual-texture GPU validation.

Root integration September12: actual Vulkan three-frame run6 exits0,
capture `artifacts/robs-park-gpu-textured.bmp` visually inspected. Of2,085 draws,
1,189 sample original diffuse textures;896 report missing resident diffuse and
remain explicitly solid. Source AddPegasusMaterialToScope82e17d0c hashes the
serialized channel name with kInstanceHash: original channel census says
`diffuse`, whereas the shader sampler is `i_diffuse`. Root corrected that
diagnostic selection after run3/5 reported zero textured draws; no arbitrary
texture substitution. Run2 exposed a VS/solid-FS UV interface mismatch, fixed
by matching the fragment input signature. Logs retain those failures. Original
lighting, alpha semantics and non-defaultenvironment vertex spaces remain
unreconstructed; this is a map inspection result, not playable skating.
