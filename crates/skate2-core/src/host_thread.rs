//! Optional host notifications for workers on the frame's dependency path.
//!
//! These lifetime hooks let a platform describe its threads to the OS. They do
//! not choose jobs, change affinity masks, or participate in simulation timing.
use std::{marker::PhantomData, rc::Rc, sync::OnceLock};

#[derive(Clone, Copy, Debug)]
pub struct WorkerThreadHooks {
    /// Called on the worker before any work. Must not panic.
    pub entered: fn(),
    /// Called on the same worker before it exits. Must not panic.
    pub leaving: fn(),
}

static HOOKS: OnceLock<WorkerThreadHooks> = OnceLock::new();

/// Install once, before starting any game workers. With no hooks installed,
/// worker lifetime registration has no platform effect.
pub fn install_worker_thread_hooks(hooks: WorkerThreadHooks) -> Result<(), WorkerThreadHooks> {
    HOOKS.set(hooks)
}

/// A worker's registration cannot move to another thread: its exit notification
/// must identify the same OS thread, including during unwinding.
#[must_use]
pub struct WorkerThreadScope {
    leaving: Option<fn()>,
    _thread_bound: PhantomData<Rc<()>>,
}

impl WorkerThreadScope {
    pub fn enter() -> Self {
        let hooks = HOOKS.get();
        if let Some(hooks) = hooks {
            (hooks.entered)();
        }
        Self {
            leaving: hooks.map(|hooks| hooks.leaving),
            _thread_bound: PhantomData,
        }
    }
}

impl Drop for WorkerThreadScope {
    fn drop(&mut self) {
        if let Some(leaving) = self.leaving {
            leaving();
        }
    }
}
