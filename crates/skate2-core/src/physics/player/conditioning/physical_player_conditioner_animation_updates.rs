//! Four original Animation conditioner callbacks. Mount below `conditioners::animation`.
//! `UpdateAnimationTurn82e2b470` remains a required intervening callback.
use super::Animation;
use crate::{
    physical_player_phys_out::trick::AnimOutTrickData,
    play_animation::{EMPTY_FAST_STRING_36, FastString36},
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};

/// Every read/store method represents a separate checked `PhysOut` accessor.
/// Implement against the bound, constructed `PhysOut` occurrence, never snapshots.
pub(crate) trait AnimationUpdateOutput {
    type Error;
    type Guard;
    fn current_animation_name(&mut self) -> Result<FastString36, Self::Error>;
    fn current_animation(&mut self) -> Result<AnimOutTrickData, Self::Error>;
    fn store_current_name(&mut self, value: FastString36) -> Result<(), Self::Error>;
    fn store_completed_flag(&mut self, value: bool) -> Result<(), Self::Error>;
    fn store_completed_name(&mut self, value: FastString36) -> Result<(), Self::Error>;
    fn body_tilt(&mut self) -> Result<Vector, Self::Error>;
    fn store_previous_body_tilt(&mut self, value: Vector) -> Result<(), Self::Error>;
    fn draw_tilt_enabled(&mut self) -> Result<bool, Self::Error>;
    /// Require actual `DebugOverlay` singleton; the final native draw call is BLR.
    fn require_tilt_overlay(&mut self) -> Result<(), Self::Error>;
    fn deck_transform(&mut self) -> Result<Matrix, Self::Error>;
    fn deck_acceleration(&mut self) -> Result<Vector, Self::Error>;
    fn sample_guard(&mut self) -> Result<Self::Guard, Self::Error>;
    fn ground_transform(&mut self, guard: Self::Guard) -> Result<Matrix, Self::Error>;
    fn store_deck_ground_acceleration(
        &mut self,
        guard: Self::Guard,
        value: Vector,
    ) -> Result<(), Self::Error>;
}

impl Animation {
    #[expect(
        clippy::unused_self,
        reason = "native callback belongs to the retained Animation conditioner"
    )]
    pub(crate) fn try_update_current_trick<O: AnimationUpdateOutput>(
        &mut self,
        output: &mut O,
    ) -> Result<(), O::Error> {
        let name = output.current_animation_name()?;
        output.store_current_name(current_trick_name(name))
    }

    pub(crate) fn try_update_completed_trick<O: AnimationUpdateOutput>(
        &mut self,
        output: &mut O,
    ) -> Result<(), O::Error> {
        let mut completed = EMPTY_FAST_STRING_36;
        let mut has_completed = false;
        if self.previous_frame_trick.name != EMPTY_FAST_STRING_36
            && output.current_animation_name()? != self.previous_frame_trick.name
        {
            completed = self.previous_frame_trick.name;
            has_completed = true;
        }
        // The second current-data access is unconditional, including invalid history.
        self.previous_frame_trick = output.current_animation()?;
        output.store_completed_flag(has_completed)?;
        output.store_completed_name(completed)
    }

    pub(crate) fn try_update_body_tilt<O: AnimationUpdateOutput>(
        &mut self,
        output: &mut O,
    ) -> Result<(), O::Error> {
        let current = output.body_tilt()?;
        output.store_previous_body_tilt(self.last_body_tilt)?;
        self.last_body_tilt = current;
        if output.draw_tilt_enabled()? {
            output.require_tilt_overlay()?;
        }
        Ok(())
    }

    #[expect(
        clippy::unused_self,
        reason = "native callback belongs to the retained Animation conditioner"
    )]
    pub(crate) fn try_update_deck_ground_acceleration<O: AnimationUpdateOutput>(
        &mut self,
        output: &mut O,
    ) -> Result<(), O::Error> {
        let deck = output.deck_transform()?;
        let acceleration = output.deck_acceleration()?;
        // 82e29a7c captures the ground guard before any vector calculation.
        let ground_guard = output.sample_guard()?;
        let mut local = transpose_direction(deck, acceleration);
        local.xyz[1] = 0.0;
        let world = direction(deck, local);
        let ground = output.ground_transform(ground_guard)?;
        // 82e29b00 captures the destination guard before the final transpose.
        let destination_guard = output.sample_guard()?;
        let mut result = transpose_direction(ground, world);
        result.xyz[1] = 0.0;
        output.store_deck_ground_acceleration(destination_guard, result)
    }
}

fn current_trick_name(name: FastString36) -> FastString36 {
    let excluded = [
        "ridingidle",
        "wipeout",
        "land",
        "tailmanual",
        "nosemanual",
        "switching",
    ];
    if excluded
        .into_iter()
        .any(|text| name == FastString36::encode(text).expect("source literal"))
    {
        EMPTY_FAST_STRING_36
    } else {
        name
    }
}

