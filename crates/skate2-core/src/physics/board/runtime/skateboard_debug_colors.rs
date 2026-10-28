//! Mutable color projection of the actual authored debug-object ranges.
//! Body constructor initializes white; `PostPhysics` recolors at 0x82b36b38
//! and 0x82b36e88. This does not reconstruct the temporary debug Instance.
use crate::skateboard_assembly::SkateboardCollisionDefinition;
use std::{ops::Range, sync::Arc};

pub(crate) struct SkateboardDebugColors {
    definition: Arc<SkateboardCollisionDefinition>,
    colors: [[f32; 4]; 32],
}
impl SkateboardDebugColors {
    pub(crate) fn from_constructed(definition: Arc<SkateboardCollisionDefinition>) -> Self {
        Self {
            definition,
            colors: [[1.0; 4]; 32],
        }
    }
    pub(crate) fn belongs_to(&self, definition: &Arc<SkateboardCollisionDefinition>) -> bool {
        Arc::ptr_eq(&self.definition, definition)
    }
    fn range(&self, part: usize) -> Option<Range<u32>> {
        match part {
            0..=3 => {
                let debug = self.definition.wheels.parts()[part].debug();
                Some(debug.index..debug.end_index())
            }
            4..=5 => {
                let index = self.definition.trucks.parts()[part - 4].debug().index;
                Some(index..index + 1)
            }
            6 => Some(self.definition.deck.debug_range()),
            _ => None,
        }
    }
    /// The immutable definition borrow excludes range mutation during these
    /// stores. The source rereads end after each color; no value is resampled
    /// from Vault and no collision/debug object is reconstructed here.
    #[cfg(test)]
    pub(crate) fn try_recolor(&mut self, part: usize, contacted: bool) -> Result<(), usize> {
        let range = self.range(part).ok_or(part)?;
        let color = if contacted {
            [0x3f19_999a, 0x3f19_999a, 0x3f19_999a, 0x3f80_0000]
        } else {
            [0x3dcc_cccd, 0x3dcc_cccd, 0x3e4c_cccd, 0x3f80_0000]
        }
        .map(f32::from_bits);
        for index in range {
            *self.colors.get_mut(index as usize).ok_or(part)? = color;
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn colors(&self) -> &[[f32; 4]; 32] {
        &self.colors
    }
}

impl crate::skateboard_contact_surface::prefix::SkateboardContactDebug for SkateboardDebugColors {
    type Error = &'static str;
    fn start(&mut self, part: usize) -> Result<i32, Self::Error> {
        i32::try_from(self.range(part).ok_or("invalid debug Part")?.start)
            .map_err(|_| "debug start overflow")
    }
    fn end(&mut self, part: usize) -> Result<i32, Self::Error> {
        i32::try_from(self.range(part).ok_or("invalid debug Part")?.end)
            .map_err(|_| "debug end overflow")
    }
    fn store_color(&mut self, index: i32, color: [f32; 4]) -> Result<(), Self::Error> {
        let index = usize::try_from(index).map_err(|_| "negative debug index")?;
        *self
            .colors
            .get_mut(index)
            .ok_or("debug index exceeds capacity")? = color;
        Ok(())
    }
}
