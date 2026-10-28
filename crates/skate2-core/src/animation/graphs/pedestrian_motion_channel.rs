//! cPedAnimData channel request metadata and AnimUpdate's native consumer.
//! Actual channel allocation/blending/time stays in shared ChannelAnimatable.
use crate::{
    actor_animation_construction::{ActorAnimationConstruction, channel_runtime::{
        ChannelParams, NewGestureChannelStatus, transition::ChannelStartFlags}},
    play_animation::{FastString36, EMPTY_FAST_STRING_36},
};

/// PDB Sk8::tPedChannelInfo::ePedChannelStatus; distinct from Andale status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum Status {
    NotStarted = 0, NotStartedSequence = 1, WaitingToTransition = 2,
    WaitingToSequence = 3, Playing = 4, BlendOut = 5, WaitingForBlendOut = 6, Finished = 7,
}
pub(crate) struct Request {
    pub(crate) status: Status,
    pub(crate) animation: FastString36,
    pub(crate) transition_time: f32,
    pub(crate) params: ChannelParams,
}
pub(crate) struct PedestrianChannels {
    pub(crate) channel: Request,
    pub(crate) sequence: Request,
}
fn params() -> ChannelParams {
    ChannelParams { priority: 0, cycle: false, mirror: false, playback_speed: 1.,
        blend_in_length: 0.2, blend_in_to_first_frame: false,
        blend_out_length: 0.2, blend_out_from_last_frame: false }
}
impl Request {
    fn new() -> Self {
        Self { status: Status::Finished, animation: EMPTY_FAST_STRING_36, transition_time: 0., params: params() }
    }
}
impl PedestrianChannels {
    /// Pedestrian::PlayChannelAnim82e91e08 resolves the actual clip using the
    /// existing RemapAnim owner, then ALWAYS calls TransitionChannelAnim.
    /// Callers supply current registered clips/selected animation data and the
    /// shared native RNG; this owns no extra resource cache or random stream.
    pub(crate) fn transition_remapped(
        &mut self, name: &str,
        registered: &crate::pedestrian_animation::PedestrianRegisteredClips,
        data: &crate::attribute_runtime::AttributeRuntimeInstance,
        random: impl FnMut() -> u32,
        time: f32, blend_in: f32, blend_out: f32, cycle: bool,
    ) -> Result<(), String> {
        let mapped = crate::pedestrian_animation::remap_animation(name, registered, data, random)
            .map_err(|e| e.to_string())?;
        let animation = FastString36::encode(&mapped)
            .map_err(|e| format!("pedestrian channel remapped name: {e:?}"))?;
        self.transition(animation, time, blend_in, blend_out, cycle)
    }

    /// Short char* overload82e6e830: raw f1=f2=f3=.2 and r8=false.
    pub(crate) fn request_named(
        &mut self, name: &str,
        registered: &crate::pedestrian_animation::PedestrianRegisteredClips,
        data: &crate::attribute_runtime::AttributeRuntimeInstance,
        random: impl FnMut() -> u32,
    ) -> Result<(), String> {
        self.transition_remapped(name, registered, data, random, 0.2, 0.2, 0.2, false)
    }

