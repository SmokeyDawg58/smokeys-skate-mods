//! `PostFX::RenderAOBoxes` 0x828465b0: the darkening under each skater's deck. After the uber
//! pass (`ApplyFastUber` 0x82846c74) or the visual FX pass (`ApplyVisualFX` 0x82846914), into
//! the same target, each skater's box draws `deckao_PS` (0x82130a38) with depth test and write
//! off, multiplying the target by its colour (`CSrcZero_CDestSrcColor_Add`).
//!
//! `CalculateDeckShadowMatrix` 0x82845f98 per skater: `m_DeckWVPMatrix` is the unit box
//! (`PfxHelper::CreateBox` 0x828305f0: corners at +-1, back faces culled with the front
//! counter-clockwise) scaled by (0.2, 0.2, 0.6), placed by `GetDeckMatrix` and projected by the
//! scene's view-projection; `g_Deck_Matrix_row1..3` are the first three rows of the transpose
//! of the camera matrix times the deck's inverse, taking a camera-space point into deck space.
//! A skater the game state does not draw gets `DeckAO_Colour` (1, 1, 1, 1), which leaves the
//! target as it was; here it has no box.
//!
//! `deckao_PS`: the depth (`g_DOFCameraInfoX/Y`) gives the pixel's camera-space point along
//! `g_FOV`; in deck space its x and z (times `DeckAO_XLen`/`ZLen`, plus 0.5) sample
//! `Board_texture` (`board_occlusion.Texture`), and its height times `DeckAO_HeightFallOff`, the
//! stencil byte (255 on characters) and `DeckAO_Colour.w` lighten it, saturated. The texture is
//! read through `m_pLinearClampSamplerState` (`PostFX::Init` 0x8284a8b8), whose mip filter
//! `D3DTEXF_NONE` (2, the base map) keeps every fetch on its full-size level, so a point beyond
//! the board clamps to its white border.
//!
//! With MSAA every depth and stencil sample is shaded and the pixel takes their mean, so an edge
//! pixel a character half covers is not shaded as all ground.
//!
//! PC: the darkening (one minus the original's factor) is 1.2 times the original's, which the
//! player asked for; characters and points off the board keep their factor of 1. With PostFX
//! off the boxes still draw, into the scene the frame drew directly.

use super::GpuTexture;
use std::sync::Arc;

/// `SkaterPresentation::mModels`: nine skater entities at most.
const BOXES: usize = 9;
/// One box's constants, padded to the uniform offset alignment.
const SLOT: u64 = 256;
/// `PfxHelper::CreateBox`'s scale in `CalculateDeckShadowMatrix` (0x8284603c..0x828460a0).
const BOX_SCALE: [f64; 4] = [0.2, 0.2, 0.6, 1.0];
/// `AABBVertex` (corner `i` has x, y, z from bits 0, 1, 2) and `AABB_Trilist` 0x82feffe0.
const CORNERS: [[f32; 3]; 8] = [
    [-1.0, -1.0, -1.0],
    [1.0, -1.0, -1.0],
    [-1.0, 1.0, -1.0],
    [1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0],
    [-1.0, 1.0, 1.0],
    [1.0, 1.0, 1.0],
];
const TRIANGLES: [u16; 36] = [0, 1, 2, 2, 1, 3, 2, 3, 6, 6, 3, 7, 5, 1, 0, 5, 0, 4, 3, 1, 5, 3, 5, 7, 4, 7, 5, 4, 6, 7, 4, 0, 2, 4, 2, 6];

const SHADER: &str = r"
struct DeckBox {
    wvp: array<vec4<f32>, 4>,
    deck: array<vec4<f32>, 3>,
    fov: vec4<f32>,
    params: vec4<f32>,
    colour: vec4<f32>,
    dof: vec4<f32>,
}
@group(0) @binding(0) var<uniform> b: DeckBox;
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var stencil: texture_2d<u32>;
@group(0) @binding(3) var board: texture_2d<f32>;
@group(0) @binding(4) var linear_clamp: sampler;

// PC: how much deeper than the original the darkening is (1 is the original's).
const STRENGTH: f32 = 1.2;

@vertex
fn vs(@location(0) corner: vec3<f32>) -> @builtin(position) vec4<f32> {
    return b.wvp[0] * corner.x + b.wvp[1] * corner.y + b.wvp[2] * corner.z + b.wvp[3];
}

