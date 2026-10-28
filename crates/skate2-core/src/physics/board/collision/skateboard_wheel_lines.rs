//! `SkateboardBody::StartWheelLineTest` 0x82b24530 and End 0x82b24700.
//! One retained batch spans physics; individual output readiness and failed
//! source prefixes survive. Actual Part/query ownership belongs to the bridge.
use crate::{
    collision_line_collection::CollisionLineInput, skeleton_root_transforms::SkeletonRootVector,
};

#[cfg(test)]
#[path = "skateboard_wheel_lines_tests.rs"]
mod tests;

const MAX_HEIGHT: f32 = f32::from_bits(0x3e4c_cccd);
const DECK_FATNESS: f32 = f32::from_bits(0x3e19_999a);
const UP: SkeletonRootVector = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);

pub(crate) struct SkateboardWheelLineHit {
    pub(crate) position: SkeletonRootVector,
    pub(crate) normal: SkeletonRootVector,
    pub(crate) parameter: f32,
    pub(crate) surface: u32,
}

/// Source calls remain separate. In particular, acquisition is retained before
/// Begin, and no implicit release runs after a later error.
pub(crate) trait SkateboardWheelLineServices {
    type Batch;
    type Error;
    fn acquire(&mut self) -> Result<Self::Batch, Self::Error>;
    fn begin(&mut self, batch: &Self::Batch) -> Result<(), Self::Error>;
    fn part_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error>;
    fn add(&mut self, batch: &Self::Batch, input: CollisionLineInput) -> Result<bool, Self::Error>;
    fn end(&mut self, batch: &Self::Batch) -> Result<(), Self::Error>;
    fn sync(&mut self, batch: &Self::Batch) -> Result<(), Self::Error>;
    fn result(
        &mut self,
        batch: &Self::Batch,
        index: u32,
    ) -> Result<Option<SkateboardWheelLineHit>, Self::Error>;
    fn release(&mut self, batch: &Self::Batch) -> Result<(), Self::Error>;
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardWheelLineError<E> {
    BatchAlreadyRetained,
    NoBatch,
    DeckTestRejected,
    Service(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for SkateboardWheelLineError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "board wheel line: {self:?}")
    }
}

#[derive(Debug, Default)]
pub(crate) struct SkateboardDeckLineState {
    pub(crate) start: Option<SkeletonRootVector>,
    pub(crate) position: Option<SkeletonRootVector>,
    pub(crate) normal: Option<SkeletonRootVector>,
    pub(crate) surface: Option<u32>,
    pub(crate) valid: Option<bool>,
}

pub(crate) struct SkateboardWheelLines<B> {
    #[cfg(test)]
    pub(crate) inputs_for_test: Vec<CollisionLineInput>,
    batch: Option<B>,
    pub(crate) normals: [Option<SkeletonRootVector>; 4],
    pub(crate) distances: [Option<f32>; 4],
    pub(crate) height: Option<f32>,
    pub(crate) deck: SkateboardDeckLineState,
}
impl<B> SkateboardWheelLines<B> {
    #[cfg(test)]
    pub(crate) fn batch_for_test(&self) -> Option<&B> {
        self.batch.as_ref()
    }
    /// Only the constructor's null batch (0x82b382fc) is claimed initialized.
    /// The selected query output fields become ready at their actual stores.
    pub(crate) fn new() -> Self {
        Self {
            #[cfg(test)]
            inputs_for_test: Vec::new(),
            batch: None,
            normals: [None; 4],
            distances: [None; 4],
            height: None,
            deck: SkateboardDeckLineState::default(),
        }
    }
    pub(crate) fn has_batch(&self) -> bool {
        self.batch.is_some()
    }

    /// `CollisionInfo::Reset82b23630`, called by the body constructor82b382dc.
    /// This initializes collision output, without claiming a completed query.
    pub(crate) fn reset_constructor_collision_output(&mut self) {
        self.height = Some(0.0); // CollisionInfo+318, store82b23710.
        for wheel in 0..4 {
            self.reset_collision_wheel_output(wheel);
        }
    }

