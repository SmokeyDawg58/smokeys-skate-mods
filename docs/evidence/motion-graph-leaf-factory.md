# Typed MotionGraph leaf factory

Date: 2026-09-07. This checkpoint installs the three recovered MotionGraph
stored-data constructors in the generic compiled StateGraph construction path.
It proves construction, direct `Param` attachment, and structural `Process`;
it does not implement activation, animation playback, generated wrappers, or
a simulation frame.

## Evidence boundary

The concrete `HasIntent`, `WillExpire`, and `PlayAnimation` implementations
retain the XEX/PDB/MAP and PPC evidence documented by their individual
checkpoints. The exact mounted onboard graph is the independent integration
oracle.

| Artifact | SHA-256 |
|---|---|
| Corrected Motion-leaf proof | `BB434DF3A561779BAB3F03F0881EE382CDD08CD5F8E2BEFF6F573734ACD8C73F` |
| Main raw instruction packet | `6398A335359DF88FDF1299D823AAFA0643294791BB6CC90E424F2997C28B73B2` |
| Focused FastString/PPC packet | `5E3EB4391D41E4FCBED685FF576FC2D92996D8A15EF57508FA008BAEBD82B8F1` |
| Mounted `MotionGraph_OnBoard.stategraph` | `E5F317CCAC9FAD0D126B3DF6810C9602CBB081F23FE39FB44662C92D4F13F7ED` |
| Focused leaf corpus oracle | `9716F1E652D4CA7F5150BEAB48612BEC97488E9209FED1A19DDCE38D3B4DBED6` |

## Routing seam

`MotionGraphLeafFactory` decorates the complete frozen registration catalog
without reordering or replacing its lists. It selects typed payloads only for
the exact `(kind, canonical registered name)` pairs `Condition/HasIntent`,
`Condition/WillExpire`, and `Behaviour/PlayAnimation`. The original requested
spelling remains diagnostic input and does not override the canonical identity
chosen by the recovered case-insensitive registry search. Every other
registered Condition, Behaviour, and Hook is delegated exactly once to the
fallback factory and retained as `RegisteredOpaque`.

The wrapper validates that the fallback exposes the three canonical entries,
has no embedded-NUL registrations, and has no ASCII-case-equivalent duplicate
within a kind. These are checked host-configuration constraints for the
already-frozen catalog, not claims about unchecked malformed native registry
behavior.

Later `AddParam` and `Process` callbacks revalidate the leaf kind/name against
the stored payload variant before touching either the typed payload or the
fallback. `PlayAnimation` Params use the recovered ordered parser. The two
conditions accept the structural Param callback as a handled no-op, and all
three recovered leaves accept structural `Process` as a no-op. Opaque return
values, errors, mutable payloads, and callback order pass through unchanged.

## Mounted integration gate

The application reads the exact 641,241-byte onboard graph, requires its
10,602-node / 20,419-attribute / depth-12 decoded census, and independently
recomputes the frozen condition occurrence digests. One typed runtime then
constructs:

- 815 `HasIntent` conditions;
- 780 `WillExpire` conditions;
- 807 `PlayAnimation` behaviours;
- 2,902 remaining registered opaque leaves.

The arena contains 1,354 States, 1,724 Transitions, 2,055 Expressions, 5,304
leaves, and 165 direct Params, with zero factory misses. All 165 Params are
marked handled, target typed `PlayAnimation` payloads, and appear in those
payloads in source order; the opaque fallback receives none. The gate compares
all recovered field distributions with the prior independent condition and
PlayAnimation oracles.

After construction, the gate derives the entire expected grouped `Process`
sequence independently from arena edges. The runtime produces the same 10,437
events and visits all 5,304 leaves. The opaque fallback additionally records
and matches its exact 2,902-leaf delegated subsequence.

## Verification and limits

Eight focused core tests cover registry preservation and rejection, exact
canonical routing, constructor-error identity, typed and opaque Param/Process
boundaries, failed-Param atomicity, and mismatch rejection before delegation.
Formatting, focused tests, strict core/application clippy, and a root-run
release `--inventory-bam-sim` probe against the supplied game root pass. Two
independent post-implementation audits cover the core routing seam and mounted
integration.

This checkpoint does not make the onboard graph runnable. Of its 75 exercised
condition selectors, 91 behaviour selectors, and four hook selectors, only two
conditions and one behaviour have typed stored-data bodies here; 2,902 leaves
remain opaque. `HasIntent` and `WillExpire` activation still need live
collaborator binding, while `PlayAnimation` still needs instance and
`Begin`/`Update` lifecycle integration. Generated parent writes and direct
generated dispatch remain separate from the dynamic arena. No animation,
physics step, generated tick, or gameplay parity is claimed.

| Source | SHA-256 |
|---|---|
| `motion_graph_leaf_factory.rs` | `D64AE90E1F9FD74997B12FF4660604F3C90FCB145590891F9ECDB009F330DB97` |
| `motion_graph_typed_corpus.rs` | `F785E0333124E132D84A9377A9916BA03E17552CCA087D40F76D4AA077224FD6` |
| `lib.rs` | `3B0861C326F54B10D331ED6694EA1574B7C3B5D3151179777490F50A3AEAF6D7` |
| `main.rs` | `7C7C966CBA937DB0AA1C4564B123AB8E1277D6F2827C920820EDEE38C560C7DC` |
