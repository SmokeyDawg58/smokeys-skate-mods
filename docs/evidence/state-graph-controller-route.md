# StateGraph generated/Dynamic route and retained onboard arena

Date: 2026-09-07. This checkpoint joins the already-recovered controller
selection and cache-path rules without claiming a cache implementation or a
runnable controller. It also changes the app's onboard corpus gate from a
summary-only result into a private retained processed arena with its complete
capability refusal.

## Evidence identity

- Matched debug XEX SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`.
- PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`.
- MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.
- Mounted/extracted `data\state\MotionGraph_OnBoard.stategraph`: 641,241
  bytes, SHA-256
  `E5F317CCAC9FAD0D126B3DF6810C9602CBB081F23FE39FB44662C92D4F13F7ED`.
- Corrected controller-boundary proof SHA-256:
  `12A5DB655DDF3FED35C8BDF244E58650B9A20C6AF05F17EE25AB129A53914CB4`.
- Generated-selection proof SHA-256:
  `465E0612BA72F5192ED47CB8BBD826F8691A5BDAB6F2CA74EA034DAC90A6043B`.

`AbstractHierarchicalController::Create` at `0x829513F0` selects generated
delegates from the final basename of the logical C-string request before a
Dynamic controller exists. `StateGraph::Cache::Load` at `0x82CF8AF0` owns the
subsequent Dynamic cache-path front. The onboard player load sites are Action
at `0x826002A4` and Motion at `0x82600350`, both through
`HierarchicalController::Load` at `0x82CEF180`.

## Exact host boundary

`route_state_graph_controller` receives the logical UTF-8 request and calls the
already-recovered selector first. A generated hit returns its typed onboard or
other-delegate route with no normalization or loader call. A required-generated
miss likewise returns before loader work. Only Dynamic fallback:

1. derives the exact `StateGraph` cache key;
2. derives the compiled `.stategraph` sibling;
3. invokes the supplied one-shot loader once; and
4. retains the logical request, cache key, compiled path, and exact loader
   result together.

For onboard Motion the identities are deliberately distinct:

| Role | Exact value |
|---|---|
| controller-selection request | `state/MotionGraph_OnBoard.xml` |
| normalized cache path | `data\state\MotionGraph_OnBoard.xml` |
| cache hash | `0x07D5E6BC` |
| physical compiled sibling | `data\state\MotionGraph_OnBoard.stategraph` |

Passing the physical `.stategraph` path to generated selection is proven to
miss the `MotionGraph_OnBoard.xml` delegate and take Dynamic fallback. The app
therefore fixes the logical request first and uses a diagnostic Motion delegate
value of zero to exercise the real compiled Dynamic corpus. This is explicitly
not the matched default: the enabled native-default Motion route is generated.

The app owner keeps the complete 1,354-state / 1,724-transition /
2,055-expression / 5,304-leaf processed arena alive along with the full 2,902
diagnostic report. Its arena field is private and it exposes no unchecked
`into_graph`; the current unsupported graph cannot be fed around the
authority-session preflight through this owner.

## Validation

Eight focused core tests cover generated laziness, exact onboard Dynamic
identities/hash, both `allow_generated=false` branches, required-miss ordering,
path-before-loader failure, exact loader-error source retention, generated
C-prefix handling before Dynamic embedded-NUL validation, the physical-name
trap, and a non-onboard generated delegate.

An ignored opt-in app test accepts only an external
`SKATE2_MOTION_GRAPH_PATH`. It was run against the exact 641,241-byte extracted
shipped payload and passed the entire combined path: PlayAnimation digest
oracle, logical route, compiled identity, binary parse, typed construction,
10,437-event structural `Process`, retained arena, and exact 2,902-entry
capability refusal. No proprietary bytes were checked into the repository.
The full mounted `--inventory-bam-sim` probe was also rerun against the supplied
game root after the final changes. It mounted 19 archive tables / 18,285
entries (one unavailable table), reached the same retained arena/refusal line,
and exited successfully.

Focused results:

- controller-route tests: 8/8;
- external exact-payload retained-route gate: 1/1 in 0.28 seconds;
- core tests: 1,925/1,925 plus six compile-fail doc tests;
- app tests: 38/38;
- formatting and strict core/app all-target Clippy: pass.

| Source | SHA-256 |
|---|---|
| `state_graph_controller_route.rs` | `7BF4B6C18B44153D448A987429D317BF34ADADE3C6CB10B90DD7B4EE18E69F00` |
| `motion_graph_typed_corpus.rs` | `2415C76F1A8A16352C540889B3483F0756616A665D5119310304359CCC1F2029` |
| `play_animation_corpus.rs` | `51D4960A1B85195735C7E1C89B9B1D55A5543C1AB3E0789FC92908A7B3747368` |
| `main.rs` | `BFF5272046ED386E3AE4992D052F5856223AA7A807AD7929761CC2111A808D87` |
| `lib.rs` | `FB3BF826E415940B053FB06E5AA8830EC7A2C4C1E121AA121F8F0B86284522E1` |

## Nonclaims

- The generic loader result has unconstrained type and its route identities do
  not certify payload provenance, particularly after `map_payload`.
- This one-shot seam does not implement cache lookup/hits, negative entries,
  flush/replacement/capacity, or shared graph lifetime.
- Old-controller deletion, Dynamic allocation/construction/reload, native
  assertion continuation, and allocation failure are not reconstructed here.
- The retained app arena is not admitted into a controller. No Context,
  runtime, allocator, activation, animation dispatch, initial-state selection,
  or frame tick occurs.
- The native-default generated Motion controller remains descriptor-only and
  untickable. Forced Dynamic still refuses all 2,902 opaque leaves.
