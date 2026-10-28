//! The existing SkaterAnim moving-object coefficient block, shared with the
//! MovingObject behaviour's instance. Native layout is 0x28 bytes (PDB).
use super::ActorAnimationConstruction;

#[derive(Clone, Copy, Debug)]
pub(crate) struct MovingObjectsCoefficients {
    pub(crate) last_full_angle: f32,
    pub(crate) last_angle: f32,
    pub(crate) last_speed: f32,
    pub(crate) last_strength: f32,
    pub(crate) last_rotation: f32,
    pub(crate) last_crouch: f32,
    pub(crate) blend_target_angle: f32,
    pub(crate) current_direction: u32,
    pub(crate) last_direction: u32,
    // Native +0x24 bit 7; the lower seven bits are unnamed in the PDB and
    // untouched by the constructor/reset. Retain their knownness on copies.
    pub(crate) flags_bits: u8,
    pub(crate) flags_known_mask: u8,
}

impl MovingObjectsCoefficients {
    /// SkaterAnim ctor82cc0a90 and MovingObject.CreateInstance82cb3948.
    pub(crate) const fn after_construction() -> Self {
        Self {
            last_full_angle: 0.0,
            last_angle: 0.0,
            last_speed: 0.0,
            last_strength: 0.0,
            last_rotation: 0.0,
            last_crouch: 0.0,
            blend_target_angle: 0.0,
            current_direction: 0,
            last_direction: 0,
            flags_bits: 0,
            flags_known_mask: 0x80,
        }
    }

    /// InitMovingObjects.Begin82ca39dc..3a10. Reset only the documented block,
    /// retaining unrelated flags and their knownness rather than resetting the
    /// entire SkaterAnim before-sprint owner.
    pub(crate) fn reset(&mut self) {
        let flags_bits = self.flags_bits & 0x7f;
        let flags_known_mask = self.flags_known_mask | 0x80;
        *self = Self {
            flags_bits,
            flags_known_mask,
            ..Self::after_construction()
        };
    }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// SetMirrorNextAnim82c8b868: two independent bits in the existing player
    /// flags, consumed by the existing Mirror tree builder on the next anim.
    pub(crate) fn set_mirror_next_anim(
        &mut self,
        mirror: bool,
        offboard: bool,
    ) -> Result<(), &'static str> {
        let player = self.player.as_mut().ok_or("SkaterAnim player fields")?;
        player.flags_bits = (player.flags_bits & !0x0060_0000)
            | (u32::from(mirror) << 22)
            | (u32::from(offboard) << 21);
        player.flags_known_mask |= 0x0060_0000;
        Ok(())
    }

    /// GetMovingObjectsCoefficients82cb7768: this component's retained block.
    pub(crate) fn moving_objects_coefficients(
        &self,
    ) -> Result<MovingObjectsCoefficients, &'static str> {
        Ok(self
            .before_sprint
            .as_ref()
            .ok_or("SkaterAnim moving-object coefficients")?
            .moving_objects)
    }

    /// SetMovingObjectsCoefficients82cb7770: copies the complete ten-word block.
    pub(crate) fn set_moving_objects_coefficients(
        &mut self,
        coefficients: MovingObjectsCoefficients,
    ) -> Result<(), &'static str> {
        self.before_sprint
            .as_mut()
            .ok_or("SkaterAnim moving-object coefficients")?
            .moving_objects = coefficients;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_clears_handled_direction_bit_but_preserves_other_flag_bits() {
        let mut coefficients = MovingObjectsCoefficients {
            last_full_angle: 2.0,
            last_angle: 0.7,
            last_speed: 0.4,
            last_strength: 0.5,
            last_rotation: 0.6,
            last_crouch: 0.8,
            blend_target_angle: 3.0,
            current_direction: 7,
            last_direction: 2,
            flags_bits: 0xdb,
            flags_known_mask: 0xbf,
        };
        coefficients.reset();
        assert_eq!(coefficients.flags_bits, 0x5b);
        assert_eq!(coefficients.flags_known_mask, 0xbf);
        for value in [
            coefficients.last_full_angle,
            coefficients.last_angle,
            coefficients.last_speed,
            coefficients.last_strength,
            coefficients.last_rotation,
            coefficients.last_crouch,
            coefficients.blend_target_angle,
        ] {
            assert_eq!(value.to_bits(), 0);
        }
        assert_eq!(
            (coefficients.current_direction, coefficients.last_direction),
            (0, 0)
        );
    }
}
