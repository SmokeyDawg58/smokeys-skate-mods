# Map startup: sub-block compressed textures

The startup failure was the host-only `validate_texture_levels` check
requiring both compressed base dimensions to be multiples of4. Read-only
inspection of original screens.big, using existing BIG/RefPack tooling and
the production RW4 dictionary/fetch-word layouts, found four textures in
`data/fe/source/controls/basic_slider.rx2`, dictionary entries15/17/19/21:
each16x2, fetch GPU format20 (DXT5/BC3). These are valid original slider skins.
Other newly admitted map/letterbox/dimmer/icon assets had aligned bases.

The shared native uploader now validates all original mip/face/block/payload
metadata as before, then expands BC1/BC3 only when base dimensions prevent
a wgpu compressed allocation. Each original mip and face becomes RGBA8 at
its original logical size; partial edge-block texels outside that size are
discarded. No padding/rescaling changes normalized UVs or sampler boundaries.
Aligned bases still upload original compressed blocks directly. Conversion is
once at upload, not in frame rendering. Existing untile/endian handling reused;
no second texture parser or external dependency introduced.

Decoder format reference: [Microsoft BC1/BC3 layouts](https://learn.microsoft.com/en-us/windows/win32/direct3d10/d3d10-graphics-programming-guide-resources-block-compression).
BC3 always uses four-colour interpolation, preserves independent alpha indices
and both alpha endpoint modes. BC1 preserves its transparent selector mode.
Integer UNORM expansion is a host representation conversion, not a claim of
bit-identical Xbox hardware interpolation rounding.

Regression cases added:16x2 BC3 edge cropping/alpha, BC1 transparent selector
versus BC3 four-colour mode, and nonaligned base validation with truncated-block
rejection retained. Rustfmt syntax parsing passed. Tests/build/runtime were
not run.
