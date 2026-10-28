# CPU Assembly/ground producer integration

2026-09-07, implementation with independent source review. This is a
connected private CPU producer/consumer, not yet the application gameplay loop.

## Evidence and ownership

Reference executable SHA-256:
`3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`.

- `AssemblyTriangleIntersectionTester::RunIntersectionTests`,
  `0x827cd1e0..0x827cd7c8`, SHA-256
  `81C7E79EA8038EC1A61F45F03A3DC203A18B7742EFCA214EBA493C83B9FE4983`.
- CPU `EA::Physics::AssemblyGroundCollisionJob`,
  `0x827cdb98..0x827cdf48`, SHA-256
  `4F99783828386F3FF669519A57C3CC936D4C69227DD98600E6FAF3AB54C217B1`.
- `BoxTest<AssemblyTriangleIntersectionTester,1>`,
  `0x827cd960..0x827cdb98`; `RipTriangles`, `0x827cd7c8..0x827cd960`.
- Existing primitive bounds, GP topology, narrow phase and final sixteen-row
  compositor evidence remains applicable. The independent complete packet is
  external: `world_probe_temp\assembly_ground_run_astra_proof.md`.

PDB `ExtraVolumeData` is eight bytes: live body at zero, PPU/table body at four.
The CPU job independently loads both from the same retained Assembly body-table
entry. `GroundData` is 24 bytes: three material floats, live body, PPU body and
collision group. Both ground roles come from the manager static body via TLS.
Four semantic roles remain separate in host records and compiler validation.

`simulation_ground_contact.rs` is a private child of the Contact compiler so
only the actual producer can construct its inert rows and binding tokens.
It borrows source-owned `VolumeData` and an exclusive body phase. The concrete
constructor refuses foreign body pools and replacement packed-image epochs.
Prepared Assembly transform/header freshness is still the outer scheduler's
precondition: image/occurrence checks are not proof of current-frame publication.

## Recovered sequence

CPU preparation first skips all-frozen/all-static assemblies. Every remaining
non-Aggregate volume receives live/table body roles and `CalculatePartBBox`,
including frozen, disabled and static individual volumes. Aggregate Extra slots
remain absent and must never be inspected by Run.

Run visits triangles, volumes and returned contact pairs in that order. Its
gates are frozen, Aggregate, pipeline culling bit, material selection, ground
body validation, exact static state one, GP construction, enabled-or-sole-volume,
inclusive triangle/volume XYZ AABB, effective velocity padding, and Sk8 narrow
phase. The pipeline culling table is NOT the assembly self-culling table.
The wrapping bit index is `columns * volume.group + ground.group`, LSB numbered.

Ground material image `0x82fef110` is friction zero/zero, restitution one.
Generator padding is `0x3f000000`; convexity epsilon is `0x3c23d70a`; bend
threshold is the retained live Assembly parameter. Padding retains the literal
`0x3c888889` timestep and original fsel graph, not the frame timestep.

Every final composition reserves first. Body COM, velocities, inertia, forces
and other final Contact rows are sampled inside the reservation closure. The
phase keeps previously validated observations stable. CPU A is volume and B is
ground, unlike the existing network producer's opposite body-side convention.
The shared compiler/Horus consumer now checks producer-specific provenance;
ground does not become a fabricated network request.

Unavailable local storage returns from ONE Run. The ripper still clears its
full triangle cache, accepts the triggering triangle and visits later batches.
Temporary tails flush after EACH processed assembly. A caught composition panic
poisons the buffer; ground has no discard/rollback operation. Global raw
reservation count and initialized accepted-prefix count are separate; see
[counter correction](contact-append-counter-correction.md), which supersedes
the older source interpretation.

The capped KD API gathers at most 256 range descriptors per mesh without
reading a post-cap branch. Native descriptor fields narrow to u16; the generic
query retains u32 and the BoxTest caller owns that conversion. World-to-local
query bounds, mesh iteration and source-owned frame scheduling remain distinct
from Run. Per-mesh triangle source coordinates alone are not globally unique.

## Numeric and validation boundaries

`HostSerialisedAssemblyBBoxNumeric` implements explicit ordinary-host min/max,
separately rounded XYZ products with `(x+y)+z`, and `sqrt().recip()` seeds.
The existing four-lane bounds/sweep graph and its refinements are unchanged.
Contact tangent and compiler reciprocal estimates are host approximations, not
Xenon estimate bits. No input sanitization is added. Exact exceptional values,
VMX reduction/estimate rounding, FPSCR and native races remain unvalidated.
Sk8's optional output still conflates zero-contact and unsupported/malformed
cases; acceptance is restricted to valid represented primitive inputs.

