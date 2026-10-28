//! Attribute.GetLength8297a0a0 -> Node.GetCount82979ec0/GetArray82979b78.
//! Reads the existing loaded node/storage; counting needs no element type or
//! element construction. Class-static storage remains an explicit boundary.
use super::{
    AttributeRuntimeData, AttributeRuntimeDataOrigin, AttributeRuntimeError,
    AttributeRuntimeInstance, RuntimeNode,
};

impl AttributeRuntimeInstance {
    pub(crate) fn try_length(&self, key: u64) -> Result<u32, AttributeRuntimeError> {
        let Some((class_key, _, _, node)) = self.resolved_node(key)? else {
            return Ok(0);
        };
        if node.flags & 0x80 == 0 {
            return Ok(0);
        }
        if node.flags & 2 == 0 {
            return Ok(1);
        }
        // Native lhz reads count at +2, not capacity at +0 or stride at +4.
        let data = self.array_range(class_key, key, node, 2, 2)?;
        Ok(u32::from(u16::from_be_bytes(
            data.bytes().try_into().expect("two-byte count"),
        )))
    }

    pub(super) fn array_range(
        &self,
        class_key: u64,
        key: u64,
        node: &RuntimeNode,
        offset: usize,
        size: usize,
    ) -> Result<AttributeRuntimeData, AttributeRuntimeError> {
        use AttributeRuntimeError as E;
        // GetArray has layout -> class-static -> pointer precedence. Unlike
        // GetPointer, the inline bit does NOT select an inline array header.
        let (slot, relative, origin) = if node.flags & 0x30 != 0 {
            if let Some(slot) = node
                .stored_word
                .filter(|slot| self.vault.pointers.contains_key(&slot.location))
            {
                return Err(E::RelocatedPayload {
                    slot: slot.location,
                });
            }
            if node.flags & 0x10 == 0 {
                return Err(E::UnsupportedClassStaticStorage {
                    class: class_key,
                    key,
                });
            }
            // Attribute.GetLength passes mInstance.mLayoutPtr, even when the
            // selected node was inherited from a different collection.
            let original = self.collection.expect("resolved node has instance");
            (
                self.vault.collections[&original].layout,
                node.value as usize,
                AttributeRuntimeDataOrigin::CollectionLayout,
            )
        } else {
            (
                node.stored_word.expect("explicit stored array"),
                0,
                AttributeRuntimeDataOrigin::StoredPointer,
            )
        };
        let base = self
            .vault
            .pointer(slot)?
            .ok_or(E::UnsupportedArray { key })?;
        let relative = relative.checked_add(offset).ok_or(E::ByteRange {
            dependency: base.dependency_index,
            offset: usize::MAX,
            size,
        })?;
        self.vault.range(base, relative, size, origin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute_runtime::tests::{
        collection, definition, graph, node, set_dependency_bytes,
    };
    use std::sync::Arc;

    #[test]
    fn counts_without_decoding_elements_and_uses_instance_layout_for_inherited_node() {
        let mut vault = graph(1, vec![definition(3, 4, 4), definition(4, 4, 4)]);
        collection(
            &mut vault,
            1,
            10,
            0,
            Some(64),
            vec![node(3, 0x12, 8), node(4, 0, 0)],
        );
        collection(&mut vault, 1, 11, 10, Some(128), vec![]);
        let mut data = vec![0; 256];
        data[74..76].copy_from_slice(&2u16.to_be_bytes());
        data[136..138].copy_from_slice(&900u16.to_be_bytes()); // capacity is not count
        data[138..140].copy_from_slice(&300u16.to_be_bytes());
        set_dependency_bytes(&mut vault, 1, data);
        let vault = Arc::new(vault);
        assert_eq!(vault.instance(1, 10).try_length(3).unwrap(), 2);
        let child = vault.instance(1, 11);
        assert_eq!(child.try_length(3).unwrap(), 300);
        // An existing scalar counts as one without dereferencing its null value.
        assert_eq!(child.try_length(4).unwrap(), 1);
        assert_eq!(child.try_length(5).unwrap(), 0);
        assert_eq!(vault.instance(1, 99).try_length(3).unwrap(), 0);
    }
}
