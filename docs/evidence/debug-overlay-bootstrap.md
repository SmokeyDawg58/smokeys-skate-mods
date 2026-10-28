# Concrete debug overlay bootstrap

Confidence: high for the selected allocation/publication and getter path.
This implements application service existence, not GPU drawing or full InitApp.

PDB `Sk8::Debug::DebugOverlay` has size1 and zero members. Actual
`DynamicMemorySingletonTypeTraits<DebugOverlay>::Construct`8272b300 first
checks the current singleton pointer atb31c..324; a second construction reaches
the native assertion. It obtains the `Singleton` allocation category atb340..348,
then allocates size1, alignment1, offset0, flags101 with the supplied debug name
atb34c..364. The resulting pointer is published atb368. There is no member
constructor, virtual-table store, renderer allocation or callback afterward.

`InitApp`827352a8 passes the actual slot83059210 and debug name
`Sk8::Debug::TheDebugOverlay`; call52b0 constructs it. Only afterward does
52b4 sample the thread identity and52c4 publish the creation-thread field.
`Get`82581e40 checks the actual current instance, asserts when absent, and
returns it. This getter does not enforce a thread check or initialize on demand.

The Rust object therefore has one uninitialized byte and a stable allocation
identity. Its shared holder retains the instance and host thread identity as
separate fields. Host allocation and synchronization are explicit platform
mechanisms; no memory-category allocator equivalence or OOM behavior is claimed.
Cloning the holder does not sample the instance: consumers call `require` at
the original read point. Missing and duplicate construction remain errors.
The simulation session retains this application service for all its skaters;
the controlled original startup explicitly initializes it before Actor work.
It is never fabricated per attribute record or lazily created by a getter.

This is needed by the unconditional ProcessAnimAttributes82b59e18..e48 check.
Toolkit's enabled draw branch82b26d90..dd8 also only checks the singleton in
this executable; an existing instance branches to the final normal copy atddc.
It contains no surviving draw call. Other debug consumers require separate
source review before their behavior can be admitted.

Root independently inspected fresh raw instructions and PDB layout; graph
independently accepted the allocation/publication implementation. A focused
test checks absence, concrete size, shared-holder visibility, creation thread
and duplicate-construction identity preservation. Integration gates pending.
