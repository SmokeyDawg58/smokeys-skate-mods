//! Same-entity pedestrian motion services. This borrows existing construction,
//! animation, AI input, state, intent queue and SpatialAdjust owners. External
//! services name the remaining actual interface owners; none has a noop default.
use super::{behaviours, conditions};
use crate::{
    actor_graph_construction::ActorGraphIntents,
    input_cvar_runtime::InputCvarReferences,
    living_world_pedestrian_entity::{EntityAnimation, PedestrianConstruction, PedestrianIdentity},
    living_world_pedestrian_state::PedestrianStateStorage,
    motion_graph_conditions::{AnimProperty, HasIntentCollaborator, WillExpireCollaborator},
    motion_graph_entity_authority::{BoundMotionGraphContext, MotionGraphEntityAuthority},
    pedestrian_animation::PedestrianMotionIntents,
    pedestrian_intent_packet::queue::{AiGraphInput, IntentQueue},
    play_animation::FastString36,
    play_remapped_animation::{PlayRemappedAnimationCollaborator, RemappedAnimationSetup,
        RemappedAnimationTransitionType as Transition, RemappedAnimationUpdate},
    reckoning_numeric::ReckoningNumeric,
};

/// UpdateAnimationData82e9a078 clears then copies the current intent prefix with
/// value1. This snapshot belongs to that phase, not every HasIntent evaluation.
pub(crate) fn capture_motion_intents(snapshot: &mut ActorGraphIntents, input: &PedestrianMotionIntents) -> Result<(), String> {
    snapshot.clear();
    for name in input.names() {
        let mut words = [0; 6];
        words[..5].copy_from_slice(&name.words);
        snapshot.insert_or_assign(FastString36 { words }, 1.).map_err(|e| format!("pedestrian graph intents: {e:?}"))?;
    }
    Ok(())
}
#[derive(Clone, Copy)]
pub(crate) struct FootMatching { pub(crate) usage: u32, pub(crate) start: i32, pub(crate) end: i32 }

/// Remaining native owners not stored in PedestrianConstruction yet. Integrator
/// must bind these to retained original state/Vault/RNG/audio/waypoint services.
/// Missing services must return errors, including optional-interface lookup that
/// has not actually occurred. This contract is NOT proof of live integration.
pub(crate) trait ExternalOwners {
    fn next_random_u32(&mut self) -> Result<u32, String>;
    fn bernoulli_random(&mut self, probability: f32) -> Result<bool, String>;
    fn drop_probability(&mut self, entity: &PedestrianConstruction) -> Result<f32, String>;
    fn interaction_hand_prop(&mut self, entity: &PedestrianConstruction) -> Result<Option<u64>, String>;
    fn set_hand_prop_to_spawn(&mut self, entity: &mut PedestrianConstruction, key: u64) -> Result<(), String>;
    fn log_spawn_hand_prop(&mut self, entity: &PedestrianConstruction) -> Result<(), String>;
}

