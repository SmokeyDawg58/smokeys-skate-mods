# Reconstruction architecture

## Non-negotiable method

This is a re3/reVC-style source reconstruction in Rust.

- No PPC interpreter or guest CPU context in the shipped game code.
- No generated one-function-per-instruction source.
- No emulator renderer, audio stack, kernel, or copied emulator subsystem.
- Original game assets remain external runtime inputs.
- Reconstructed code uses meaningful types, ownership, modules, and tests.
- Platform-facing code is implemented through native PC backends.

The executable is a behavioral specification, not a runtime dependency.

## Evidence model

Each reconstructed subsystem gets an evidence note containing:

1. original decorated and demangled symbol names;
2. reference virtual addresses and object/library ownership;
3. recovered type layouts, enums, vtables, and important constants;
4. summarized control flow and call relationships;
5. known inputs/outputs and an executable validation procedure;
6. confidence: `exact`, `high`, `medium`, or `hypothesis`.

Rust source should use `#[doc = "Reference: 0x..."]` on important reconstructed
functions/types. Address references are traceability metadata only.

## Planned layers

1. `skate2-symbols`: parses and indexes map/IDA evidence.
2. `skate2-core`: platform-neutral reconstructed systems and shared contracts.
3. Native services: filesystem, jobs, timing, input, audio, renderer, and window.
4. Asset formats: BIG archives and the RX2/XSF/XST/GEO/CONST families.
5. World: streaming, inventory/resources, scene, collision islands, and entities.
6. Skater: motion graph, board/body physics, contacts, gestures, tricks, animation.
7. `skate2-pc`: bootstrap and native application wiring.

## Validation strategy

Behavior is compared at subsystem boundaries, not by visual intuition alone.
Parsers use byte-level fixtures and offset assertions. Simulation uses recorded
input traces and per-tick state snapshots. Transform, velocity, contact, and
trick-state divergence will be reported with the first mismatching tick.

Where the reference build cannot be instrumented directly, constants and branch
behavior are triangulated from disassembly, callers/callees, data files, and
repeatable observations. Guesses remain labeled until verified.

