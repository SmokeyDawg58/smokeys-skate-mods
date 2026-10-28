//! Actor constructor825fef00..825ff0d0: CAC selection and body/hand Initialize.
use super::{ActorConstruction, ActorConstructionError, ActorPrefixPhase, Arc, Mutex};
use crate::{
    actor_animation_construction::initialization::{
        ActorAnimationInitializeError, ActorAnimationInitializeInputs,
        ActorAnimationInitializeSource,
    },
    actor_face_playback::{FaceCommandManager, FacePlaybackData, FacePlaybackManager},
    simulation_animation_startup::AnimationAttributePool,
};

/// Actual backend queries are deliberately separate: named pro skaters must
/// never cause custom gestures/style reads. Source `GetCACSettings`825f0770.
pub(crate) trait ActorAnimationStartSource: ActorAnimationInitializeSource {
    fn is_stance_regular(&mut self, player: u32) -> Result<bool, Self::Error>;
    fn skater_id(&mut self, player: u32) -> Result<u64, Self::Error>;
    fn player_gestures(&mut self, player: u32) -> Result<[u32; 4], Self::Error>;
    fn player_style(&mut self, player: u32) -> Result<u32, Self::Error>;
    fn command_manager(&mut self) -> Result<Arc<Mutex<FaceCommandManager>>, Self::Error>;
    fn playback_manager(&mut self) -> Result<Arc<Mutex<FacePlaybackManager>>, Self::Error>;
}

pub(crate) struct ActorAnimationStart {
    stance: Option<u32>,
    gestures: [u32; 4],
    pro: Option<&'static [u8]>,
    body_returned: bool,
    left_returned: bool,
    right_returned: bool,
}
#[derive(Debug)]
pub(crate) enum ActorAnimationStartError<E> {
    Actor(ActorConstructionError),
    Source(E),
    Lock,
    InvalidStyle(u32),
    WrongPhase(ActorPrefixPhase),
    Body(ActorAnimationInitializeError<E>),
    Hand(ActorAnimationInitializeError<std::convert::Infallible>),
}
impl<E> From<ActorConstructionError> for ActorAnimationStartError<E> {
    fn from(error: ActorConstructionError) -> Self {
        Self::Actor(error)
    }
}
impl<E: std::fmt::Debug> std::fmt::Display for ActorAnimationStartError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "Actor animation startup: {error}"),
            Self::Source(error) => write!(f, "Actor animation source: {error:?}"),
            Self::Lock => f.write_str("Actor animation startup owner lock poisoned"),
            Self::InvalidStyle(style) => write!(f, "Actor CAC style assertion: {style}"),
            Self::WrongPhase(phase) => write!(f, "Actor animation startup phase: {phase:?}"),
            Self::Body(error) => write!(f, "Actor body Initialize: {error:?}"),
            Self::Hand(error) => write!(f, "Actor hand Initialize: {error:?}"),
        }
    }
}