pub(crate) struct LiveServices<'a, N, E> {
    entity: &'a mut PedestrianConstruction,
    snapshot: &'a ActorGraphIntents,
    cvars: &'a InputCvarReferences,
    numeric: &'a mut N,
    external: &'a mut E,
}
impl<'a, N, E> LiveServices<'a, N, E> {
    pub(crate) fn new(entity: &'a mut PedestrianConstruction, context: &BoundMotionGraphContext<PedestrianIdentity>,
        snapshot: &'a ActorGraphIntents, cvars: &'a InputCvarReferences, numeric: &'a mut N, external: &'a mut E,
    ) -> Result<Self, String> {
        let context_entity = context.entity().ok_or("pedestrian motion context entity absent")?;
        let animation = entity.animation.as_ref().ok_or("pedestrian animation not constructed")?;
        if !context_entity.is_same_issuance(&entity.authority.entity_handle())
            || !context_entity.is_same_issuance(animation.core.entity()) {
            return Err("pedestrian motion services have foreign entity authority".into());
        }
        Ok(Self { entity, snapshot, cvars, numeric, external })
    }
    fn animation(&self) -> Result<&EntityAnimation, String> {
        self.entity.animation.as_ref().ok_or_else(|| "pedestrian animation not constructed".into())
    }
    fn animation_mut(&mut self) -> Result<&mut EntityAnimation, String> {
        self.entity.animation.as_mut().ok_or_else(|| "pedestrian animation not constructed".into())
    }
    fn foot_matching(&self) -> Result<Option<FootMatching>, String> {
        Ok(self.animation()?.registered_attributes.map(|a| FootMatching {
            usage: a.foot_usage, start: a.start_foot, end: a.end_foot,
        }))
    }
    /// GetCurrentBranchWindow82e6e668, authored order and inclusive bounds.
    fn current_branch_foot(&self) -> Result<Option<i32>, String> {
        let animation = self.animation()?;
        let Some(attributes) = &animation.registered_attributes else { return Ok(None); };
        if attributes.branch_count <= 0 { return Ok(None); }
        let windows = attributes.branches.get(..attributes.branch_count as usize)
            .ok_or("pedestrian authored branch count exceeds fixed three windows")?;
        let base = animation.core.base().ok_or("pedestrian base absent")?;
        let root = base.body_root.as_ref().ok_or("pedestrian animation absent")?;
        let time = animation.core.body_reference_clock(root, true).map_err(|e| format!("pedestrian branch time: {e:?}"))?;
        // Raw82e6e6f4/fc BLT rejection, then82e6e708 BLE acceptance.
        Ok(windows.iter().find(|w| !(w.start<0.) && !(time<w.start) && time<=w.end).map(|w| w.foot))
    }
    /// RegisterAnimAttributes82e79ba8 clears the retained pointer before type
    /// acceptance. Metadata uses the semantic name, before RemapAnim/RNG.
    fn register_animation_attributes(&mut self, name: &str) -> Result<(), String> {
        let animation = self.animation_mut()?;
        animation.registered_attributes = None;
        animation.registered_attributes = animation.selected_animation_data()?
            .try_pedestrian_anim_attributes(skate2_formats::attrib::string_hash64(name))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    /// GetAnimInfoWithSpecialTags82e80e58 and shared float lookup82e7c168.
    fn special_number_tag(&self, tag: &[u8]) -> Result<f32, String> {
        let name = std::str::from_utf8(tag).map_err(|e| e.to_string())?;
        let key = skate2_formats::attrib::string_hash64(name);
        let animation = self.animation()?;
        if key == skate2_formats::attrib::string_hash64("RegisteredFrameOffset") {
            return Ok(animation.motion_intents.frame_offset);
        }
        if key == skate2_formats::attrib::string_hash64("RegisteredTransitionInfo_TransitionBlendTime") {
            return Ok(animation.motion_intents.registered_clips.transition.as_ref().map_or(0., |v| v.blend_time));
        }
        Ok(f32::from_bits(animation.selected_animation_data()?.try_scalar_word(key, 0)
            .map_err(|e| e.to_string())?.unwrap_or(0)))
    }
    fn transitioning(&self) -> Result<bool, String> {
        self.animation()?.core.try_is_transitioning().map_err(|e| format!("pedestrian Transitioning: {e:?}"))
    }
    fn check_owner(&self, owner: &MotionGraphEntityAuthority<PedestrianIdentity>) -> Result<(), String> {
        if owner.is_same_issuance(&self.entity.authority.entity_handle()) { Ok(()) }
        else { Err("pedestrian animation interface has foreign authority".into()) }
    }
}
impl<N, E> HasIntentCollaborator for LiveServices<'_, N, E> {
    type Error = String;
    type MotionGraph = MotionGraphEntityAuthority<PedestrianIdentity>;
    type Intents = ();
    fn cached_motion_graph(&mut self) -> Result<Option<Self::MotionGraph>, String> { Ok(Some(self.entity.authority.entity_handle())) }
    fn find_motion_graph(&mut self) -> Result<Option<Self::MotionGraph>, String> { Ok(Some(self.entity.authority.entity_handle())) }
    fn motion_graph_intents(&mut self, owner: &Self::MotionGraph) -> Result<(), String> { self.check_owner(owner) }
    fn lookup_intent(&mut self, _: &(), name: &[u8]) -> Result<Option<f32>, String> {
        Ok(self.snapshot.get(&ActorGraphIntents::native_name(name)))
    }
}
impl<N, E: ExternalOwners> WillExpireCollaborator for LiveServices<'_, N, E> {
    type Error = String;
    type AnimInfo = MotionGraphEntityAuthority<PedestrianIdentity>;
    type Animatable = MotionGraphEntityAuthority<PedestrianIdentity>;
    fn find_anim_info(&mut self) -> Result<Option<Self::AnimInfo>, String> { Ok(Some(self.entity.authority.entity_handle())) }
    fn resolve_special_tag(&mut self, owner: &Self::AnimInfo, tag: &[u8]) -> Result<f32, String> {
        self.check_owner(owner)?;
        self.special_number_tag(tag)
    }
    fn cached_animatable(&mut self) -> Result<Option<Self::Animatable>, String> {
        self.animation()?;
        Ok(Some(self.entity.authority.entity_handle()))
    }
    fn find_animatable(&mut self) -> Result<Option<Self::Animatable>, String> { self.cached_animatable() }
    fn transitioning(&mut self, owner: &Self::Animatable) -> Result<bool, String> {
        self.check_owner(owner)?;
        LiveServices::transitioning(self)
    }
    fn anim_property(&mut self, owner: &Self::Animatable) -> Result<Option<AnimProperty>, String> {
        self.check_owner(owner)?;
        let p = &self.animation()?.properties;
        Ok(Some(AnimProperty { is_playing_raw: u8::from(p.is_playing), has_expired_or_looped_raw: u8::from(p.expired_or_looped),
            has_expired_or_looped_by_time: p.expired_by, time_to_expire_or_loop: p.time_to_expire }))
    }
}
impl<N, E: ExternalOwners> conditions::Services for LiveServices<'_, N, E> {
    fn has_plugin(&mut self) -> Result<bool, String> { Ok(self.entity.plugin.is_some()) }
    fn has_animatable(&self) -> bool { self.entity.animation.is_some() }
    fn branch_attributes(&self) -> Result<Option<conditions::BranchAttributes<'_>>, String> {
        Ok(self.animation()?.registered_attributes.as_ref().map(|a| conditions::BranchAttributes {
            count: a.branch_count, windows: &a.branches,
        }))
    }
    fn branch_info(&mut self, name: &str) -> Result<Option<conditions::BranchInfo>, String> {
        // Shared GetAnimInfo<tAnimBranchInfo>82705660 returns DefaultDataArea8
        // on a missing attribute, rather than a missing interface.
        let data = self.animation()?.selected_animation_data()?
            .try_get_data(skate2_formats::attrib::string_hash64(name), 0).map_err(|e| e.to_string())?;
        let (kind,time) = if let Some(data) = data {
            let bytes = data.bytes();
            if bytes.len() != 8 { return Err("pedestrian branch info width".into()); }
            (u32::from_be_bytes(bytes[..4].try_into().expect("branch kind")),
                f32::from_bits(u32::from_be_bytes(bytes[4..].try_into().expect("branch time"))))
        } else { (0,0.) };
        Ok(Some(conditions::BranchInfo { kind, time }))
    }
    fn branch_window_duration(&self) -> Result<f32, String> { self.cvars.try_animation_branch_window().map_err(|e| format!("branch window cvar: {e:?}")) }
    fn animation_time(&self) -> Result<f32, String> {
        let animation = self.animation()?;
        let base = animation.core.base().ok_or("pedestrian Animatable base absent")?;
        let root = base.body_root.as_ref().ok_or("pedestrian base animation absent")?;
        animation.core.body_reference_clock(root, true).map_err(|e|format!("pedestrian animation time: {e:?}"))
    }
    fn animation_transitioning(&self)->Result<bool,String>{self.transitioning()}
    fn animation_expiration(&self)->Result<(bool,f32),String>{
        let properties=&self.animation()?.properties;
        Ok((properties.expired_or_looped,properties.time_to_expire))
    }
    fn is_jumping(&self) -> Result<Option<bool>, String> {
        Ok(Some(self.entity.spatial_adjust.as_ref().ok_or("pedestrian SpatialAdjust absent")?.jump.is_jumping))
    }
    fn is_jumping_downward(&self) -> Result<Option<bool>, String> {
        Ok(Some(self.entity.spatial_adjust.as_ref().ok_or("pedestrian SpatialAdjust absent")?.jump.downward_motion
            .ok_or("pedestrian jump phase was never produced")?))
    }
}
impl<N: ReckoningNumeric, E: ExternalOwners> behaviours::Services for LiveServices<'_, N, E>
where N::Error: std::fmt::Debug {
    fn has_entity(&self) -> bool { true }
    fn state(&mut self) -> Result<&mut PedestrianStateStorage, String> { self.entity.state.as_mut().ok_or_else(|| "pedestrian state absent".into()) }
    fn intents(&mut self) -> Result<(&mut IntentQueue, &mut AiGraphInput), String> {
        Ok((self.entity.intent_queue.as_mut().ok_or("pedestrian intent queue absent")?,
            &mut self.entity.animation.as_mut().ok_or("pedestrian animation absent")?.ai_input))
    }
    fn complete_locomotion_turn(&mut self) -> Result<(), String> { self.animation_mut()?.complete_locomotion_turn(); Ok(()) }
    fn complete_locomotion_transition(&mut self) -> Result<(), String> { self.animation_mut()?.complete_locomotion_transition(); Ok(()) }
    fn trigger_jump(&mut self) -> Result<(), String> { self.entity.jump_now(self.cvars, self.numeric) }
    fn jump_time(&mut self, downward: bool) -> Result<f32, String> {
        let jump = &self.entity.spatial_adjust.as_ref().ok_or("pedestrian SpatialAdjust absent")?.jump;
        Ok(if downward { jump.down_time } else { jump.up_time })
    }
    fn stop_channels(&mut self, blend: f32) -> Result<(), String> { self.animation_mut()?.channels.stop(blend); Ok(()) }
    fn set_attempting_takedown(&mut self, value: bool) -> Result<(), String> {
        //82e6fd68: state+3240 bit2, then inline StopChannels(.2) when true.
        let state = self.entity.state.as_mut().ok_or("pedestrian state absent")?;
        state.flags[0] = (state.flags[0] & !2) | if value { 2 } else { 0 };
        if value { self.animation_mut()?.channels.stop(0.2); }
        Ok(())
    }
    fn has_hand_prop(&mut self) -> Result<bool, String> { self.entity.has_hand_prop() }
    fn drop_on_large_avoid_probability(&mut self) -> Result<f32, String> { self.external.drop_probability(self.entity) }
    fn bernoulli_random(&mut self, probability: f32) -> Result<bool, String> { self.external.bernoulli_random(probability) }
    fn throw_hand_prop_away(&mut self) -> Result<(), String> { self.entity.throw_hand_prop_away(self.cvars) }
    fn interaction_hand_prop(&mut self) -> Result<Option<u64>, String> { self.external.interaction_hand_prop(self.entity) }
    fn set_hand_prop_to_spawn(&mut self, key: u64) -> Result<(), String> { self.external.set_hand_prop_to_spawn(self.entity, key) }
    fn log_spawn_vending_machine_hand_prop(&mut self) -> Result<(), String> { self.external.log_spawn_hand_prop(self.entity) }
    fn set_speech_state(&mut self, state: u32) -> Result<(), String> {
        // SendSpeechEvent826ed200 -> Entity vslotb8 -> SetAudioState82e94ca8.
        // The native callback only publishes the retained audio state output.
        self.entity.state_out.state = state;
        Ok(())
    }
}

