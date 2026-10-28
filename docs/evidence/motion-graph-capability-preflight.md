# MotionGraph leaf-capability preflight

Date: 2026-09-07. This checkpoint adds an immutable, graph-wide refusal gate
between typed `MotionGraph` construction and any future dynamic controller. It
proves which leaf bodies have reconstructed typed implementations and reports
every remaining unsupported leaf before a Context, entity-bound adapter, or
controller is constructed. A successful coverage report is a snapshot, not an
execution permit.

## Evidence identity

- Matched debug XEX SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`.
- PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`.
- MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.
- Mounted `data\state\MotionGraph_OnBoard.stategraph`: 641,241 bytes,
  SHA-256
  `E5F317CCAC9FAD0D126B3DF6810C9602CBB081F23FE39FB44662C92D4F13F7ED`.
- Frozen selector oracle SHA-256:
  `4DE75A06646047ABAC6DF56708C71D4BBA096681E0971978A6476E00B19AFA46`.
- Corrected dynamic-controller proof SHA-256:
  `12A5DB655DDF3FED35C8BDF244E58650B9A20C6AF05F17EE25AB129A53914CB4`.

## Exact boundary

Preflight first requires a successfully completed structural `Process`, then
scans the complete leaf arena in ascending `LeafId` allocation order. Disabled,
unreachable, replaced-expression, and duplicate-path leaves are not elided.
The only covered triples are exact canonical byte identities:

| Kind | Canonical registration | Payload variant |
|---|---|---|
| Condition | `HasIntent` | `HasIntent` |
| Condition | `WillExpire` | `WillExpire` |
| Behaviour | `PlayAnimation` | `PlayAnimation` |

Catalog registration and successful structural `Process` are never treated as
proof of executable semantics. Every `RegisteredOpaque` payload remains
unsupported. A known canonical identity paired with any wrong variant,
including `RegisteredOpaque`, is a payload-identity mismatch. An unknown
identity with a typed variant is an unsupported identity; an unknown identity
with an opaque variant is an unsupported registered leaf.

Each failure retains raw `LeafId`, path, parent, kind, requested name,
canonical registered name, optional payload variant, required implementation,
and reason. Raw bytes are not collapsed through UTF-8, and duplicate paths
remain distinct by `LeafId`. Checked malformed-host precedence is requested
embedded NUL, canonical embedded NUL, requested/canonical ASCII-case mismatch,
missing payload, known-identity variant mismatch, then unsupported identity.
For an unknown canonical identity, the final reason is `RegisteredOpaque` for
an opaque payload or `UnsupportedIdentity` for a typed payload.

The API accepts only an immutable graph reference. It cannot receive a backend,
Context, entity, allocator, or controller and therefore cannot invoke dynamic
Condition activation or Behaviour/Hook lifecycle calls. Construction factory,
AddParam, and structural Process callbacks necessarily occurred earlier to
produce the processed graph; the zero-effect claim does not include those
construction steps.

## Mounted refusal oracle

The existing typed mounted gate runs preflight on the already constructed and
processed graph. It reproduces all and only the 2,902 opaque leaf occurrences:

| Kind | Unsupported occurrences | Kind/name selector keys |
|---|---:|---:|
| Condition | 1,173 | 73 |
| Behaviour | 1,662 | 90 |
| Hook | 67 | 4 |
| Total | 2,902 | 167 |

The four Hook groups are `GrabSlide` 2, `InitializeLocomotion` 6,
`MongoPushToAntic` 2, and the native spelling
`OverideNextAnimTransitionHook` 57. The first refusal is `LeafId(0)`
Condition/`PhysFilteredState`; the last is `LeafId(5303)`
Condition/`IsInDebugAnimationsMode`.

Two independent digests freeze both aggregation and exact unsupported-leaf
placement in the full 5,304-leaf arena (2,402 typed plus 2,902 unsupported):

- 167 kind/name/count rows, 4,820 bytes:
  `067590A558AA8EE01919FB67EE7B70045C814804E1B83943FE858D68A84BE456`.
- 2,902 `LeafId`/kind/name rows, 86,183 bytes:
  `3FD010B7611FF8662AB21779B51B7290828C0FBE9EAC3DF2178CC782C6458600`.

The root release archive probe passed against the supplied corpus. Seven core
tests cover exact triples and requested-case retention, every mismatch class,
unprocessed graphs, duplicate paths, disabled/replaced-unreachable leaves,
complete raw diagnostics, and deterministic malformed-host precedence. Three
independent audits were used; one caught and corrected the typed-canonical plus
opaque mismatch classification before acceptance.

| Source | SHA-256 |
|---|---|
| `motion_graph_capability_preflight.rs` | `321832F1B3EDA22CF3ECD0500EEE5DA47DA9EDA46AF25FACBFEFE0C3ADAF88EB` |
| `motion_graph_leaf_factory.rs` | `D64AE90E1F9FD74997B12FF4660604F3C90FCB145590891F9ECDB009F330DB97` |
| `state_graph_runtime.rs` | `6AF512D03DD8D653C1C80D313ED8BD0666767B52BD4424688C619BD141548721` |
| `motion_graph_typed_corpus.rs` | `D55D5D0CAEF2C671E7E553FB091C2B611A971887A7BA311CF283A581351A18F5` |
| `main.rs` | `070A53F6B66EB67A638DB21B089A43ACBE6B62811A7A03BC32F03933234A17C4` |
| `lib.rs` | `BA098290724D667F7640F754A137818DD0E37E947236E86079122A0D420F7596` |

## Nonclaims

- The three typed payload variants are not yet wired through one concrete,
  entity-bound dynamic backend. Full leaf coverage would not by itself prove a
  runnable frame.
- The current mounted onboard graph is explicitly refused, not partially run.
- No controller is constructed, no initial State is selected, and no
  activation, animation dispatch, physics step, input, or gameplay frame is
  claimed.
- Generated ActionGraph/MotionGraph controllers remain separate from this
  generic dynamic-fallback preflight.
- The report is not a reusable capability token. A future consuming adapter
  must revalidate the exact triple at every callback boundary.