fn shade(screen: vec2<f32>, pixel: vec2<i32>, index: i32) -> vec4<f32> {
    let z = b.dof.x / (textureLoad(depth, pixel, index) - b.dof.y);
    let view = vec4<f32>(screen * b.fov.xy * z, z, 1.0);
    let deck = vec3<f32>(dot(b.deck[0], view), dot(b.deck[1], view), dot(b.deck[2], view));
    let uv = vec2<f32>(deck.x * b.params.x, deck.z * b.params.y) + vec2<f32>(0.5);
    let texel = textureSampleLevel(board, linear_clamp, uv, 0.0).rgb;
    let ao = abs(deck.y) * b.params.w + texel;
    let lift = b.colour.w + saturate(f32(textureLoad(stencil, pixel, index).x) / 255.0);
    return saturate(vec4<f32>(ao, ao.z) + lift);
}

@fragment
fn fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(position.xy);
    let screen = position.xy / vec2<f32>(textureDimensions(depth)) - vec2<f32>(0.5);
    let samples = 1u;
    var sum = vec4<f32>(0.0);
    for (var index = 0u; index < samples; index++) {
        sum += shade(screen, pixel, i32(index));
    }
    return saturate(vec4<f32>(1.0) - (vec4<f32>(1.0) - sum / f32(samples)) * STRENGTH);
}
";

/// One skater's `m_DeckWVPMatrix` and `g_Deck_Matrix_row1..3`, as native rows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DeckAoBox {
    pub(crate) wvp: [[f32; 4]; 4],
    pub(crate) deck: [[f32; 4]; 3],
}

/// This frame's boxes, `Board_texture` and `g_FOV`.
#[derive(Clone)]
pub(crate) struct DeckAoFrame {
    pub(crate) texture: GpuTexture,
    pub(crate) boxes: Vec<DeckAoBox>,
    pub(crate) fov: [f32; 4],
}

fn rows(matrix: &[f32; 16]) -> [[f64; 4]; 4] {
    std::array::from_fn(|row| std::array::from_fn(|column| f64::from(matrix[row * 4 + column])))
}

/// The row-vector product `a * b`.
fn product(a: &[[f64; 4]; 4], b: &[[f64; 4]; 4]) -> [[f64; 4]; 4] {
    std::array::from_fn(|row| std::array::from_fn(|column| (0..4).map(|k| a[row][k] * b[k][column]).sum()))
}

/// `rw::math::fpu::Inverse<double>`: a cofactor inverse; none for a singular matrix.
fn inverse(m: &[[f64; 4]; 4]) -> Option<[[f64; 4]; 4]> {
    let minor = |skip_row: usize, skip_column: usize| {
        let pick: Vec<[f64; 3]> = (0..4)
            .filter(|&row| row != skip_row)
            .map(|row| {
                let mut lanes = (0..4).filter(|&column| column != skip_column).map(|column| m[row][column]);
                [lanes.next().unwrap_or(0.0), lanes.next().unwrap_or(0.0), lanes.next().unwrap_or(0.0)]
            })
            .collect();
        pick[0][0] * (pick[1][1] * pick[2][2] - pick[1][2] * pick[2][1]) - pick[0][1] * (pick[1][0] * pick[2][2] - pick[1][2] * pick[2][0])
            + pick[0][2] * (pick[1][0] * pick[2][1] - pick[1][1] * pick[2][0])
    };
    let cofactor = |row: usize, column: usize| if (row + column) % 2 == 0 { minor(row, column) } else { -minor(row, column) };
    let determinant: f64 = (0..4).map(|column| m[0][column] * cofactor(0, column)).sum();
    if determinant == 0.0 || !determinant.is_finite() {
        return None;
    }
    Some(std::array::from_fn(|row| std::array::from_fn(|column| cofactor(column, row) / determinant)))
}

/// `CalculateDeckShadowMatrix` for a skater: its deck matrix (`GetDeckMatrix`), the scene's
/// clip-from-world and the camera's x, y and z axes and position (`CameraPresentation::
/// GetMatrix`), all as native rows (the columns of the column-major matrices).
#[must_use]
pub(crate) fn deck_box(deck: &[f32; 16], clip_from_world: &[f32; 16], camera: [[f32; 3]; 4]) -> Option<DeckAoBox> {
    let deck = rows(deck);
    let scaled: [[f64; 4]; 4] = std::array::from_fn(|row| deck[row].map(|lane| lane * BOX_SCALE[row]));
    let wvp = product(&scaled, &rows(clip_from_world));
    let camera: [[f64; 4]; 4] = std::array::from_fn(|row| {
        let axis = camera[row].map(f64::from);
        [axis[0], axis[1], axis[2], if row == 3 { 1.0 } else { 0.0 }]
    });
    let to_deck = product(&camera, &inverse(&deck)?);
    #[expect(clippy::cast_possible_truncation, reason = "the shader constants' single-precision lanes")]
    let wvp = wvp.map(|row| row.map(|lane| lane as f32));
    #[expect(clippy::cast_possible_truncation, reason = "the shader constants' single-precision lanes")]
    let deck_rows = std::array::from_fn(|row| std::array::from_fn(|column| to_deck[column][row] as f32));
    Some(DeckAoBox { wvp, deck: deck_rows })
}

