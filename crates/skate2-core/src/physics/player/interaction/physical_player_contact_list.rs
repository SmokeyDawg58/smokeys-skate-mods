//! `Island::CreateContactSpyList` 0x827ba710..0x827bab14 and `GetCollisions`
//! 0x827ba6b0. Actual component handles and retained construction owners supply
//! Body/Entity equality; only the sealed current solved-spy frame supplies data.
//! Skater Body callbacks are no-ops. LivingWorld collision bodies dispatch
//! their first accepted entity contact during World.Update3; the selected car
//! caller is in living_world_vehicle_frame.rs, after physical output.
//!
//! XYZ semantics omit the native velocity intrusive-link W and packed spy W.
//! Ordinary host subtraction/multiplication is not a Xenon exceptional-bit claim.
//! Exclusive world access replaces the Island lock. Checked allocation and
//! current-owner/frame gates are host lifetime policy; partial creation preserves
//! its cleared/appended prefix and cannot replay until the next actual frame.
use super::{
    SimulationAssemblyConstraintSession,
    contact_feedback::SimulationContactFeedbackProgress,
    physical_island::{SimulationIslandBodyHandle, SimulationPhysicalBodyError},
};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    simulation_contact_jacobian::contact_spy::{
        SimulationContactSpy, SimulationContactSpyError, SimulationContactSpyFrame,
    },
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_hook::SkateboardHookDefinition,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{fmt, sync::Arc};

const CONTACTS_PER_BODY: usize = 16;

#[cfg(test)]
#[path = "physical_player_contact_list_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContactEndpoint {
    A,
    B,
}

/// Actual source cap precedes all reads/calculation; a failed make retains the
/// old vector. Host reservation occurs before pure reads, not as a batch commit.
fn try_push_capped<T, E>(
    contacts: &mut Vec<T>,
    allocation_error: impl FnOnce() -> E,
    make: impl FnOnce() -> Result<T, E>,
) -> Result<bool, E> {
    if contacts.len() >= CONTACTS_PER_BODY {
        return Ok(false);
    }
    contacts.try_reserve(1).map_err(|_| allocation_error())?;
    let contact = make()?;
    contacts.push(contact);
    Ok(true)
}

fn contact_relative_velocity<E>(
    is_body_a: bool,
    mut read: impl FnMut(ContactEndpoint) -> Result<[f32; 3], E>,
) -> Result<[f32; 3], E> {
    let (other, this) = if is_body_a {
        (ContactEndpoint::B, ContactEndpoint::A)
    } else {
        (ContactEndpoint::A, ContactEndpoint::B)
    };
    let other = read(other)?;
    let this = read(this)?;
    Ok(std::array::from_fn(|lane| this[lane] - other[lane]))
}

fn contact_normal(right: [f32; 3], is_body_a: bool) -> [f32; 3] {
    let sign = if is_body_a { 1.0 } else { -1.0 };
    right.map(|value| value * sign)
}

/// The original source Spy reference is a retained actual frame plus dense
/// index. No constructor accepts arbitrary contact records or numeric identities.
#[derive(Clone)]
pub(crate) struct PhysicalContactInfo {
    spy_frame: Arc<SimulationContactSpyFrame>,
    spy_index: usize,
    normal: [f32; 3],
    position: [f32; 3],
    relative_velocity: [f32; 3],
    part: u32,
    surface: u32,
    this_body: SimulationIslandBodyHandle,
    other_body: Option<SimulationIslandBodyHandle>,
    self_colliding: bool,
    is_body_a: bool,
}
impl PhysicalContactInfo {
    pub(crate) fn spy(&self) -> &SimulationContactSpy {
        &self.spy_frame.records()[self.spy_index]
    }
    pub(crate) fn spy_frame(&self) -> &Arc<SimulationContactSpyFrame> {
        &self.spy_frame
    }
    pub(crate) fn spy_index(&self) -> usize {
        self.spy_index
    }
    pub(crate) fn normal(&self) -> &[f32; 3] {
        &self.normal
    }
    pub(crate) fn position(&self) -> &[f32; 3] {
        &self.position
    }
    pub(crate) fn relative_velocity(&self) -> &[f32; 3] {
        &self.relative_velocity
    }
    pub(crate) fn part(&self) -> u32 {
        self.part
    }
    pub(crate) fn surface(&self) -> u32 {
        self.surface
    }
    pub(crate) fn this_body(&self) -> &SimulationIslandBodyHandle {
        &self.this_body
    }
    pub(crate) fn other_body(&self) -> Option<&SimulationIslandBodyHandle> {
        self.other_body.as_ref()
    }
    pub(crate) fn self_colliding(&self) -> bool {
        self.self_colliding
    }
    pub(crate) fn is_body_a(&self) -> bool {
        self.is_body_a
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationPhysicalContactListProgress {
    NotStarted,
    Building {
        completed_spies: usize,
        appended_contacts: usize,
    },
    Complete {
        completed_spies: usize,
        appended_contacts: usize,
    },
}

#[derive(Debug)]
pub(crate) enum SimulationPhysicalContactListError {
    MissingSpyFrame,
    FeedbackIncomplete,
    AlreadyAttempted,
    CreationIncomplete,
    FrameMismatch,
    Allocation,
    Owner(SimulationPhysicalBodyError),
    Spy(SimulationContactSpyError),
    Body(SimulationRigidBodyPoolError),
}
impl fmt::Display for SimulationPhysicalContactListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSpyFrame => {
                f.write_str("physical contact lists require an actual solved-spy frame")
            }
            Self::FeedbackIncomplete => {
                f.write_str("physical contact lists require completed reset/freezing")
            }
            Self::AlreadyAttempted => {
                f.write_str("physical contact list creation already attempted this frame")
            }
            Self::CreationIncomplete => {
                f.write_str("physical contact list creation has not completed")
            }
            Self::FrameMismatch => {
                f.write_str("physical contact list retains a different solved-spy frame")
            }
            Self::Allocation => f.write_str("physical contact list host allocation failed"),
            Self::Owner(error) => write!(f, "physical contact list owner: {error}"),
            Self::Spy(error) => write!(f, "physical contact list spy: {error}"),
            Self::Body(error) => write!(f, "physical contact list body: {error}"),
        }
    }
}
impl std::error::Error for SimulationPhysicalContactListError {}

