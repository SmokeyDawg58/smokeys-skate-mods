//! Recovery animation coordinates owned by `SkaterAnimShared`.
//!
//! `InitWipeoutRecoverGraph` 0x82ccc340 constructs each original animation tree
//! and reads its ordered `GetAttributes(..., 15)` output. The resulting graph
//! is fixed at manager initialization; selection does not sample attributes
//! again during `PlayWipeoutRecoverAnim::Begin`.

use crate::{
    actor_animation_tree::{AnimationTreeError, AnimationTreePool, attributes::TreeAttributeError},
    actor_face_playback::FacePlaybackData,
    play_animation::{FastString30, FastString36},
    simulation_animation_startup::AnimationAttributePool,
};
use skate2_formats::andale::{AndaleError, AndaleRuntimeAttribute};

/// Source order at 0x82ccc340. The repeated final name is present in the
/// source and affects strict first-tie selection.
const RECOVERY_ANIMATIONS: [&str; 10] = [
    "W_RECOVERY_ONBACKFACE_N_0_N",
    "W_RECOVERY_ONBACKLGUT_N_0_N",
    "W_RECOVERY_ONBACKLSPRAWL_N_0_N",
    "W_RECOVERY_ONBACKRFACE_N_0_N",
    "W_RECOVERY_ONBACKRSPRAWL_N_0_N",
    "W_RECOVERY_ONBACKSPRAWL_N_0_N",
    "W_RECOVERY_ONFRONT_N_0_N",
    "W_RECOVERY_ONLEFT_N_0_N",
    "W_RECOVERY_ONRIGHT_N_0_N",
    "W_RECOVERY_ONRIGHT_N_0_N",
];

const FULL_TURN: f32 = f32::from_bits(0x40c9_0fdb); // Source 6.2831855f.

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WipeoutRecoveryPoint {
    pub(crate) lean: f32,
    pub(crate) twist: f32,
    pub(crate) animation: FastString36,
}

#[derive(Debug, Default)]
pub(crate) struct SkaterWipeoutRecoveryGraph {
    points: Vec<WipeoutRecoveryPoint>,
}

#[derive(Debug)]
pub(crate) enum WipeoutRecoveryGraphError {
    Name(AndaleError),
    Tree(AnimationTreeError),
    Attributes(TreeAttributeError),
    MissingAnimation(FastString36),
    NonScalarCoordinate(FastString30),
}

impl std::fmt::Display for WipeoutRecoveryGraphError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Name(error) => error.fmt(formatter),
            Self::Tree(error) => error.fmt(formatter),
            Self::Attributes(error) => error.fmt(formatter),
            Self::MissingAnimation(name) => {
                write!(formatter, "missing recovery animation {name:?}")
            }
            Self::NonScalarCoordinate(name) => {
                write!(formatter, "recovery coordinate {name:?} is not scalar")
            }
        }
    }
}

impl SkaterWipeoutRecoveryGraph {
    /// Builds the graph from the original `Skater` playback and the shared
    /// animation attribute pool. The native function creates a dedicated
    /// 0x100-byte/0x100-element temporary PoolStack and passes a null
    /// construction map to `PlaybackData::GetAnimTree`.
    ///
    /// `SkaterAnimShared::InitWipeoutRecoverGraph` 0x82ccc340.
    pub(crate) fn try_build(
        playback: &FacePlaybackData,
        attributes_pool: &mut AnimationAttributePool,
    ) -> Result<Self, WipeoutRecoveryGraphError> {
        let mut graph = Self::default();
        let mut tree_pool = AnimationTreePool::new(0x100, 0x100);
        for text in RECOVERY_ANIMATIONS {
            let name = FastString36::encode(text).map_err(WipeoutRecoveryGraphError::Name)?;
            tree_pool.clear();
            let tree = playback
                .try_construct_animation_tree(&mut tree_pool, &name, &[])
                .map_err(WipeoutRecoveryGraphError::Tree)?
                .ok_or(WipeoutRecoveryGraphError::MissingAnimation(name))?;
            let mut attributes = Vec::new();
            tree_pool
                .get_attributes(&tree, 15, attributes_pool, &mut attributes)
                .map_err(WipeoutRecoveryGraphError::Attributes)?;
            let (lean, twist) = scan_coordinates(&attributes)?;
            graph.append(lean, twist, name);
        }
        tree_pool.clear();
        Ok(graph)
    }

    /// `SkaterAnimShared::GetWipeoutRecoverAnim` 0x82c92738. The native leaf
    /// has no empty-graph guard; `None` exposes that missing owner safely.
    #[must_use]
    pub(crate) fn select(&self, lean: f32, twist: f32) -> Option<FastString36> {
        let mut nearest = f32::MAX;
        let mut selected = self.points.first().map(|point| point.animation);
        for point in &self.points {
            let lean_delta = lean - point.lean;
            let lean_squared = lean_delta * lean_delta;
            let twist_delta = twist - point.twist;
            let distance = twist_delta.mul_add(twist_delta, lean_squared);
            if distance < nearest {
                nearest = distance;
                selected = Some(point.animation);
            }
        }
        selected
    }

