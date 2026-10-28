# Mounted animation source extraction

This packet extracts host services only. Native scheduling remains in the existing
`SimulationAnimationStartup::try_initialize`, recovered from
`Sk8::Simulation::InitAnimation` at 0x828a5640 and its `Andale::Init` call at
0x82d74228. The latter records the calling thread in the actual retained attribute
pool; the former samples `kUseVBR` independently for files and codec, loads the
selected databases, and retains the actual job owner through existing dispatch.

`MountedAnimationSource` owns supplied Arc references to the original MountSet and
AnimationJobManager, references the same live cvars, and samples a supplied fallible
thread callback only when requested by that existing startup owner. It performs no
environment lookup, archive selection, job initialization, default thread selection,
or early tuning reads. Mount, cvar and thread failures remain typed.

The shared patch registers the source and changes the original-data source wrapper
to delegate every operation. Only the test wrapper retains read counters, its
existing explicit thread-1 fixture policy, external-root discovery and assertions.
Both the fresh Actor startup and older isolated corpus drivers therefore use the
same production mounted source without changing their constructor sequence.

Focused tests cover thread failure after attribute-owner allocation but before any
cvar/database work, actual job identity, cvar changes after adapter creation, and
typed missing mounted data. Root runs Cargo and original-data validation; no result
is claimed by this packet. This is not yet the retained Actor startup/tick facade.
