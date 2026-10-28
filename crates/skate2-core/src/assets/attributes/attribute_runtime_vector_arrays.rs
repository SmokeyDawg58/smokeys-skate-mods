//! Read-only float3/float4 array elements from the existing loaded Vault.
//! Collection.GetData8297b7e8: count, encoded header padding, then stride*index.
use super::{AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance};

impl AttributeRuntimeInstance {
    pub(crate) fn try_vector_array_data(
        &self,
        key: u64,
        index: u32,
        lanes: usize,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        use AttributeRuntimeError as E;
        let expected_type = match lanes {
            3 => 0x5034_3731_81f5_86e0,
            4 => 0x1ae2_6d4c_3727_1167,
            _ => {
                return Err(E::PayloadSize {
                    key,
                    expected: 3,
                    actual: lanes,
                });
            }
        };
        self.try_plain_array_data(key,index,expected_type,lanes*4)
    }

    /// Same checked POD storage route for source-declared fixed-size records.
    /// Caller supplies the original type identity, never just a matching width.
    pub(crate) fn try_plain_array_data(
        &self,key:u64,index:u32,expected_type:u64,size:usize,
    )->Result<Option<AttributeRuntimeData>,AttributeRuntimeError>{
        self.try_array_field_data(key, index, expected_type, size, 0..size)
    }

    /// Numeric fields of a typed record that may also contain relocated
    /// pointers. Only the requested field is projected as plain bytes; a
    /// relocation overlapping that field still fails in the existing reader.
    pub(crate) fn try_array_field_data(
        &self, key: u64, index: u32, expected_type: u64, size: usize,
        field: std::ops::Range<usize>,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        use AttributeRuntimeError as E;
        if field.start > field.end || field.end > size {
            return Err(E::PayloadSize { key, expected: size, actual: field.end.max(field.start) });
        }
        self.try_typed_array_projection(key, index, expected_type, size,
            super::text::Projection::PlainField { offset: field.start, size: field.len() })
    }

    pub(crate) fn try_array_text_field(
        &self, key: u64, index: u32, expected_type: u64, size: usize, offset: usize,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        if offset.checked_add(4).is_none_or(|end| end > size) {
            return Err(AttributeRuntimeError::PayloadSize { key, expected: size, actual: offset });
        }
        self.try_typed_array_projection(key, index, expected_type, size,
            super::text::Projection::TextField { offset })
    }

    fn try_typed_array_projection(
        &self, key: u64, index: u32, expected_type: u64, size: usize,
        projection: super::text::Projection,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        use AttributeRuntimeError as E;
        let Some((class_key,class,collection,node))=self.resolved_node(key)? else{return Ok(None)};
        let definition = class.definitions.get(&key).ok_or(E::MissingDefinition {
            class: class_key,
            key,
        })?;
        if definition.type_id != expected_type || node.type_key != Some(expected_type) {
            return Err(E::NodeTypeMismatch {
                key,
                node_type: node.type_key.unwrap_or(0),
                definition_type: expected_type,
            });
        }
        if usize::from(definition.size) != size {
            return Err(E::PayloadSize {
                key,
                expected: size,
                actual: usize::from(definition.size),
            });
        }
        if node.flags & 2 == 0 {
            // Source ignores index for a scalar. Reuse its existing projection.
            return self.node_data(
                class_key,
                class,
                collection,
                node,
                key,
                projection,
            );
        }
        // The shipped colour/SH arrays are stored-pointer POD nodes (flags2).
        // Do not assume array_range's Instance-layout choice is equivalent to
        // Collection.GetData's resolved-owner layout for other storage forms.
        if node.flags & 0x70 != 0 {
            return Err(E::UnsupportedArray { key });
        }
        if (node.flags & 8 != 0 || definition.flags & 0x20 != 0)
            && !serialized_living_world_field(expected_type, size, projection)
        {
            return Err(E::UnsupportedConstructed { key });
        }
        let header = self.array_range(class_key, key, node, 0, 8)?;
        let half = |offset: usize| {
            u16::from_be_bytes(
                header.bytes()[offset..offset + 2]
                    .try_into()
                    .expect("two bytes"),
            )
        };
        if index >= u32::from(half(2)) {
            return Ok(None);
        }
        let stride = usize::from(half(4));
        if stride != size {
            return Err(E::PayloadSize {
                key,
                expected: size,
                actual: stride,
            });
        }
        // Encoded bit15 adds eight padding bytes: float4 data starts at16;
        // float3 SH data starts at8. Count is at+2, not capacity at+0.
        let offset = 8 + usize::from((half(6) >> 12) & 8) + index as usize * stride;
        if let super::text::Projection::PlainField { offset: field, size } = projection {
            return self.array_range(class_key, key, node, offset + field, size).map(Some);
        }
        let location = self.vault.pointer(node.stored_word.ok_or(E::UnsupportedArray { key })?)?
            .ok_or(E::UnsupportedArray { key })?;
        self.vault.project_data(location, offset, size,
            super::AttributeRuntimeDataOrigin::StoredPointer, projection)
    }
}

