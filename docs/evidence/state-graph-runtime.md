# StateGraph structural runtime

Date: 2026-09-07. This checkpoint reconstructs the matched debug build's
compiled-StateGraph construction and structural `Process` layer. It does not
claim the gameplay semantics of the concrete condition, behaviour, or
transition-hook classes.

## Reference identity

- Matched debug XEX SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`.
- PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`.
- MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.
- Runtime proof packet SHA-256:
  `8634C4F5E34FEA2A2008707CE9C8688A7318B2BF327E67C1E9FB9546B6DFA669`.
- Runtime corpus oracle SHA-256:
  `3E89771E6B73AAF2C48DD9F897370EFCBF647E9EE12262B2CC23F6976034C7A7`.
- Registry proof packet SHA-256:
  `4E8E9F77BEEA216F51C9EA5C363961FDC10070EA1B8B17AEBAB58EC975CC2871`.
- Registry oracle SHA-256:
  `39BA23D6660D791FB795CD49993BD4F664258050266769570A2D79C97F14C9D4`.
- Registration TSV SHA-256:
  `E588A10B051A6A41AA79A70E6F300779B09BA6890AD1736B62BB1A0BB45CF99B`.
- Per-file factory census SHA-256:
  `92B2C25DD3B7AB50F305BF33EF8277C09F2FA405EC19EB1F4AB52383F60FC4AE`.
- Mounted `miscload.big` SHA-256:
  `0512157AAADD88A6E1992509944D0AAFF49EFAD2DB7BDD8C32AA4774A92F7F13`.

The runtime packet is backed by named MAP/PDB records and matched-XEX
assembly/pseudocode for `Parser::ReadElement`, `Parser::ProcessElement`, the
direct Base/State/Transition/Expression constructors, factory base
constructors, `State::Find`, and the root structural `Process` traversal.

## Reconstructed behavior

The core runtime consumes the existing lossless binary element decoder and
constructs typed arena records in exact preorder. It preserves the 24-entry
object/null stack, top-only context casts, first-State root selection,
constructor-time parent attachment, separate child/behaviour/transition/hook
orders, second-expression replacement, direct defaults and case-sensitive
enum parsing, and raw byte names, paths, and attributes.

Factory selection uses the recovered ASCII case-insensitive name domain. The
application catalog freezes 286 condition, 283 behaviour, and 18 hook linked
registrations, retaining their exact canonical spelling and registry domain.
The core exposes strict assertion-equivalent misses and native safe-null misses
as explicit policies.

Structural finalization preserves:

1. State interrupt resolution, expression, child states, behaviours, then
   transitions.
2. Transition expression, upward target lookup, checked error on a missing
   target, disable on a resolved disabled target, then hooks.
3. Expression child order and synchronous opaque leaf `Process` callbacks.

`State::Find` implements last-dot recursive prefix resolution, then a
non-upward suffix lookup. The first mounted run exposed an incorrect
single-level prefix implementation on
`TrickOutOfTipSlashGrind.At90.TrickFrom90Tail`; correcting it to the recovered
recursive call made all shipped targets resolve. This is why the mounted-data
gate is required in addition to synthetic unit tests.

## Mounted corpus result

The release probe reads all 36 compiled StateGraphs from the real mounted
archive and independently checks each file's frozen structure before and after
finalization:

- 2,681 states;
- 2,457 transitions, all resolved;
- 3,561 expressions;
- 8,393 factory requests;
- 8,375 attached opaque leaves;
- 165 behaviour params, all routed to the callback override;
- 68 named interrupt targets, all resolved to self or an ancestor;
- 18 exact misses under the explicit native-safe-null host policy, all the
  unregistered condition `DistanceFromWaypoint` in the six expected
  living-world files.

The gate pins every payload SHA-256, recomputes key-only hashes over all 587
embedded registry names, and recomputes the frozen selector-count histograms
(205 condition, 246 behaviour, and 15 hook names). An independent decoded-
preorder trace checks every successful factory callback's typed ID, parent,
requested/canonical name, and complete ordered raw attributes; all push/pop and
Param null-slot windows; and exact, once-only leaf callback order during
structural `Process`. The frozen oracle does not contain resolved full-path IDs,
so target success and the exact named-target text histogram are checked, but
resolved target identity is not falsely claimed as an independent corpus
oracle.

`DistanceFromWaypointXZ` remains a distinct registered hit; the missing name
is not silently aliased. The attached-condition mask totals are 3,624
precondition, 21 sustain, 822 always, and zero postcondition. They differ from
the raw request totals because all 18 safe misses have `mask="always"` and do
not construct a Condition base.

The mounted release output was:

> parsed, constructed, resolved, and structurally finalized 36 compiled
> StateGraphs / 2681 states / 2457 transitions / 3561 expressions / 8375
> opaque registered leaves / 18 exact safe misses / 1017704 bytes

## Deliberate boundaries

- The 205 condition, 246 behaviour, and 15 hook selector names exercised by
  the shipped corpus are opaque payloads. Their constructor parameters,
  activation, gameplay `Process`, and most `AddParam` behavior remain open.
- The linked catalog proves registration membership, not any concrete class
  implementation.
- XML parsing, include loading/substitution, graph-cache orchestration, and
  controller ownership are outside this runtime seam.
- Checked Rust errors replace debug asserts/crashes for malformed public input.
- Arena IDs preserve reachable graph edges and ordering. Unreachable objects
  created under null contexts and expressions replaced by a later Node
  expression remain diagnostic arena tombstones; native allocator addresses,
  intrusive reference-count timing, and destruction timing are not modeled.
- A failed callback is not rolled back. Structural `Process` is one-shot so a
  checked-host retry cannot duplicate callback effects.
- Ordinary finite-input float behavior is irrelevant to this construction
  layer; this checkpoint does not make any physics-parity claim.

The result is therefore a proven structural runtime and a reliable dispatch
surface for recovering concrete gameplay leaves next. It is not yet a
gameplay-ready StateGraph controller.
