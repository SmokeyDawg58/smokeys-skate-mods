# WriteSkater matrix Q4 compression

Unregistered draft; parent independently reviewed original source and this
author independently compared the shared quaternion helper. Cargo and original
data validation remain pending. No registered files were changed.

`WriteSkater`0x827d8ab8 contains the body matrix loop0x827d8c94..0x827d8e20.
The signed nonpositive count check8ba0/8ba4 skips the loop; an empty original
offset list is therefore a genuine empty operation, unlike the Q3 primitive's
separate final-element tail. Index byte8cb8 chooses a local matrix by stride64.
The Rust API borrows the actual matrix buffer and the already-admitted
`PresentationOffsets.matrix_rotations: Vec<usize>` in original order. It does
not gather or reorder a second pose buffer. Destination capacity is an explicit
host memory admission check; matrix indices are checked only as reached.

`try_simulation_quaternion_candidate` in world.rs matches the complete raw graph:

- Sign globals x83091fa0/y83091f80/z83091f70 are initialized at82f7eec8,
  82f7eef8,82f7ef28. Their lanes are [0,0,sign,sign], [0,sign,0,sign] and
  [0,sign,sign,0], respectively, where sign=80000000. Static IDA BSS zeros are
  not the initialized values.
- One and half come from integer1 converted at8bc8/8bd0 with scales0/1.
  Trace comparison zero is82110df0, loaded8c98. Magnitudes at8d14/1c/38 are
  `(signedX+signedY)+(signedZ+1)`, retaining that addition grouping.
- The all-lane estimate8d50 includes unselected zero or negative candidates.
  Two standard VMX negative-fused residuals8d68/8d78 and Newton steps8d6c/8d7c
  exactly match the helper's `-magnitude.mul_add(seedSquared,-1)` graph.
- Root/factor multiplies8d84/8d88 and the original permute constants produce
  the same four candidates. Strict selects8db0/8db4/8db8 use yy>zz,
  xx>yy&&xx>zz, and trace>0; ties and unordered lanes preserve source behavior.
- There is no dot4, reciprocal norm, normalization or orthonormalization
  between candidate selection8db8 and caller-ratio multiplication8dbc.
  Input axis W and translation do not participate in quaternion conversion.

The actual ratio vector is retained from caller r27 at8bb4 and supplied to the
new API; it is not replaced by a constant. Four fctiwz conversions occur X at
8dcc, Y8dd4, W8dec, Z8df8. All four finish before the first output write.
The source extracts each integer's low16 bits and writes X+0 at8e08, Y+2 at
8e0c, W+6 at8e14, Z+4 at8e18. Logical `[u16;4]` destinations retain this mutation
order. The future stream writer must emit their logical XYZW lanes big endian.
No clamping, saturating float cast or hemisphere adjustment is added.

API: `try_compress_matrix_q4(matrices, indices, destination, ratio, numeric)`
in the new skater_presentation_compression_q4.rs. The existing ReckoningNumeric
scalar estimate interface adapts the one vector estimate in lane order0..3;
an unsupported estimate stops before integer conversion or destination writes.
Xenon estimate seeds, denormals, FP exceptions and NaN payloads remain explicit
host numeric boundaries. This does not introduce another quaternion algorithm.

Six drafted tests distinguish strict tied-candidate signs, original index order,
no final normalization, irrelevant NaN W/translation, every unselected estimate,
X/Y/W/Z integer conversion, low16 wrap, per-conversion failure atomicity for the
current record, prior-record preservation, source bounds and genuine empty list
skip. Only rustfmt has run; tests await the parent registration/gate.


## Root independent code review, September12

Root reviewed the Q4 draft against the previously reviewed WriteSkater raw
body and existing quaternion-candidate helper. It keeps all estimate lanes,
no extra normalization, X/Y/W/Z conversion order, and X/Y/W/Z halfword stores
at their actual offsets. Late matrix failures preserve prior records. This
unregistered primitive still awaits compiler/tests and live WriteSkater use.
