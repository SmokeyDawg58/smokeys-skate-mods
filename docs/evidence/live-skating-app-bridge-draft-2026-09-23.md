# First live skating view: app bridge packet, 2026-09-23

The first app patch `artifacts/live-skating-static-map-reuse-2026-09-23-draft.patch`
has been independently reviewed and applied to `render_inspection.rs`. The
original inspection-only log remains in `run()`; the shared scene loader
prints only neutral texture/material diagnostics. `cargo fmt --all --check`
and `cargo check --workspace` pass after integration in
`artifacts/check-live-map-reuse-2026-09-23.log`. This is a static-map reuse
prefix, not a runnable skating command.

`apps/skate2-pc/src/render_inspection.rs::run` currently mounts original archives, decodes A_RobsPark presentation and texture scenes once, resolves shader manifests, builds `NativeRenderQueue`, then lets `Inspection` upload/update map `GpuObject`s. The patch moves that existing decode into `Inspection::load(&MountSet)` so the future selected session and static visual backend can receive the *same* mounted archive owner. It exposes `set_external_clip([f32;16])` and `map_object_count()` while preserving `--render-inspection` behavior and its explicit inspection-only diagnostic label. The existing `clip_from_mesh` retains the `defaultenvironment` world-space exception; no parser, queue or GPU backend is copied. The inspection camera remains the default only for `--render-inspection`; an external clip drives the same map object updater in the future live client.

`apps/skate2-pc/src/render_window.rs::RenderWindowClient` already supplies a GPU/window lifetime and a per-redraw `Instant`; `GpuRenderer::update_object` accepts a column-major clip-from-local matrix and `GpuRenderer::render` presents it. `apps/skate2-pc/src/selected_character_material.rs::{create_objects,update_object}` already binds the selected palette/material owner and same-frame GPU skin rows; its bindings guard palette and object identity. The app live client should keep map objects first, append character objects only after a completed selected tick, and then update both from one visual snapshot: `provisional_clip_from_world` for map transforms and `&CharacterPalette`/`&CharacterMaterialAdmission` for selected skater/board meshes. It must retain character binding indices and fail on material/palette admission instead of substituting a mesh or texture.

The core facade author is preparing an opaque `SelectedLiveSession` API with explicit controlled boot policy, one queued pad/physics/camera/presentation tick, and a borrowed visual snapshot gated on completion. App input must pass `WindowsPadPlatform` into that *same* retained core input owner; `windows_selected_input.rs` documents poll/cache/consume order but should not cause a second manager/map/Juice. A fixed host 1/60 simulation clock can schedule ticks independently of `render_window`'s display cadence; it is a host first-test policy, not restored native timer/network/Flow scheduling. The selected producer projection is explicitly provisional, not native presentation restore/interpolation.

The separately reviewed app-only visual consumer prefix is now registered as
`apps/skate2-pc/src/live_skating_view.rs`. It keeps the static map objects as
the prefix of the one GPU object vector and appends/updates the already
implemented selected-character bindings only for a completed same-frame
palette/material/camera snapshot. Its identical unregistered draft was
removed. `cargo fmt --all --check` and `cargo check --workspace` pass in
`artifacts/check-live-skating-view-2026-09-23.log`. A concrete live window
client is still being prepared against the core facade.

The mounted cross-owner test now proves original-data queued input, Actor, world/terrain, physical step, camera producer and palette/materials in one `SelectedSession`. Its host policies still include controlled Actor construction while registration is pending, test-selected Juice disable, stream/terrain parameters, `RootPositionWTransport::ReadSkaterOpaque { bits: 0 }`, camera submit mask 3, first-test grind exclusion and authored spawn. A live CLI must either source these from retained owners or present typed pending/controlled labels; it cannot infer native spawn command timing or claim enabled PROXY kind0/2/10 activation. Static A_RobsPark inspection geometry is preloaded, not native streamed presentation. A view built on it should say so in the window/log.

The unregistered `apps/skate2-pc/src/windows_juice_platform.rs` is a separate Windows host prerequisite. Its `rustix::net::wsa_startup/wsa_cleanup` calls exist in the locally pinned rustix 1.1.4 source and are safe public APIs under the Windows `net` feature. The app currently has no direct rustix dependency or module registration. A reviewable integration would add Windows-target `rustix = { version = "=1.1.4", features = ["net"] }` and register that adapter, with an explicit PC Winsock/TcpStream-to-XNet limitation; it does not establish native network parity.