    /// cPedAnimData82e8d268 writes BOTH full request records identically.
    pub(crate) fn new() -> Self { Self { channel: Request::new(), sequence: Request::new() } }
    /// StopChannels82e6d350. Sequence and current channel runtime are untouched.
    pub(crate) fn stop(&mut self, blend_out: f32) {
        if self.channel.status != Status::Finished {
            self.channel.params.blend_out_length = blend_out;
            self.channel.status = Status::BlendOut;
        }
    }
    /// TransitionChannelAnim82e905a8 resets params, then supplies these fields.
    pub(crate) fn transition(&mut self, animation: FastString36, time: f32,
        blend_in: f32, blend_out: f32, cycle: bool) -> Result<(), String>
    {
        if animation == EMPTY_FAST_STRING_36 { return Err("pedestrian channel empty animation assertion".into()); }
        self.channel.animation = animation;
        self.channel.transition_time = time;
        self.channel.params = params();
        self.channel.params.blend_in_length = blend_in;
        self.channel.params.blend_out_length = blend_out;
        self.channel.params.blend_in_to_first_frame = blend_in > 0.;
        self.channel.params.blend_out_from_last_frame = blend_out > 0.;
        self.channel.params.cycle = cycle;
        self.channel.status = if self.channel.status == Status::Finished { Status::NotStarted } else { Status::WaitingToTransition };
        Ok(())
    }
    /// PlayChannelAnim82e93f20: nonfinished path passes blendOut as BOTH blend
    /// lengths to TransitionChannelAnim(.2), confirmed raw f2/f3=f30 at940c0/c8.
    pub(crate) fn play(&mut self, animation: FastString36, blend_in: f32,
        blend_out: f32, cycle: bool) -> Result<(), String>
    {
        if self.channel.status != Status::Finished {
            return self.transition(animation, 0.2, blend_out, blend_out, cycle);
        }
        if animation == EMPTY_FAST_STRING_36 { return Err("pedestrian channel empty animation assertion".into()); }
        self.channel.animation = animation;
        self.channel.params = params();
        self.channel.params.blend_in_length = blend_in;
        self.channel.params.blend_in_to_first_frame = blend_in > 0.;
        self.channel.params.blend_out_length = blend_out;
        self.channel.params.cycle = cycle;
        self.channel.status = Status::NotStarted;
        self.channel.params.blend_out_from_last_frame = blend_out > 0.;
        Ok(())
    }
    /// Pedestrian::AnimUpdate82e96198..6458 runs this AFTER BuildAnimTree,
    /// foot/turn tags and clearing OffsetTime/ValidBlendPoseBuffer.
    pub(crate) fn update<K, W>(&mut self, core: &mut ActorAnimationConstruction<K, W>) -> Result<(), String> {
        let name = FastString36::encode("PedChannel").expect("native channel name82faba00");
        let flags = ChannelStartFlags { resurrect_if_blending_out: true, create_if_missing: true, use_attributes: false };
        match self.channel.status {
            Status::NotStarted | Status::NotStartedSequence | Status::WaitingToTransition => {
                let before = self.channel.status;
                let length = if before == Status::WaitingToTransition { self.channel.transition_time } else { 0.2 };
                let result = core.try_transition_channel_to(name, self.channel.animation, length, self.channel.params, flags)
                    .map_err(|e| format!("pedestrian channel transition: {e:?}"))?;
                if result != NewGestureChannelStatus::Created { missing_channel(core, name)?; }
                self.channel.status = if before == Status::NotStartedSequence { Status::WaitingToSequence } else { Status::Playing };
            }
            Status::WaitingToSequence => {
                let result = core.try_sequence_channel_to(name, self.sequence.animation, self.sequence.params, flags)
                    .map_err(|e| format!("pedestrian channel sequence: {e:?}"))?;
                if result != NewGestureChannelStatus::Created { missing_channel(core, name)?; }
                self.channel.status = Status::Playing;
            }
            Status::Playing => {
                if !core.has_channel(&name).map_err(|e| format!("pedestrian HasChannel: {e:?}"))? { self.channel.status = Status::Finished; }
            }
            Status::BlendOut => {
                core.try_end_all_channels_prematurely(self.channel.params.blend_out_length, true)
                    .map_err(|e| format!("pedestrian EndAllChannelsPrematurely: {e:?}"))?;
                self.channel.status = Status::WaitingForBlendOut;
            }
            Status::WaitingForBlendOut => {
                if !core.has_channel(&name).map_err(|e| format!("pedestrian HasChannel: {e:?}"))? { self.channel.status = Status::Finished; }
            }
            Status::Finished => {},
        }
        Ok(())
    }
}
/// PlayMissingChannelAnimation82e91ee8: default params, length.01, resurrect/
/// create true, attributes false. Native asserts ANY nonzero Andale status.
fn missing_channel<K, W>(core: &mut ActorAnimationConstruction<K, W>, name: FastString36) -> Result<(), String> {
    let animation = FastString36::encode("HIP_TEST_01").expect("native missing-channel name");
    let status = core.try_transition_channel_to(name, animation, 0.01, params(),
        ChannelStartFlags { resurrect_if_blending_out: true, create_if_missing: true, use_attributes: false })
        .map_err(|e| format!("pedestrian missing-channel playback: {e:?}"))?;
    if status == NewGestureChannelStatus::Created { Ok(()) }
    else { Err(format!("native PlayMissingChannelAnimation assertion: {status:?}")) }
}
