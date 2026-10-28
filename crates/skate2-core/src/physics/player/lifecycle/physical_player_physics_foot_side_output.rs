//! Actual `CalcFootSideOfDeck`82e25b28 output captures and publication.
use super::super::{
    PhysOutComponent, PhysOutOrigin, PhysOutSystemVector, PhysOutValue, PhysicalPlayerPhysOut,
    SkeletonRootVector,
};
use super::prior_input::PriorInputReadError;
use crate::reckoning_numeric::ReckoningNumeric;

#[derive(Debug)]
pub(crate) enum FootSideError<E> {
    ForeignOutput,
    MissingPhysics,
    Output(PriorInputReadError),
    Numeric(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for FootSideError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignOutput => f.write_str("foot-side continuation belongs to another output"),
            Self::MissingPhysics => {
                f.write_str("foot-side calculation requires published Physics conditioner")
            }
            Self::Output(e) => write!(f, "foot-side output access: {e:?}"),
            Self::Numeric(e) => write!(f, "foot-side dot product: {e:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    fn foot_side_guard(&self, component: PhysOutComponent) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    pub(crate) fn try_calc_foot_side<N: ReckoningNumeric>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), FootSideError<N::Error>> {
        use FootSideError as E;
        self.foot_side_guard(PhysOutComponent::SkateboardReckoning)
            .map_err(E::Output)?; //5b4c
        let board = self.concrete.skateboard_reckoning.as_ref().unwrap(); //5b70
        self.foot_side_guard(PhysOutComponent::Skeleton)
            .map_err(E::Output)?; //5b74
        let left_owner = self.concrete.skeleton.as_ref().unwrap(); //5b98
        self.foot_side_guard(PhysOutComponent::Skeleton)
            .map_err(E::Output)?; //5b9c
        let right_owner = self.concrete.skeleton.as_ref().unwrap(); //5bbc
        let axis = board.transform.columns()[0]; //5bc8
        let left = left_owner.skeleton_foot_left_look; //5bdc
        let right = right_owner.skeleton_foot_right_look; //5be0
        let sum = SkeletonRootVector {
            xyz: std::array::from_fn(|i| right.xyz[i] + left.xyz[i]),
            w: right.w.zip(left.w).map(|(r, l)| r + l),
        }; //5be8
        let dot = numeric.dot_vector(axis, sum).map_err(E::Numeric)?; //5bfc
        let selected = if dot > 0.0 {
            board.transform.columns()[0] //5c1c/5c40: fresh board-axis load
        } else {
            axis.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000)) //5c30..38
        };
        self.foot_side_guard(PhysOutComponent::SystemReckoning)
            .map_err(E::Output)?; //5c44
        // Existing Right slot is native SystemReckoning.mFootSideOfDeck+70.
        self.system_vectors[PhysOutSystemVector::Right as usize] = Some(PhysOutValue {
            value: selected,
            origin: PhysOutOrigin::Publication,
        }); //5c6c
        Ok(())
    }
}
