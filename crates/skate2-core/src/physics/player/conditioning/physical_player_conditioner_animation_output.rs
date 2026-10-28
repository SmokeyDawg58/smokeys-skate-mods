//! Actual `PhysOut` adapter for all five Animation conditioner callbacks.
//! Mount as a child of `PhysOut::construct`; no output snapshots are retained.
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_phys_out::{
        PhysOutAccessError, PhysOutMotionScalar, PhysOutWriteGuard,
        conditioners::animation::{
            AnimationAttributes,
            turn::{AnimationTurnLayoutError, AnimationTurnSource, TurnField, TurnFilter},
            updates::AnimationUpdateOutput,
        },
        trick::AnimOutTrickData,
    },
    play_animation::FastString36,
    reckoning_pid::PhysicsControllerData,
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};

#[derive(Debug)]
pub(crate) enum AnimationOutputError {
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<std::convert::Infallible>),
    Attribute(AttributeRuntimeError),
    Layout(AnimationTurnLayoutError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    InvalidGraph { offset: usize, points: usize },
}
impl std::fmt::Display for AnimationOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(e) => write!(f, "animation output access: {e:?}"),
            Self::Component(e) => write!(f, "animation output component: {e:?}"),
            Self::Attribute(e) => write!(f, "animation attribute: {e}"),
            Self::Layout(e) => write!(f, "animation layout: {e:?}"),
            Self::Cvar(e) => write!(f, "animation cvar: {e:?}"),
            Self::Overlay(e) => write!(f, "animation overlay: {e}"),
            Self::InvalidGraph { offset, points } => {
                write!(f, "unknown animation graph {offset:x}/{points}")
            }
        }
    }
}

pub(crate) struct AnimationOutput<'a> {
    pub(crate) output: &'a mut PhysicalPlayerPhysOut,
    pub(crate) attributes: &'a AnimationAttributes,
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) overlay: &'a DebugOverlaySlot,
}
impl AnimationOutput<'_> {
    fn guard(&self, component: PhysOutComponent) -> Result<(), AnimationOutputError> {
        self.output
            .check_current_guard()
            .map_err(AnimationOutputError::Access)?;
        self.output
            .require_complete_component(component)
            .map_err(AnimationOutputError::Component)
    }
    fn captured_guard(
        &self,
        guard: &PhysOutWriteGuard,
        component: PhysOutComponent,
    ) -> Result<(), AnimationOutputError> {
        self.output
            .check_write_guard(guard)
            .map_err(AnimationOutputError::Access)?;
        self.output
            .require_complete_component(component)
            .map_err(AnimationOutputError::Component)
    }
}
impl AnimationUpdateOutput for AnimationOutput<'_> {
    type Error = AnimationOutputError;
    type Guard = PhysOutWriteGuard;
    fn current_animation_name(&mut self) -> Result<FastString36, Self::Error> {
        self.guard(PhysOutComponent::Trick)?;
        Ok(self
            .output
            .concrete
            .trick
            .as_ref()
            .unwrap()
            .current_animation
            .name)
    }
    fn current_animation(&mut self) -> Result<AnimOutTrickData, Self::Error> {
        self.guard(PhysOutComponent::Trick)?;
        Ok(self
            .output
            .concrete
            .trick
            .as_ref()
            .unwrap()
            .current_animation
            .clone())
    }
    fn store_current_name(&mut self, value: FastString36) -> Result<(), Self::Error> {
        self.guard(PhysOutComponent::Trick)?;
        self.output.concrete.trick.as_mut().unwrap().current_name = value;
        Ok(())
    }
    fn store_completed_flag(&mut self, value: bool) -> Result<(), Self::Error> {
        self.guard(PhysOutComponent::Trick)?;
        self.output
            .concrete
            .trick
            .as_mut()
            .unwrap()
            .has_completed_trick = value;
        Ok(())
    }
    fn store_completed_name(&mut self, value: FastString36) -> Result<(), Self::Error> {
        self.guard(PhysOutComponent::Trick)?;
        self.output.concrete.trick.as_mut().unwrap().completed_name = value;
        Ok(())
    }
    fn body_tilt(&mut self) -> Result<Vector, Self::Error> {
        self.guard(PhysOutComponent::Animation)?;
        Ok(self.output.concrete.animation.as_ref().unwrap().anim_tilt)
    }
    fn store_previous_body_tilt(&mut self, value: Vector) -> Result<(), Self::Error> {
        self.guard(PhysOutComponent::Animation)?;
        self.output
            .concrete
            .animation
            .as_mut()
            .unwrap()
            .last_anim_tilt = value;
        Ok(())
    }
    fn draw_tilt_enabled(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_draw_animation_tilt()
            .map_err(AnimationOutputError::Cvar)
    }
    fn require_tilt_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(AnimationOutputError::Overlay)
    }
    fn deck_transform(&mut self) -> Result<Matrix, Self::Error> {
        self.guard(PhysOutComponent::SkateboardReckoning)?;
        Ok(self
            .output
            .concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .transform)
    }
    fn deck_acceleration(&mut self) -> Result<Vector, Self::Error> {
        self.guard(PhysOutComponent::Motion)?;
        Ok(self
            .output
            .concrete
            .motion
            .as_ref()
            .unwrap()
            .deck_acceleration)
    }
    fn sample_guard(&mut self) -> Result<PhysOutWriteGuard, Self::Error> {
        self.output
            .sample_write_guard()
            .map_err(AnimationOutputError::Access)
    }
    fn ground_transform(&mut self, guard: PhysOutWriteGuard) -> Result<Matrix, Self::Error> {
        self.captured_guard(&guard, PhysOutComponent::Ground)?;
        Ok(self
            .output
            .ground_transform
            .ok_or(AnimationOutputError::Access(
                PhysOutAccessError::NotConstructed,
            ))?
            .value)
    }
    fn store_deck_ground_acceleration(
        &mut self,
        guard: PhysOutWriteGuard,
        value: Vector,
    ) -> Result<(), Self::Error> {
        self.captured_guard(&guard, PhysOutComponent::Animation)?;
        self.output
            .concrete
            .animation
            .as_mut()
            .unwrap()
            .deck_ground_acceleration = value;
        Ok(())
    }
}

