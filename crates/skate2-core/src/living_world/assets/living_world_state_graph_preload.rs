//! Recovered collaborator order for `LivingWorld::Manager::PreLoadStateGraphs`.
//!
//! StateGraph/BehaviourGraph parsing remains behind a typed backend. This
//! module owns the recovered live-cVar target sampling, generated-delegate
//! writes, and ordered path dispatch.

pub const LIVING_WORLD_STATE_GRAPH_PATHS: [&str; 18] = [
    "state/livingworldentities/pedestrian/MotionGraph_Pedestrian.xml",
    "state/livingworldentities/pedestrian/Pedestrian.xml",
    "state/livingworldentities/pedestrian/Big_Black.xml",
    "state/livingworldentities/pedestrian/plugin/actortracker_spectate.xml",
    "state/livingworldentities/pedestrian/plugin/conversation.xml",
    "state/livingworldentities/pedestrian/plugin/lookat.xml",
    "state/livingworldentities/pedestrian/plugin/sit.xml",
    "state/livingworldentities/pedestrian/plugin/spectate.xml",
    "state/livingworldentities/pedestrian/plugin/useatm.xml",
    "state/livingworldentities/pedestrian/plugin/usenewspaperbox.xml",
    "state/livingworldentities/pedestrian/plugin/usetrashbin.xml",
    "state/livingworldentities/pedestrian/plugin/usevendingmachine.xml",
    "state/livingworldentities/pedestrian/plugin/usewaterfountain.xml",
    "state/livingworldentities/marquee/marquee.xml",
    "state/livingworldentities/marquee/motiongraph_marquee.xml",
    "state/livingworldentities/marquee/Photographer.xml",
    "state/livingworldentities/marquee/Motiongraph_Photographer.xml",
    "state/livingworldentities/vehicle/Vehicle.xml",
];

pub const LIVING_WORLD_BEHAVIOUR_GRAPH_PATHS: [&str; 10] = [
    "livingworld/PluginDescriptor/actortracker_spectate.xml",
    "livingworld/PluginDescriptor/conversation.xml",
    "livingworld/PluginDescriptor/lookat.xml",
    "livingworld/PluginDescriptor/sit.xml",
    "livingworld/PluginDescriptor/spectate.xml",
    "livingworld/PluginDescriptor/useatm.xml",
    "livingworld/PluginDescriptor/usenewspaperbox.xml",
    "livingworld/PluginDescriptor/usetrashbin.xml",
    "livingworld/PluginDescriptor/usevendingmachine.xml",
    "livingworld/PluginDescriptor/usewaterfountain.xml",
];

/// Independent `strstr` checks made by `EnableGeneratedStateGraph`, in write
/// order. Matching is case-sensitive and deliberately permits overlaps.
pub const GENERATED_STATE_GRAPH_DELEGATE_BASENAMES: [&str; 20] = [
    "ActionGraph_OnBoard.xml",
    "MotionGraph_OnBoard.xml",
    "Vehicle.xml",
    "Big_Black.xml",
    "MotionGraph_Pedestrian.xml",
    "Pedestrian.xml",
    "actortracker_spectate.xml",
    "conversation.xml",
    "lookat.xml",
    "sit.xml",
    "spectate.xml",
    "useatm.xml",
    "usenewspaperbox.xml",
    "usetrashbin.xml",
    "usevendingmachine.xml",
    "usewaterfountain.xml",
    "marquee.xml",
    "motiongraph_marquee.xml",
    "Motiongraph_Photographer.xml",
    "Photographer.xml",
];

/// Stable target behind the pointer stored in `kbUseGeneratedStateGraphForLW`.
pub trait LivingWorldGeneratedStateGraphTarget {
    /// Dereferences the target's current byte. Implementations must not return
    /// a value snapshotted when the target handle was acquired.
    fn read_raw(&self) -> u8;
}

/// Storage effects of the recovered generated-controller delegate table.
pub trait GeneratedStateGraphDelegateStore {
    fn store_generated_delegate_raw(
        &mut self,
        delegate_index: usize,
        basename: &'static str,
        enabled_raw: u8,
    );
}