impl<N, E: ExternalOwners> LiveServices<'_, N, E> {
    fn remap(&mut self, name: &[u8]) -> Result<FastString36, String> {
        let name = std::str::from_utf8(name).map_err(|e| e.to_string())?;
        let animation = self.animation()?;
        let registered = animation.motion_intents.registered_clips.clone();
        let data = animation.selected_animation_data()?.clone();
        let mut failure = None;
        let mapped = crate::pedestrian_animation::remap_animation(name, &registered, &data, || match self.external.next_random_u32() {
            Ok(word) => word, Err(error) => { failure = Some(error); 0 }
        })
            .map_err(|e| e.to_string())?;
        if let Some(error) = failure { return Err(error); }
        FastString36::encode(&mapped).map_err(|e| format!("pedestrian remapped name: {e:?}"))
    }
    fn number_tag(&mut self, value: u32, tag: &[u8]) -> Result<f32, String> {
        if tag.is_empty() { Ok(f32::from_bits(value)) } else { self.special_number_tag(tag) }
    }
    fn play(&mut self, kind: Transition, name: FastString36, speed: f32, time: f32, blend: f32) -> Result<bool, String> {
        let core = &mut self.animation_mut()?.core;
        match kind {
            Transition::Play => core.try_play_body_animation(name, speed, time),
            Transition::Sequence => core.try_sequence_body_animation(name, speed, time, 0),
            Transition::Blend => core.try_blend_body_animation(name, speed, blend, time, false, 0),
        }.map_err(|e| format!("pedestrian playback: {e:?}"))
    }
    fn missing_animation(&mut self) -> Result<(), String> {
        if self.cvars.try_pedestrian_assert_on_missing_animation().map_err(|e| format!("missing animation cvar: {e:?}"))? { return Err("LWAnimHelper native missing-animation assertion".into()); }
        let name = FastString36::encode("HIP_TEST_01").expect("native fallback name");
        if self.play(Transition::Play, name, 1., 0., 0.)? { Ok(()) }
        else { Err("LWAnimHelper native HIP_TEST_01 assertion".into()) }
    }
}
impl<N, E: ExternalOwners> PlayRemappedAnimationCollaborator for LiveServices<'_, N, E> {
    type Error = String;
    fn write_mirror_override_raw(&mut self, value: u8) -> Result<(), String> {
        self.animation_mut()?.mirrored = value != 0;
        Ok(())
    }
    /// LWAnimHelper::SetupRemappedAnimation82e90730. Interface owners are known
    /// present on the retained Pedestrian; actual missing storage is an error.
    fn setup_remapped_animation(&mut self, request: RemappedAnimationSetup<'_>) -> Result<(), String> {
        let from = self.foot_matching()?;
        let from_window = self.current_branch_foot()?;
        self.register_animation_attributes(std::str::from_utf8(request.animation_name).map_err(|e| e.to_string())?)?;
        let to = self.foot_matching()?;
        if let (Some(from), Some(to)) = (from, to) {
            // MirrorToMatchFeet82e6d370; PDB enums Into1 Out2 Both3, feet-1/+1.
            if matches!(from.usage, 2 | 3) && matches!(to.usage, 1 | 3)
                && to.start != from_window.unwrap_or(from.end) {
                let animation = self.animation_mut()?;
                animation.mirrored = !animation.mirrored;
            }
        }
        let name = self.remap(request.animation_name)?;
        let time = self.number_tag(request.frame_offset_bits, request.frame_offset_tag)?;
        let override_params = &self.animation()?.transition_override;
        let mut kind = request.transition_type;
        let mut blend = f32::from_bits(request.blend_length_bits);
        match override_params.transition_type {
            1 => kind = Transition::Play,
            2 => { kind = Transition::Blend; blend = override_params.blend_length; }
            4 => kind = Transition::Sequence,
            _ => {},
        }
        self.animation_mut()?.transition_override.clear_override();
        if self.play(kind, name, f32::from_bits(request.playback_speed_bits), time, blend)? {
            let cycle = self.number_tag(request.cycle_percentage_bits, request.cycle_percentage_tag)?;
            // Pedestrian IAnimOperation vtable823a4e30 slots+4/+8/+12
            // (Play/Sequence/Blend notification) all point to raw blr825187c0.
            self.animation_mut()?.motion_intents.cycle_swapping_percentage = cycle;
            Ok(())
        } else { self.missing_animation() }
    }
    /// Update82e90b50 does NOT set speed every frame. It conditionally selects
    /// another original remap on loop completion, retaining the global RNG order.
    fn update_remapped_animation(&mut self, request: RemappedAnimationUpdate<'_>) -> Result<(), String> {
        let probability = self.animation()?.motion_intents.cycle_swapping_percentage;
        let probability = if -probability >= 0. { 0. } else { probability };
        let probability = if 1. - probability >= 0. { probability } else { 1. };
        if probability > 0. && !LiveServices::transitioning(self)? && self.animation()?.properties.expired_or_looped
            && self.external.bernoulli_random(probability)? {
            let name = self.remap(request.animation_name)?;
            if !self.play(Transition::Play, name, f32::from_bits(request.playback_speed_bits), 0., 0.)? {
                self.missing_animation()?;
            } //success notification is the same native noop825187c0
        }
        Ok(())
    }
}
