//! Original live handplant cVar targets, appended without duplicating522.
use super::{InputCvarError, InputCvarReferences};
#[derive(Clone, Copy)]
pub(crate) enum HandPlantFloat {
    ApexUpBlend,
    GroundNormalY,
    MaxArmLength,
    HalfBoxX,
    HalfBoxY,
    HalfBoxZ,
    MinDotWithX,
    UpVectorBlend,
    MinTimeToGround,
    ControlFrames,
    TrajectoryStartDistance,
    AngleDelta,
    LandingOffset,
    MinDistanceToLanding,
    SurfaceRadius,
    UpVectorMaxDelta,
}
#[derive(Clone, Copy)]
pub(crate) enum HandPlantBool {
    StateDebug,
    ManagerDebug,
    UseIk,
    PositionDebug,
    DrawPrimitives,
    TrajectoryDebug,
}
impl InputCvarReferences {
    pub(crate) fn try_handplant_float(&self, value: HandPlantFloat) -> Result<f32, InputCvarError> {
        use HandPlantFloat::*;
        self.try_current_float(match value {
            ApexUpBlend => 937,
            GroundNormalY => 938,
            MaxArmLength => 940,
            HalfBoxX => 942,
            HalfBoxY => 943,
            HalfBoxZ => 944,
            MinDotWithX => 945,
            UpVectorBlend => 947,
            MinTimeToGround => 948,
            ControlFrames => 949,
            TrajectoryStartDistance => 950,
            AngleDelta => 951,
            LandingOffset => 952,
            MinDistanceToLanding => 953,
            SurfaceRadius => 954,
            UpVectorMaxDelta => 956,
        })
    }
    pub(crate) fn try_handplant_bool(&self, value: HandPlantBool) -> Result<bool, InputCvarError> {
        use HandPlantBool::*;
        self.try_current_boolean(match value {
            StateDebug => 936,
            ManagerDebug => 522,
            UseIk => 939,
            PositionDebug => 941,
            DrawPrimitives => 946,
            TrajectoryDebug => 955,
        })
    }
}