impl ActorConstruction {
    pub(crate) fn try_initialize_animation<S: ActorAnimationStartSource>(
        &self,
        source: &mut S,
        shared_attributes: Option<Arc<Mutex<AnimationAttributePool>>>,
    ) -> Result<(), ActorAnimationStartError<S::Error>> {
        use ActorAnimationStartError as Error;
        let mut prefix = self.prefix()?;
        if prefix.phase != ActorPrefixPhase::AnimationConstructorReturned {
            return Err(Error::WrongPhase(prefix.phase));
        }
        prefix.phase = ActorPrefixPhase::AnimationInitializing;
        let player = prefix
            .fields
            .as_ref()
            .expect("constructed Actor fields")
            .player_index;
        let retained = Arc::clone(prefix.animation.as_ref().expect("published animation"));
        // CAC GestureSet.Reset82561d80 runs before querying the backend and
        // again inside GetCACSettings. Its allowed flags remain local only.
        prefix.animation_start = Some(ActorAnimationStart {
            stance: None,
            gestures: [0, 1, 2, 3],
            pro: None,
            body_returned: false,
            left_returned: false,
            right_returned: false,
        });
        let start = prefix
            .animation_start
            .as_mut()
            .expect("retained caller continuation");
        let regular = source.is_stance_regular(player).map_err(Error::Source)?;
        start.stance = Some(u32::from(!regular));
        start.gestures = [0, 1, 2, 3];
        let id = source.skater_id(player).map_err(Error::Source)?;
        start.pro = match pro_skater_name(id) {
            Some(name) => Some(name),
            None if id == u64::MAX => {
                start.gestures = source.player_gestures(player).map_err(Error::Source)?;
                match source.player_style(player).map_err(Error::Source)? {
                    0 => None,
                    1 => Some(b"Loose"),
                    2 => Some(b"Aggressive"),
                    style => return Err(Error::InvalidStyle(style)),
                }
            }
            _ => None,
        };
        let command = source.command_manager().map_err(Error::Source)?;
        let body_manager = source.playback_manager().map_err(Error::Source)?;
        let scheduler = Arc::clone(&command.lock().map_err(|_| Error::Lock)?.scheduler);
        let playback = lookup_captured(&body_manager, b"Skater", player)?;
        let inputs = ActorAnimationInitializeInputs {
            playback,
            scheduler: Some(scheduler),
            shared_attributes,
            stance: start.stance.expect("source stance"),
            gestures: start.gestures,
            pro_skater: start.pro.map(|name| {
                crate::play_animation::FastString30::encode(
                    std::str::from_utf8(name).expect("source literal ASCII"),
                )
                .expect("source pro name fits FastString30")
            }),
        };
        let mut animation = retained.lock().map_err(|_| Error::Lock)?;
        animation
            .try_initialize_body(inputs, source)
            .map_err(Error::Body)?;
        start.body_returned = true;
        animation
            .try_initialize_hand(false, lookup(source, b"SkaterLeftHand", 0)?)
            .map_err(Error::Hand)?;
        start.left_returned = true;
        animation
            .try_initialize_hand(true, lookup(source, b"SkaterRightHand", 0)?)
            .map_err(Error::Hand)?;
        start.right_returned = true;
        prefix.phase = ActorPrefixPhase::AnimationInitialized;
        Ok(())
    }
}

fn lookup<S: ActorAnimationStartSource>(
    source: &mut S,
    name: &[u8],
    player: u32,
) -> Result<Option<Arc<Mutex<FacePlaybackData>>>, ActorAnimationStartError<S::Error>> {
    let manager = source
        .playback_manager()
        .map_err(ActorAnimationStartError::Source)?;
    lookup_captured(&manager, name, player)
}

fn lookup_captured<E>(
    manager: &Arc<Mutex<FacePlaybackManager>>,
    name: &[u8],
    player: u32,
) -> Result<Option<Arc<Mutex<FacePlaybackData>>>, ActorAnimationStartError<E>> {
    let manager = manager.lock().map_err(|_| ActorAnimationStartError::Lock)?;
    let key = (
        crate::actor_graph_construction::ActorGraphIntents::native_name(name),
        i32::from_ne_bytes(player.to_ne_bytes()),
    );
    Ok(manager.entries.get(&key).map(Arc::clone))
}

/// Full 64-bit character comparisons in `GetCACSettings`825f07e0..825f08dc.
pub(crate) fn pro_skater_name(id: u64) -> Option<&'static [u8]> {
    match id {
        0x1d93_9610_fea0_90ee => Some(b"JerryHsu"),
        0x0ded_8332_42f2_f857 => Some(b"JasonDill"),
        0x146d_3b7d_8838_5267 => Some(b"MikeCarroll"),
        0x1733_a97b_7064_d4c2 => Some(b"PJLadd"),
        0x4254_5b09_38e3_41c9 => Some(b"DannyWay"),
        0xbe33_09ce_9ba4_1097 => Some(b"RobDyrdek"),
        _ => None,
    }
}

#[cfg(test)]
mod character_identity_tests {
    use super::pro_skater_name;
    #[test]
    fn original_full_character_ids_do_not_alias_their_low_words() {
        let originals: [(u64, &[u8]); 6] = [
            (0x1d93_9610_fea0_90ee, b"JerryHsu"),
            (0x0ded_8332_42f2_f857, b"JasonDill"),
            (0x146d_3b7d_8838_5267, b"MikeCarroll"),
            (0x1733_a97b_7064_d4c2, b"PJLadd"),
            (0x4254_5b09_38e3_41c9, b"DannyWay"),
            (0xbe33_09ce_9ba4_1097, b"RobDyrdek"),
        ];
        for (id, name) in originals {
            assert_eq!(pro_skater_name(id), Some(name));
            assert_eq!(pro_skater_name(id & 0xffff_ffff), None);
            assert_eq!(pro_skater_name(id ^ (1 << 32)), None);
        }
        assert_eq!(pro_skater_name(u64::MAX), None);
        assert_eq!(pro_skater_name(u64::from(u32::MAX)), None);
    }
}
