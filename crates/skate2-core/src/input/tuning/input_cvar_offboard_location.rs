//! Source-default OffboardLocationTest live references507..516.
use super::{InputCvarError, InputCvarReferences};
use crate::physical_player_remaining_constructor_owners::location_runtime::LocationTuning;
impl LocationTuning for InputCvarReferences {
    type Error = InputCvarError;
    fn test_radius(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(507)
    }
    fn line_count(&mut self) -> Result<i32, Self::Error> {
        self.try_current_integer(508)
    }
    fn start_height(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(509)
    }
    fn line_length(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(510)
    }
    fn minimum_normal_dot(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(511)
    }
    fn minimum_y_normal(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(512)
    }
    fn turn_speed(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(513)
    }
    fn trajectory_end_length(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(514)
    }
    fn trajectory_start_height(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(515)
    }
    fn trajectory_end_height(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(516)
    }
    fn step_normal_y_threshold(&mut self) -> Result<f32, Self::Error> {
        self.try_step_normal_y_threshold()
    }
}