impl AnimationTurnSource for AnimationOutput<'_> {
    type Error = AnimationOutputError;
    type Scalar = Option<AttributeRuntimeData>;
    fn mirrored(&mut self) -> Result<bool, Self::Error> {
        self.guard(PhysOutComponent::Animation)?;
        Ok(self.output.concrete.animation.as_ref().unwrap().is_mirrored)
    }
    fn deck_speed(&mut self) -> Result<f32, Self::Error> {
        self.guard(PhysOutComponent::Motion)?;
        Ok(
            self.output.motion_scalars[PhysOutMotionScalar::DeckSpeed as usize]
                .ok_or(AnimationOutputError::Access(
                    PhysOutAccessError::NotConstructed,
                ))?
                .value,
        )
    }
    fn target_lean(&mut self) -> Result<f32, Self::Error> {
        self.guard(PhysOutComponent::SystemReckoning)?;
        Ok(self
            .output
            .concrete
            .system
            .as_ref()
            .unwrap()
            .target_lean_angle)
    }
    fn ground_turn(&mut self) -> Result<f32, Self::Error> {
        self.guard(PhysOutComponent::Ground)?;
        Ok(self.output.concrete.ground.as_ref().unwrap().turn)
    }
    fn wobble(&mut self) -> Result<f32, Self::Error> {
        self.guard(PhysOutComponent::Motion)?;
        Ok(self
            .output
            .concrete
            .motion
            .as_ref()
            .unwrap()
            .speed_wobble_effect)
    }
    fn carving(&mut self) -> Result<bool, Self::Error> {
        self.guard(PhysOutComponent::Animation)?;
        Ok(self.output.concrete.animation.as_ref().unwrap().is_carving)
    }
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        self.attributes
            .turn_scalar(key)
            .map_err(AnimationOutputError::Attribute)
    }
    fn capture_scalar(&mut self, key: u64) -> Result<Self::Scalar, Self::Error> {
        self.attributes
            .turn_capture_scalar(key)
            .map_err(AnimationOutputError::Attribute)
    }
    fn read_scalar(&mut self, value: &Self::Scalar) -> Result<f32, Self::Error> {
        AnimationAttributes::turn_read_scalar(value.as_ref())
            .map_err(AnimationOutputError::Attribute)
    }
    fn control(&mut self, filter: TurnFilter) -> Result<PhysicsControllerData, Self::Error> {
        self.attributes
            .turn_control(filter)
            .map_err(AnimationOutputError::Layout)
    }
    fn graph(&mut self, offset: usize, points: usize, value: f32) -> Result<f32, Self::Error> {
        match (offset, points) {
            (0x10 | 0x90 | 0xd0, 8) => self.attributes.turn_graph::<8>(offset, value),
            (0x120, 4) => self.attributes.turn_graph::<4>(offset, value),
            _ => return Err(AnimationOutputError::InvalidGraph { offset, points }),
        }
        .map_err(AnimationOutputError::Layout)
    }
    fn store(&mut self, field: TurnField, value: f32) -> Result<(), Self::Error> {
        self.guard(PhysOutComponent::Animation)?;
        let animation = self.output.concrete.animation.as_mut().unwrap();
        match field {
            TurnField::Turn => animation.animation_turn = value,
            TurnField::TargetLean => animation.animation_target_lean = value,
            TurnField::InputLean => animation.animation_input_lean = value,
            TurnField::Quickness => animation.animation_lean_quickness = value,
            TurnField::Direction => animation.animation_lean_direction = value,
            TurnField::Holding => animation.animation_holding = value,
            TurnField::CameraTurn => animation.cam_animation_turn = value,
            TurnField::CameraInputLean => animation.cam_animation_input_lean = value,
        }
        Ok(())
    }
    fn draw_carve_enabled(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_draw_animation_carve()
            .map_err(AnimationOutputError::Cvar)
    }
    fn require_carve_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(AnimationOutputError::Overlay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        attribute_runtime::tests::graph,
        physical_player_phys_out::{PhysOutOrigin, PhysOutValue, construct::PhysOutAllocation},
    };
    use std::sync::Arc;

    fn output() -> PhysicalPlayerPhysOut {
        let mut out = PhysicalPlayerPhysOut::new();
        out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        out
    }

    #[test]
    fn captured_ground_guard_is_not_replaced_by_a_later_latch_sample() {
        let mut out = output();
        let attrs = AnimationAttributes::bind(&Arc::new(graph(0, vec![])));
        let cvars = InputCvarReferences::new_static_initialized();
        let overlay = DebugOverlaySlot::default();
        let mut source = AnimationOutput {
            output: &mut out,
            attributes: &attrs,
            cvars: &cvars,
            overlay: &overlay,
        };
        let guard = source.sample_guard().unwrap();
        source.output.locked = Some(true);
        assert!(source.ground_transform(guard).is_ok());
        assert!(matches!(
            source.deck_transform(),
            Err(AnimationOutputError::Access(
                PhysOutAccessError::LockedForWrite
            ))
        ));
        let locked_guard = source.sample_guard().unwrap();
        source.output.locked = Some(false);
        assert!(matches!(
            source.ground_transform(locked_guard),
            Err(AnimationOutputError::Access(
                PhysOutAccessError::LockedForWrite
            ))
        ));
    }

    #[test]
    fn deck_acceleration_adapter_uses_actual_motion_and_ground_fields() {
        let mut out = output();
        let basis = [
            Vector {
                xyz: [1., 0., 0.],
                w: None,
            },
            Vector {
                xyz: [0., 1., 0.],
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
        ];
        out.concrete
            .skateboard_reckoning
            .as_mut()
            .unwrap()
            .transform = Matrix::from_columns(basis);
        out.ground_transform = Some(PhysOutValue {
            value: Matrix::from_columns(basis),
            origin: PhysOutOrigin::Publication,
        });
        out.concrete.motion.as_mut().unwrap().deck_acceleration = Vector {
            xyz: [5., 6., 7.],
            w: None,
        };
        let attrs = AnimationAttributes::bind(&Arc::new(graph(0, vec![])));
        let cvars = InputCvarReferences::new_static_initialized();
        let overlay = DebugOverlaySlot::default();
        let mut source = AnimationOutput {
            output: &mut out,
            attributes: &attrs,
            cvars: &cvars,
            overlay: &overlay,
        };
        assert_eq!(
            source.deck_transform().unwrap(),
            Matrix::from_columns(basis)
        );
        assert_eq!(
            source.deck_acceleration().unwrap(),
            Vector {
                xyz: [5., 6., 7.],
                w: None
            }
        );
        let ground_guard = source.sample_guard().unwrap();
        assert_eq!(
            source.ground_transform(ground_guard).unwrap(),
            Matrix::from_columns(basis)
        );
        let store_guard = source.sample_guard().unwrap();
        source
            .store_deck_ground_acceleration(store_guard, Vector::known([5., 0., 7., 38.]))
            .unwrap();
        assert_eq!(
            out.concrete
                .animation
                .as_ref()
                .unwrap()
                .deck_ground_acceleration,
            Vector::known([5., 0., 7., 38.])
        );
        // Both bump consumers read PhysOut+38 -> Animation+70. Deliberately
        // keep Motion+70 different, so selecting that same-offset field fails.
        assert_eq!(
            out.try_motion_bump_acceleration().unwrap(),
            Vector::known([5., 0., 7., 38.])
        );
        assert_ne!(
            out.try_motion_bump_acceleration().unwrap(),
            out.concrete.motion.as_ref().unwrap().deck_acceleration
        );
    }
}
