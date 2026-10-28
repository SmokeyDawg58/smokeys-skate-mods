//! Vehicle.CheckEntitiesAgainstVision82d245e0. Consumes the SAME RAIN lists;
//! their per-frame producer and the vehicle pre-state caller remain required.
use super::Vehicle;
use crate::{
    attribute_runtime::AttributeRuntimeError,
    living_world_grind_services::vehicle_motion::{
        VehicleCharacteristics,
        vision::geometry::{self, VisionGeometryError},
    },
    living_world_rain::RainMan,
    reckoning_numeric::{ReckoningNumeric, try_length},
};

#[derive(Debug)]
pub(crate) enum PerceptionError<E> {
    Attribute(AttributeRuntimeError),
    Numeric(E),
    Geometry(VisionGeometryError<E>),
    MissingVisionArea,
}

impl Vehicle {
    pub(crate) fn check_entities_against_vision<N: ReckoningNumeric>(
        &mut self,
        rain: &RainMan,
        numeric: &mut N,
    ) -> Result<(), PerceptionError<N::Error>> {
        use PerceptionError as E;
        self.entity_in_vision = false;
        let mut nearest = [f32::MAX; 4];
        // Entity points FIRST, then skater points. Equal-distance ties keep
        // the first point. Source retains old nearest fields if no point wins.
        for (points, speed_key, avoid) in [
            (
                &rain.entity_points,
                0xe939_890e_1f14_dc0b,
                self.route.avoid_npcs,
            ),
            (
                &rain.skater_points,
                0x6367_f305_4cef_b534,
                self.route.avoid_skaters,
            ),
        ] {
            for point in points {
                if !point.compulsory_stop {
                    let speed = self.motion.speed * f32::from_bits(0x4066_6666);
                    let threshold = self.driver.scalar(speed_key).map_err(E::Attribute)?;
                    // Raw bgt: unordered follows the flag/WillRunRedLight arm.
                    if !(speed > threshold)
                        && (!avoid
                            || self
                                .motion
                                .will_run_red_light(&self.driver)
                                .map_err(E::Attribute)?)
                    {
                        continue;
                    }
                }
                let half_length = self.length() * 0.5;
                // Raw core+80 = TARGET translation, not actual transform+40.
                let delta: [f32; 4] =
                    std::array::from_fn(|i| point.position[i] - self.core.target_transform[3][i]);
                let distance = try_length(delta, numeric).map_err(E::Numeric)?;
                let gap: [f32; 4] = point.radius.map(|radius| (distance - radius) - half_length);
                let limit = self.motion.minimum_entity_distance + self.motion.speed;
                if !gap.iter().all(|&gap| limit > gap) {
                    continue;
                }
                let quad = self.vision.area.as_ref().ok_or(E::MissingVisionArea)?;
                let cylinder_hit = point.radius.iter().all(|&r| r > 0.)
                    && geometry::cylinder_in_region(point.position, point.radius[0], quad, numeric)
                        .map_err(E::Geometry)?;
                if cylinder_hit
                    || geometry::point_in_quad(point.position, quad, numeric).map_err(E::Numeric)?
                {
                    self.entity_in_vision = true;
                    if (0..4).all(|i| nearest[i] > gap[i]) {
                        nearest = gap;
                        self.nearest_vision_position = point.position;
                        self.nearest_vision_radius = point.radius;
                        self.nearest_compulsory_stop = point.compulsory_stop;
                        self.nearest_pedestrian =
                            (point.pedestrian_id != -1).then_some(point.pedestrian_id as u32);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::factory::tests::vehicle_fixture;
    use super::*;
    use crate::{
        attribute_runtime::tests::graph, living_world_rain::RainPoint,
        reckoning_numeric::HostReckoningNumeric,
    };
    use std::sync::Arc;

    #[test]
    fn perception_uses_target_keeps_first_equal_point_and_stale_inactive_payload() {
        let mut vehicle = vehicle_fixture();
        vehicle.core.transform[3] = [1000., 0., 1000., 0.];
        vehicle.motion.minimum_entity_distance = 10.;
        vehicle.vision.area = Some([
            [-1., 0., 0., 0.],
            [1., 0., 0., 0.],
            [-1., 0., 2., 0.],
            [1., 0., 2., 0.],
        ]);
        let mut rain = RainMan::construct(&Arc::new(graph(1, vec![])));
        let point = RainPoint {
            position: [0., 0., 3., 0.],
            radius: [0.; 4],
            compulsory_stop: true,
            pedestrian_id: 17,
        };
        rain.entity_points.push(point);
        rain.skater_points.push(RainPoint {
            pedestrian_id: -1,
            ..point
        });
        vehicle
            .check_entities_against_vision(&rain, &mut HostReckoningNumeric)
            .unwrap();
        assert!(vehicle.entity_in_vision);
        assert_eq!(vehicle.nearest_pedestrian, Some(17));
        rain.entity_points.clear();
        rain.skater_points.clear();
        vehicle
            .check_entities_against_vision(&rain, &mut HostReckoningNumeric)
            .unwrap();
        assert!(!vehicle.entity_in_vision);
        assert_eq!(vehicle.nearest_pedestrian, Some(17));
        assert_eq!(vehicle.nearest_vision_position, point.position);
    }
}
