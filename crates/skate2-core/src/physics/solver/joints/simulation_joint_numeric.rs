//! Hardware-sensitive numeric seam shared by staged Joint Build recovery.
//!
//! Site tags preserve native callback order while keeping Xenon estimate and
//! reduction policy outside the source-shaped ordinary-finite value graph.

pub(crate) type SimulationJointVector = [f32; 4];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationJointSumSite {
    QuaternionA,
    QuaternionB,
    LimitFrameQuaternion,
    MotionUp,
    MotionRight,
    MotionAt,
    PredictedUp,
    PredictedRight,
    PredictedAt,
    AngularUp,
    AngularRight,
    AngularAt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationJointReciprocalSite {
    Cone,
    Hinge,
    TwistArc,
    FinalAngular,
    FinalLinear,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationJointReciprocalSqrtSite {
    Cone,
}

/// Backend for the twelve reductions and six lazy estimates in native
/// `rw::physics::JointJacobian::Build`.
///
/// A callback may retain its own effects when returning an error. Staged
/// builders retain that prefix, make no later callback, and return no value.
pub(crate) trait SimulationJointNumericBackend {
    type Error;

    fn sum3(
        &mut self,
        site: SimulationJointSumSite,
        left: [f32; 3],
        right: [f32; 3],
    ) -> Result<f32, Self::Error>;

    fn reciprocal_estimate(
        &mut self,
        site: SimulationJointReciprocalSite,
        input: SimulationJointVector,
    ) -> Result<SimulationJointVector, Self::Error>;

    fn reciprocal_sqrt_estimate(
        &mut self,
        site: SimulationJointReciprocalSqrtSite,
        input: SimulationJointVector,
    ) -> Result<SimulationJointVector, Self::Error>;
}
