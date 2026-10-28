use super::*;

// Deliberately no Clone implementation: live pointer binding must not copy M.
#[derive(Debug, PartialEq)]
struct Material(u32);
struct Backend {
    authority: u32,
    next: usize,
    live: std::collections::BTreeSet<usize>,
}
impl DynamicAssemblySimulationBackend for Backend {
    type Inertia = ();
    type Material = Material;
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
                material: Material(ordinal),
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
    };
    session.attach(&mut backend, 4).unwrap();
    (session, backend)
}

#[test]
fn four_parts_retain_one_nonclone_owner_and_leave_definition_and_other_parts_unchanged() {
    let (mut session, backend) = fixture();
    let material = Arc::new(Material(90));
    let weak = Arc::downgrade(&material);
    for ordinal in 0..4 {
        session
            .try_set_part_material(&backend, ordinal, material.clone())
            .unwrap();
    }
    for ordinal in 0..4 {
        let retained = session.part_attachment.parts[ordinal]
            .material_override
            .as_ref()
            .unwrap();
        assert!(Arc::ptr_eq(retained, &material));
        assert_eq!(
            session.current_part_material(ordinal).unwrap(),
            &Material(90)
        );
    }
    drop(material);
    assert!(weak.upgrade().is_some());
    for ordinal in 0..7 {
        assert_eq!(
            session.template.part_payloads()[ordinal].material.0,
            u32::try_from(ordinal).unwrap()
        );
    }
    for ordinal in 4..7 {
        assert_eq!(
            session.current_part_material(ordinal).unwrap().0,
            u32::try_from(ordinal).unwrap()
        );
    }
    assert_eq!(
        session
            .part_attachment
            .material_revision
            .load(Ordering::Acquire),
        4
    );
}

#[test]
fn later_part_failure_preserves_successful_pointer_store_prefix() {
    let (mut session, backend) = fixture();
    session.part_attachment.parts[2].body = None; // private fault injection
    let selected = Arc::new(Material(55));
    let result = (0..4)
        .try_for_each(|ordinal| session.try_set_part_material(&backend, ordinal, selected.clone()));
    assert_eq!(
        result,
        Err(DynamicAssemblyPartMaterialError::MissingBody { ordinal: 2 })
    );
    assert_eq!(session.current_part_material(0).unwrap(), &Material(55));
    assert_eq!(session.current_part_material(1).unwrap(), &Material(55));
    assert_eq!(session.current_part_material(2).unwrap(), &Material(2));
    assert_eq!(session.current_part_material(3).unwrap(), &Material(3));
    assert_eq!(
        session
            .part_attachment
            .material_revision
            .load(Ordering::Acquire),
        2
    );
}

#[test]
fn detach_and_reattach_preserve_live_part_material_binding() {
    let (mut session, mut backend) = fixture();
    let material = Arc::new(Material(77));
    session
        .try_set_part_material(&backend, 0, material.clone())
        .unwrap();
    session.detach(&mut backend).unwrap();
    assert_eq!(
        session.try_set_part_material(&backend, 0, Arc::new(Material(99))),
        Err(DynamicAssemblyPartMaterialError::NotAttached)
    );
    assert_eq!(session.current_part_material(0).unwrap(), &Material(77));
    session.attach(&mut backend, 4).unwrap();
    assert!(Arc::ptr_eq(
        session.part_attachment.parts[0]
            .material_override
            .as_ref()
            .unwrap(),
        &material
    ));
    assert_eq!(
        session
            .part_attachment
            .material_revision
            .load(Ordering::Acquire),
        1
    );
}

#[test]
fn authority_range_and_revision_failures_do_not_replace_material() {
    let (mut session, backend) = fixture();
    let foreign = Backend {
        authority: 18,
        next: 0,
        live: std::collections::BTreeSet::default(),
    };
    assert_eq!(
        session.try_set_part_material(&foreign, 0, Arc::new(Material(90))),
        Err(DynamicAssemblyPartMaterialError::BackendAuthorityMismatch)
    );
    assert_eq!(
        session.try_set_part_material(&backend, 7, Arc::new(Material(90))),
        Err(DynamicAssemblyPartMaterialError::PartOutOfRange {
            ordinal: 7,
            count: 7
        })
    );
    session
        .part_attachment
        .material_revision
        .store(u64::MAX, Ordering::Release);
    assert_eq!(
        session.try_set_part_material(&backend, 0, Arc::new(Material(90))),
        Err(DynamicAssemblyPartMaterialError::RevisionExhausted)
    );
    assert_eq!(session.current_part_material(0).unwrap(), &Material(0));
    assert_eq!(
        session
            .part_attachment
            .material_revision
            .load(Ordering::Acquire),
        u64::MAX
    );
}

#[test]
fn equal_values_from_different_owners_remain_distinct_and_repeat_store_invalidates() {
    let (mut session, backend) = fixture();
    let first = Arc::new(Material(4));
    let second = Arc::new(Material(4));
    session
        .try_set_part_material(&backend, 0, first.clone())
        .unwrap();
    session
        .try_set_part_material(&backend, 0, second.clone())
        .unwrap();
    let current = session.part_attachment.parts[0]
        .material_override
        .as_ref()
        .unwrap();
    assert!(!Arc::ptr_eq(current, &first));
    assert!(Arc::ptr_eq(current, &second));
    session.try_set_part_material(&backend, 0, second).unwrap();
    assert_eq!(
        session
            .part_attachment
            .material_revision
            .load(Ordering::Acquire),
        3
    );
}
