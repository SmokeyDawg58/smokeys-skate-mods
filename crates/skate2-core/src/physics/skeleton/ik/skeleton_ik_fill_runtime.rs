//! Unregistered aggregate child: borrows the existing attributes, geometry and histories.
use super::SkeletonIk;
use crate::{
    attribute_runtime::AttributeRuntimeError,
    reckoning_numeric::ReckoningNumeric,
    skeleton_ik_state::fill_phys_out::{
        SkeletonIkFillError, SkeletonIkFillOutput, SkeletonIkFillServices,
    },
};
impl SkeletonIk {
    #[expect(
        clippy::type_complexity,
        reason = "preserve independent source, attribute, output and numeric failures"
    )]
    pub(crate) fn try_fill_phys_out<
        S: SkeletonIkFillServices,
        O: SkeletonIkFillOutput,
        N: ReckoningNumeric,
    >(
        &mut self,
        services: &mut S,
        output: &mut O,
        numeric: &mut N,
    ) -> Result<(), SkeletonIkFillError<S::Error, AttributeRuntimeError, O::Error, N::Error>> {
        self.state.try_fill_phys_out(
            &self.geometry,
            &mut self.attributes,
            services,
            output,
            numeric,
        )
    }
}
