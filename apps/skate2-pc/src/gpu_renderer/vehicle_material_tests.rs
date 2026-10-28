use super::*;

/// Execute the production WGSL using the production per-entity byte packing.
/// Golden ambient values come from the final Xenos body/glass MAX instructions,
/// rather than the older UPDB source (which has no minimum ambient term).
#[test]
fn authored_paint_and_native_ambient_floor_survive_gpu_publication() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no adapter; vehicle GPU material check skipped");
        return;
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
        .expect("vehicle test device");
    let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production vehicle paint and ambient probe"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{}{}{}",
                object_source(0),
                include_str!("vehicle_material.wgsl"),
                r"
@group(0) @binding(1) var<storage,read_write> result: array<vec4<f32>,5>;
@compute @workgroup_size(1) fn main() {
    let n=vec3<f32>(0.0,1.0,0.0);
    let ambient=vehicle_ambient(n,1.0);
    result[0]=vec4<f32>(ambient,1.0);
    result[1]=vec4<f32>(vehicle_paint(vec3<f32>(1.0,0.0,0.0))*ambient,1.0);
    result[2]=vec4<f32>(vehicle_paint(vec3<f32>(0.0,0.0,1.0))*ambient,1.0);
    result[3]=vec4<f32>(vehicle_paint(vec3<f32>(0.4,0.1,0.2)),1.0);
    result[4]=vec4<f32>(vehicle_ambient(n,0.5),1.0);
}
"
            )
            .into(),
        ),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        cache: None,
        compilation_options: Default::default(),
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("main"),
    });
    let uniform = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: OBJECT_BYTES as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 80,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 80,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let object = Arc::new(GpuDynamicObject {
        model: Arc::new(GpuDynamicObjectModel {
            meshes: Vec::new(),
            bounds: [[0.; 3]; 2],
        }),
        uniform,
        binding,
    });
    let paint = [[0.6, 0.05, 0.05, 1.0], [0.84, 0.66, 0.15, 1.0]];
    for (coefficient, expected_ambient, colours) in [
        ([0.; 4], [0.036, 0.040, 0.042], paint),
        (
            [-1., -2., -3., 0.],
            [0.036, 0.040, 0.042],
            [paint[1], paint[0]],
        ),
        ([0.1, 0.2, 0.3, 0.], [0.1, 0.2, 0.3], paint),
    ] {
        let mut irradiance = [[0.; 4]; 9];
        irradiance[0] = coefficient;
        let draw = DynamicObjectDraw {
            object: Arc::clone(&object),
            part_transform: [0.; 16],
            opacity: 1.,
            ghosted: false,
            vehicle: Some(VehicleDrawConstants {
                colours,
                lights: [0.; 4],
                irradiance,
                rows: Vec::new(),
            }),
            hips: None,
        };
        draw.upload(&queue, [0.; 3]);
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &object.binding, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 80);
        queue.submit(Some(encoder.finish()));
        let (send, recv) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| send.send(r).unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        recv.recv().unwrap().unwrap();
        let bytes = readback.slice(..).get_mapped_range().unwrap();
        let values: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
            .collect();
        let expected = [
            expected_ambient,
            std::array::from_fn(|i| colours[0][i] * expected_ambient[i]),
            std::array::from_fn(|i| colours[1][i] * expected_ambient[i]),
            [0.4, 0.1, 0.2],
            expected_ambient.map(|v| v * 0.5),
        ];
        for (pixel, rgb) in expected.iter().enumerate() {
            for lane in 0..3 {
                assert!(
                    (values[pixel * 4 + lane] - rgb[lane]).abs() < 1e-6,
                    "vehicle result {pixel}/{lane}: {} != {}",
                    values[pixel * 4 + lane],
                    rgb[lane]
                );
            }
        }
        drop(bytes);
        readback.unmap();
    }
    assert!(pollster::block_on(error_scope.pop()).is_none());
}
