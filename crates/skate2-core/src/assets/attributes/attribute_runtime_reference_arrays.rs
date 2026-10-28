//! Read-only projections of the two census Members array types. Uses the
//! existing loaded Vault/node resolution; no second attribute database.
//! Array header/GetData8297b7e8/GetCount82979ec0, RefSpec and tLWGroupEntry PDB.
use super::{AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance};

pub(crate) const REFERENCE_TYPE: u64 = 0x97c5_113d_e9bd_1ad6;
pub(crate) const GROUP_ENTRY_TYPE: u64 = 0xe80b_bad3_bfa4_a172;

/// Retains the SAME serialized storage. RefSpec's collection cache is irrelevant
/// to these consumers, which read the authored class/collection keys only.
pub(crate) struct ReferenceArray {
    data: AttributeRuntimeData,
    count: u16,
    stride: usize,
}
impl ReferenceArray {
    pub(crate) fn len(&self) -> u32 {
        u32::from(self.count)
    }
    pub(crate) fn get(&self, index: u32) -> Option<&[u8]> {
        if index >= self.len() {
            return None;
        }
        let offset = index as usize * self.stride;
        Some(&self.data.bytes()[offset..offset + self.stride])
    }
}
impl AttributeRuntimeInstance {
    pub(crate) fn try_reference_members(
        &self,
        key: u64,
        group_entries: bool,
    ) -> Result<Option<ReferenceArray>, AttributeRuntimeError> {
        use AttributeRuntimeError as E;
        let Some((class_key, class, _collection, node)) = self.resolved_node(key)? else {
            return Ok(None);
        };
        let expected_type = if group_entries {
            GROUP_ENTRY_TYPE
        } else {
            REFERENCE_TYPE
        };
        let expected_size = if group_entries { 32 } else { 24 };
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
        if usize::from(definition.size) != expected_size {
            return Err(E::PayloadSize {
                key,
                expected: expected_size,
                actual: usize::from(definition.size),
            });
        }
        // These shipped arrays are explicit stored-pointer nodes, not inline,
        // class-static, or layout-relative. Leave other forms explicit errors.
        if node.flags & 2 == 0 || node.flags & 0x70 != 0 {
            return Err(E::UnsupportedArray { key });
        }
        let slot = node.stored_word.expect("explicit array node");
        let Some(_) = self.vault.pointer(slot)? else {
            return Ok(None);
        };
        let header = self.array_range(class_key, key, node, 0, 8)?;
        let half = |offset: usize| {
            u16::from_be_bytes(
                header.bytes()[offset..offset + 2]
                    .try_into()
                    .expect("two bytes"),
            )
        };
        let count = half(2);
        let stride = usize::from(half(4));
        if stride != expected_size {
            return Err(E::PayloadSize {
                key,
                expected: expected_size,
                actual: stride,
            });
        }
        // Source Array.GetValue: header8 + ((encoded >> 12) & 8) + stride*i.
        let offset = 8 + usize::from((half(6) >> 12) & 8);
        let data = self.array_range(class_key, key, node, offset, usize::from(count) * stride)?;
        Ok(Some(ReferenceArray {
            data,
            count,
            stride,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute_runtime::{
        AttributePointerValue,
        tests::{collection, definition, graph, node, set_dependency_bytes},
    };
    use std::sync::Arc;
    #[test]
    fn inherited_padded_members_keep_identity_and_check_index() {
        let mut def = definition(3, 0x25, 24);
        def.type_id = REFERENCE_TYPE;
        def.maximum_count = 10;
        let mut vault = graph(1, vec![def]);
        collection(&mut vault, 1, 10, 0, None, vec![node(3, 0x0a, 1)]);
        collection(&mut vault, 1, 11, 10, None, vec![]);
        let entry = vault
            .collections
            .get_mut(&(1, 10))
            .unwrap()
            .nodes
            .get_mut(&3)
            .unwrap();
        entry.type_key = Some(REFERENCE_TYPE);
        let pointer = entry.stored_word.unwrap().location;
        vault.pointers.insert(
            pointer,
            AttributePointerValue::Direct {
                dependency_index: 1,
                offset: 0,
            },
        );
        let mut bytes = vec![0; 128];
        for (at, value) in [(0, 2u16), (2, 2), (4, 24), (6, 0x8000)] {
            bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
        }
        bytes[24..32].copy_from_slice(&0x1234_5678_9abc_def0u64.to_be_bytes());
        bytes[48..56].copy_from_slice(&0xfedc_ba98_7654_3210u64.to_be_bytes());
        set_dependency_bytes(&mut vault, 1, bytes);
        let instance = Arc::new(vault).instance(1, 11);
        assert_eq!(instance.try_length(3).unwrap(), 2);
        let array = instance.try_reference_members(3, false).unwrap().unwrap();
        assert_eq!(array.len(), 2);
        assert_eq!(
            &array.get(0).unwrap()[8..16],
            &0x1234_5678_9abc_def0u64.to_be_bytes()
        );
        assert_eq!(
            &array.get(1).unwrap()[8..16],
            &0xfedc_ba98_7654_3210u64.to_be_bytes()
        );
        assert!(array.get(2).is_none());
    }
}
