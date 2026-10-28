//! `Skeleton::UpdateRootTransformsWipeout` 0x82b3b4c8 on retained roots.
use super::{SkeletonRootTransforms, SkeletonRootVector};
pub(crate) trait WipeoutRootInput {
    type Error;
    fn blend_camera_root(&mut self) -> Result<bool, Self::Error>;
    fn state_timer(&mut self) -> Result<f32, Self::Error>;
    fn height_offset(&mut self) -> Result<f32, Self::Error>;
    fn centre_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}
impl SkeletonRootTransforms {
    pub(crate) fn try_update_wipeout<I: WipeoutRootInput>(
        &mut self,
        centre: SkeletonRootVector,
        input: &mut I,
    ) -> Result<(), I::Error> {
        // UpdateWipeout82b5ea54 publishes this before the first root cvar read.
        self.update_heading = false;
        let position = if input.blend_camera_root()? {
            let dt = f32::from_bits(0x3c88_8889);
            let time = input.state_timer()? + dt;
            let height = input.height_offset()?;
            let old = self.animation_to_world.columns()[3];
            let velocity = input.centre_velocity()?;
            let lower = if -time >= 0.0 { 0.0 } else { time };
            let weight = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
            let predicted = velocity.zip(old, |v, p| v.mul_add(dt, p));
            let desired = centre.zip(
                SkeletonRootVector::known([0.0, height, 0.0, 0.0]),
                |v, offset| v + offset,
            );
            let weighted_previous = predicted.map(|v| v * (1.0 - weight));
            desired.zip(weighted_previous, |v, p| v.mul_add(weight, p))
        } else {
            centre
        };
        self.animation_to_world.set_column(3, position);
        // Native basis transpose; translation Z product, then Y and X FMA.
        self.world_to_animation = self.animation_to_world.inverse();
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Input {
        blend: bool,
        timer: f32,
        reads: Vec<&'static str>,
    }
    impl WipeoutRootInput for Input {
        type Error = &'static str;
        fn blend_camera_root(&mut self) -> Result<bool, Self::Error> {
            self.reads.push("blend");
            Ok(self.blend)
        }
        fn state_timer(&mut self) -> Result<f32, Self::Error> {
            self.reads.push("timer");
            Ok(self.timer)
        }
        fn height_offset(&mut self) -> Result<f32, Self::Error> {
            self.reads.push("height");
            Ok(-0.8)
        }
        fn centre_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
            self.reads.push("velocity");
            Ok(SkeletonRootVector::known([0.0; 4]))
        }
    }
    #[test]
    fn disabled_camera_copies_full_com_without_reading_blend_inputs() {
        let mut roots = SkeletonRootTransforms::new();
        let centre = SkeletonRootVector {
            xyz: [1.0, 2.0, 3.0],
            w: None,
        };
        let mut input = Input {
            blend: false,
            timer: f32::NAN,
            reads: vec![],
        };
        roots.try_update_wipeout(centre, &mut input).unwrap();
        assert_eq!(input.reads, ["blend"]);
        assert!(!roots.update_heading);
        assert_eq!(
            roots.animation_to_world.columns()[3].xyz.map(f32::to_bits),
            centre.xyz.map(f32::to_bits)
        );
        assert_eq!(roots.animation_to_world.columns()[3].w, None);
    }
    #[test]
    fn unordered_timer_fsel_selects_complete_com_blend() {
        let mut roots = SkeletonRootTransforms::new();
        let centre = SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0]);
        let mut input = Input {
            blend: true,
            timer: f32::NAN,
            reads: vec![],
        };
        roots.try_update_wipeout(centre, &mut input).unwrap();
        assert_eq!(input.reads, ["blend", "timer", "height", "velocity"]);
        assert_eq!(
            roots.animation_to_world.columns()[3].xyz.map(f32::to_bits),
            [
                1.0_f32.to_bits(),
                (2.0_f32 - 0.8).to_bits(),
                3.0_f32.to_bits()
            ]
        );
        assert_eq!(
            roots.animation_to_world.columns()[3].w.map(f32::to_bits),
            Some(4.0_f32.to_bits())
        );
    }
}
