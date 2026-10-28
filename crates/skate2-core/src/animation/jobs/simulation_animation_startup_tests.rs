use super::*;

struct FailingSource {
    calls: Vec<&'static str>,
    fail_thread: bool,
    vbr: bool,
}
impl AnimationStartupSource for FailingSource {
    type Error = &'static str;
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error> {
        panic!("IO failure precedes any NewACS manager capture")
    }
    fn current_thread_id(&mut self) -> Result<u64, Self::Error> {
        self.calls.push("thread");
        if self.fail_thread {
            Err("thread unavailable")
        } else {
            Ok(71)
        }
    }
    fn read_use_vbr(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("cvar");
        Ok(self.vbr)
    }
    fn read_animation(&mut self, path: &str) -> Result<Box<[u8]>, Self::Error> {
        assert_eq!(
            path,
            if self.vbr {
                "data\\anim/OffBoard.abin"
            } else {
                "data\\anim/OffBoardRAW.abin"
            }
        );
        self.calls.push("offboard");
        Err("missing source asset")
    }
}

#[test]
fn first_io_failure_retains_database_slot_and_initialized_pool_before_no_playbacks() {
    for vbr in [false, true] {
        let mut source = FailingSource {
            calls: Vec::new(),
            fail_thread: false,
            vbr,
        };
        let managers = FaceAnimationManagers::new();
        let mut startup = SimulationAnimationStartup::new(managers.clone());
        assert_eq!(
            startup.try_initialize(&mut source),
            Err(AnimationStartupError::Source("missing source asset"))
        );
        assert_eq!(source.calls, ["thread", "cvar", "offboard"]);
        assert_eq!(startup.databases.len(), 1);
        let database = startup.databases[0].lock().unwrap();
        assert_eq!(database.slot, 0);
        assert!(database.parsed.is_none());
        let pool = startup.attributes.as_ref().unwrap().lock().unwrap();
        assert_eq!(pool.creation_thread, Some(71));
        assert_eq!(pool.memory.len(), 0x19000);
        assert_eq!(pool.elements.len(), 20);
        assert_eq!(pool.element_count, 0);
        assert!(managers.playback.lock().unwrap().entries.is_empty());
        assert!(managers.command.lock().unwrap().entries.is_empty());
        assert!(!startup.databases_and_fixed_hands_ready);
        assert_eq!(startup.selected_vbr_codec, None);
        drop(pool);
        drop(database);
        assert_eq!(
            startup.try_initialize(&mut source),
            Err(AnimationStartupError::AlreadyAttempted)
        );
        assert_eq!(source.calls.len(), 3);
    }
}

#[test]
fn thread_failure_precedes_pool_resize_cvar_and_any_database() {
    let mut source = FailingSource {
        calls: Vec::new(),
        fail_thread: true,
        vbr: true,
    };
    let mut startup = SimulationAnimationStartup::new(FaceAnimationManagers::new());
    assert_eq!(
        startup.try_initialize(&mut source),
        Err(AnimationStartupError::Source("thread unavailable"))
    );
    assert_eq!(source.calls, ["thread"]);
    let pool = startup.attributes.as_ref().unwrap().lock().unwrap();
    assert!(pool.creation_thread.is_none());
    assert!(pool.memory.is_empty());
    assert!(startup.databases.is_empty());
}
