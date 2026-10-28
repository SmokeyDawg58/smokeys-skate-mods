use super::*;
use std::convert::Infallible;

#[test]
fn model_selection_filters_only_while_queues_have_work_and_keeps_rng_calls() {
    let mut owner = RecipeMan::new();
    for key in [1, 2] {
        owner
            .mappings
            .append(
                77,
                super::super::RecipeMappingEntry {
                    model_id: key,
                    required: false,
                },
            )
            .unwrap();
    }
    let mut services = Services::default();
    owner.load_model(model(1), &mut services).unwrap();
    owner.loaded(0, 100, &mut services).unwrap();
    let mut calls = 0;
    let mut random = || {
        calls += 1;
        Ok::<_, Infallible>(1)
    };
    let mut filename = |key| Ok::<_, Infallible>(model(key).filename);
    owner.queues.secondary.push(2);
    assert_eq!(
        owner
            .select_sub_model(77, &mut filename, &mut random)
            .unwrap(),
        1
    );
    owner.queues.secondary.clear();
    assert_eq!(
        owner
            .select_sub_model(77, &mut filename, &mut random)
            .unwrap(),
        2
    );
    assert_eq!(
        owner
            .select_sub_model(88, &mut filename, &mut random)
            .unwrap(),
        88
    );
    assert_eq!(calls, 2);
    assert_eq!(owner.asset_id_for_filename("PED_1"), 100);
    assert_eq!(owner.asset_id_for_filename("PED_2"), NULL_ASSET_ID);
}
#[derive(Debug, PartialEq, Eq)]
enum Event {
    Load(u32, String),
    SimReg(u64, u64),
    PresReg(u64, u64),
    SimRemove(u64),
    PresRemove(u64),
    Unload(u64),
}
#[derive(Default)]
struct Services {
    next: u32,
    events: Vec<Event>,
}
fn model(key: u64) -> RecipeModel {
    RecipeModel {
        key,
        filename: format!("ped_{key}"),
        category: ModelCategory::RegularPed,
    }
}
impl RecipeServices for Services {
    type Error = Infallible;
    fn model(&mut self, key: u64) -> Result<RecipeModel, Infallible> {
        Ok(model(key))
    }
    fn load(&mut self, category: u32, filename: &str) -> Result<u32, Infallible> {
        self.events.push(Event::Load(category, filename.to_owned()));
        let id = self.next;
        self.next += 1;
        Ok(id)
    }
    fn unload(&mut self, id: u64) -> Result<u32, Infallible> {
        self.events.push(Event::Unload(id));
        let rid = self.next;
        self.next += 1;
        Ok(rid)
    }
    fn register_simulation(&mut self, a: u64, k: u64) -> Result<(), Infallible> {
        self.events.push(Event::SimReg(a, k));
        Ok(())
    }
    fn register_presentation(&mut self, a: u64, k: u64) -> Result<(), Infallible> {
        self.events.push(Event::PresReg(a, k));
        Ok(())
    }
    fn deregister_simulation(&mut self, a: u64) -> Result<(), Infallible> {
        self.events.push(Event::SimRemove(a));
        Ok(())
    }
    fn deregister_presentation(&mut self, a: u64) -> Result<(), Infallible> {
        self.events.push(Event::PresRemove(a));
        Ok(())
    }
}

#[test]
fn release_during_load_completes_registration_then_deregisters_on_update() {
    let mut man = RecipeMan::new();
    let mut s = Services::default();
    man.initialization = RecipeManagerInitializationState::from_raw(1, 0);
    assert!(man.load_key(3, &mut s).unwrap());
    man.release("PED_3", &mut s).unwrap();
    man.loaded(0, 17, &mut s).unwrap();
    assert_eq!(
        man.records[&filename_hash("ped_3")].status,
        RecipeStatus::LoadedToUnload
    );
    man.update(&mut s).unwrap();
    assert_eq!(
        s.events,
        [
            Event::Load(1, "ped_3".into()),
            Event::SimReg(17, 3),
            Event::PresReg(17, 3),
            Event::SimRemove(17),
            Event::PresRemove(17),
            Event::Unload(17)
        ]
    );
    assert_eq!(man.active_count(ModelCategory::RegularPed), 1);
    assert!(!man.load_key(3, &mut s).unwrap());
    man.unloaded::<Infallible>(1).unwrap();
    assert_eq!(man.active_count(ModelCategory::RegularPed), 0);
}

#[test]
fn reacquire_cancels_pending_unload_and_failed_asset_never_registers() {
    let mut man = RecipeMan::new();
    let mut s = Services::default();
    man.load_key(8, &mut s).unwrap();
    man.release("ped_8", &mut s).unwrap();
    man.load_key(8, &mut s).unwrap();
    man.loaded(0, NULL_ASSET_ID, &mut s).unwrap();
    assert_eq!(
        man.records[&filename_hash("ped_8")].status,
        RecipeStatus::Loaded
    );
    man.release("ped_8", &mut s).unwrap();
    assert!(man.records.is_empty());
    assert_eq!(s.events, [Event::Load(1, "ped_8".into())]);
}

#[test]
fn required_queue_is_lifo_and_inflight_load_freezes_120_frame_delay() {
    let mut man = RecipeMan::new();
    let mut s = Services::default();
    man.initialization = RecipeManagerInitializationState::from_raw(1, 0);
    man.queues.required = vec![1, 2];
    man.queues.initial = vec![3];
    man.update(&mut s).unwrap();
    assert_eq!(s.events, [Event::Load(1, "ped_2".into())]);
    for _ in 0..200 {
        man.update(&mut s).unwrap();
    }
    assert_eq!(man.initialization.frame_delay_raw, 120);
    man.loaded(0, 22, &mut s).unwrap();
    for _ in 0..120 {
        man.update(&mut s).unwrap();
    }
    assert_eq!(man.initialization.frame_delay_raw, 0);
    assert_eq!(man.queues.required, [1]);
    man.update(&mut s).unwrap();
    assert_eq!(s.events.last(), Some(&Event::Load(1, "ped_1".into())));
}

#[test]
fn category_limit_includes_waiting_unloads_and_category_mapping_is_distinct() {
    let mut man = RecipeMan::new();
    let mut s = Services::default();
    let mut m = model(1);
    m.category = ModelCategory::Prop;
    assert!(man.load_model(m.clone(), &mut s).unwrap());
    man.loaded(0, 42, &mut s).unwrap();
    man.release(&m.filename, &mut s).unwrap();
    m.key = 2;
    m.filename = "second_prop".into();
    assert!(!man.load_model(m.clone(), &mut s).unwrap());
    man.unloaded::<Infallible>(1).unwrap();
    assert!(man.load_model(m, &mut s).unwrap());
    assert_eq!(ModelCategory::Vehicle.character_category(), 2);
    assert_eq!(ModelCategory::Marquee.character_category(), 0);
    assert_eq!(ModelCategory::DynamicObject.character_category(), 1);
}
