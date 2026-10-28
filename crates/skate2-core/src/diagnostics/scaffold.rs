//! Reconstructed application scheduling and asynchronous-load state machine.

/// Reference simulation frequency configured by `AppMan::Run` at `0x8298d458`.
pub const TIMER_HZ: u32 = 60;

/// Reference simulation period configured by `AppMan::Run` at `0x8298d458`.
pub const TIMER_PERIOD_SECONDS: f32 = 0.016_666_668;

/// `AppMan::eLoadState` (IDA type ordinal 2986).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u32)]
pub enum LoadState {
    #[default]
    Idle = 0,
    ModuleActive = 1,
    ModuleFinished = 2,
    BackgroundActive = 3,
    BackgroundFinished = 4,
    End = 5,
}

/// Operations the reconstructed app loop performs on an application module.
pub trait AppModule {
    /// Gives the current module a chance to delay replacement or shutdown.
    fn lose_focus(&mut self) -> bool;

    /// Releases runtime resources before the module is destroyed.
    fn purge(&mut self);

    /// Activates a module after its background load completes.
    fn gain_focus(&mut self);
}

/// Background execution boundary used by the original application manager.
pub trait LoadExecutor<M, B> {
    /// True when the single background worker is available or has completed.
    fn is_load_done(&self) -> bool;

    /// Starts the reference `AppMan::LoadModuleFunction` operation.
    fn start_module_load(&mut self, module: &mut M);

    /// Starts a non-module background operation.
    fn start_background_load(&mut self, request: &mut B);

    /// Runs the completion callback on the main thread.
    fn finish_background_load(&mut self, request: B);
}

/// Readable reconstruction of the load-related fields and transitions in
/// `AppMan::MainLoop` at `0x82989ae0`.
pub struct LoadController<M, B> {
    state: LoadState,
    active_module: Option<M>,
    loading_module: Option<M>,
    background_request: Option<B>,
}

impl<M, B> Default for LoadController<M, B> {
    fn default() -> Self {
        Self {
            state: LoadState::Idle,
            active_module: None,
            loading_module: None,
            background_request: None,
        }
    }
}

impl<M: AppModule, B> LoadController<M, B> {
    #[must_use]
    pub const fn state(&self) -> LoadState {
        self.state
    }

    #[must_use]
    pub const fn active_module(&self) -> Option<&M> {
        self.active_module.as_ref()
    }

    pub fn queue_module(&mut self, module: M) {
        self.loading_module = Some(module);
    }

    pub fn queue_background(&mut self, request: B) {
        self.background_request = Some(request);
    }

    pub fn request_end(&mut self) {
        self.state = LoadState::End;
    }

    /// Advances the load state once, matching the switch in `AppMan::MainLoop`.
    ///
    /// Returns `true` after the active module has accepted shutdown.
    pub fn update(&mut self, executor: &mut impl LoadExecutor<M, B>) -> bool {
        match self.state {
            LoadState::Idle => {
                if let Some(module) = self.loading_module.as_mut()
                    && executor.is_load_done()
                {
                    executor.start_module_load(module);
                    self.state = LoadState::ModuleActive;
                } else if let Some(request) = self.background_request.as_mut()
                    && executor.is_load_done()
                {
                    executor.start_background_load(request);
                    self.state = LoadState::BackgroundActive;
                }
                false
            }
            LoadState::ModuleActive => {
                if executor.is_load_done() {
                    self.state = LoadState::ModuleFinished;
                }
                false
            }
            LoadState::ModuleFinished => {
                if let Some(active) = self.active_module.as_mut()
                    && active.lose_focus()
                {
                    active.purge();
                    self.active_module = None;
                }

                if self.active_module.is_none()
                    && let Some(mut loaded) = self.loading_module.take()
                {
                    loaded.gain_focus();
                    self.active_module = Some(loaded);
                    self.state = LoadState::Idle;
                }
                false
            }
            LoadState::BackgroundActive => {
                if executor.is_load_done() {
                    self.state = LoadState::BackgroundFinished;
                }
                false
            }
            LoadState::BackgroundFinished => {
                if let Some(request) = self.background_request.take() {
                    executor.finish_background_load(request);
                }
                self.state = LoadState::Idle;
                false
            }
            LoadState::End => {
                if let Some(active) = self.active_module.as_mut() {
                    if !active.lose_focus() {
                        return false;
                    }
                    active.purge();
                    self.active_module = None;
                }
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct TestModule {
        accepts_focus_loss: bool,
        gained_focus: usize,
        purged: usize,
    }

    impl AppModule for TestModule {
        fn lose_focus(&mut self) -> bool {
            self.accepts_focus_loss
        }

        fn purge(&mut self) {
            self.purged += 1;
        }

        fn gain_focus(&mut self) {
            self.gained_focus += 1;
        }
    }

    #[derive(Default)]
    struct TestExecutor {
        done: bool,
        module_loads: usize,
        background_loads: usize,
        background_callbacks: Vec<u32>,
    }

    impl LoadExecutor<TestModule, u32> for TestExecutor {
        fn is_load_done(&self) -> bool {
            self.done
        }

        fn start_module_load(&mut self, _module: &mut TestModule) {
            self.module_loads += 1;
            self.done = false;
        }

        fn start_background_load(&mut self, _request: &mut u32) {
            self.background_loads += 1;
            self.done = false;
        }

        fn finish_background_load(&mut self, request: u32) {
            self.background_callbacks.push(request);
        }
    }

    #[test]
    fn reference_timer_is_sixty_hertz() {
        assert_eq!(TIMER_HZ, 60);
        assert!((TIMER_PERIOD_SECONDS - 0.016_666_668).abs() < f32::EPSILON);
    }

    #[test]
    fn loads_and_activates_a_module() {
        let mut controller = LoadController::<TestModule, u32>::default();
        let mut executor = TestExecutor {
            done: true,
            ..TestExecutor::default()
        };
        controller.queue_module(TestModule::default());

        assert!(!controller.update(&mut executor));
        assert_eq!(controller.state(), LoadState::ModuleActive);
        assert_eq!(executor.module_loads, 1);

        executor.done = true;
        controller.update(&mut executor);
        assert_eq!(controller.state(), LoadState::ModuleFinished);

        controller.update(&mut executor);
        assert_eq!(controller.state(), LoadState::Idle);
        assert_eq!(controller.active_module().unwrap().gained_focus, 1);
    }

    #[test]
    fn module_load_takes_priority_over_background_load() {
        let mut controller = LoadController::default();
        let mut executor = TestExecutor {
            done: true,
            ..TestExecutor::default()
        };
        controller.queue_module(TestModule::default());
        controller.queue_background(42);

        controller.update(&mut executor);

        assert_eq!(executor.module_loads, 1);
        assert_eq!(executor.background_loads, 0);
    }

    #[test]
    fn background_callback_runs_on_the_following_main_loop_transition() {
        let mut controller = LoadController::<TestModule, u32>::default();
        let mut executor = TestExecutor {
            done: true,
            ..TestExecutor::default()
        };
        controller.queue_background(42);

        controller.update(&mut executor);
        executor.done = true;
        controller.update(&mut executor);
        assert_eq!(executor.background_callbacks, Vec::<u32>::new());

        controller.update(&mut executor);
        assert_eq!(executor.background_callbacks, vec![42]);
        assert_eq!(controller.state(), LoadState::Idle);
    }
}
