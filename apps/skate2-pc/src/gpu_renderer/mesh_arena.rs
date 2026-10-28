//! Append-only GPU storage for immutable 20-byte position/UV meshes. Sharing
//! buffers reduces per-frame binds and resource tracking without merging draws.
//! Uploads never overwrite a published range. A full page leaves the allocator;
//! mesh/cache/command references own it until its last user retires.
use super::{RenderResult, environment_shading::LIGHTING_STRIDE};
use std::sync::Arc;

const VERTEX_STRIDE: u64 = 20;
const PAGE_VERTICES: u32 = 65_536;
const PAGE_INDICES: u32 = 262_144;

struct Page {
    vertices: Arc<wgpu::Buffer>,
    indices: Arc<wgpu::Buffer>,
    lighting: Option<Arc<wgpu::Buffer>>,
    vertex_used: u32,
    index_used: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_stream_bytes_survive_append_lazy_lighting_and_page_retirement() {
        let instance = wgpu::Instance::default();
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default()))
        else {
            eprintln!("no adapter; geometry arena GPU test skipped");
            return;
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
            .expect("GPU device");
        let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut arena = MeshArena {
            page: None,
            enabled: true,
            vertex_capacity: 4,
            index_capacity: 6,
        };
        let a = arena
            .upload(&device, &queue, &[0x11; 40], &[0x33; 8], None)
            .unwrap()
            .unwrap();
        let b = arena
            .upload(&device, &queue, &[0x22; 40], &[0x44; 8], Some(&[0x55; 72]))
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&a.vertices, &b.vertices));
        assert!(Arc::ptr_eq(&a.indices, &b.indices));
        assert_eq!((b.vertex_base, b.index_base), (2, 2));
        // Oversized meshes use the original dedicated upload, without consuming
        // or overwriting the current page.
        assert!(
            arena
                .upload(&device, &queue, &[0; 100], &[0; 4], None)
                .unwrap()
                .is_none()
        );
        let c = arena
            .upload(&device, &queue, &[0x66; 20], &[0x77; 4], Some(&[0x88; 36]))
            .unwrap()
            .unwrap();
        assert!(!Arc::ptr_eq(&a.vertices, &c.vertices));
        assert_eq!((c.vertex_base, c.index_base), (0, 0));
        // The allocator can retire entirely while mesh/command owners retain
        // both pages. Read actual GPU bytes after all later writes.
        drop(arena);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 228,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        let mut output = 0;
        for (buffer, offset, size) in [
            (&a.vertices, 0, 80),
            (&a.indices, 0, 16),
            (b.lighting.as_ref().unwrap(), 72, 72),
            (&c.vertices, 0, 20),
            (&c.indices, 0, 4),
            (c.lighting.as_ref().unwrap(), 0, 36),
        ] {
            encoder.copy_buffer_to_buffer(buffer, offset, &readback, output, size);
            output += size;
        }
        queue.submit(Some(encoder.finish()));
        let (send, recv) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                send.send(result).unwrap();
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        recv.recv().unwrap().unwrap();
        let mapped = readback.slice(..).get_mapped_range().unwrap();
        let expected: Vec<u8> = [
            (0x11, 40),
            (0x22, 40),
            (0x33, 8),
            (0x44, 8),
            (0x55, 72),
            (0x66, 20),
            (0x77, 4),
            (0x88, 36),
        ]
        .into_iter()
        .flat_map(|(byte, count)| std::iter::repeat_n(byte, count))
        .collect();
        assert_eq!(mapped.as_ref(), expected.as_slice());
        drop(mapped);
        readback.unmap();
        assert!(pollster::block_on(error_scope.pop()).is_none());
    }
}

pub(super) struct MeshArena {
    page: Option<Page>,
    enabled: bool,
    vertex_capacity: u32,
    index_capacity: u32,
}

pub(super) struct Allocation {
    pub vertices: Arc<wgpu::Buffer>,
    pub indices: Arc<wgpu::Buffer>,
    pub lighting: Option<Arc<wgpu::Buffer>>,
    pub vertex_base: i32,
    pub index_base: u32,
}

impl Default for MeshArena {
    fn default() -> Self {
        Self {
            page: None,
            enabled: std::env::var_os("SKATE2_SEPARATE_MESH_BUFFERS").is_none(),
            vertex_capacity: PAGE_VERTICES,
            index_capacity: PAGE_INDICES,
        }
    }
}

impl MeshArena {
    /// Large meshes retain the existing dedicated-buffer upload path. Page
    /// sizes are host allocation units, never geometry/culling limits.
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[u8],
        indices: &[u8],
        lighting: Option<&[u8]>,
    ) -> RenderResult<Option<Allocation>> {
        if !self.enabled {
            return Ok(None);
        }
        let vertex_count = u32::try_from(vertices.len() / VERTEX_STRIDE as usize)?;
        let index_count = u32::try_from(indices.len() / 4)?;
        if vertices.len() % VERTEX_STRIDE as usize != 0
            || indices.len() % 4 != 0
            || lighting
                .is_some_and(|data| data.len() as u64 != u64::from(vertex_count) * LIGHTING_STRIDE)
        {
            return Err("immutable geometry upload has inconsistent stream lengths".into());
        }
        if vertex_count > self.vertex_capacity || index_count > self.index_capacity {
            return Ok(None);
        }
        let buffer = |label, size, usage| {
            Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }))
        };
        let fits = self.page.as_ref().is_some_and(|page| {
            vertex_count <= self.vertex_capacity - page.vertex_used
                && index_count <= self.index_capacity - page.index_used
        });
        if !fits {
            self.page = Some(Page {
                vertices: buffer(
                    "immutable mesh vertex page",
                    u64::from(self.vertex_capacity) * VERTEX_STRIDE,
                    wgpu::BufferUsages::VERTEX,
                ),
                indices: buffer(
                    "immutable mesh index page",
                    u64::from(self.index_capacity) * 4,
                    wgpu::BufferUsages::INDEX,
                ),
                lighting: None,
                vertex_used: 0,
                index_used: 0,
            });
        }
        let page = self.page.as_mut().expect("allocated geometry page");
        queue.write_buffer(
            &page.vertices,
            u64::from(page.vertex_used) * VERTEX_STRIDE,
            vertices,
        );
        queue.write_buffer(&page.indices, u64::from(page.index_used) * 4, indices);
        let lighting = if let Some(data) = lighting {
            let target = page.lighting.get_or_insert_with(|| {
                buffer(
                    "immutable mesh lighting page",
                    u64::from(self.vertex_capacity) * LIGHTING_STRIDE,
                    wgpu::BufferUsages::VERTEX,
                )
            });
            queue.write_buffer(target, u64::from(page.vertex_used) * LIGHTING_STRIDE, data);
            Some(Arc::clone(target))
        } else {
            None
        };
        let allocation = Allocation {
            vertices: Arc::clone(&page.vertices),
            indices: Arc::clone(&page.indices),
            lighting,
            vertex_base: i32::try_from(page.vertex_used)?,
            index_base: page.index_used,
        };
        page.vertex_used += vertex_count;
        page.index_used += index_count;
        Ok(Some(allocation))
    }
}
