# PROXY parent kind-0 producer gap and shader table draft

Unregistered source draft: `crates/skate2-core/src/rendering/proxy_shader_texture_table_draft.rs`.
This is a bounded part of the parent kind-0 route, not completed PROXY boot.
No registered module was edited and no Cargo command was run. A standalone
`rustc --test` run passed its four focused table tests; its temporary
executable was removed. `rustfmt --edition 2024 --check` passed.

## Observed parent input and source production

The PROXY parent `cPres_Global_proxy.xsf` contains 93 distinct outer arena IDs,
each with one raw `ac462e4a` Texture TOC row. All 93 TOC GUIDs are even and
unique after full-64-bit low-bit normalization. No parent ID or TOC GUID
overlaps the spatial Texture arenas. The 87 spatial Texture occurrences also
use even GUIDs, but share 35 outer arena identities across pairs of cells.
Thus the parent corpus exercises 93 new normalized shader keys; the odd-ID
low-resolution transition needs a synthetic proof and later runtime evidence.

Native `ProcessArena82d60058` takes its single-Texture branch. Fresh IDA:
`Texture::GetParameters82bca610` samples the source texture;
`GetAliasedParameters82bca760` samples again, sets alias flag bit 0, takes the
fixed-up source fetch address masked with `fffff000`, and adjusts width only
if source/target pixel-format bits per pixel differ. This caller passes the
source format (`82d60108`), so the bits-per-pixel ratio is one for the observed
branch. `GetResourceDescriptor8282f7e0` on alias flags requests a 52-byte,
4-aligned texture object in resource lane 0 and zero-size, alignment-1 lanes
1 through 4. Pixel-data lane 2 is allocated only when the alias flag is clear.
`ProcessArena` requests that descriptor from the Asset allocator at `82d60168`,
calls `Texture::Initialize` at `82d60174`, then `RegisterTexture82d601cc`; the latter
wraps the texture and five resource bases, inserts a Presentation kind-0 asset,
and appends it to the owning arena. Source confidence: high for this call and
parameter order; the actual allocator bases, fixup ownership, failure prefixes,
and host GPU texture construction remain unimplemented.

The follow-up raw caller audit narrows that ownership: `RegisterTexture82d5ea00`
allocates a `0x1c` wrapper, initially copies the five original base words into
`+0x08..+0x18`, then overwrites `+0x08` with the newly aliased Texture pointer
at `82d5eaa0`. The final wrapper retains that new pointer at `+0x00/+0x08`
and original lanes 1..4 only. The special `ProcessArena` path frees original
lane 0 at `82d60228` and nulls its local arena pointer at `82d6022c`; a host
owner must not assume the original lane-0 allocation remains live. The outer
container is published at `82d6048c`. On unprocessing, the null arena-pointer
branch removes the container, removes the first processed Presentation asset by
full ID, and defers release. `Presentation::DRegArena827f17b4` calls shader
`RemTexture` before that release. The first-use receiver path is
`OnLoadParentComplete82d5c978` (Sim then Pres),
`PresThreadMsgMan::ReceiveRegCollectionMsg82780958`, then
`Presentation::RegArena827f1430`/shader `AddTexture827f156c`.

`DecodedArenaAsset` retains the XST allocation descriptors and the decoded RW4
resource buffers. `own_texture_arena` locates the TOC Texture and GPU buffer,
uses the existing `Texture::parse` and decoder, and returns a `NativeTexture`
whose encoded/linear levels are owned copies. It neither retains the live
fixed-up fetch address nor creates the separate 52-byte aliased texture
allocation. A `NativeTexture` copy cannot serve as the pointer-equivalent
`tGameTextureAsset` passed to native `RegArena`. The smallest next producer
step is a source-owned processed texture object tied to the **same** retained
resident `DecodedArenaAsset` allocation, carrying the full TOC GUID, source
Texture/GPU-buffer locators, symbolic source-resource binding, aliased
parameters/descriptor, and a host renderer texture object. Its construction
and release need a concrete allocation/GPU service; no standalone parser or
streamer is needed. `OwnedResidentAssetsView` already offers scoped access and
the existing registry weak-handle patterns preserve collection lifetime.

## Shader table and lifetime boundary

`Presentation::RegArena827f155c..156c` passes the processed Texture pointer
and full 64-bit TOC GUID to `Sk8::ShaderSystem::gTexMan.AddTexture82e1b058`.
`FindTexture82e1a090` clears only low bit 0 of that full ID (`clrrdi` at
`82e1a0a0`). On a new key `AddTexture` pops the last available native free
index, installs the normalized mapping, and points active/high/low to the same
texture. Constructor `82e1b400..82e1b534` appends free indexes `0..5119`
ascending and constructs 5,120 empty table slots; the first new key therefore
receives slot 5119. An existing odd ID replaces low only; an existing even ID
replaces high and active. No-free-index enters the native assertion path. The
draft uses that constructor order and pointer-like tokens; its typed no-slot
error leaves the host table unchanged.

Fresh `BeginFrame82e13498` replaces `mNullTexture` with its supplied pointer
on every call; the constructor initializes it to raw null. `RemTexture82e1b548`
normalizes the same full ID and finds the slot. It changes low to the *current*
`mNullTexture` only if low equals the removed pointer or is **raw null**;
then it applies the same independent condition to high and, on that branch,
makes active equal to low. The comparisons do not use equality with the
current null-texture pointer. Only when both high and low equal the current
`mNullTexture` does it erase the key, clear all three slot pointers to raw null,
and return the slot to the free-index vector. A nonzero fallback written in an
earlier frame may remain in one slot after BeginFrame changes the fallback,
leaving a partially removed entry. The draft distinguishes raw null from a
nonzero live null-texture token and tests that transition. It does not own textures;
the eventual processed arena owner must keep each exact allocation alive
through registration and call removal before release. Pointer identity and
the source's failure-after-prefix behavior need integration tests against
that owner. The `ArenaRegistrationRefCounter` first-use decision belongs to
the Presentation collection receiver and must gate this table call; the table
is not a substitute for the counter or for parent callbacks.

The first usable packet needs the concrete aliased producer, its owner-bound
kind-0 registry entry, actual first-use Presentation delivery, and this shader
table in one lifetime. Spatial material kind 2 and admitted Environment kind 10
are separate required effects before proxy activation can be acknowledged.
The parent shader table draft therefore makes no activation, material binding,
or rendered-world claim.
