//! Optional GPU timestamps. Readback never waits for the GPU: busy slots cause
//! a skipped measurement, not a delayed frame. CSV output reports sample counts.
use std::sync::mpsc::{self, Receiver};

const STAMPS: u32 = 5;
const BYTES: u64 = STAMPS as u64 * 8;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Measurements {
    pub samples: u64,
    pub totals_ms: [f64; 4],
}

struct Slot {
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    pending: Option<Receiver<Result<(), wgpu::BufferAsyncError>>>,
}

pub(super) struct GpuProfile {
    slots: Vec<Slot>,
    active: Option<usize>,
    period_ns: f64,
    frame: u64,
}

impl GpuProfile {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
        {
            return None;
        }
        let slots = (0..4)
            .map(|_| Slot {
                queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("optional frame timestamps"),
                    ty: wgpu::QueryType::Timestamp,
                    count: STAMPS,
                }),
                resolve: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("optional timestamp resolve"),
                    size: BYTES,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                readback: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("optional timestamp readback"),
                    size: BYTES,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                pending: None,
            })
            .collect();
        Some(Self {
            slots,
            active: None,
            period_ns: f64::from(queue.get_timestamp_period()),
            frame: 0,
        })
    }

    pub fn begin(&mut self, device: &wgpu::Device) -> Measurements {
        // A failed poll leaves the readbacks pending; this frame reports none of them.
        if self.slots.iter().any(|slot| slot.pending.is_some())
            && device.poll(wgpu::PollType::Poll).is_err()
        {
            return Measurements::default();
        }
        let mut measurements = Measurements::default();
        for slot in &mut self.slots {
            let Some(result) = slot.pending.as_ref().and_then(|rx| rx.try_recv().ok()) else {
                continue;
            };
            slot.pending = None;
            if result.is_ok()
                && let Ok(mapped) = slot.readback.slice(..).get_mapped_range()
            {
                let times: [u64; STAMPS as usize] = std::array::from_fn(|i| {
                    u64::from_le_bytes(mapped[i * 8..(i + 1) * 8].try_into().unwrap())
                });
                // A disjoint/nonmonotonic measurement is unavailable, not zero.
                if times.windows(2).all(|w| w[1] >= w[0]) {
                    measurements.samples += 1;
                    for (total, pair) in measurements.totals_ms.iter_mut().zip(times.windows(2)) {
                        *total += (pair[1] - pair[0]) as f64 * self.period_ns / 1_000_000.;
                    }
                }
                drop(mapped);
            }
            slot.readback.unmap();
        }
        // Sample one in sixteen frames to limit query/readback perturbation.
        self.active = if self.frame % 16 == 0 {
            self.slots.iter().position(|slot| slot.pending.is_none())
        } else {
            None
        };
        self.frame = self.frame.wrapping_add(1);
        measurements
    }

    pub fn stamp(&self, encoder: &mut wgpu::CommandEncoder, index: u32) {
        if let Some(i) = self.active {
            encoder.write_timestamp(&self.slots[i].queries, index);
        }
    }

    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        if let Some(i) = self.active {
            let slot = &self.slots[i];
            encoder.resolve_query_set(&slot.queries, 0..STAMPS, &slot.resolve, 0);
            encoder.copy_buffer_to_buffer(&slot.resolve, 0, &slot.readback, 0, BYTES);
        }
    }

    pub fn submitted(&mut self) {
        if let Some(i) = self.active.take() {
            let (tx, rx) = mpsc::channel();
            let slot = &mut self.slots[i];
            slot.readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    let _ = tx.send(result);
                });
            slot.pending = Some(rx);
        }
    }
}