struct BodyContacts {
    body: SimulationIslandBodyHandle,
    contacts: Vec<PhysicalContactInfo>,
}

/// Source Body->vector entries. Dense host entry lookup is not a claim about
/// native pointer-map iteration order. Clear retains entry/vector capacities.
pub(crate) struct SimulationPhysicalContactLists {
    entries: Vec<BodyContacts>,
    progress: SimulationPhysicalContactListProgress,
    frame: Option<Arc<SimulationContactSpyFrame>>,
}
impl SimulationPhysicalContactLists {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
            progress: SimulationPhysicalContactListProgress::NotStarted,
            frame: None,
        }
    }
    /// Actual `StartFrame` invalidates success WITHOUT clearing native vectors
    /// early. Their contents/Arc storage survive until the next actual Create.
    pub(crate) fn begin_frame(&mut self) {
        self.progress = SimulationPhysicalContactListProgress::NotStarted;
        self.frame = None;
    }
    /// Source cleanup erases this Body's vector BEFORE `SkaterPool` removal.
    pub(crate) fn remove_body(&mut self, body: &SimulationSkaterAssemblyHandle) {
        self.entries.retain(|entry| !entry.body.same_as(body));
    }
    pub(crate) fn remove_island_body(&mut self, body: &SimulationIslandBodyHandle) {
        self.entries.retain(|entry| !entry.body.same_body(body));
    }
    pub(crate) fn progress(&self) -> SimulationPhysicalContactListProgress {
        self.progress
    }
    /// Completion of this exact export, including legitimate zero-spy frames.
    /// Current manager/pool validation remains the enclosing world's job.
    pub(crate) fn try_validate_completed_frame(
        &self,
        frame: &Arc<SimulationContactSpyFrame>,
    ) -> Result<(), SimulationPhysicalContactListError> {
        if !matches!(
            self.progress,
            SimulationPhysicalContactListProgress::Complete { .. }
        ) {
            return Err(SimulationPhysicalContactListError::CreationIncomplete);
        }
        if self
            .frame
            .as_ref()
            .is_none_or(|retained| !Arc::ptr_eq(retained, frame))
        {
            return Err(SimulationPhysicalContactListError::FrameMismatch);
        }
        Ok(())
    }
    pub(crate) fn retained_entry_count(&self) -> usize {
        self.entries.len()
    }
    /// Borrow for the exclusive world response after its checked `GetCollisions`
    /// and exact frame validation; this is not a standalone freshness proof.
    pub(crate) fn retained_contacts(
        &self,
        body: &SimulationSkaterAssemblyHandle,
    ) -> &[PhysicalContactInfo] {
        self.entry_index(body)
            .map_or(&[], |index| self.entries[index].contacts.as_slice())
    }
    fn entry_index(&self, body: &SimulationSkaterAssemblyHandle) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.body.same_as(body))
    }
    fn island_entry_index(&self, body: &SimulationIslandBodyHandle) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.body.same_body(body))
    }
    fn try_entry(
        &mut self,
        body: &SimulationIslandBodyHandle,
    ) -> Result<usize, SimulationPhysicalContactListError> {
        if let Some(index) = self.island_entry_index(body) {
            return Ok(index);
        }
        self.entries
            .try_reserve(1)
            .map_err(|_| SimulationPhysicalContactListError::Allocation)?;
        self.entries.push(BodyContacts {
            body: body.clone(),
            contacts: Vec::new(),
        });
        Ok(self.entries.len() - 1)
    }
}

