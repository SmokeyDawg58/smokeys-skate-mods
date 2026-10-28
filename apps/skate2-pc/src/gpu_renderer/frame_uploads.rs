//! Reusable host staging memory for per-frame character constants.
//!
//! Each write retains its bytes, destination and order. The copy command buffer
//! precedes every draw in the next submission, matching Queue::write_buffer's
//! visibility. Reuse is asynchronous and only after the GPU finishes the copy.
use std::cell::RefCell;

struct Pending {
    belt: wgpu::util::StagingBelt,
    encoder: Option<wgpu::CommandEncoder>,
}

pub(super) struct FrameUploads {
    device: wgpu::Device,
    pending: RefCell<Pending>,
}

impl FrameUploads {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self {
            device: device.clone(),
            pending: RefCell::new(Pending {
                // A host allocation size, not a native gameplay constant.
                // Palettes are about 3 KiB; many share one mapped allocation.
                belt: wgpu::util::StagingBelt::new(device.clone(), 64 * 1024),
                encoder: None,
            }),
        }
    }

    pub(super) fn write(&self, buffer: &wgpu::Buffer, offset: u64, bytes: &[u8]) {
        let Some(size) = wgpu::BufferSize::new(bytes.len() as u64) else { return };
        let mut pending = self.pending.borrow_mut();
        let Pending { belt, encoder } = &mut *pending;
        let encoder = encoder.get_or_insert_with(|| {
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("retained character constant uploads"),
            })
        });
        belt.write_buffer(encoder, buffer, offset, size).copy_from_slice(bytes);
    }

    /// Call only once all draw encoding succeeded and immediately before submit.
    /// Acquisition/encoding failures leave pending writes available for retry.
    pub(super) fn finish(&mut self) -> Option<wgpu::CommandBuffer> {
        let Pending { belt, encoder } = self.pending.get_mut();
        let encoder = encoder.take()?;
        belt.finish_and_recall_on_submit(&encoder);
        Some(encoder.finish())
    }
}
