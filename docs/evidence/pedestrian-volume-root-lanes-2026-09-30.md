# Pedestrian Actor avoidance root lanes

First-frame runtime error: `pedestrian volume Actor root contains unknown lanes`.
The adapter required SkeletonRootMatrix.known_lanes(), which requires all four
W lanes although XYZ is already known. Existing physical roots deliberately
retain unknown SIMD W lanes rather than invent native pointer/padding values.

IDA Actor.GetRootTransform825ed2c0 copies the root at PhysOut+368. Native
CreateVolumeNavRigLineTestBatch82e868d8 reads that getter82e86a58, offsets world
Y by0.9, and adds the volume82e86a84 with extents(.5,1,.5). AddVolume82d16060
copies the data. BoxVolume.LineSegIntersect82ad7170 transposes XYZ for the
inverse point transform82ad72d4..7378; geometric tests and distance reductions
consume XYZ. SIMD W does not affect their hit parameter or XYZ outputs.

The adapter now passes the typed root to add_skater. That query boundary uses
the existing xyz_scratch projection, without changing the retained PhysOut,
shape extents, orientation, mask, batch ordering or geometry implementation.
This is a host query projection, not a claim to recover native W values.

Added a regression case comparing avoidance intersection for unknown, zero and
NaN W with the same rotated/translated XYZ root. Syntax checks passed; tests
and compilation were not run.

## Follow-up: Prop Actor root lanes

The next run reached the same overly strict condition in Prop preframe.
Actor.GetPosition825f9df0 reads PhysOut+416, exactly the root translation.
ActorTracker.PreStateGraphUpdate82e64990 obtains the local Actor's position,
then Prop.SetPropPosition82e66a38 calls LWEntityCoreDataProxy.SetPosition82e57770:
native copies the vector and builds its XZ projection.

The host adapter now uses the existing xyz_scratch projection for this Prop
input too. The retained PhysOut is untouched. Existing known W is retained;
unknown W becomes scratch zero in the host geometric core, NOT a claim to
recover the native copied W bit pattern. Player distance uses dot3 (including
its previous-vector delta); actor-tracker spatial consumers use XYZ/XZ. No
position, distance, timing, behavior, or fallback spawn location is invented.
This host representation limitation is explicit rather than stopping gameplay
for a non-geometric lane. Remaining known_lanes checks in living-world source
concern ground-query results and head COM, not this Actor-root adapter.

Source syntax parsing passed; this follow-up is not yet compiled or tested.
