use super::*;
use crate::dynamic_assembly_session::*;
struct Backend {
    authority: u32,
    next: usize,
    live: std::collections::BTreeSet<usize>,
    states: [u32; 7],
    cooldowns: [u32; 7],
    log: Vec<(&'static str, usize)>,
    fail: Option<(&'static str, usize)>,
}
impl DynamicAssemblySimulationBackend for Backend {
    type Inertia = ();
    type Material = u32;
    type JointFrames = ();
    type JointLimits = ();
    type DriveFrames = ();
    type DriveDynamics = ();
    type Authority = u32;
    type RigidBodyHandle = usize;
    type JointHandle = ();
    type DriveHandle = ();
    fn authority(&self) -> u32 {
        self.authority
    }
    fn add_rigid_body(&mut self, _: &[f32; 16], (): &(), _: u32) -> Option<usize> {
        let result = self.next;
        self.next += 1;
        assert!(self.live.insert(result));
        Some(result)
    }
    fn rigid_body_pose(&self, body: &usize) -> [f32; 16] {
        assert!(self.live.contains(body));
        IDENTITY
    }
    fn remove_rigid_body(&mut self, body: &usize) {
        assert!(self.live.remove(body));
    }
    fn add_joint(&mut self, _: &usize, _: &usize, (): &(), (): &()) -> Option<()> {
        panic!("no joints")
    }
    fn remove_joint(&mut self, (): &()) {
        panic!("no joints")
    }
    fn add_drive(&mut self, _: &usize, _: &usize, (): &(), (): &()) -> Option<()> {
        panic!("no drives")
    }
    fn remove_drive(&mut self, (): &()) {
        panic!("no drives")
    }
}
const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];
fn fixture() -> (DynamicAssemblySession<Backend, ()>, Backend) {
    let attachment = DynamicAssemblyPartAttachmentTemplate::try_from_parts(
        (0..7)
            .map(|_| DynamicAssemblyPartAttachmentMetadata {
                inverse_body_transform: IDENTITY,
                has_inverse_body_transform: 0,
                is_static_body: 0,
            })
            .collect(),
        (0_u32..7)
            .map(|ordinal| DynamicAssemblyPartPayload {
                inertia: (),
                material: ordinal,
            })
            .collect(),
    )
    .unwrap();
    let template = DynamicAssemblyRuntimeTemplate::try_from_owned_parts(
        Arc::new(()),
        Arc::new(attachment),
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    let mut session =
        DynamicAssemblySession::try_new(Arc::new(template), vec![IDENTITY; 7]).unwrap();
    let mut backend = Backend {
        authority: 17,
        next: 0,
        live: std::collections::BTreeSet::default(),
        states: [1, 2, 4, 10, 4, 4, 4],
        cooldowns: [30; 7],
        log: Vec::new(),
        fail: None,
    };
    session.attach(&mut backend, 4).unwrap();
    (session, backend)
}

impl Backend {
    fn visit(&mut self, stage: &'static str, body: usize) -> Result<(), &'static str> {
        self.log.push((stage, body));
        if self.fail == Some((stage, body)) {
            Err(stage)
        } else {
            Ok(())
        }
    }
}
impl DynamicAssemblyActivationBackend for Backend {
    type ActivationError = &'static str;
    fn try_activation_state(&mut self, body: &usize) -> Result<u32, Self::ActivationError> {
        self.visit("state", *body)?;
        Ok(self.states[*body])
    }
    fn try_activate_frozen(&mut self, body: &usize) -> Result<(), Self::ActivationError> {
        self.visit("activate", *body)?;
        self.states[*body] = (self.states[*body] & 8) | 4;
        self.cooldowns[*body] = 29;
        Ok(())
    }
    fn try_clear_cooldown(&mut self, body: &usize) -> Result<(), Self::ActivationError> {
        self.visit("clear", *body)?;
        self.cooldowns[*body] = 0;
        Ok(())
    }
}
#[test]
fn ascending_loop_conditional_activation_then_unconditional_clear() {
    let (session, mut backend) = fixture();
    session.try_activate(&mut backend).unwrap();
    assert_eq!(
        &backend.log[..10],
        &[
            ("state", 0),
            ("clear", 0),
            ("state", 1),
            ("activate", 1),
            ("clear", 1),
            ("state", 2),
            ("clear", 2),
            ("state", 3),
            ("activate", 3),
            ("clear", 3)
        ]
    );
    assert_eq!(backend.states, [1, 4, 4, 12, 4, 4, 4]);
    assert_eq!(backend.cooldowns, [0; 7]);
}
#[test]
fn clear_failure_keeps_just_activated_body_and_prior_parts_without_later_reads() {
    let (session, mut backend) = fixture();
    backend.fail = Some(("clear", 1));
    assert_eq!(
        session.try_activate(&mut backend),
        Err(DynamicAssemblyActivationError::Backend {
            ordinal: 1,
            source: "clear"
        })
    );
    assert_eq!(
        backend.log,
        vec![
            ("state", 0),
            ("clear", 0),
            ("state", 1),
            ("activate", 1),
            ("clear", 1)
        ]
    );
    assert_eq!(backend.states[1], 4);
    assert_eq!(backend.cooldowns[..3], [0, 29, 30]);
}
#[test]
fn later_missing_body_is_not_eagerly_preflighted() {
    let (mut session, mut backend) = fixture();
    session.part_attachment.parts[2].body = None;
    assert_eq!(
        session.try_activate(&mut backend),
        Err(DynamicAssemblyActivationError::MissingBody { ordinal: 2 })
    );
    assert_eq!(backend.cooldowns[..3], [0, 0, 30]);
    assert_eq!(backend.log.last(), Some(&("clear", 1)));
}
#[test]
fn detached_and_foreign_backend_refuse_before_source_reads() {
    let (mut session, mut backend) = fixture();
    backend.authority += 1;
    assert_eq!(
        session.try_activate(&mut backend),
        Err(DynamicAssemblyActivationError::ForeignBackend)
    );
    session.attached_authority = None;
    assert_eq!(
        session.try_activate(&mut backend),
        Err(DynamicAssemblyActivationError::NotAttached)
    );
    assert!(backend.log.is_empty());
}