    #[must_use]
    pub(crate) fn points(&self) -> &[WipeoutRecoveryPoint] {
        &self.points
    }

    /// Native insertion appends an alias immediately after its source point.
    fn append(&mut self, lean: f32, twist: f32, animation: FastString36) {
        self.points.push(WipeoutRecoveryPoint {
            lean,
            twist,
            animation,
        });
        let alias = if twist > 0.0 {
            Some(twist - FULL_TURN)
        } else if twist < 0.0 {
            Some(twist + FULL_TURN)
        } else {
            None
        };
        if let Some(twist) = alias {
            self.points.push(WipeoutRecoveryPoint {
                lean,
                twist,
                animation,
            });
        }
    }
}

/// The source scans all 64-byte output records in order, starting both
/// coordinates at +0.0. A later matching record replaces an earlier value.
fn scan_coordinates(
    attributes: &[AndaleRuntimeAttribute],
) -> Result<(f32, f32), WipeoutRecoveryGraphError> {
    let lean_name = FastString30::encode("Lean").map_err(WipeoutRecoveryGraphError::Name)?;
    let twist_name = FastString30::encode("Twist").map_err(WipeoutRecoveryGraphError::Name)?;
    let mut lean = 0.0;
    let mut twist = 0.0;
    for attribute in attributes {
        let name =
            FastString30::encode(&attribute.name).map_err(WipeoutRecoveryGraphError::Name)?;
        if name == lean_name {
            lean = attribute
                .scalar_value()
                .ok_or(WipeoutRecoveryGraphError::NonScalarCoordinate(name))?;
        } else if name == twist_name {
            twist = attribute
                .scalar_value()
                .ok_or(WipeoutRecoveryGraphError::NonScalarCoordinate(name))?;
        }
    }
    Ok((lean, twist))
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
    fn ordered_scan_defaults_absent_coordinate_and_keeps_last_match() {
        assert_eq!(scan_coordinates(&[]).unwrap(), (0.0, 0.0));
        let attributes = [
            scalar("Lean", -0.25),
            scalar("Other", 3.0),
            scalar("Lean", 0.75),
        ];
        assert_eq!(scan_coordinates(&attributes).unwrap(), (0.75, 0.0));
    }

    #[test]
    fn source_order_aliases_and_strict_first_tie() {
        let a = FastString36::encode("A").unwrap();
        let b = FastString36::encode("B").unwrap();
        let mut graph = SkaterWipeoutRecoveryGraph::default();
        assert_eq!(graph.select(0.0, 0.0), None);
        graph.append(0.0, 1.0, a);
        graph.append(0.0, -1.0, b);
        assert_eq!(graph.points().len(), 4);
        assert_eq!(graph.points()[0].animation, a);
        assert_eq!(
            graph.points()[1].twist.to_bits(),
            (1.0 - FULL_TURN).to_bits()
        );
        assert_eq!(graph.points()[2].animation, b);
        assert_eq!(
            graph.points()[3].twist.to_bits(),
            (-1.0 + FULL_TURN).to_bits()
        );
        assert_eq!(graph.select(0.0, 0.0), Some(a));
        assert_eq!(graph.select(0.0, -1.0), Some(b));
        assert_eq!(graph.select(f32::NAN, 0.0), Some(a));
    }

    #[test]
    fn authored_name_list_retains_duplicate_last_entry() {
        assert_eq!(RECOVERY_ANIMATIONS.len(), 10);
        assert_eq!(RECOVERY_ANIMATIONS[8], RECOVERY_ANIMATIONS[9]);
        for name in RECOVERY_ANIMATIONS {
            assert!(FastString36::encode(name).is_ok(), "{name}");
        }
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn original_skater_playback_builds_source_recovery_graph() {
        let startup = crate::simulation_animation_startup::corpus::load_original_startup();
        let playback = startup.body[0].lock().unwrap();
        let mut attributes = startup.attributes.as_ref().unwrap().lock().unwrap();
        let graph = SkaterWipeoutRecoveryGraph::try_build(&playback, &mut attributes).unwrap();
        assert!(graph.points().len() >= RECOVERY_ANIMATIONS.len());
        assert!(
            graph
                .points()
                .iter()
                .all(|point| point.lean.is_finite() && point.twist.is_finite())
        );
        assert!(graph.select(0.0, 0.0).is_some());
        for point in graph.points() {
            println!(
                "recovery graph {} lean={:08x} twist={:08x}",
                point.animation.decode().unwrap(),
                point.lean.to_bits(),
                point.twist.to_bits()
            );
        }
    }
}
