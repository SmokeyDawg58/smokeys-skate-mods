# Constructor animation projection without placeholder assets

`ActorPhysicalAnimationInputs::try_assembly_inputs` builds the physical assembly
input directly from the same retained Actor constructor animation result. The
caller supplies cvars, the retained Vault, selected spawn and actual selected
hat; there are no corpus imports or placeholder skeletons. Root owns its later
production facade caller; this method alone does not establish session startup.

The existing `Actor825fef00..825ff16c` producer is unchanged: animation
initialization, animation/physics input, actual `phys_Tpose` parameters, sampled
physical constructor flags, and evaluated bind pose retain their established
ordering. No new source behavior or flag producer is introduced.

Both paths use shared hierarchy identity validation/extraction and evaluated pose
extraction, the existing `AnimationPhysicsRef::snapshot`, and the same sampled
flags. The assembly's mapping and pose hierarchies point to the same projected
Arc. Existing immutable constructor copies are preserved rather than adding a
new animation/physics owner or resampling animation/flags.

`try_apply_to` retains its exact prior partial-write boundaries: identity and
hierarchy read may fail before mutation; hierarchy is assigned before parameter
snapshot can fail; parameters and pose hierarchy are assigned before pose read
can fail; flags are assigned only after successful pose extraction. Cvars,
Vault, spawn and hat remain untouched by this existing apply method. The new
constructor has no pre-existing caller state to mutate on failure.

Artifact `artifacts/physical-animation-projection-2026-09-22.patch` changes only
the existing projection module and stale assembly input documentation. No
registered edits or Cargo run by author. Existing mounted startup continues to
exercise `try_apply_to`; production constructor acceptance belongs to root's
facade integration. This refactor adds no test-only services or substitute data.

Independent source/code review approved the shared projection and verified the
existing assignment/failure boundaries against the registered implementation.
Author and reviewer apply checks passed. Root retains Cargo and caller validation.
