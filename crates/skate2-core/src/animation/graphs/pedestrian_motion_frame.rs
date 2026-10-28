//! Pedestrian::AnimUpdate82e95da0, over retained native animation owners.
use super::live::{ExternalOwners, LiveServices};
use crate::{input_cvar_runtime::InputCvarReferences,
    living_world_pedestrian_entity::PedestrianConstruction,
    play_animation::FastString30, reckoning_numeric::ReckoningNumeric};

pub(crate) struct FrameMetadata {
    pub(crate) offset_time: f32,
    pub(crate) valid_blend_pose_buffer: bool,
    /// cPedAnimData::FootTags: valid80000000, RT40000000, LT20000000,
    /// RH10000000, LH08000000. Native clears then rebuilds this each AnimUpdate.
    pub(crate) foot_tags: u32,
    pub(crate) in_turn_branch_window: bool,
}
/// Pedestrian+0xa80. The owning entity supplies mpPedestrian through its
/// existing authority; these are the remaining native PedStateOut fields.
pub(crate) struct PedStateOut {
    pub(crate) state: u32,
    pub(crate) length: u32,
    pub(crate) topic: u32,
    pub(crate) target_greet_character: u32,
    pub(crate) flags: u8,
}
impl PedStateOut {
    /// Pedestrian ctor82e97a38..80. Low four flag bits are unused padding.
    pub(crate) fn new() -> Self {
        Self { state: 72, length: 3, topic: 18, target_greet_character: 100, flags: 0 }
    }
}
impl FrameMetadata {
    /// cPedAnimData82e8d268 then tAnimData82e934a0.
    pub(crate) fn new() -> Self {
        Self { offset_time: 0., valid_blend_pose_buffer: false, foot_tags: 0, in_turn_branch_window: false }
    }
}
pub(crate) struct FootContactOutput {
    /// Writes mPedStateOut byte20 bits4/5, preserving all other bits.
    pub(crate) right_down: bool,
    pub(crate) left_down: bool,
}
impl FootContactOutput {
    pub(crate) fn write_flags(&self, flags: &mut u8) {
        *flags = (*flags & !0x30) | (u8::from(self.right_down) << 4) | (u8::from(self.left_down) << 5);
    }
}

/// Caller supplies the actual TestLOSim result and registered runtime cvars.
/// UpdateAnimationData (intent snapshot) must already have run in its native
/// frame phase; this routine deliberately does not refresh that separate owner.
pub(crate) fn update<N: ReckoningNumeric, E: ExternalOwners>(
    entity: &mut PedestrianConstruction, dt: f32, active_pool: u32,
    pools_enabled: bool, test_losim: u32, generate_foot_tags_for_all: bool,
    cvars: &InputCvarReferences, numeric: &mut N, external: &mut E,
) -> Result<FootContactOutput, String>
where N::Error: std::fmt::Debug {
    let (animation_dt, graph_dt) = {
        let animation = entity.animation.as_mut().ok_or("pedestrian animation absent")?;
        let animation_dt = animation.frame.offset_time + dt;
        let has_animation = animation.has_current_animation();
        let graph_dt = animation.schedule.graph_update_dt(animation_dt, active_pool,
            pools_enabled, animation.motion_intents.valid, has_animation).map_err(|e| e.to_string())?;
        (animation_dt, graph_dt)
    };
    if let Some(graph_dt) = graph_dt {
        // Move only the graph owners out while services borrow the SAME entity.
        // Validate all slots first; restore owners even on an effectful failure.
        let (mut delegate, objects, intents) = {
            let animation = entity.animation.as_mut().ok_or("pedestrian animation absent")?;
            if animation.motion.is_none() || animation.motion_objects.is_none() || animation.graph_intents.is_none() {
                return Err("pedestrian motion graph Load incomplete".into());
            }
            (animation.motion.take().expect("checked delegate"),
                animation.motion_objects.take().expect("checked objects"),
                animation.graph_intents.take().expect("checked intents"))
        };
        let result = (|| {
            let mut services = LiveServices::new(entity, delegate.context(), &intents, cvars, numeric, external)?;
            objects.update(&mut delegate, graph_dt, &mut services)
        })();
        let animation = entity.animation.as_mut().ok_or("pedestrian animation removed during graph update")?;
        animation.motion = Some(delegate);
        animation.motion_objects = Some(objects);
        animation.graph_intents = Some(intents);
        result?;
    }
    let animation = entity.animation.as_mut().ok_or("pedestrian animation absent")?;
    animation.advance(animation_dt, cvars).map_err(|e| format!("pedestrian BuildAnimTree: {e:?}"))?;
    animation.frame.foot_tags = 0;
    if test_losim != 2 || generate_foot_tags_for_all {
        for (name, bit) in [("RightToeDown", 0x4000_0000), ("LeftToeDown", 0x2000_0000),
            ("RightHeelDown", 0x1000_0000), ("LeftHeelDown", 0x0800_0000)] {
            let found = animation.core.try_tree_attribute(FastString30::encode(name).expect("native foot tag"))
                .map_err(|e| format!("pedestrian foot tag: {e:?}"))?.is_some();
            animation.frame.foot_tags = (animation.frame.foot_tags & !bit) | 0x8000_0000 | if found { bit } else { 0 };
        }
        if animation.mirrored && animation.frame.foot_tags & 0x8000_0000 != 0 {
            let tags = animation.frame.foot_tags;
            animation.frame.foot_tags = (tags & !0x7800_0000)
                | ((tags & 0x4000_0000) >> 1) | ((tags & 0x2000_0000) << 1)
                | ((tags & 0x1000_0000) >> 1) | ((tags & 0x0800_0000) << 1);
        }
    }
    let output = FootContactOutput { right_down: animation.frame.foot_tags & 0x5000_0000 != 0,
        left_down: animation.frame.foot_tags & 0x2800_0000 != 0 };
    // Native publishes these bits before TurnBranchWindow and channel work.
    // A later callback failure must not erase this already completed mutation.
    output.write_flags(&mut entity.state_out.flags);
    animation.frame.in_turn_branch_window = animation.core.try_tree_attribute(
        FastString30::encode("TurnBranchWindow").expect("native turn tag"))
        .map_err(|e| format!("pedestrian turn tag: {e:?}"))?.is_some();
    animation.frame.offset_time = 0.;
    animation.frame.valid_blend_pose_buffer = false;
    animation.channels.update(&mut animation.core)?;
    Ok(output)
}
