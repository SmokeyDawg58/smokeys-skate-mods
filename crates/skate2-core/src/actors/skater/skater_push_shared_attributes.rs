//! SkaterManager's retained push animation measurements.
//!
//! `SkaterAnimShared::Init` 0x82cd48b0 constructs these two blocks before its
//! wipeout recovery graph. `GetPushAttributesForAnim` 0x82cb6d98 measures each
//! original body animation once; Motion Update reads the retained result.

use crate::{
    actor_animation_tree::{AnimationTreeError, AnimationTreePool, attributes::TreeAttributeError},
    actor_face_playback::FacePlaybackData,
    play_animation::{FastString30, FastString36},
    simulation_animation_startup::AnimationAttributePool,
};
use skate2_formats::andale::{AndaleError, AndaleRuntimeAttribute};

const REGULAR_NAMES: [&str; 4] = [
    "R_PUSHLSP_HSTR_N_0_CYC1",
    "R_PUSHHSP_HSTR_N_0_CYC1",
    "R_PUSHLSP_LSTR_N_0_CYC1",
    "R_PUSHHSP_LSTR_N_0_CYC1",
];
const MONGO_NAMES: [&str; 4] = [
    "R_PUSHLSP_HSTR_MONGO_0_CYC1",
    "R_PUSHHSP_HSTR_MONGO_0_CYC1",
    "R_PUSHLSP_LSTR_MONGO_0_CYC1",
    "R_PUSHHSP_LSTR_MONGO_0_CYC1",
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PushAnimationAttributes {
    pub(crate) length: f32,
    pub(crate) begin_velocity: f32,
    pub(crate) end_velocity: f32,
}

/// PDB `Sk8::Skater::PushAttributes`, 0x40 bytes at shared+0 or shared+0x40.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PushAttributes {
    pub(crate) lsp_hst: PushAnimationAttributes,
    pub(crate) hsp_hst: PushAnimationAttributes,
    pub(crate) lsp_lst: PushAnimationAttributes,
    pub(crate) hsp_lst: PushAnimationAttributes,
    pub(crate) begin_velocity_min: f32,
    pub(crate) begin_velocity_max: f32,
    pub(crate) dv_min: f32,
    pub(crate) dv_max: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SkaterPushSharedAttributes {
    pub(crate) regular: PushAttributes,
    pub(crate) mongo: PushAttributes,
}

#[derive(Debug)]
pub(crate) enum PushSharedAttributesError {
    Name(AndaleError),
    Tree(AnimationTreeError),
    Attributes(TreeAttributeError),
    MissingAnimation(FastString36),
    NonScalarAttribute(String),
}

impl std::fmt::Display for PushSharedAttributesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl SkaterPushSharedAttributes {
    /// `SkaterAnimShared::InitPushAttributes` 0x82cbf508, in native call order.
    pub(crate) fn try_build(
        playback: &FacePlaybackData,
        attributes_pool: &mut AnimationAttributePool,
    ) -> Result<Self, PushSharedAttributesError> {
        Ok(Self {
            regular: PushAttributes::try_build(playback, attributes_pool, REGULAR_NAMES)?,
            mongo: PushAttributes::try_build(playback, attributes_pool, MONGO_NAMES)?,
        })
    }

    /// `SetCoefs` 0x82ca0768..080c: regular iff authored flag differs from
    /// `ISkaterAnim::GetIsSwitch` (virtual +0x18).
    #[must_use]
    pub(crate) fn selected(&self, regular_attributes: bool, is_switch: bool) -> &PushAttributes {
        if regular_attributes != is_switch {
            &self.regular
        } else {
            &self.mongo
        }
    }
}

impl PushAttributes {
    fn try_build(
        playback: &FacePlaybackData,
        attributes_pool: &mut AnimationAttributePool,
        names: [&str; 4],
    ) -> Result<Self, PushSharedAttributesError> {
        // The native SkaterAnimShared constructor zeroes all 32 float fields.
        // A missing begin/end attribute therefore retains zero, while a later
        // matching attribute overwrites an earlier one.
        let mut block = Self::default();
        block.lsp_hst = read_animation(playback, attributes_pool, names[0])?;
        block.hsp_hst = read_animation(playback, attributes_pool, names[1])?;
        block.lsp_lst = read_animation(playback, attributes_pool, names[2])?;
        block.hsp_lst = read_animation(playback, attributes_pool, names[3])?;
        block.compute_bounds();
        Ok(block)
    }

    /// Staged `fsubs`/`fsel` order at 0x82cbf5a0..0x82cbf654.
    fn compute_bounds(&mut self) {
        let a = self.lsp_hst;
        let b = self.hsp_hst;
        let c = self.lsp_lst;
        let d = self.hsp_lst;
        let min_hst = fsel(
            a.begin_velocity - b.begin_velocity,
            b.begin_velocity,
            a.begin_velocity,
        );
        let min_lst = fsel(
            c.begin_velocity - d.begin_velocity,
            d.begin_velocity,
            c.begin_velocity,
        );
        self.begin_velocity_min = fsel(min_hst - min_lst, min_lst, min_hst);

        let dv_a = a.end_velocity - a.begin_velocity;
        let dv_b = b.end_velocity - b.begin_velocity;
        let dv_c = c.end_velocity - c.begin_velocity;
        let dv_d = d.end_velocity - d.begin_velocity;
        let min_hst = fsel(dv_a - dv_b, dv_b, dv_a);
        let min_lst = fsel(dv_c - dv_d, dv_d, dv_c);
        self.dv_min = fsel(min_hst - min_lst, min_lst, min_hst);
        let max_hst = fsel(dv_a - dv_b, dv_a, dv_b);
        let max_lst = fsel(dv_c - dv_d, dv_c, dv_d);
        self.dv_max = fsel(max_hst - max_lst, max_hst, max_lst);

        let max_hst = fsel(
            a.begin_velocity - b.begin_velocity,
            a.begin_velocity,
            b.begin_velocity,
        );
        let max_lst = fsel(
            c.begin_velocity - d.begin_velocity,
            c.begin_velocity,
            d.begin_velocity,
        );
        self.begin_velocity_max = fsel(max_hst - max_lst, max_hst, max_lst);
    }
}

/// `fsel`: negative and unordered selector choose the last operand.
fn fsel(selector: f32, nonnegative: f32, negative: f32) -> f32 {
    if selector >= 0.0 {
        nonnegative
    } else {
        negative
    }
}

fn read_animation(
    playback: &FacePlaybackData,
    attributes_pool: &mut AnimationAttributePool,
    text: &str,
) -> Result<PushAnimationAttributes, PushSharedAttributesError> {
    let name = FastString36::encode(text).map_err(PushSharedAttributesError::Name)?;
    // Each native call uses a fresh 0x100 element temporary animation pool.
    let mut tree_pool = AnimationTreePool::new(0x100, 0x100);
    let tree = playback
        .try_construct_animation_tree(&mut tree_pool, &name, &[])
        .map_err(PushSharedAttributesError::Tree)?
        .ok_or(PushSharedAttributesError::MissingAnimation(name))?;
    let mut result = PushAnimationAttributes {
        length: tree_pool
            .length(&tree)
            .map_err(PushSharedAttributesError::Tree)?,
        ..PushAnimationAttributes::default()
    };
    let mut attributes = Vec::new();
    tree_pool
        .get_attributes(&tree, 15, attributes_pool, &mut attributes)
        .map_err(PushSharedAttributesError::Attributes)?;
    scan_velocities(&attributes, &mut result)?;
    tree_pool.clear();
    Ok(result)
}

/// Scalar scan at 0x82cb6eac..0x82cb70a0, including last-match wins.
fn scan_velocities(
    attributes: &[AndaleRuntimeAttribute],
    result: &mut PushAnimationAttributes,
) -> Result<(), PushSharedAttributesError> {
    // Native compares the five packed words, whose alphabet folds ASCII case.
    // Serialized names decode to lowercase; source spelling is not identity.
    let high = FastString30::encode("HStr_Vel_B").expect("native attribute name");
    let low = FastString30::encode("LStr_Vel_B").expect("native attribute name");
    let end = FastString30::encode("Vel_E").expect("native attribute name");
    for attribute in attributes {
        let name =
            FastString30::encode(&attribute.name).map_err(PushSharedAttributesError::Name)?;
        let destination = if name == high || name == low {
            &mut result.begin_velocity
        } else if name == end {
            &mut result.end_velocity
        } else {
            continue;
        };
        *destination = attribute
            .scalar_value()
            .ok_or_else(|| PushSharedAttributesError::NonScalarAttribute(attribute.name.clone()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::andale::AndaleAttributeValue;

    fn scalar(name: &str, value: f32) -> AndaleRuntimeAttribute {
        AndaleRuntimeAttribute {
            sequence_id: -1,
            begin_seconds: -1.0,
            end_seconds: -1.0,
            name: name.to_owned(),
            status: 6,
            value: AndaleAttributeValue::Float(value),
        }
    }

    #[test]
    fn serialized_lowercase_names_retain_authored_push_velocities() {
        let mut result = PushAnimationAttributes::default();
        let decoded = |name: &str| FastString30::encode(name).unwrap().decode().unwrap();
        scan_velocities(
            &[
                scalar(&decoded("HStr_Vel_B"), 2.0),
                scalar(&decoded("LStr_Vel_B"), 3.0),
                scalar(&decoded("Vel_E"), 7.0),
            ],
            &mut result,
        )
        .unwrap();
        assert_eq!(result.begin_velocity, 3.0);
        assert_eq!(result.end_velocity, 7.0);
    }

    #[test]
    fn missing_values_remain_constructor_zero_and_last_match_wins() {
        let mut result = PushAnimationAttributes::default();
        scan_velocities(
            &[
                scalar("Other", 7.0),
                scalar("HStr_Vel_B", 1.0),
                scalar("LStr_Vel_B", 2.0),
            ],
            &mut result,
        )
        .unwrap();
        assert_eq!(result.begin_velocity, 2.0);
        assert_eq!(result.end_velocity, 0.0);
    }

    #[test]
    fn bounds_use_all_four_clip_measurements() {
        let mut block = PushAttributes {
            lsp_hst: PushAnimationAttributes {
                length: 1.0,
                begin_velocity: 4.0,
                end_velocity: 6.0,
            },
            hsp_hst: PushAnimationAttributes {
                length: 2.0,
                begin_velocity: 8.0,
                end_velocity: 11.0,
            },
            lsp_lst: PushAnimationAttributes {
                length: 3.0,
                begin_velocity: 3.0,
                end_velocity: 8.0,
            },
            hsp_lst: PushAnimationAttributes {
                length: 4.0,
                begin_velocity: 9.0,
                end_velocity: 10.0,
            },
            ..PushAttributes::default()
        };
        block.compute_bounds();
        assert_eq!(
            (block.begin_velocity_min, block.begin_velocity_max),
            (3.0, 9.0)
        );
        assert_eq!((block.dv_min, block.dv_max), (1.0, 5.0));
    }
}
