//! HandPlantManager::UpdateIK82db7d30. Hand/shoulder inputs are the actual
//! raw animation transforms, before this frame's IK adjustment.
use super::{
    HandPlantManager,
    manager::{DT, E},
    position::{graph4, scalar},
};
use crate::{
    input_cvar_runtime::{
        InputCvarReferences,
        handplant::{HandPlantBool as B, HandPlantFloat as F},
    },
    physical_player_grind_manager::{
        math::{GrindNum, V4, madd, neg1, sub},
        view::error,
    },
};
impl HandPlantManager {
    pub(crate) fn reset_ik(&mut self) {
        self.ik.blend = 0.;
        self.ik.distance = [-1.; 4];
        self.ik.flags &= 0x3fff_ffff;
    }
    pub(crate) fn right_hand(&self, natural_stance: bool) -> bool {
        ((self.animation.flags >> 29) ^ u32::from(natural_stance)) & 1 != 0
    }
    pub(crate) fn prepare_ik(&mut self, reset: bool) -> Result<bool, E> {
        if reset {
            self.reset_ik();
        }
        let active =
            self.estimated_time_to_apex[0] > neg1(scalar(&self.attributes, 0x9c02_b9e6_998d_1321)?);
        if !active {
            self.ik.flags |= 0x8000_0000;
        }
        Ok(active)
    }
    pub(crate) fn update_ik(
        &mut self,
        hand: V4,
        shoulder: V4,
        active: bool,
        refs: &InputCvarReferences,
        n: &mut GrindNum<'_>,
    ) -> Result<Option<(V4, f32, f32)>, E> {
        let plant = self.animation.position;
        let mut target = hand;
        let small = f32::from_bits(0x3780_0000);
        if self.ik.flags & 0x8000_0000 == 0 {
            let axis = sub(shoulder, plant);
            if n.length(axis)? > small {
                let axis = n.normalize_unsafe(axis)?;
                let along = n.dot(sub(hand, plant), axis)?;
                if 0. >= along {
                    self.ik.flags |= 0x8000_0000;
                } else {
                    let projected = madd(axis, along, plant);
                    if self.ik.flags & 0x4000_0000 != 0 {
                        target = projected;
                    } else {
                        let delta = sub(hand, projected);
                        let limit = graph4(&self.attributes, 0, along)?;
                        if n.length(delta)? > limit {
                            target = madd(n.normalize_unsafe(delta)?, limit, projected);
                        }
                    }
                }
                let delta = sub(target, plant);
                let length = n.length(delta)?;
                if 0. > self.ik.distance[0] {
                    self.ik.distance = [length; 4];
                }
                let step = scalar(&self.attributes, 0x7c8b_87f7_4352_21c4)?;
                self.ik.distance = self.ik.distance.map(|v| length.min(v - step));
                if small > self.ik.distance[0] || small > length {
                    self.ik.flags |= 0x8000_0000;
                } else {
                    target = madd(n.normalize_unsafe(delta)?, self.ik.distance[0], plant);
                }
                if self.ik.flags & 0xc000_0000 == 0 {
                    let along = n.dot(sub(target, plant), axis)?;
                    let projected = madd(axis, along, plant);
                    if 0.02 > n.length(sub(target, projected))? {
                        self.ik.flags |= 0x4000_0000;
                    }
                }
            }
        }
        if self.ik.flags & 0x8000_0000 != 0 {
            target = plant;
        }
        let maximum = refs.try_handplant_float(F::MaxArmLength).map_err(error)?;
        let delta = sub(target, shoulder);
        if n.length(delta)? > maximum {
            target = madd(n.normalize_unsafe(delta)?, maximum, shoulder);
        }
        let (dt, key) = if active {
            (DT, 0x01eb_855a_0acb_d84c)
        } else {
            (-DT, 0xe089_5c18_9d77_ce16)
        };
        let blend = self.ik.blend + dt / scalar(&self.attributes, key)?;
        // Original scalar fsel clamp preserves unordered operands.
        let blend = if -blend >= 0. { 0. } else { blend };
        self.ik.blend = if 1. - blend >= 0. { blend } else { 1. };
        Ok(
            (refs.try_handplant_bool(B::UseIk).map_err(error)? && self.ik.blend > 0.).then_some((
                target,
                maximum,
                self.ik.blend,
            )),
        )
    }
}