// Raw VMX transposes all four columns, including P. W is the dot product
// with P.xyz, not zero, and basis W does not enter the result of this operation.
fn transpose_direction(matrix: Matrix, value: Vector) -> Vector {
    let dot = |column: Vector| {
        let x = column.xyz[0] * value.xyz[0];
        let xy = column.xyz[1].mul_add(value.xyz[1], x);
        column.xyz[2].mul_add(value.xyz[2], xy)
    };
    Vector {
        xyz: std::array::from_fn(|index| dot(matrix.columns()[index])),
        w: Some(dot(matrix.columns()[3])),
    }
}
fn direction(matrix: Matrix, value: Vector) -> Vector {
    let columns = matrix.columns();
    let x = columns[0].map(|lane| lane * value.xyz[0]);
    let xy = columns[1].zip(x, |lane, sum| lane.mul_add(value.xyz[1], sum));
    columns[2].zip(xy, |lane, sum| lane.mul_add(value.xyz[2], sum))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Output {
        current: AnimOutTrickData,
        events: Vec<&'static str>,
        fail_name: bool,
    }
    impl AnimationUpdateOutput for Output {
        type Error = &'static str;
        type Guard = ();
        fn current_animation_name(&mut self) -> Result<FastString36, Self::Error> {
            self.events.push("name");
            Ok(self.current.name)
        }
        fn current_animation(&mut self) -> Result<AnimOutTrickData, Self::Error> {
            self.events.push("data");
            Ok(self.current.clone())
        }
        fn store_current_name(&mut self, _: FastString36) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn store_completed_flag(&mut self, _: bool) -> Result<(), Self::Error> {
            self.events.push("flag");
            Ok(())
        }
        fn store_completed_name(&mut self, _: FastString36) -> Result<(), Self::Error> {
            self.events.push("completed");
            if self.fail_name {
                Err("name store")
            } else {
                Ok(())
            }
        }
        fn body_tilt(&mut self) -> Result<Vector, Self::Error> {
            unreachable!()
        }
        fn store_previous_body_tilt(&mut self, _: Vector) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn draw_tilt_enabled(&mut self) -> Result<bool, Self::Error> {
            unreachable!()
        }
        fn require_tilt_overlay(&mut self) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn deck_transform(&mut self) -> Result<Matrix, Self::Error> {
            unreachable!()
        }
        fn deck_acceleration(&mut self) -> Result<Vector, Self::Error> {
            unreachable!()
        }
        fn sample_guard(&mut self) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn ground_transform(&mut self, (): ()) -> Result<Matrix, Self::Error> {
            unreachable!()
        }
        fn store_deck_ground_acceleration(&mut self, (): (), _: Vector) -> Result<(), Self::Error> {
            unreachable!()
        }
    }

    #[test]
    fn completed_trick_lazy_read_and_failed_final_store_retain_full_history() {
        let mut animation = Animation::before_filters();
        let mut current = AnimOutTrickData::reset_values();
        current.name = FastString36::encode("kickflip").unwrap();
        current.bits = 0x1020_3040;
        let mut output = Output {
            current,
            events: Vec::new(),
            fail_name: false,
        };
        animation.try_update_completed_trick(&mut output).unwrap();
        assert_eq!(output.events, ["data", "flag", "completed"]);
        output.current.name = FastString36::encode("land").unwrap();
        output.current.bits = 0x5566_7788;
        output.events.clear();
        output.fail_name = true;
        assert_eq!(
            animation.try_update_completed_trick(&mut output),
            Err("name store")
        );
        assert_eq!(output.events, ["name", "data", "flag", "completed"]);
        assert_eq!(animation.previous_frame_trick, output.current);
    }

    #[test]
    fn excluded_names_are_packed_case_insensitively_and_other_names_survive() {
        for name in [
            "RIDINGIDLE",
            "wipeout",
            "land",
            "tailmanual",
            "nosemanual",
            "switching",
        ] {
            assert_eq!(
                current_trick_name(FastString36::encode(name).unwrap()),
                EMPTY_FAST_STRING_36
            );
        }
        let kickflip = FastString36::encode("kickflip").unwrap();
        assert_eq!(current_trick_name(kickflip), kickflip);
    }

    #[test]
    fn transpose_preserves_native_translation_dot_w_without_basis_w() {
        let matrix = Matrix::from_columns([
            Vector {
                xyz: [0., 1., 0.],
                w: None,
            },
            Vector {
                xyz: [-1., 0., 0.],
                w: None,
            },
            Vector {
                xyz: [0., 0., 1.],
                w: None,
            },
            Vector {
                xyz: [2., 3., 4.],
                w: None,
            },
        ]);
        let value = Vector {
            xyz: [5., 6., 7.],
            w: None,
        };
        assert_eq!(
            transpose_direction(matrix, value),
            Vector::known([6., -5., 7., 56.])
        );
        assert_eq!(direction(matrix, value).w, None);
        let contaminated = transpose_direction(
            matrix,
            Vector {
                xyz: [f32::NAN, 0., 0.],
                w: None,
            },
        );
        assert!(contaminated.xyz.into_iter().all(f32::is_nan));
        assert!(contaminated.w.unwrap().is_nan());
    }
}
