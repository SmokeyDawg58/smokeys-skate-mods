//! One technique pass's rasterizer state (`cRasterizerStateDefinition`) and
//! its wgpu form (docs/evidence/render-cull-bias-hair-decal-2026-09-27.md).
//!
//! - `cFxParser::EndElement("Pass")` 0x82e1f778 resets every pass to CULLMODE
//!   NONE, DEPTHBIAS 0 and SLOPESCALEDEPTHBIAS 0 before the next one parses.
//! - `cRasterizerStateHandle::SetState` 0x82e19d68 reads CULLMODE through
//!   `ShaderSystem::CullMode` 0x82e15dd0 and both biases with `ConvertAToF`.
//! - `cFxStateHandle<cRasterizerStateDefinition>::SetupState` 0x82e143c8 maps
//!   the cull mode through `cullModeTranslation` 0x83024f8c (`[0, 2, 1]`) and
//!   keeps both biases unscaled. `renderengine::Device::SetState` 0x82bcbf08
//!   hands the value to `D3DDevice_SetRenderState_CullMode` 0x82c34770, which
//!   writes it into PA_SU_SC_MODE_CNTL bits 0-2 (CULL_FRONT, CULL_BACK, FACE;
//!   FACE 0 makes counter-clockwise the front face). FRONT is D3DCULL_CW and
//!   culls screen-clockwise triangles; BACK culls screen-counter-clockwise ones.
//! - The camera projection is the original's
//!   (`ViewCamera::SetPerspectiveMatrixRadians` 0x82840770), so every triangle
//!   keeps its original screen winding. wgpu `FrontFace::Ccw` names the same
//!   counter-clockwise screen order.

/// `State_RasterizerCullMode` as a manifest spells it.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum CullMode {
    /// The per-pass default: every triangle is drawn.
    #[default]
    None,
    /// D3DCULL_CW: screen-clockwise triangles are culled.
    Front,
    /// CULL_FRONT with FACE 0: screen-counter-clockwise triangles are culled.
    Back,
}

impl CullMode {
    /// `ShaderSystem::CullMode` accepts exactly these values; anything else
    /// is a `FailAssert` in the original.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "NONE" => Some(Self::None),
            "FRONT" => Some(Self::Front),
            "BACK" => Some(Self::Back),
            _ => None,
        }
    }
}

/// One pass's CULLMODE, DEPTHBIAS and SLOPESCALEDEPTHBIAS.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RasterState {
    pub(crate) cull: CullMode,
    /// D3DRS_DEPTHBIAS: an offset in normalized depth.
    pub(crate) depth_bias: f32,
    /// D3DRS_SLOPESCALEDEPTHBIAS: a factor on the per-pixel depth slope.
    pub(crate) slope_scale: f32,
}

/// Orderable pipeline-cache identity of a [`RasterState`].
pub(crate) type RasterKey = (CullMode, u32, u32);

/// Convert normalized depth to the selected D24S8 target's 24-bit bias units.
/// Depth24PlusStencil8 may use a wider format on other backends; that host
/// precision difference is not a claim of bit-exact Xenon rasterization.
const DEPTH_UNITS_PER_DEPTH: f32 = 16_777_216.0;

impl RasterState {
    /// CULLMODE FRONT without bias: every ported world and character default
    /// pass states exactly this.
    pub(crate) const FRONT: Self = Self::culled(CullMode::Front);

    pub(crate) const fn culled(cull: CullMode) -> Self {
        Self {
            cull,
            depth_bias: 0.0,
            slope_scale: 0.0,
        }
    }

    /// One manifest pass's states over the per-pass defaults; None for a value
    /// the original parser would not accept.
    pub(crate) fn from_pass_states<'a>(
        states: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Option<Self> {
        let mut state = Self::default();
        for (name, value) in states {
            match name {
                "CULLMODE" => state.cull = CullMode::parse(value)?,
                "DEPTHBIAS" => state.depth_bias = value.parse().ok()?,
                "SLOPESCALEDEPTHBIAS" => state.slope_scale = value.parse().ok()?,
                _ => {}
            }
        }
        Some(state)
    }