/// `g_FOV` (0x828463d8..0x82846454) from the projection the scene drew with: the camera-space
/// x and y a screen offset from the centre spans per unit of depth, `2 / x_scale` and
/// `-2 / y_scale` (the original's 16:9 projection gives its constants).
#[must_use]
pub(crate) fn fov(clip_from_world: &[f32; 16], right: [f32; 3], up: [f32; 3]) -> [f32; 4] {
    let x_scale = clip_from_world[0] * right[0] + clip_from_world[4] * right[1] + clip_from_world[8] * right[2];
    let y_scale = clip_from_world[1] * up[0] + clip_from_world[5] * up[1] + clip_from_world[9] * up[2];
    [2.0 / x_scale, -2.0 / y_scale, 0.0, 0.0]
}

pub(super) struct DeckAo {
    /// Into the scene's format, then into the visual FX pass's 8-bit target.
    pipelines: [(wgpu::TextureFormat, wgpu::RenderPipeline); 2],
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    corners: wgpu::Buffer,
    triangles: wgpu::Buffer,
    sampler: wgpu::Sampler,
    depth: wgpu::TextureView,
    stencil: wgpu::TextureView,
    /// The bind group of the last `Board_texture`.
    binding: Option<(Arc<wgpu::TextureView>, wgpu::BindGroup)>,
}

