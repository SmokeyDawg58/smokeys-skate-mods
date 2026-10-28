//! `rw::audio::core::SubMix`: the bus Send and Route plug-ins of earlier voices mix into; the
//! first plug-in of a bus voice outputs it (`Process` 0x82bf2ad0).

use crate::rwac::FRAME_SAMPLES;
use crate::rwac::kernels::declick;
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;

/// `mpSubMixBuffer` plus the declick totals of one SubMix.
#[derive(Debug)]
pub struct SubMixBuffer {
    pub name: String,
    pub channels: u8,
    pub data: Vec<f32>,
    /// `mDeClickValueTotal[6]`.
    pub declick_total: [f32; 6],
    /// `mDeClickRequired`.
    pub declick_required: bool,
}

impl SubMixBuffer {
    pub fn channel_mut(&mut self, channel: usize) -> &mut [f32] {
        &mut self.data[channel * FRAME_SAMPLES..(channel + 1) * FRAME_SAMPLES]
    }
}

/// The SubMix buffers of all live SubMix plug-ins, newest first in `order` (`sSubMixList`).
#[derive(Debug, Default)]
pub struct SubMixArena {
    slots: Vec<Option<SubMixBuffer>>,
    order: Vec<usize>,
}

impl SubMixArena {
    fn allocate(&mut self, name: String, channels: u8) -> usize {
        let buffer = SubMixBuffer {
            name,
            channels,
            data: vec![0.0; usize::from(channels) * FRAME_SAMPLES],
            declick_total: [0.0; 6],
            declick_required: false,
        };
        let slot = match self.slots.iter().position(Option::is_none) {
            Some(slot) => {
                self.slots[slot] = Some(buffer);
                slot
            }
            None => {
                self.slots.push(Some(buffer));
                self.slots.len() - 1
            }
        };
        self.order.insert(0, slot);
        slot
    }

    fn free(&mut self, slot: usize) {
        self.slots[slot] = None;
        self.order.retain(|&s| s != slot);
    }

    pub fn get_mut(&mut self, slot: usize) -> Option<&mut SubMixBuffer> {
        self.slots.get_mut(slot).and_then(Option::as_mut)
    }

    pub fn channels(&self, slot: usize) -> u8 {
        self.slots.get(slot).and_then(Option::as_ref).map_or(0, |b| b.channels)
    }

    /// `Send::ConnectByNameHandler`: the newest SubMix with this name.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.order
            .iter()
            .copied()
            .find(|&slot| self.slots[slot].as_ref().is_some_and(|b| b.name == name))
    }

    /// `SubMixConnector::Disconnect` 0x82bf2a10 with declick values: adds them to the totals.
    pub fn add_declick(&mut self, slot: usize, values: &[f32; 6]) {
        if let Some(buffer) = self.get_mut(slot) {
            buffer.declick_required = true;
            let channels = usize::from(buffer.channels);
            for (total, value) in buffer.declick_total.iter_mut().zip(values).take(channels) {
                *total += value;
            }
        }
    }
}

pub struct SubMix {
    base: PlugInBase,
    pub slot: usize,
}

impl SubMix {
    /// `SubMix::CreateInstance` 0x82bfc1e0; the buffer is registered by name at once (the
    /// native `CreateInstanceHandler` command runs before any command that could look it up).
    pub fn new(arena: &mut SubMixArena, name: &str, input_channels: u8, output_channels: u8) -> Self {
        let slot = arena.allocate(name.to_owned(), output_channels);
        Self {
            base: PlugInBase::new(&super::SUB_MIX, input_channels, output_channels),
            slot,
        }
    }
}

impl PlugIn for SubMix {
    impl_plugin_any!();

    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        let channels = usize::from(self.base.output_channels);
        let Some(buffer) = services.submixes.get_mut(self.slot) else {
            return BufferStatus::Available;
        };
        for channel in 0..channels {
            let source = channel * FRAME_SAMPLES;
            services.mixer.scratch_mut(channel).copy_from_slice(&buffer.data[source..source + FRAME_SAMPLES]);
        }
        if buffer.declick_required {
            let mut totals = buffer.declick_total;
            {
                let mixer = &mut *services.mixer;
                let mut views: Vec<&mut [f32]> = Vec::new();
                let (_, scratch) = mixer.current_and_scratch();
                for chunk in scratch.chunks_exact_mut(FRAME_SAMPLES).take(channels) {
                    views.push(chunk);
                }
                declick(&mut views, &mut totals[..channels.min(6)]);
            }
            buffer.declick_total = totals;
            buffer.declick_required = false;
        }
        buffer.data.fill(0.0);
        services.mixer.swap();
        BufferStatus::Available
    }

    fn release(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>) {
        services.submixes.free(self.slot);
    }
}