    /// CollisionInfo::Reset 0x82b23740..48 interleaves normal and distance
    /// for one wheel; the caller writes that wheel's validity byte next.
    pub(crate) fn reset_collision_wheel_output(&mut self, wheel: usize) {
        self.normals[wheel] = Some(UP);
        self.distances[wheel] = Some(0.0);
    }

    /// The caller already copied the actual Reckoning up vector by value.
    pub(crate) fn try_start<S: SkateboardWheelLineServices<Batch = B>>(
        &mut self,
        up: SkeletonRootVector,
        services: &mut S,
    ) -> Result<(), SkateboardWheelLineError<S::Error>> {
        use SkateboardWheelLineError as E;
        if self.batch.is_some() {
            return Err(E::BatchAlreadyRetained);
        }
        self.batch = Some(services.acquire().map_err(E::Service)?);
        #[cfg(test)]
        self.inputs_for_test.clear();
        let batch = self.batch.as_ref().expect("just acquired");
        services.begin(batch).map_err(E::Service)?;
        for part in 0..4 {
            let position = services.part_position(part).map_err(E::Service)?;
            // Native separate multiply and subtract, including the fourth lane.
            let end = SkeletonRootVector {
                xyz: std::array::from_fn(|lane| position.xyz[lane] - up.xyz[lane] * MAX_HEIGHT),
                w: position
                    .w
                    .zip(up.w)
                    .map(|(position, up)| position - up * MAX_HEIGHT),
            };
            #[cfg(test)]
            self.inputs_for_test.push(CollisionLineInput {
                start: position,
                end,
                fatness: 0.0,
            });
            let _ = services
                .add(
                    batch,
                    CollisionLineInput {
                        start: position,
                        end,
                        fatness: 0.0,
                    },
                )
                .map_err(E::Service)?;
        }
        let position = services.part_position(6).map_err(E::Service)?;
        self.deck.valid = Some(false);
        self.deck.start = Some(position);
        let down = [0.0, f32::from_bits(0xc2c8_0000), 0.0];
        let end = SkeletonRootVector {
            xyz: std::array::from_fn(|lane| position.xyz[lane] + down[lane]),
            w: position.w.map(|lane| lane + 0.0),
        };
        #[cfg(test)]
        self.inputs_for_test.push(CollisionLineInput {
            start: position,
            end,
            fatness: DECK_FATNESS,
        });
        if !services
            .add(
                batch,
                CollisionLineInput {
                    start: position,
                    end,
                    fatness: DECK_FATNESS,
                },
            )
            .map_err(E::Service)?
        {
            return Err(E::DeckTestRejected);
        }
        services.end(batch).map_err(E::Service)
    }

    pub(crate) fn try_end<S: SkateboardWheelLineServices<Batch = B>>(
        &mut self,
        services: &mut S,
    ) -> Result<(), SkateboardWheelLineError<S::Error>> {
        use SkateboardWheelLineError as E;
        let batch = self.batch.as_ref().ok_or(E::NoBatch)?;
        services.sync(batch).map_err(E::Service)?;
        self.height = Some(MAX_HEIGHT);
        for index in 0..4_u32 {
            let slot = index as usize;
            self.normals[slot] = Some(UP);
            self.distances[slot] = Some(0.0);
            if let Some(hit) = services.result(batch, index).map_err(E::Service)? {
                let distance = hit.parameter * MAX_HEIGHT;
                let old = self.height.expect("written after Sync");
                // Native fsel uses subtraction >=0; unordered selects distance.
                self.height = Some(if distance - old >= 0.0 { old } else { distance });
                self.normals[slot] = Some(hit.normal);
                self.distances[slot] = Some(distance);
            }
        }
        if let Some(hit) = services.result(batch, 4).map_err(E::Service)? {
            self.deck.valid = Some(true);
            self.deck.position = Some(hit.position);
            self.deck.normal = Some(hit.normal);
            self.deck.surface = Some(hit.surface);
        } else {
            self.deck.valid = Some(false);
        }
        services.release(batch).map_err(E::Service)?;
        self.batch = None;
        Ok(())
    }
}
