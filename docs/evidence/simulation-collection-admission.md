# Simulation collection admission

## Scope and provenance

`simulation_collection_admission` contains the pure message-route decisions for
Simulation collection registration and deregistration after the collection's
raw stream type has been resolved. It preserves the exact deferral gates,
unsigned size comparison, conditional size query, and callback-only class. It
does not perform the notifier's loaded-count increment, live type-map lookup,
queue sends, registration, callback invocation, or acknowledgement.

Reference inputs are:

- XEX SHA-256
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB SHA-256
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- MAP SHA-256
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- Simulation dispatch packet SHA-256
  `A9D58D47623F59C478AE220859EE3F7A0333DBC1887C580AC41D222178B60207`;
- external admission oracle source SHA-256
  `443EC1D40D1C934A2037584B2EA1E14EB7787FB56B7C75F4299E469AEDA58265`.

## Registration decision

`cAssetStreamManager::NotifyCollectionSimReg` at
`0x82d5c468..0x82d5c67c` first unconditionally calls
`UpCollectionLoadedCount` at `0x82d5c48c..0x82d5c494`, then locks and resolves
the full collection ID in the stream-type map at `0x82d5c4a4..0x82d5c500`.
A missing entry asserts/debug-breaks and has no safe native continuation; the
pure selector therefore accepts only an already-resolved raw type.

Only raw type exactly zero or one proceeds to a registration message. For
those types, the branch order is:

1. live `kbEnableCollectionRegDefer` is true;
2. manager word at `+0x2a834` is nonzero;
3. per-call `forceImmediate` is false;
4. unsigned `GetStreamSize` is strictly greater than `0x0007d000`.

The size function is called only after the first three gates pass. Equality is
immediate, and high-bit sizes remain ordinary unsigned values. A passing route
retains native deferred state and returns at `0x82d5c60c`; its Arena,
activation-zero Collection, and Callback messages are emitted later by
`UpdateSimCollectionDefer`. A supported nondeferred route separately sends
`RegCollection(activate = 1)` at `0x82d5c614..0x82d5c644`, then Callback at
`0x82d5c648..0x82d5c670`. Other raw types send Callback only. Null callbacks
still occupy a message position.

The Rust selectors take the first four already-sampled values and a lazy size
closure. The fallible form propagates a size-provider failure only if execution
reaches the native `GetStreamSize` call site; the original infallible API is a
compatibility wrapper over it. This exactly preserves whether the size query is
reached, but live sampling and lookup locking remain responsibilities of the
outer notifier. The separate notification router consumes an already-selected
route and does not duplicate these gates.

## Deregistration decision

`NotifyCollectionSimDReg` at `0x82d59a00..0x82d59b10` independently resolves
the current stream type. Raw zero or one separately sends DRegCollection at
`0x82d59ab0..0x82d59ad8`, then Callback at `0x82d59adc..0x82d59b04`. Every
other raw value sends Callback only. This notifier performs no loaded-count
mutation and has no deferred branch. A changed stream class between Reg and
DReg is a later ownership-policy boundary; the pure classifier does not infer
or erase a retained host registration.

## Verification and boundaries

Six focused tests freeze raw types `0`, `1`, `2`, `3`, and `u32::MAX`; the
complete boolean/nonzero defer-gate matrix; strict threshold edges; unsigned
high-bit sizes; conditional size-query suppression; fallible size errors at
only the reached native call site; and the deregistration class.

The optimized `--inventory-bam-sim` gate applies enabled defer controls and
`forceImmediate = false` to every real Simulation XSM record. It classifies
205 of 284 collections immediate-eligible and 79 deferred-eligible, with none
equal to the threshold. All existing 2,135-arena FIFO/manager calls and seven
frozen registration/deregistration signatures remain unchanged and teardown
is quiescent. The independent Simulation admission signature is
`f0f413cd2abb59b2`.

The corpus partition is size eligibility, not a claim that all 79 defer at
runtime. A second isolated corpus pass now executes the selected immediate or
deferred route and matching stable-class DReg through the unified notification
router; see `simulation-collection-notification-router.md`. The live cVar,
manager word, caller override, and cross-thread timing are runtime inputs.
Loaded-hold ownership, missing-map behavior, native callback invocation,
class changes between Reg and DReg, and hull acknowledgement remain open.