/// Reconstructs `AbstractHierarchicalController::EnableGeneratedStateGraph`
/// (`0x828ec858`).
///
/// Every test is independent. A null name writes all 20 delegates, while a
/// non-null name writes every basename occurring as a case-sensitive
/// substring. The raw low byte is never normalized. This helper's exact claim
/// is limited to the NUL-free UTF-8 path domain used by the recovered caller;
/// native `strstr` additionally accepts arbitrary C bytes.
pub fn enable_generated_state_graph<S: GeneratedStateGraphDelegateStore>(
    store: &mut S,
    state_graph_name: Option<&str>,
    enabled_raw: u8,
) {
    for (delegate_index, basename) in GENERATED_STATE_GRAPH_DELEGATE_BASENAMES
        .into_iter()
        .enumerate()
    {
        if state_graph_name.is_none_or(|name| name.contains(basename)) {
            store.store_generated_delegate_raw(delegate_index, basename, enabled_raw);
        }
    }
}

/// Effects invoked by the recovered preload control flow.
pub trait LivingWorldStateGraphPreloadRuntime: GeneratedStateGraphDelegateStore {
    type UseGeneratedTarget: LivingWorldGeneratedStateGraphTarget;
    /// Returned ownership is released before the next descriptor load.
    type BehaviourGraphHandle;

    /// Resolves the current target pointer of `kbUseGeneratedStateGraphForLW`.
    fn acquire_use_generated_state_graph_target(&mut self) -> Self::UseGeneratedTarget;

