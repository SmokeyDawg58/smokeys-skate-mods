# Vehicle visual array crash

Reported: `Visual / VehicleFrameError / UnsupportedArray` key
`16102294838065856219` (`0xdf76d7d773857edb`). This is the first vehicle
paint-colour array. The visual publication used scalar `try_get_data`, whose
explicit array rejection caused the crash after a vehicle reached publication.
This report does not yet establish that a car was rendered successfully.

## Original reference and asset checks

IDA `Attrib::Instance::GetAttributePointer` 0x82976308 delegates to
`Attrib::Collection::GetData` 0x8297b7e8. The latter resolves the node/owner,
returns scalar data without applying the index, and for arrays checks index
against the u16 count at header+2. Element address is:

`base + 8 + ((encoded_type_and_pad >> 12) & 8) + stride * index`

Stride is u16 at+4, encoded padding is u16 at+6. Out-of-range yields null.
GetData uses the resolved owner's layout, whereas the existing GetLength
helper uses the original instance layout. The new vector projection therefore
only admits the stored-pointer array form verified in the shipped assets;
other storage forms remain explicit errors rather than adopting wrong layout
semantics.

Read-only inspection of the Jan17 `skater.vlt`/`skater.bin` (not the Nov28 Vault):

- Livingworld_models class `4e740a6670f241a7`: colour keys
  `df76d7d773857edb` and `12026e2eed18cc8d`, type
  `1ae26d4c37271167`, size16, definition flags5, explicit node flags2.
  Headers include `0001000100108000` and `000a000a00108000`:
  float4 elements begin at+16, including eight padding bytes.
- SH key `f45dfcab6ec195cd`, type `5034373181f586e0`, size12,
  maximum9, definition flags5, explicit node flags2. Header
  `00090009000c0000`: float3 elements begin at+8.
  Render_locations class `6a2b5946514b8490` default data at BIN699772.

## Change and validation boundary

`attribute_runtime_vector_arrays.rs` shares existing node/parent resolution,
retained Vault storage and `array_range`. It validates type and stride, returns
the selected element, and preserves the existing scalar projection. The generic
scalar API still rejects arrays. No second Vault, model registry or colour
selection logic was introduced.

`selected_living_world_vehicle_visual.rs` now uses this projection for both
selected paint colours and all nine default SH vectors. Existing big-endian
float decoding and vehicle colour indices are retained.

Scoped rustfmt check passed. No Cargo, build, check, tests or game commands run;
compilation and runtime validation remain pending. Other vehicle runtime paths
may still expose separate unimplemented behavior on the next road test.
