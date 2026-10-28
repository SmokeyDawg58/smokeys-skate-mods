//! Buffered `GetNextLoc82a24c28` reuse of the same bounded SQT allocation.
use super::{ActorFaceError, AndaleSqt, BindPoseError, TreeCommands};
use std::{collections::BTreeMap, ops::Range};

#[derive(Clone, Default)]
pub(super) struct ReuseState {
    pub enabled: bool,
    pub live: BTreeMap<u16, LiveOutput>,
    /// Unconsumed fixed graph roots in operand order (including popped inputs).
    /// Each root retains its separate graph-node order for dispatch counting.
    pub fixed: Vec<FixedRoot>,
    pub ordinal: usize,
    pub high_water: usize,
    pub segments: Vec<Segment>,
    pub fetch_start: usize,
    pub operation_start: usize,
    pub external_start: usize,
    pub prior_segments: usize,
    /// Latest output-table relocation, separate from already encoded command inputs.
    pub last_remap: Vec<(u16, u16)>,
    pub previous_boundary: Option<usize>,
}
#[derive(Clone, Copy)]
pub(super) struct LiveOutput {
    order: usize,
    /// Original command field, distinct from the mutable output-table slot.
    source: u16,
}
#[derive(Clone, Copy)]
pub(super) struct FixedRoot {
    order: usize,
    id: u16,
    /// SetDataById opcode12 bit0 nodes and fixed reuse nodes. Other opcodes
    /// must produce dynamic outputs, including Mirror of a fixed input.
    modeled: bool,
}
#[derive(Clone)]
pub(crate) struct Segment {
    pub(crate) external_sqts: Range<usize>,
    pub(crate) fetches: Range<usize>,
    pub(crate) operations: Range<usize>,
    /// Native destination/source pointer pairs in graph-node order.
    pub(crate) copies: Vec<(u16, u16)>,
}
impl TreeCommands {
    pub(crate) fn enable_reuse(&mut self) {
        self.reuse.enabled = true;
    }
    pub(super) fn produced(&mut self, inputs: &[u16], output: u16) {
        for input in inputs {
            if input & super::fixed::FIXED_SQT_BIT == 0 {
                self.reuse.live.remove(input);
            } else if let Some(position) =
                self.reuse.fixed.iter().rposition(|root| root.id == *input)
            {
                // Operands leave the stack top first, so the newest root is consumed.
                self.reuse.fixed.remove(position);
            }
        }
        if output & super::fixed::FIXED_SQT_BIT == 0 {
            self.reuse.live.insert(
                output,
                LiveOutput {
                    order: self.reuse.ordinal,
                    source: output,
                },
            );
        } else {
            self.reuse.fixed.push(FixedRoot {
                order: self.reuse.ordinal,
                id: output,
                modeled: false,
            });
        }
        self.reuse.ordinal += 1;
    }
    /// Buffered `SetDataById82a24890` emits opcode12 with bit0 set and pushes a
    /// pid -1 graph node: a fixed root in node order, never a GetNextLoc slot.
    pub(super) fn produced_fixed(&mut self, id: u16) {
        self.reuse.fixed.push(FixedRoot {
            order: self.reuse.ordinal,
            id,
            modeled: true,
        });
        self.reuse.ordinal += 1;
    }
    pub(super) fn allocate_reusing(&mut self) -> Result<u16, BindPoseError> {
        // GetNextLoc checks the seven-reuse limit even before exhausting the
        // next segment's allocation (GetNextLoc82a24c28).
        if self.reuse.enabled && self.reuse.prior_segments + self.reuse.segments.len() >= 7 {
            return Err(BindPoseError::ReuseLimitReached);
        }
        let maximum = self
            .owner
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .max_dynamic_sqts;
        if usize::from(self.next_sqt) >= maximum && self.reuse.enabled {
            self.compact(maximum)?;
        }
        if usize::from(self.next_sqt) >= maximum || self.next_sqt > 0x0fff {
            return Err(BindPoseError::DynamicSqtCapacity {
                next: self.next_sqt,
                maximum,
            });
        }
        let result = self.next_sqt;
        self.next_sqt += 1;
        self.reuse.high_water = self.reuse.high_water.max(usize::from(self.next_sqt));
        Ok(result)
    }
    fn compact(&mut self, maximum: usize) -> Result<(), BindPoseError> {
        // Only SetDataById and fixed reuse nodes are recovered fixed root kinds.
        if self.reuse.fixed.iter().any(|root| !root.modeled) {
            return Err(BindPoseError::ReuseFixedRootsPending);
        }
        // Graph outputs stay live until their consuming command is inserted,
        // including operands already popped from the command stack.
        let mut live = self
            .reuse
            .live
            .iter()
            .map(|(&slot, &node)| (node.order, slot, node.source))
            .collect::<Vec<_>>();
        live.sort_unstable();
        self.next_sqt = 0;
        if live.len() >= maximum {
            return Err(BindPoseError::DynamicSqtCapacity { next: 0, maximum });
        }
        let remapped = live
            .iter()
            .enumerate()
            .map(|(destination, (_, current, _))| {
                u16::try_from(destination)
                    .map(|destination| (destination, *current))
                    .map_err(|_| BindPoseError::Allocation)
            })
            .collect::<Result<Vec<_>, _>>()?;
        // DispatchReuseBufferCmd82a2fd20..fe94 advances the destination for
        // every live root, but copies only nodes after mStartBuffSegmentId.
        // Its source is the original command output, not mOutputTable. The
        // cursor (r5, fe6c) also advances past fixed roots (fd90), while
        // GetNextLoc numbers readers over dynamic roots only.
        let copies = live
            .iter()
            .zip(&remapped)
            .filter(|((order, _, _), _)| {
                self.reuse
                    .previous_boundary
                    .is_none_or(|last| *order > last)
            })
            .map(|((order, _, source), (destination, _))| {
                let fixed_before = self
                    .reuse
                    .fixed
                    .iter()
                    .filter(|root| root.order < *order)
                    .count();
                u16::try_from(usize::from(*destination) + fixed_before)
                    .map(|cursor| (cursor, *source))
                    .map_err(|_| BindPoseError::Allocation)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let remap = |value: &mut u16| {
            if let Some((destination, _)) = remapped.iter().find(|(_, source)| source == value) {
                *value = *destination;
            }
        };
        for value in &mut self.stack {
            remap(value);
        }
        self.reuse.live.clear();
        for ((order, _, source), (destination, _)) in live.into_iter().zip(&remapped) {
            self.reuse
                .live
                .insert(*destination, LiveOutput { order, source });
        }
        self.next_sqt = u16::try_from(remapped.len()).map_err(|_| BindPoseError::Allocation)?;
        self.reuse.last_remap = remapped;
        // GetNextLoc24f1c..24fa4 /252ac..25330 inserts a reuse producer for
        // ONLY the previous stack top. Its encoded output is that root's
        // remapped reference; the requested new allocation is next_sqt.
        if let Some(&top) = self.stack.last() {
        if top & super::fixed::FIXED_SQT_BIT == 0 {
            self.reuse.live.insert(
                top,
                LiveOutput {
                    order: self.reuse.ordinal,
                    source: top,
                },
            );
        } else {
            // A fixed top keeps its id and bit0 (24f60/24fa4): the reuse node is
            // a fixed root replacing the top node. Stack fixed roots precede the
            // operands the current command already popped.
            let on_stack = self
                .stack
                .iter()
                .filter(|id| **id & super::fixed::FIXED_SQT_BIT != 0)
                .count();
            let index = on_stack
                .checked_sub(1)
                .filter(|index| {
                    self.reuse
                        .fixed
                        .get(*index)
                        .is_some_and(|root| root.id == top)
                })
                .ok_or(BindPoseError::ReuseFixedRootsPending)?;
            // Replace this stack occurrence in place. Its new node order is
            // retained separately; moving it after popped operands would make
            // a later same-ID consume incorrectly remove the reuse root.
            self.reuse.fixed[index] = FixedRoot {
                order: self.reuse.ordinal,
                id: top,
                modeled: true,
            };
        }
        }
        self.reuse.previous_boundary = Some(self.reuse.ordinal);
        self.reuse.ordinal += 1;
        self.reuse.segments.push(Segment {
            external_sqts: self.reuse.external_start..self.external_sqts.len(),
            fetches: self.reuse.fetch_start..self.fetches.len(),
            operations: self.reuse.operation_start..self.operations.len(),
            copies,
        });
        self.reuse.fetch_start = self.fetches.len();
        self.reuse.operation_start = self.operations.len();
        self.reuse.external_start = self.external_sqts.len();
        self.owner
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .commands
            .push(super::Command::ReuseBuffer {
                segment: self.reuse.prior_segments + self.reuse.segments.len() - 1,
            });
        Ok(())
    }
    pub(crate) fn segments(&self) -> Vec<Segment> {
        let mut segments = self.reuse.segments.clone();
        segments.push(Segment {
            external_sqts: self.reuse.external_start..self.external_sqts.len(),
            fetches: self.reuse.fetch_start..self.fetches.len(),
            operations: self.reuse.operation_start..self.operations.len(),
            copies: vec![],
        });
        segments
    }
    pub(crate) fn has_reuse(&self) -> bool {
        !self.reuse.segments.is_empty()
    }
    pub(crate) fn allocation_end(&self) -> usize { usize::from(self.next_sqt) }
    pub(crate) fn reuse_count(&self) -> usize { self.reuse.prior_segments + self.reuse.segments.len() }
}
/// `ACSreuseBuffer_job82a22ac0`: ordered copies, not a snapshot permutation.
pub(crate) fn execute(
    copies: &[(u16, u16)],
    sqts: &mut [Option<Vec<AndaleSqt>>],
    bones: usize,
) -> Result<(), BindPoseError> {
    for &(destination, source) in copies {
        if destination == source {
            continue;
        }
        let source = sqts
            .get(usize::from(source))
            .and_then(Option::as_ref)
            .ok_or(BindPoseError::MissingBindPose)?;
        if source.len() != bones {
            return Err(BindPoseError::InvalidHierarchy);
        }
        let copy = source.clone();
        *sqts
            .get_mut(usize::from(destination))
            .ok_or(BindPoseError::InvalidHierarchy)? = Some(copy);
    }
    Ok(())
}
#[cfg(test)]
#[path = "actor_animation_reuse_buffer_tests.rs"]
mod tests;