    /// Calls `StateGraph::Cache::Load(path, false)`.
    fn load_state_graph(&mut self, path: &'static str, flush: bool);

    /// Synchronously calls `BehaviourGraph::BehaviourMan::Load`, including a
    /// successful transferable behaviour's `Process` against the shared
    /// `StateGraph` cache. The returned intrusive ownership is released by the
    /// caller before the next descriptor.
    fn load_behaviour_graph(&mut self, path: &'static str) -> Self::BehaviourGraphHandle;

    /// Calls `LivingWorld::Manager::InitPropData` after both loops.
    fn initialize_prop_data(&mut self);
}

/// Reconstructs `LivingWorld::Manager::PreLoadStateGraphs`
/// (`0x82724280..0x82725538`).
///
/// The initial cVar byte chooses one whole state-graph branch. In the generated
/// branch, the byte is re-read for every path and passed through verbatim; a
/// later zero does not switch to cache loading. Behaviour descriptors and prop
/// initialization always follow.
///
pub fn preload_living_world_state_graphs<R: LivingWorldStateGraphPreloadRuntime>(runtime: &mut R) {
    match try_preload_living_world_controllers(&mut InfallibleControllers(runtime)) {
        Ok(()) => {}
        Err(never) => match never {},
    }
    for path in LIVING_WORLD_BEHAVIOUR_GRAPH_PATHS {
        let behaviour = runtime.load_behaviour_graph(path);
        drop(behaviour);
    }
    runtime.initialize_prop_data();
}

/// Checked host adapter for the SAME controller prefix. Descriptor loading
/// and InitPropData are distinct later stages, not silently successful stubs.
pub trait LivingWorldControllerPreloadRuntime: GeneratedStateGraphDelegateStore {
    type Error;
    type Target;
    fn acquire_target(&mut self) -> Result<Self::Target, Self::Error>;
    fn read_target(target: &Self::Target) -> Result<u8, Self::Error>;
    fn load_controller(&mut self, path: &'static str, flush: bool) -> Result<(), Self::Error>;
}

/// Source82724b04..82724c24: first branch read and first path read use the
/// SAME captured target; subsequent paths reacquire it before reading.
pub fn try_preload_living_world_controllers<R: LivingWorldControllerPreloadRuntime>(
    runtime: &mut R,
) -> Result<(), R::Error> {
    let mut use_generated_target = runtime.acquire_target()?;
    if R::read_target(&use_generated_target)? != 0 {
        for (path_index, path) in LIVING_WORLD_STATE_GRAPH_PATHS.into_iter().enumerate() {
            if path_index != 0 {
                use_generated_target = runtime.acquire_target()?;
            }
            let enabled_raw = R::read_target(&use_generated_target)?;
            enable_generated_state_graph(runtime, Some(path), enabled_raw);
        }
    } else {
        for path in LIVING_WORLD_STATE_GRAPH_PATHS {
            runtime.load_controller(path, false)?;
        }
    }
    Ok(())
}

struct InfallibleControllers<'a, R>(&'a mut R);
impl<R: LivingWorldStateGraphPreloadRuntime> GeneratedStateGraphDelegateStore
    for InfallibleControllers<'_, R>
{
    fn store_generated_delegate_raw(&mut self, index: usize, name: &'static str, raw: u8) {
        self.0.store_generated_delegate_raw(index, name, raw);
    }
}
impl<R: LivingWorldStateGraphPreloadRuntime> LivingWorldControllerPreloadRuntime
    for InfallibleControllers<'_, R>
{
    type Error = std::convert::Infallible;
    type Target = R::UseGeneratedTarget;
    fn acquire_target(&mut self) -> Result<Self::Target, Self::Error> {
        Ok(self.0.acquire_use_generated_state_graph_target())
    }
    fn read_target(target: &Self::Target) -> Result<u8, Self::Error> {
        Ok(target.read_raw())
    }
    fn load_controller(&mut self, path: &'static str, flush: bool) -> Result<(), Self::Error> {
        self.0.load_state_graph(path, flush);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::VecDeque, rc::Rc};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Event {
        Acquire(u8),
        Read(u8),
        Store(usize, &'static str, u8),
        LoadState(&'static str, bool),
        LoadBehaviour(&'static str),
        DropBehaviour(&'static str),
        InitPropData,
    }

    struct Target {
        values: RefCell<VecDeque<u8>>,
        events: Rc<RefCell<Vec<Event>>>,
    }

    impl LivingWorldGeneratedStateGraphTarget for Target {
        fn read_raw(&self) -> u8 {
            let value = self
                .values
                .borrow_mut()
                .pop_front()
                .expect("test target must provide every live read");
            self.events.borrow_mut().push(Event::Read(value));
            value
        }
    }

    struct BehaviourHandle {
        path: &'static str,
        events: Rc<RefCell<Vec<Event>>>,
    }

    impl Drop for BehaviourHandle {
        fn drop(&mut self) {
            self.events
                .borrow_mut()
                .push(Event::DropBehaviour(self.path));
        }
    }

    struct Runtime {
        targets: Vec<Vec<u8>>,
        next_target: usize,
        delegate_flags: [u8; 20],
        events: Rc<RefCell<Vec<Event>>>,
    }

    impl GeneratedStateGraphDelegateStore for Runtime {
        fn store_generated_delegate_raw(
            &mut self,
            delegate_index: usize,
            basename: &'static str,
            enabled_raw: u8,
        ) {
            self.delegate_flags[delegate_index] = enabled_raw;
            self.events
                .borrow_mut()
                .push(Event::Store(delegate_index, basename, enabled_raw));
        }
    }

    impl LivingWorldStateGraphPreloadRuntime for Runtime {
        type UseGeneratedTarget = Target;
        type BehaviourGraphHandle = BehaviourHandle;

        fn acquire_use_generated_state_graph_target(&mut self) -> Self::UseGeneratedTarget {
            let values = self.targets[self.next_target].clone();
            self.next_target += 1;
            self.events.borrow_mut().push(Event::Acquire(values[0]));
            Target {
                values: RefCell::new(values.into()),
                events: Rc::clone(&self.events),
            }
        }

        fn load_state_graph(&mut self, path: &'static str, flush: bool) {
            self.events.borrow_mut().push(Event::LoadState(path, flush));
        }

        fn load_behaviour_graph(&mut self, path: &'static str) -> Self::BehaviourGraphHandle {
            self.events.borrow_mut().push(Event::LoadBehaviour(path));
            BehaviourHandle {
                path,
                events: Rc::clone(&self.events),
            }
        }

        fn initialize_prop_data(&mut self) {
            self.events.borrow_mut().push(Event::InitPropData);
        }
    }

    #[derive(Default)]
    struct Store(Vec<(usize, &'static str, u8)>);

    impl GeneratedStateGraphDelegateStore for Store {
        fn store_generated_delegate_raw(
            &mut self,
            delegate_index: usize,
            basename: &'static str,
            enabled_raw: u8,
        ) {
            self.0.push((delegate_index, basename, enabled_raw));
        }
    }

    #[test]
    fn generated_delegate_helper_preserves_null_order_raw_byte_and_case() {
        let mut store = Store::default();
        enable_generated_state_graph(&mut store, None, 0xa5);
        assert_eq!(
            store.0,
            GENERATED_STATE_GRAPH_DELEGATE_BASENAMES
                .into_iter()
                .enumerate()
                .map(|(index, basename)| (index, basename, 0xa5))
                .collect::<Vec<_>>()
        );

        store.0.clear();
        enable_generated_state_graph(&mut store, Some("vehicle.xml"), 7);
        enable_generated_state_graph(&mut store, Some("unrelated.xml"), 9);
        assert!(store.0.is_empty());
    }

    #[test]
    fn generated_delegate_helper_preserves_four_independent_overlap_sequences() {
        let cases = [
            ("MotionGraph_Pedestrian.xml", vec![4, 5]),
            ("actortracker_spectate.xml", vec![6, 10]),
            ("motiongraph_marquee.xml", vec![16, 17]),
            ("Motiongraph_Photographer.xml", vec![18, 19]),
        ];
        for (name, expected_indices) in cases {
            let mut store = Store::default();
            enable_generated_state_graph(&mut store, Some(name), 0x81);
            assert_eq!(
                store
                    .0
                    .iter()
                    .map(|&(index, _, raw)| (index, raw))
                    .collect::<Vec<_>>(),
                expected_indices
                    .into_iter()
                    .map(|index| (index, 0x81))
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn generated_branch_reuses_first_target_then_reacquires_for_remaining_paths() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut targets = vec![vec![0x81, 0]];
        targets.extend((1_u8..18).map(|raw| vec![raw]));
        let mut runtime = Runtime {
            targets,
            next_target: 0,
            delegate_flags: [0; 20],
            events: Rc::clone(&events),
        };

        preload_living_world_state_graphs(&mut runtime);

        assert_eq!(runtime.next_target, 18);
        let events = events.borrow();
        assert_eq!(
            events[0..3],
            [Event::Acquire(0x81), Event::Read(0x81), Event::Read(0)]
        );
        let acquired = events
            .iter()
            .filter_map(|event| match event {
                Event::Acquire(raw) => Some(*raw),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            acquired,
            std::iter::once(0x81).chain(1_u8..18).collect::<Vec<_>>()
        );
        assert!(
            events
                .iter()
                .any(|event| { matches!(event, Event::Store(4, "MotionGraph_Pedestrian.xml", 0)) })
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::LoadState(_, _)))
                .count(),
            0
        );
        assert_eq!(events.last(), Some(&Event::InitPropData));
    }

    #[test]
    fn cache_branch_samples_once_and_loads_all_states_then_behaviours() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = Runtime {
            targets: vec![vec![0]],
            next_target: 0,
            delegate_flags: [0x5a; 20],
            events: Rc::clone(&events),
        };

        preload_living_world_state_graphs(&mut runtime);

        assert_eq!(runtime.next_target, 1);
        assert_eq!(runtime.delegate_flags, [0x5a; 20]);
        let events = events.borrow();
        assert_eq!(events[0..2], [Event::Acquire(0), Event::Read(0)]);
        assert_eq!(
            &events[2..20],
            LIVING_WORLD_STATE_GRAPH_PATHS.map(|path| Event::LoadState(path, false))
        );
        let behaviour_events = events[20..40]
            .chunks_exact(2)
            .map(|pair| (&pair[0], &pair[1]))
            .collect::<Vec<_>>();
        assert_eq!(behaviour_events.len(), 10);
        for ((load, drop), path) in behaviour_events
            .into_iter()
            .zip(LIVING_WORLD_BEHAVIOUR_GRAPH_PATHS)
        {
            assert_eq!(load, &Event::LoadBehaviour(path));
            assert_eq!(drop, &Event::DropBehaviour(path));
        }
        assert_eq!(events[40], Event::InitPropData);
    }
}