Nine source-derived synthetic ground tests passed, including actual clustered
triangle decoding/ripping, sphere/triangle narrow phase, final material/tag and
role rows, and shared Contact/Joint/Drive solving followed by live body
integration. The penetrating sphere receives upward position correction and
integration clears its RF rows. Other discriminators cover gates, single-volume
exception, 50-slot exhaustion without global rip abort, per-assembly tail flush,
53-record global accepted prefix, asymmetric/wrapping culling, inclusive XYZ
overlap and replacement-image rejection.

The input module separately passed independent PPC review and five tests.
Neither synthetic tests nor a successful mounted-data probe constitutes a
captured original-game per-tick trace. Current confidence is **high** for the
represented immutable CPU path and source order, not complete physics parity.
## Mounted real-data checkpoint

The root independently reran the ignored test
`real_dmo_primitive_and_robs_park_terrain_compile_solve_and_integrate` successfully
against the external original corpus. The new corpus test retains real decoded
owners and uses the production Create/image/inertia path, not a fabricated
arena or substitute mass/material. Original collection fingerprints (FNV-1a64,
regression identifiers rather than cryptographic identity):

| Logical collection | Assets | Bytes | Fingerprint |
|---|---:|---:|---|
| data/content/world/dmo/DMO/cPres_Global.xsf | 125 | 3092992 | 36447f74e4675ca0 |
| data/content/world/stream/BAM/cSim_500_900_high.xsf | 21 | 1108224 | aec60e479f36d040 |

Selected DMO outer `d3096f13cf1c0aab`, arena `e75f30b6`, Assembly `0x219`,
Definition `0x217` has one movable Part and TWO original VolumeData records:
Aggregate followed by Sphere. The wrapper remains present and Run skips it.
Sphere radius is 0.043058835, authored friction 0.2/0.2, restitution 0.1,
inverse mass 1. Terrain outer `a114c7540da18d0e`, arena `0cb6a4ca`, mesh
dictionary 3 has 113408 original bytes, fingerprint `a463b42229b8fb97`.

One capped range, one triangle and one contact are produced. Contact source is
volume 1, cluster 18, unit offset 351, primitive 0, point 0; full Contact words
have host fingerprint `0e7c704e6a10f08a`. After four controlled solver iterations
and integration the center changes from `[588.8619,-54.422665,996.5048]` to
`[588.8621,-54.401142,996.5048]`; RF rows are cleared.

The corpus also verifies actual static Environment operation 0, TOC14,
dictionary1, resource0/offset576, instance0/GUID `80629fce1595539c`:
model0/dictionary4 -> root0/dictionary2 -> mesh0/dictionary3.
`StaticEnvironmentSource` construction succeeds with one root. The corpus now
registers that source in the actual `StaticGroundPool` and executes the complete
isolated serial CPU job before compilation, solving and integration. Its Contact
words/source match the retained direct-mesh harness. World-to-local BoxTest
culling uses the KD-root bounds proved by `ClusteredMeshData::Reset`, not the
separate mesh header box.

Body placement, image bases, island label and iteration count are controlled
harness inputs, not captured game spawn/frame values. This validates one real
primitive/terrain host path, not skater/wheel behavior or bit-exact native
physics. Next integration is persistent source-owned ground-job/frame execution
and actual skater construction. The full reference-tick oracle remains absent.

## Resident static-ground CPU job

`simulation_assembly_ground_job.rs` implements the isolated CPU profile of
`AssemblyGroundCollisionJob` (`0x827cdb98`) and `BoxTest<...,1>` (`0x827cd960`).
The private pool view borrows actual retained mesh owners in current slot order,
including swap-last removal, and keeps saved KD-bound errors lazy. The job
holds one 50-Contact buffer, creates one 20-triangle cache per processed Assembly
across all meshes, transforms the original Assembly Header AABB per mesh,
gathers at most 256 descriptors before ripping, narrows three descriptor fields
to u16, and always performs the final Run and per-Assembly tail flush.
Storage exhaustion ends one Run, not the rest of the ripper's traversal.

Independent live-IDA review found no valid-data mismatch in this scope. The
original resolves a cluster-table pointer even before a zero-count Rip; the
checked formats visitor skips that read. Malformed-memory/read-observation
parity is not claimed. Checked host errors abandon the isolated output without
mutating bodies, RF rows or resident meshes; this is not native fault rollback.
Global storage starts empty: competing jobs and atomic inter-worker order are
not reconstructed by this owner. Assembly freshness remains the frame caller's
precondition.

The real-corpus test also passes with no ground meshes (one unconditional empty
Run) and with 21 duplicate registrations of the original Environment (21
triangles/Contacts but only two Runs). This distinguishes a cross-mesh cache
from incorrectly creating a fresh tester for every mesh. Duplicate Contacts
are validated as a stress fixture, not fed to the one-contact pose-update test.
The production job still has test-only callers. A subsequent integration now
runs it across eight retained frames using the recovered AssemblyPool bounds
refresh; see [pool refresh evidence](assembly-pool-refresh.md). Persistent
gameplay session execution remains incomplete.