// Resolved target references remain real component handles, never dummy Body
// tokens. Owner reads are deliberately separate and follow BOTH target lookups.
struct TargetAddress {
    member: SimulationIslandBodyHandle,
    part: u32,
}
#[derive(Clone, Copy)]
struct ContactSide<'a> {
    this: &'a TargetAddress,
    other: Option<&'a TargetAddress>,
    self_colliding: bool,
    is_body_a: bool,
}

impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    /// Actual `Step_Solver3` continuation, AFTER reset/current-config/freezing.
    /// Host preflight leaves previous vectors intact on failure; once clearing
    /// begins, later errors retain the actual completed prefix without replay.
    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered target/owner sampling and separately retained A/B failure prefixes"
    )]
    pub(crate) fn try_create_physical_contact_lists(
        &mut self,
    ) -> Result<(), SimulationPhysicalContactListError> {
        use SimulationPhysicalContactListError as Error;
        use SimulationPhysicalContactListProgress as Progress;
        let frame = self.contact_spies.as_ref().ok_or(Error::MissingSpyFrame)?;
        frame
            .try_validate_current(&self.contacts, self.backend.collision_body_pool())
            .map_err(Error::Spy)?;
        if !matches!(
            self.contact_feedback_progress,
            SimulationContactFeedbackProgress::Complete { .. }
        ) {
            return Err(Error::FeedbackIncomplete);
        }
        if self.physical_contacts.progress != Progress::NotStarted {
            return Err(Error::AlreadyAttempted);
        }
        let frame = Arc::clone(frame);
        self.physical_contacts.progress = Progress::Building {
            completed_spies: 0,
            appended_contacts: 0,
        };
        self.physical_contacts.frame = Some(Arc::clone(&frame));
        for entry in &mut self.physical_contacts.entries {
            entry.contacts.clear();
        }
        let mut appended_contacts = 0;
        for (index, spy) in frame.records().iter().enumerate() {
            let target_a = self
                .island_body_target(spy.body_a())
                .map_err(Error::Owner)?
                .map(|target| TargetAddress {
                    member: target.member,
                    part: target.part,
                });
            let target_b = self
                .island_body_target(spy.body_b())
                .map_err(Error::Owner)?
                .map(|target| TargetAddress {
                    member: target.member,
                    part: target.part,
                });
            let same_body = match (&target_a, &target_b) {
                (Some(a), Some(b)) => a.member.same_body(&b.member),
                _ => false,
            };
            // Source loads both Entity owners even when same_body is already
            // true. A found but unsupported owner MUST error, not become null.
            let owner_a = target_a
                .as_ref()
                .map(|target| self.island_body_owner_for_contacts(&target.member))
                .transpose()
                .map_err(Error::Owner)?;
            let owner_b = target_b
                .as_ref()
                .map(|target| self.island_body_owner_for_contacts(&target.member))
                .transpose()
                .map_err(Error::Owner)?;
            let same_owner = match (&owner_a, &owner_b) {
                (Some(a), Some(b)) => a.same_owner(b),
                (None, None) => true,
                _ => false,
            };
            let self_colliding = same_body || same_owner;
            if let Some(this) = &target_a {
                if self.try_append_physical_contact_side(
                    &frame,
                    index,
                    ContactSide {
                        this,
                        other: target_b.as_ref(),
                        self_colliding,
                        is_body_a: true,
                    },
                )? {
                    appended_contacts += 1;
                }
                self.physical_contacts.progress = Progress::Building {
                    completed_spies: index,
                    appended_contacts,
                };
            }
            if let Some(this) = &target_b {
                if self.try_append_physical_contact_side(
                    &frame,
                    index,
                    ContactSide {
                        this,
                        other: target_a.as_ref(),
                        self_colliding,
                        is_body_a: false,
                    },
                )? {
                    appended_contacts += 1;
                }
            }
            self.physical_contacts.progress = Progress::Building {
                completed_spies: index + 1,
                appended_contacts,
            };
        }
        self.physical_contacts.progress = Progress::Complete {
            completed_spies: frame.records().len(),
            appended_contacts,
        };
        Ok(())
    }

    fn try_append_physical_contact_side(
        &mut self,
        frame: &Arc<SimulationContactSpyFrame>,
        index: usize,
        side: ContactSide<'_>,
    ) -> Result<bool, SimulationPhysicalContactListError> {
        use SimulationPhysicalContactListError as Error;
        let entry = self.physical_contacts.try_entry(&side.this.member)?;
        let pool = self.backend.collision_body_pool();
        try_push_capped(
            &mut self.physical_contacts.entries[entry].contacts,
            || Error::Allocation,
            || {
                let spy = &frame.records()[index];
                // Independent reads for EACH side, even when A already appended or
                // both registered targets share the same Body/Entity owner.
                let relative_velocity = contact_relative_velocity(side.is_body_a, |endpoint| {
                    let body = match endpoint {
                        ContactEndpoint::A => spy.body_a(),
                        ContactEndpoint::B => spy.body_b(),
                    };
                    pool.try_contact_spy_linear_velocity(body)
                        .map_err(Error::Body)
                })?;
                Ok(PhysicalContactInfo {
                    spy_frame: Arc::clone(frame),
                    spy_index: index,
                    normal: contact_normal(spy.frame()[0], side.is_body_a),
                    position: spy.frame()[3],
                    relative_velocity,
                    part: side.this.part,
                    surface: if side.is_body_a {
                        spy.tag() & 0xffff
                    } else {
                        spy.tag() >> 16
                    },
                    this_body: side.this.member.clone(),
                    other_body: side.other.map(|target| target.member.clone()),
                    self_colliding: side.self_colliding,
                    is_body_a: side.is_body_a,
                })
            },
        )
    }

    /// Native `GetCollisions` simply obtains/creates the SAME vector, without
    /// draining it. Empty vectors are available before any frame. A nonempty
    /// host view additionally requires completed creation and current sealed
    /// provenance; partial/stale retained entries have a separate diagnostic API.
    pub(crate) fn try_get_physical_collisions(
        &mut self,
        body: &SimulationSkaterAssemblyHandle,
    ) -> Result<&[PhysicalContactInfo], SimulationPhysicalContactListError> {
        self.try_get_island_collisions(&SimulationIslandBodyHandle::Skater(body.clone()))
    }

    pub(crate) fn try_get_island_collisions(
        &mut self,
        body: &SimulationIslandBodyHandle,
    ) -> Result<&[PhysicalContactInfo], SimulationPhysicalContactListError> {
        use SimulationPhysicalContactListError as Error;
        self.island_body_owner_for_contacts(body)
            .map_err(Error::Owner)?;
        let index = self.physical_contacts.try_entry(body)?;
        if !self.physical_contacts.entries[index].contacts.is_empty() {
            if !matches!(
                self.physical_contacts.progress,
                SimulationPhysicalContactListProgress::Complete { .. }
            ) {
                return Err(Error::CreationIncomplete);
            }
            let frame = self.contact_spies.as_ref().ok_or(Error::MissingSpyFrame)?;
            if self
                .physical_contacts
                .frame
                .as_ref()
                .is_none_or(|retained| !Arc::ptr_eq(retained, frame))
            {
                return Err(Error::FrameMismatch);
            }
            frame
                .try_validate_current(&self.contacts, self.backend.collision_body_pool())
                .map_err(Error::Spy)?;
            for contact in &self.physical_contacts.entries[index].contacts {
                if !Arc::ptr_eq(&contact.spy_frame, frame) {
                    return Err(Error::FrameMismatch);
                }
            }
        }
        Ok(&self.physical_contacts.entries[index].contacts)
    }

    pub(crate) fn physical_contact_list_progress(&self) -> SimulationPhysicalContactListProgress {
        self.physical_contacts.progress()
    }

    /// Raw retained-entry diagnostics only; neither certifies current contacts.
    pub(crate) fn physical_contact_list_entry_count(&self) -> usize {
        self.physical_contacts.retained_entry_count()
    }
    pub(crate) fn physical_contact_list_has_entry(
        &self,
        body: &SimulationSkaterAssemblyHandle,
    ) -> bool {
        self.physical_contacts.entry_index(body).is_some()
    }

    /// Retained partial/old values only, NOT a completed/current contact view.
    /// Arcs keep actual solved data alive; inert member handles can be stale.
    /// No entry is created, and no Body callback or response is dispatched.
    pub(crate) fn physical_contact_list_prefix(
        &self,
        body: &SimulationSkaterAssemblyHandle,
    ) -> Result<&[PhysicalContactInfo], SimulationPhysicalContactListError> {
        self.physical_body_owner_for_contacts(body)
            .map_err(SimulationPhysicalContactListError::Owner)?;
        Ok(self
            .physical_contacts
            .entry_index(body)
            .map_or(&[], |index| {
                self.physical_contacts.entries[index].contacts.as_slice()
            }))
    }
}
