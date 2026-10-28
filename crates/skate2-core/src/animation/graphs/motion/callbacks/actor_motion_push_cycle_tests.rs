use super::{PushCycleInstance, Source, begin, update};
use crate::play_animation::FastString36;
use std::convert::Infallible;

#[derive(Default)]
struct Spy {
    holding: bool,
    pushing: bool,
    new_push: bool,
    configured: bool,
    push_time: f32,
    speed: f32,
    push_dv: f32,
    flag: bool,
    cap: bool,
    top: f32,
    max_holding: f32,
    calls: Vec<&'static str>,
}
impl Source for Spy {
    type Error = Infallible;
    fn resolve_begin(&mut self) -> Result<(), Infallible> {
        self.calls.push("begin");
        Ok(())
    }
    fn resolve_update(&mut self) -> Result<(), Infallible> {
        self.calls.push("update");
        Ok(())
    }
    fn graph(&mut self, offset: usize, x: f32) -> Result<f32, Infallible> {
        self.calls.push(if offset == 0x230 {
            "duration"
        } else {
            "dv graph"
        });
        Ok(if offset == 0x230 { 1.0 } else { x * 10.0 })
    }
    fn allow_holding(&mut self) -> Result<bool, Infallible> {
        self.calls.push("allow holding");
        Ok(self.holding)
    }
    fn has_intent(&mut self, name: &FastString36) -> Result<bool, Infallible> {
        self.calls.push("has intent");
        Ok(
            if name == &crate::actor_graph_construction::ActorGraphIntents::native_name(b"Pushing")
            {
                self.pushing
            } else if name
                == &crate::actor_graph_construction::ActorGraphIntents::native_name(b"NewPush")
            {
                self.new_push
            } else {
                self.configured
            },
        )
    }
    fn intent_value(&mut self, _: &FastString36) -> Result<f32, Infallible> {
        self.calls.push("intent value");
        Ok(self.push_time)
    }
    fn pushing_flag(&mut self, value: bool) -> Result<(), Infallible> {
        self.calls.push("flag");
        self.flag = value;
        Ok(())
    }
    fn speed(&mut self) -> Result<f32, Infallible> {
        self.calls.push("speed");
        Ok(self.speed)
    }
    fn max_holding_acc(&mut self) -> Result<f32, Infallible> {
        self.calls.push("maximum");
        Ok(self.max_holding)
    }
    fn top_speed(&mut self) -> Result<f32, Infallible> {
        self.calls.push("top speed");
        Ok(self.top)
    }
    fn cap_dv_at_top_speed(&mut self) -> Result<bool, Infallible> {
        self.calls.push("cap");
        Ok(self.cap)
    }
    fn push_dv(&mut self) -> Result<f32, Infallible> {
        self.calls.push("read dv");
        Ok(self.push_dv)
    }
    fn set_push_dv(&mut self, value: f32) -> Result<(), Infallible> {
        self.calls.push("write dv");
        self.push_dv = value;
        Ok(())
    }
}

#[test]
fn hold_release_then_new_push_release_follows_native_stages() {
    let name = crate::actor_graph_construction::ActorGraphIntents::native_name(b"NewPushRight");
    let mut source = Spy {
        holding: true,
        pushing: true,
        configured: true,
        push_time: 0.8,
        max_holding: 4.0,
        top: 5.0,
        ..Spy::default()
    };
    let mut instance = PushCycleInstance::after_allocation(true).unwrap();
    assert_eq!((instance.stage, instance.change_velocity), (1, 0.0));
    begin(&mut source, &mut instance, &name).unwrap();
    assert_eq!(instance.stage, 0);
    assert!(source.flag);
    update(&mut source, &mut instance, &name).unwrap();
    assert_eq!(instance.change_velocity, 4.0);
    assert_eq!(source.push_dv, 4.0);
    assert_eq!(
        &source.calls[source.calls.len() - 3..],
        ["maximum", "write dv", "read dv"]
    );
    source.pushing = false;
    update(&mut source, &mut instance, &name).unwrap();
    assert_eq!(instance.stage, 1);
    assert_eq!(instance.change_velocity, 0.0);
    assert!(!source.flag);
    source.new_push = true;
    source.pushing = true;
    update(&mut source, &mut instance, &name).unwrap();
    assert_eq!(instance.stage, 2);
    assert!(source.flag);
    source.configured = false;
    update(&mut source, &mut instance, &name).unwrap();
    assert_eq!(instance.stage, 3);
    assert_eq!(source.push_dv, instance.change_velocity);
}

#[test]
fn top_speed_cap_retains_previous_change_velocity() {
    let name = crate::actor_graph_construction::ActorGraphIntents::native_name(b"NewPushRight");
    let mut source = Spy {
        speed: 8.0,
        top: 5.0,
        cap: true,
        push_time: 0.8,
        new_push: true,
        configured: true,
        ..Spy::default()
    };
    let mut instance = PushCycleInstance {
        stage: 1,
        change_velocity: 2.0,
    };
    update(&mut source, &mut instance, &name).unwrap();
    assert_eq!(instance.stage, 2);
    assert_eq!(instance.change_velocity, 2.0);
    assert_eq!(source.push_dv, 2.0);
    assert!(source.flag);
    assert!(!source.calls.contains(&"intent value"));
}

#[test]
fn unordered_comparisons_follow_ppc_fcmpu_branches() {
    let name = crate::actor_graph_construction::ActorGraphIntents::native_name(b"NewPushRight");
    let mut holding = Spy {
        holding: true,
        pushing: true,
        push_time: 0.5,
        max_holding: f32::NAN,
        push_dv: 1.0,
        ..Spy::default()
    };
    let mut instance = PushCycleInstance {
        stage: 0,
        change_velocity: 0.0,
    };
    update(&mut holding, &mut instance, &name).unwrap();
    assert_eq!(instance.change_velocity, 5.0);
    assert_eq!(holding.push_dv, 5.0);
    assert_eq!(
        holding
            .calls
            .iter()
            .filter(|&&call| call == "write dv")
            .count(),
        1
    );

    let mut pending = Spy {
        new_push: true,
        configured: true,
        push_dv: f32::NAN,
        top: f32::NAN,
        speed: 8.0,
        cap: true,
        push_time: 0.5,
        ..Spy::default()
    };
    let mut instance = PushCycleInstance {
        stage: 1,
        change_velocity: 0.0,
    };
    update(&mut pending, &mut instance, &name).unwrap();
    assert_eq!(instance.change_velocity, 5.0);
    assert!(pending.push_dv.is_nan());
}