impl DeckAo {
    /// `multisampled`: the scene's depth and stencil are multisampled; every sample is shaded.
    pub(super) fn new(device: &wgpu::Device, formats: [wgpu::TextureFormat; 2], depth: &wgpu::TextureView, stencil: &wgpu::TextureView, multisampled: bool) -> Self {
        use wgpu::util::DeviceExt;
        let texture = |binding: u32, sample_type: wgpu::TextureSampleType| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: multisampled && binding < 3,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("RenderAOBoxes"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: wgpu::BufferSize::new(SLOT) },
                    count: None,
                },
                texture(1, wgpu::TextureSampleType::Depth),
                texture(2, wgpu::TextureSampleType::Uint),
                texture(3, wgpu::TextureSampleType::Float { filterable: true }),
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let source: std::borrow::Cow<'_, str> = if multisampled {
            SHADER
                .replace("texture_depth_2d", "texture_depth_multisampled_2d")
                .replace("texture_2d<u32>", "texture_multisampled_2d<u32>")
                .replace("let samples = 1u;", "let samples = textureNumSamples(depth);")
                .into()
        } else {
            SHADER.into()
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("deckao_PS"), source: wgpu::ShaderSource::Wgsl(source) });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("RenderAOBoxes"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let pipeline = |format: wgpu::TextureFormat| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some("RenderAOBoxes"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    compilation_options: Default::default(),
                    module: &shader,
                    entry_point: Some("vs"),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: 12,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    compilation_options: Default::default(),
                    module: &shader,
                    entry_point: Some("fs"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::Src, operation: wgpu::BlendOperation::Add },
                            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::Src, operation: wgpu::BlendOperation::Add },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState { front_face: wgpu::FrontFace::Ccw, cull_mode: Some(wgpu::Face::Back), ..Default::default() },
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
            })
        };
        let corners: Vec<u8> = CORNERS.iter().flatten().flat_map(|lane| lane.to_ne_bytes()).collect();
        let triangles: Vec<u8> = TRIANGLES.iter().flat_map(|index| index.to_ne_bytes()).collect();
        Self {
            pipelines: formats.map(|format| (format, pipeline(format))),
            layout,
            uniform: device.create_buffer(&wgpu::BufferDescriptor { label: Some("RenderAOBoxes"), size: SLOT * BOXES as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }),
            corners: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("AABBVertex"), contents: &corners, usage: wgpu::BufferUsages::VERTEX }),
            triangles: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("AABB_Trilist"), contents: &triangles, usage: wgpu::BufferUsages::INDEX }),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("m_pLinearClampSamplerState"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                ..Default::default()
            }),
            depth: depth.clone(),
            stencil: stencil.clone(),
            binding: None,
        }
    }

    /// The boxes into `target` (of the format the pass before them wrote): `params` are
    /// `DeckAO_XLen`, `ZLen`, `Radius` and `HeightFallOff`, `bloom` the first bloom coefficient
    /// (`m_bloomCoefficient[0]`, whose positive value lightens the boxes as (b, b, b, 4b)) and
    /// `dof` `g_DOFCameraInfoX/Y`.
    #[expect(clippy::too_many_arguments, reason = "RenderAOBoxes' inputs")]
    pub(super) fn record(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        frame: &DeckAoFrame,
        params: [f32; 4],
        bloom: f32,
        dof: [f32; 2],
    ) {
        let Some((_, pipeline)) = self.pipelines.iter().find(|(candidate, _)| *candidate == format) else { return };
        if frame.boxes.is_empty() {
            return;
        }
        if self.binding.as_ref().is_none_or(|(view, _)| !Arc::ptr_eq(view, &frame.texture.view)) {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("RenderAOBoxes"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &self.uniform, offset: 0, size: wgpu::BufferSize::new(SLOT) }) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.depth) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&self.stencil) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&frame.texture.view) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            });
            self.binding = Some((Arc::clone(&frame.texture.view), group));
        }
        let Some((_, group)) = self.binding.as_ref() else { return };
        let colour = if bloom > 0.0 { [bloom, bloom, bloom, bloom * 4.0] } else { [0.0; 4] };
        let camera_info = [dof[0], dof[1], 0.0, 0.0];
        let boxes = &frame.boxes[..frame.boxes.len().min(BOXES)];
        let mut bytes = vec![0_u8; boxes.len() * SLOT as usize];
        for (slot, deck_box) in bytes.chunks_exact_mut(SLOT as usize).zip(boxes) {
            let lanes = deck_box.wvp.iter().chain(&deck_box.deck).chain([&frame.fov, &params, &colour, &camera_info]).flatten();
            for (destination, lane) in slot.chunks_exact_mut(4).zip(lanes) {
                destination.copy_from_slice(&lane.to_ne_bytes());
            }
        }
        queue.write_buffer(&self.uniform, 0, &bytes);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RenderAOBoxes"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_vertex_buffer(0, self.corners.slice(..));
        pass.set_index_buffer(self.triangles.slice(..), wgpu::IndexFormat::Uint16);
        for slot in 0..boxes.len() {
            let offset = u32::try_from(slot as u64 * SLOT).unwrap_or(0);
            pass.set_bind_group(0, group, &[offset]);
            pass.draw_indexed(0..36, 0, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> [f32; 16] {
        std::array::from_fn(|index| if index % 5 == 0 { 1.0 } else { 0.0 })
    }

    #[test]
    fn the_deck_rows_take_a_camera_point_into_deck_space() {
        let mut deck = identity();
        deck[12] = 5.0;
        deck[14] = -3.0;
        let camera = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [2.0, 1.0, 0.0]];
        let deck_box = deck_box(&deck, &identity(), camera).expect("invertible deck");
        let point = [3.0_f32, -1.0, -3.0, 1.0];
        let deck_space: Vec<f32> = deck_box.deck.iter().map(|row| row.iter().zip(point).map(|(a, b)| a * b).sum()).collect();
        assert_eq!(deck_space, vec![0.0, 0.0, 0.0]);
        assert_eq!(deck_box.wvp[0], [0.2, 0.0, 0.0, 0.0]);
        assert_eq!(deck_box.wvp[2], [0.0, 0.0, 0.6, 0.0]);
        assert_eq!(deck_box.wvp[3], [5.0, 0.0, -3.0, 1.0]);
    }

    #[test]
    fn the_fov_undoes_the_original_projection() {
        let normalized = 60.0_f32.to_radians() * std::f32::consts::FRAC_2_PI;
        let mut clip = [0.0_f32; 16];
        clip[0] = -1.0 / (normalized * 16.0 / 9.0);
        clip[5] = 1.0 / normalized;
        let fov = fov(&clip, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert!((fov[0] - normalized * 16.0 / 9.0 * -2.0).abs() < 1e-6);
        assert!((fov[1] - normalized * -2.0).abs() < 1e-6);
    }
}
