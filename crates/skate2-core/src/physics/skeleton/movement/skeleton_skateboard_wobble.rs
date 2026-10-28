//! Skeleton-owned `SkateboardWobble` ctor82b4ae68; distinct from board `SpeedWobble`.
use crate::attribute_runtime::{
    AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::sync::Arc;
#[path = "../../player/wipeout/physical_player_post_wipeout_prefix_wobble.rs"]
pub(crate) mod post_wipeout_update;

/// Complete PDB `PointNegGraphData8` copy; this does not evaluate the graph.
#[expect(
    dead_code,
    reason = "constructor graph data retained for pending original Skeleton SkateboardWobble update"
)]
pub(crate) struct WobbleGraph {
    pub(crate) min_x: f32,
    pub(crate) min_y: f32,
    pub(crate) max_x: f32,
    pub(crate) max_y: f32,
    pub(crate) x: [f32; 8],
    pub(crate) y: [f32; 8],
}
impl WobbleGraph {
    fn try_read(
        attributes: &AttributeRuntimeInstance,
        key: u64,
    ) -> Result<Self, AttributeRuntimeError> {
        let data = attributes.try_get_data(key, 0)?;
        let mut words = [0.0; 20];
        if let Some(data) = data {
            if data.bytes().len() < 0x50 {
                return Err(AttributeRuntimeError::PayloadSize {
                    key,
                    expected: 0x50,
                    actual: data.bytes().len(),
                });
            }
            for (i, word) in words.iter_mut().enumerate() {
                *word = f32::from_be_bytes(
                    data.bytes()[i * 4..i * 4 + 4]
                        .try_into()
                        .expect("checked 80-byte graph"),
                );
            }
        } // Source absent attribute -> DefaultDataArea80, then memcpy80.
        Ok(Self {
            min_x: words[0],
            min_y: words[1],
            max_x: words[2],
            max_y: words[3],
            x: words[4..12].try_into().expect("eight x values"),
            y: words[12..20].try_into().expect("eight y values"),
        })
    }
}
pub(crate) struct SkeletonSkateboardWobble {
    pub(crate) attributes: AttributeRuntimeInstance,
    pub(crate) tilt: Option<WobbleGraph>,
    pub(crate) squish: Option<WobbleGraph>,
    pub(crate) active: Option<bool>,
    pub(crate) landing: Option<bool>,
    pub(crate) time: Option<f32>,
    pub(crate) strength: Option<f32>,
    pub(crate) mirrored_scalar: Option<f32>,
    attempted: bool,
}
impl SkeletonSkateboardWobble {
    /// Skeleton::TeleportReset 0x82b5428c..9c writes the retained wobble
    /// scalars in source order, leaving graph buffers and attributes intact.
    pub(crate) fn reset_after_teleport(&mut self) {
        self.time = Some(0.0);
        self.active = Some(false);
        self.strength = Some(0.0);
        self.landing = Some(false);
        self.mirrored_scalar = Some(1.0);
    }
    /// Source `PhysicsSkater` allocation0xc0; partial fields are not zero-cleared.
    pub(crate) fn before_constructor(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            attributes: vault.instance(0x1ca5_e1dd_0b2e_a896, 0xd7ed_bd36_2d7d_2152),
            tilt: None,
            squish: None,
            active: None,
            landing: None,
            time: None,
            strength: None,
            mirrored_scalar: None,
            attempted: false,
        }
    }
    pub(crate) fn try_construct(&mut self) -> Result<(), WobbleConstructorError> {
        if self.attempted {
            return Err(WobbleConstructorError::AlreadyAttempted);
        }
        self.attempted = true;
        self.tilt = Some(
            WobbleGraph::try_read(&self.attributes, 0x68d5_052a_44cd_5a16)
                .map_err(WobbleConstructorError::Attribute)?,
        );
        self.squish = Some(
            WobbleGraph::try_read(&self.attributes, 0x9330_7f84_fd4c_8c1f)
                .map_err(WobbleConstructorError::Attribute)?,
        );
        self.active = Some(false);
        self.landing = Some(false);
        self.time = Some(0.0);
        self.strength = Some(0.0);
        self.mirrored_scalar = Some(1.0);
        Ok(())
    }

    /// StartNewWobble82b3e388, selected by Skeleton.StartDeckLandingWobble82b420e8.
    /// Both graph lookups precede the first scalar store; failed lookups retain
    /// the earlier graph copy just as the native ordered calls do.
    pub(crate) fn try_start_landing(
        &mut self,
        mirrored: bool,
        strength: f32,
    ) -> Result<(), AttributeRuntimeError> {
        self.try_start(true, mirrored, strength)
    }
    pub(crate) fn try_start(
        &mut self,
        landing: bool,
        mirrored: bool,
        strength: f32,
    ) -> Result<(), AttributeRuntimeError> {
        self.tilt = Some(WobbleGraph::try_read(
            &self.attributes,
            if landing {
                0x68d5_052a_44cd_5a16
            } else {
                0x8025_a9d1_45ca_1251
            },
        )?);
        self.squish = Some(WobbleGraph::try_read(
            &self.attributes,
            if landing {
                0x9330_7f84_fd4c_8c1f
            } else {
                0xfc6b_fc42_7873_47e0
            },
        )?);
        self.strength = Some(strength);
        self.landing = Some(landing);
        self.active = Some(true);
        self.time = Some(0.0);
        self.mirrored_scalar = Some(if mirrored { -1.0 } else { 1.0 });
        Ok(())
    }
}
#[derive(Debug)]
pub(crate) enum WobbleConstructorError {
    AlreadyAttempted,
    Attribute(AttributeRuntimeError),
}
impl std::fmt::Display for WobbleConstructorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttempted => {
                f.write_str("SkateboardWobble construction already attempted")
            }
            Self::Attribute(e) => write!(f, "SkateboardWobble graph: {e}"),
        }
    }
}
