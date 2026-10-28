# Physical construction state retention

This is a host ownership refactor of the existing source-reviewed constructor,
not a new native construction path or a production session facade.

`PhysicalPlayerAssemblyConstruction` previously retained both `&mut
PhysicalAssemblyWorld` and every partial candidate, child constructor, drive,
handle and stage flag. An owning startup object could not store it beside the
world without a self-reference.

The existing borrowed adapter now contains the same world borrow and one
`PhysicalPlayerAssemblyConstructionState`. `into_retained` moves that entire
state out, releasing the world borrow. Consuming `try_bind` checks the retained
physical owner's `SimulationManagerContactAuthority` against the supplied world
before returning the same adapter. A failed bind returns the state intact.
Authority is the existing globally issued nonzero identity, not a capacity or
pointer/address comparison; moving the original world does not invalidate it.

All existing stages remain on the original adapter. New construction still
allocates the owner and board constructor in the original order. Mechanical
`self.state` field projections leave constructor calls, stores, guards and
failure prefixes unchanged. Binding does not reset a stage, recreate a child,
run teardown, or authorize retry of a failed constructor stage. Explicit teardown
still follows the original skeleton/hook/board ordering and retains candidates.

The selected completion method still executes board, skeleton, live finishing,
controllers, physics states, HiLOD/Ground entry and Actor publication. No caller
collaborators have been replaced: capsule allocation, assertion and hat-enabled
services remain supplied by the enclosing caller.

Compatibility: existing callers still use `new(&mut world, inputs)` and existing
methods. Child diagnostics inspecting private fields follow their moved fields.
The repeated original-data ground driver detaches/rebinds the genuine allocated
factory before completing that same physical construction. The focused test
detaches a real board-definition failure, rejects a different world without
changing either backend snapshot, recovers the same owner and partial child,
verifies reconstruction stays forbidden, then uses existing explicit teardown
and detaches/rebinds the removed state.

The session caller must recover state after an ordinary constructor `Result`,
including `Err`, before returning. This move-owned API does not install an
automatic unwind guard or claim resumability of native constructor stages.
No Cargo was run by the author; root owns compile/test acceptance.