/// These original constructed types have no-op Construct/Destruct slots
/// (825187c0, vtables82167b24/82167b10/82167b38/82167afc). Read only their authored collection
/// key and numeric suffix; never expose RefSpec's runtime cache or padding.
/// All other constructed types/projections retain their explicit rejection.
fn serialized_living_world_field(type_id: u64, record_size: usize, projection: super::text::Projection) -> bool {
    use super::text::Projection::PlainField;
    matches!((type_id, record_size, projection),
        (0x37ee_ae39_0851_cff9, 32, PlainField { offset: 8, size: 8 } | PlainField { offset: 24, size: 4 })
        // ClassRefSpec_cac_stamp: Construct82167a48 is empty;
        // CASStampHelper82595724 reads the serialized collection qword only.
        | (0x761c_0328_0f99_4647, 16, PlainField { offset: 0, size: 8 })
        // tLWRefspecProbabilityPair: RefSpec key + authored float probability.
        // Assign828b5658 copies exactly these fields; cache16..20 is excluded.
        | (0xe4aa_c2ea_bc05_3d0a, 32, PlainField { offset: 8, size: 8 } | PlainField { offset: 24, size: 4 })
        | (0xa0a8_bf1b_a3b3_6a68, 40, PlainField { offset: 8, size: 8 } | PlainField { offset: 24, size: 12 })
        // tLWHandPropUsage Assign828b56a0: two RefSpecs and two floats.
        // Neither RefSpec cache (16/48) nor its padding is authored POD.
        | (0xd677_8895_b505_d1db, 56,
            PlainField { offset: 0 | 8 | 32 | 40, size: 8 }
            | PlainField { offset: 24 | 28, size: 4 }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute_runtime::{AttributePointerValue,
        tests::{collection, definition, graph, node, set_dependency_bytes}};
    use std::sync::Arc;

    #[test]
    fn constructed_living_world_fields_preserve_inheritance_stride_and_reject_cache_reads() {
        for (type_id, stride, suffix) in [(0x37ee_ae39_0851_cff9, 32, 24..28),
                                         (0xe4aa_c2ea_bc05_3d0a, 32, 24..28),
                                         (0xa0a8_bf1b_a3b3_6a68, 40, 24..36),
                                         (0xd677_8895_b505_d1db, 56, 24..28)] {
            let mut def = definition(3, 0x25, stride as u16);
            def.type_id = type_id;
            let mut vault = graph(1, vec![def]);
            collection(&mut vault, 1, 10, 0, None, vec![node(3, 0x0a, 1)]);
            collection(&mut vault, 1, 11, 10, None, vec![]);
            let entry = vault.collections.get_mut(&(1, 10)).unwrap().nodes.get_mut(&3).unwrap();
            entry.type_key = Some(type_id);
            let slot = entry.stored_word.unwrap().location;
            vault.pointers.insert(slot, AttributePointerValue::Direct { dependency_index: 1, offset: 0 });
            let mut bytes = vec![0; 128];
            for (at, value) in [(0, 2u16), (2, 2), (4, stride as u16), (6, 0x8000)] {
                bytes[at..at+2].copy_from_slice(&value.to_be_bytes());
            }
            let base = 16 + stride;
            let key = 0x1234_5678_9abc_def0u64.to_be_bytes();
            bytes[base+8..base+16].copy_from_slice(&key);
            bytes[base+suffix.start..base+suffix.end].fill(0x35);
            set_dependency_bytes(&mut vault, 1, bytes);
            let instance = Arc::new(vault).instance(1, 11);
            assert_eq!(instance.try_length(3).unwrap(), 2);
            assert_eq!(instance.try_array_field_data(3, 1, type_id, stride, 8..16)
                .unwrap().unwrap().bytes(), &key);
            assert_eq!(instance.try_array_field_data(3, 1, type_id, stride, suffix.clone())
                .unwrap().unwrap().bytes(), vec![0x35; suffix.len()]);
            assert!(instance.try_array_field_data(3, 2, type_id, stride, 8..16).unwrap().is_none());
            assert!(matches!(instance.try_array_field_data(3, 0, type_id, stride, 16..20),
                Err(AttributeRuntimeError::UnsupportedConstructed { .. })));
            assert!(matches!(instance.try_plain_array_data(3, 0, type_id, stride),
                Err(AttributeRuntimeError::UnsupportedConstructed { .. })));
            if stride == 56 {
                for field in [0..8, 28..32, 32..40, 40..48] {
                    assert!(instance.try_array_field_data(3, 1, type_id, stride, field).unwrap().is_some());
                }
                assert!(matches!(instance.try_array_field_data(3, 1, type_id, stride, 48..52),
                    Err(AttributeRuntimeError::UnsupportedConstructed { .. })));
            }
        }
    }
}
