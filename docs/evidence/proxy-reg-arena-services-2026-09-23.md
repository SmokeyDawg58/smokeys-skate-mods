# Presentation RegArena service readiness

Read-only preparation while the original PROXY census compiles. No asset kind
is assumed present, no handlers implemented, no Cargo run.

Fresh `Presentation::RegArena827f1430` traverses the already processed asset
list forward, stopping at end/null. It does not enumerate the raw dictionary
itself. Its calls preserve interleaved order; grouping operations by kind would
change effects and failure prefixes.

| Processed kind | Native consumer | Existing reusable code / missing connection |
| --- | --- | --- |
| 0 | Shader-system texture manager AddTexture, call827f156c | Texture decoding and `TextureScene` resident handles exist; actual native texture-manager registration is not supplied by the scene inventory. |
| 2 | `TextureMan::AddMaterial8282c838`, call827f1580 | Existing whole-material source handle and ordered DMO dispatch seam; parsers and selected character bindings. Missing general mutable channel-stream fixup backed by the actual texture registry/manager. |
| 10 | `UnlockableLocationMan::AddEnviroInstance828b02d8`, call827f1590 | Simulation-side unlock router proves shared lookup/lock logic but calls the Simulation world. Presentation needs its own retained unlock contexts and real `PresUnlockableLocationMan::AddToWorld827e5360` → `WorldPresentation::AddRenderInstance827e4f00` consumer. |
| 11 | LivingWorld then DMO manager, calls827f15ac/15c8, only userdata bit8 | Existing narrow DMO adapter and concrete DMO work are separate owners; proxy activation supplies userdata0, so this arm is source-gated off without needing a fabricated DMO service. |
| 17 / 18 / 19 | `VIPresentation` streamed spline82805a08 / instanced spline828050a8 / mesh828058d8 | No concrete matching retained Presentation registration consumer found. Geometry decode alone is not these calls. |
| 20 | `LooseWorldPresentation::AddRenderInstance827e10b0`, call827f1638 | Existing model/mesh/instance decode and app GPU objects are usable data; the GUID-keyed native entity owner and render-view publication are not yet connected. |
| 29 | particle waypoint manager, call827f1648 | Do not substitute the existing Simulation waypoint registration consumer. Actual Presentation owner required if observed. |
| 34 / 35 | render SH hull-light-probe / anti-frustum managers, calls827f15f0/1608 | No concrete corresponding retained registration consumer found in the selected path. |
| other | native default branch | No RegArena dispatch; still requires the real preceding processed-asset producer. |

Fresh material function confirms forward materials/channels, starting at channel
index2. Non-scalar, nonzero GUID channels look up variant GUID|1 then GUID&~1,
then the actual configured missing-texture asset. It checks the old channel
stream, consults the shader texture manager and writes the resolved stream.
Existing copied `PresentationMaterial` values and selected character shader
admission do not perform these native mutations or recover missing-texture
startup. See the prior `material-textures.md` source proof; no new fallback is
proposed.

Fresh loose-world function confirms a null input assertion, then null-model and
zero-mesh no-op gates; otherwise it allocates/constructs a retained entity,
inserts it in its instance map and calls `AddEntityToRenderViews(..., All)` at
827e11a4. A mesh uploaded by a diagnostic scene loader does not prove this
native entity/view publication completed. The raw key packing still requires
an implementation-time audit; decompiler low/full-width guesses must not be
used as identity policy.

Common prerequisite remains the source-ordered processed Presentation asset
list (the census prints raw TOC and InstanceData types; those must be mapped
through the actual processing producer). Reuse its same decoded allocation and
existing source locators, rather than re-decoding into an independent registry.
`ArenaRegistrationRefCounter` and the shared collection storage/lookup machinery
remain reusable before those service calls. The separate activation callback
may advance only after every preceding registration message has actually run.

Scope after census: recover only observed processed kinds and their necessary
producer branches. In particular, userdata0 proves no DMO expansion is needed
for this proxy activation call even if a raw DMO record is encountered.