    pub(crate) fn key(self) -> RasterKey {
        (
            self.cull,
            self.depth_bias.to_bits(),
            self.slope_scale.to_bits(),
        )
    }

    pub(crate) fn primitive(self) -> wgpu::PrimitiveState {
        wgpu::PrimitiveState {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: match self.cull {
                CullMode::None => None,
                CullMode::Front => Some(wgpu::Face::Back),
                CullMode::Back => Some(wgpu::Face::Front),
            },
            ..Default::default()
        }
    }

    /// `D3DDevice_SetRenderState_DepthBias` 0x82c35338 stores DEPTHBIAS as is
    /// in PA_SU_POLY_OFFSET_{FRONT,BACK}_OFFSET, a normalized-depth offset.
    /// wgpu's constant counts minimum resolvable differences; the selected
    /// D24S8 target uses 24-bit depth units. This conversion also matches the
    /// previous Depth32Float target for depths in [0.5,1).
    /// `D3DDevice_SetRenderState_SlopeScaleDepthBias` 0x82c35270 stores the
    /// scale times 16 in the register's 1/16-pixel unit, so the per-pixel
    /// slope factor is the manifest value. D3D9 has no bias clamp.
    pub(crate) fn depth_bias(self) -> wgpu::DepthBiasState {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "manifest biases are far inside the i32 range"
        )]
        let constant = (self.depth_bias * DEPTH_UNITS_PER_DEPTH).round() as i32;
        wgpu::DepthBiasState {
            constant,
            slope_scale: self.slope_scale,
            clamp: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CullMode, RasterState};

    #[test]
    fn cull_values_are_the_shader_system_names() {
        assert_eq!(CullMode::parse("NONE"), Some(CullMode::None));
        assert_eq!(CullMode::parse("FRONT"), Some(CullMode::Front));
        assert_eq!(CullMode::parse("BACK"), Some(CullMode::Back));
        assert_eq!(CullMode::parse("front"), None);
    }

    #[test]
    fn front_culls_screen_clockwise_and_back_counter_clockwise() {
        let front = RasterState::FRONT.primitive();
        assert_eq!(front.front_face, wgpu::FrontFace::Ccw);
        assert_eq!(front.cull_mode, Some(wgpu::Face::Back));
        let back = RasterState::culled(CullMode::Back).primitive();
        assert_eq!(back.front_face, wgpu::FrontFace::Ccw);
        assert_eq!(back.cull_mode, Some(wgpu::Face::Front));
        assert_eq!(RasterState::default().primitive().cull_mode, None);
    }

    #[test]
    fn pass_states_start_from_the_pass_reset() {
        assert_eq!(
            RasterState::from_pass_states([]),
            Some(RasterState::default())
        );
        let decal = RasterState::from_pass_states([
            ("CULLMODE", "FRONT"),
            ("ZWRITEENABLE", "true"),
            ("SLOPESCALEDEPTHBIAS", "1.0"),
            ("DEPTHBIAS", "-0.0001"),
        ])
        .unwrap();
        assert_eq!(decal.cull, CullMode::Front);
        assert_eq!(decal.slope_scale, 1.0);
        assert_eq!(decal.depth_bias, -0.0001);
        assert_eq!(RasterState::from_pass_states([("CULLMODE", "CW")]), None);
    }

    #[test]
    fn decal_bias_becomes_depth32_units() {
        let bias = RasterState {
            cull: CullMode::Front,
            depth_bias: -0.0001,
            slope_scale: 1.0,
        }
        .depth_bias();
        assert_eq!(bias.constant, -1678);
        assert_eq!(bias.slope_scale, 1.0);
        assert_eq!(bias.clamp, 0.0);
        assert_eq!(
            RasterState::FRONT.depth_bias(),
            wgpu::DepthBiasState::default()
        );
    }
}
