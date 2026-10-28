# Selected retained physical constructor completion

This extension runs on the existing PhysicalPlayerAssemblyConstruction, not a new
factory or physical owner. Actor allocation/registration remains the existing
try_allocate_physical path (Actor allocation 0x825ff2d8, IPhysical registration
0x82dfe5d0). Host identity admission checks that the supplied Actor retains that
same factory owner before any constructor store.

The successful sequence is unchanged from the fresh original-data driver: borrow
actual Actor fields while constructing board and skeleton using live cvars;
release the Actor prefix guard; finish skeleton through the existing live
Processed/history adapter; construct skateboard and skeleton controllers, then
the existing 26-state sequence; complete HiLOD tail; publish the Actor physical
constructor return. It supplies no stance, hat, cvar, allocation or numeric defaults.
The host passes assertion, hat-enable, capsule allocation and numeric capabilities.

The original factory stays with the caller after every typed failure, preserving
its partial allocations, definitions and retained handles. There is no automatic
cleanup, rollback or retry of the constructor. Successful output contains the two
actual retained assembly handles.

Only fresh repeated-frame corpus mode uses this production completion. Existing
isolated constructor/ground diagnostic modes keep their original individual calls
and checks. Fresh mode retains same-owner cached/registered identity and initial
Ground checks, then calls the existing three-frame driver (now production tick).
It no longer invokes other constructor diagnostic check functions before its
first tick. Root owns compilation and original-data validation; no pass claimed.
