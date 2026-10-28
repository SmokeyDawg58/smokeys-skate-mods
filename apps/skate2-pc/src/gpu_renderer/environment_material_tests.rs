use super::COMMON;

/// Execute the production helper against the final programs' multiply-chain
/// Fresnel values, including a negative dot that the previous WRAP lookup
/// could fold back toward the facing end of its table.
#[test]
fn final_program_fresnel_and_specular_execute_on_gpu() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("final environment Fresnel regression"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{COMMON}{}",
                r"
@group(0) @binding(1) var<storage,read_write> result: array<vec4<f32>,2>;
@compute @workgroup_size(1) fn probe() {
    let n=vec3<f32>(0.,0.,1.);
    result[0]=vec4<f32>(environment_fresnel(vec3<f32>(1.,0.,0.),n),
        environment_fresnel(n,n),environment_fresnel(vec3<f32>(0.8660254,0.,0.5),n),
        environment_fresnel(-n,n));
    result[1]=vec4<f32>(calculated_specular_term(n,n,n,vec3<f32>(1.,0.,0.),1.),1.);
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
        entry_point: Some("probe"),
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 32,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 32,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 1,
            resource: output.as_entire_binding(),
        }],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &binding, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 32);
    queue.submit(Some(encoder.finish()));
    let (send, receive) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| send.send(r).unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    receive.recv().unwrap().unwrap();
    let bytes = readback.slice(..).get_mapped_range().unwrap();
    let actual: Vec<_> = bytes
        .chunks_exact(4)
        .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
        .collect();
    let expected = [5., 1., 1.015625, 5., 16.2, 13.5, 9.45, 1.];
    for (actual, expected) in actual.iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.00001,
            "native material value {actual} != {expected}"
        );
    }
    drop(bytes);
    readback.unmap();
    assert!(pollster::block_on(error_scope.pop()).is_none());
}
