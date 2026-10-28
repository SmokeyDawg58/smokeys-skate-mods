//! `UpdatePhysicsError82b23aa8`: orientation error on the retained outer board.
use super::SkateboardSteeringState;
use crate::{reckoning_numeric::ReckoningNumeric, skeleton_root_transforms::SkeletonRootMatrix};

pub(crate) trait PhysicsErrorSource {
    type Error;
    fn orientation(&mut self) -> Result<SkeletonRootMatrix, Self::Error>;
    fn part_six(&mut self) -> Result<SkeletonRootMatrix, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum PhysicsErrorUpdateError<S, N> {
    Source(S),
    Numeric(N),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for PhysicsErrorUpdateError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => write!(f, "board physics-error input: {error:?}"),
            Self::Numeric(error) => write!(f, "board physics-error numeric: {error:?}"),
        }
    }
}

pub(super) fn refine(squared: f32, seed: f32) -> f32 {
    let square = seed * seed;
    let residual = -squared.mul_add(square, -1.0);
    let refined = (seed * 0.5).mul_add(residual, seed);
    let square = refined * refined;
    let residual = -squared.mul_add(square, -1.0);
    (refined * 0.5).mul_add(residual, refined)
}

impl SkateboardSteeringState {
    pub(crate) fn try_update_physics_error<S: PhysicsErrorSource, N: ReckoningNumeric>(
        &mut self,
        source: &mut S,
        numeric: &mut N,
    ) -> Result<(), PhysicsErrorUpdateError<S::Error, N::Error>> {
        use PhysicsErrorUpdateError as E;
        // Native inverse is computed before the fresh Body.GetPosition(6).
        let inverse = source.orientation().map_err(E::Source)?.inverse();
        let part = source.part_six().map_err(E::Source)?;
        let relative = inverse.compose(&part);
        let normalized =
            crate::skeleton_affine_interpolation::try_normalize_rotation_with_refinement(
                &relative, numeric, refine,
            )
            .map_err(E::Numeric)?;
        // The candidate consumes only XYZ. Its quaternion W is newly derived;
        // no unknown affine W is published as a known homogeneous lane.
        let quaternion = crate::world::try_simulation_quaternion_candidate(
            normalized.xyz_scratch(),
            |magnitudes| {
                let mut estimates = [0.0; 4];
                for (target, value) in estimates.iter_mut().zip(magnitudes) {
                    *target = numeric.reciprocal_sqrt_estimate(value)?;
                }
                Ok::<_, N::Error>(estimates)
            },
        )
        .map_err(E::Numeric)?;
        // 3e14 then3e18. There is no extra quaternion normalization.
        self.physics_error = quaternion;
        self.physics_error_blending = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::{HostReckoningNumeric, host_dot3};
    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    struct Source {
        fail: bool,
        calls: Vec<&'static str>,
    }
    impl PhysicsErrorSource for Source {
        type Error = &'static str;
        fn orientation(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
            self.calls.push("orientation");
            Ok(SkeletonRootMatrix::known(IDENTITY))
        }
        fn part_six(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
            self.calls.push("part6");
            if self.fail {
                return Err("part unavailable");
            }
            let mut columns = *SkeletonRootMatrix::known(IDENTITY).columns();
            for column in &mut columns {
                column.w = None;
            }
            Ok(SkeletonRootMatrix::from_columns(columns))
        }
    }
    #[test]
    fn constructor_and_source_failure_preserve_real_board_state() {
        let mut board = SkateboardSteeringState::new();
        assert_eq!(
            board.physics_error().map(f32::to_bits),
            [0, 0, 0, 0x3f80_0000]
        );
        assert!(!board.physics_error_blending());
        let mut source = Source {
            fail: true,
            calls: Vec::new(),
        };
        assert!(matches!(
            board.try_update_physics_error(&mut source, &mut HostReckoningNumeric),
            Err(PhysicsErrorUpdateError::Source("part unavailable"))
        ));
        assert_eq!(source.calls, ["orientation", "part6"]);
        assert!(!board.physics_error_blending());
        assert_eq!(
            board.physics_error().map(f32::to_bits),
            [0, 0, 0, 0x3f80_0000]
        );
    }
    struct Numeric {
        calls: usize,
        fail: usize,
    }
    impl ReckoningNumeric for Numeric {
        type Error = &'static str;
        fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
            self.step()?;
            Ok(host_dot3(a, b))
        }
        fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
            self.step()?;
            // Deliberately imperfect candidate-W seed proves no final norm.
            Ok(if self.calls == 10 {
                0.25
            } else {
                1.0 / value.sqrt()
            })
        }
        fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            panic!("not used")
        }
        fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
            panic!("not used")
        }
    }
    impl Numeric {
        fn step(&mut self) -> Result<(), &'static str> {
            self.calls += 1;
            if self.calls == self.fail {
                Err("numeric refused")
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn unknown_affine_w_does_not_block_quaternion_or_add_a_normalization() {
        let mut board = SkateboardSteeringState::new();
        let mut numeric = Numeric {
            calls: 0,
            fail: usize::MAX,
        };
        board
            .try_update_physics_error(
                &mut Source {
                    fail: false,
                    calls: Vec::new(),
                },
                &mut numeric,
            )
            .unwrap();
        assert_eq!(numeric.calls, 13); // Nine ortho boundaries, four vector-estimate lanes.
        assert_eq!(
            board.physics_error().map(f32::to_bits),
            [0, 0, 0, 0x3f5e_6800]
        );
        assert!(board.physics_error_blending());
    }
    #[test]
    fn each_numeric_failure_precedes_both_final_stores() {
        for fail in 1..=13 {
            let mut board = SkateboardSteeringState::new();
            board.physics_error = [2.0, 3.0, 4.0, 5.0];
            let mut numeric = Numeric { calls: 0, fail };
            assert!(matches!(
                board.try_update_physics_error(
                    &mut Source {
                        fail: false,
                        calls: Vec::new()
                    },
                    &mut numeric
                ),
                Err(PhysicsErrorUpdateError::Numeric("numeric refused"))
            ));
            assert_eq!(
                board.physics_error().map(f32::to_bits),
                [2.0_f32, 3.0, 4.0, 5.0].map(f32::to_bits)
            );
            assert!(!board.physics_error_blending());
        }
    }
}
