//! Direct pedestrian AI callback stores, checked against native callback bodies.
//! Uses existing state/rig owners; graph lifecycle dispatch remains the caller.
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    living_world_guard_graph::binding,
    living_world_pedestrian_state::timer_type,
    pedestrian_graph_data as data};

pub(crate) enum SimpleBehaviour {
    NativeNoop,
    RestFromChase,
    ClearExhaustion,
    StartInvestigating,
    EndInvestigating,
    SetUnreachableTimer,
    UnsetWantOnEnd { want: usize },
    ClearTakeDownTargetOnEnd,
    SetTimer { timer: u32, length: f32 },
    ClearWaiting,
    SetWaiting,
    BeginReacting,
    EndReacting,
    AllowJumping,
    Speech(u32),
}
impl SimpleBehaviour {
    pub(crate) fn bind(occurrence: u16) -> Result<Option<Self>, String> {
        let name = data::BEHAVIOURS.get(usize::from(occurrence))
            .ok_or("pedestrian behaviour declaration absent")?.name;
        Ok(Some(match name {
            // Registrations82f60768/82f618a8/82f61d28 use the folded
            // factory82c90c88, which installs PrintText2D vtable82300930.
            // Begin/Update/End are all the actual empty function825187c0.
            "ResumeChase" | "SuggestMusicState" | "SpawnConversationArea" => Self::NativeNoop,
            "RestFromChase" => Self::RestFromChase,
            "SendSpeechEvent"=>{
                //Ctor826ed130 copies the debug name, then truncates the
                //float speechvalue and asserts the actual enum range1..71.
                binding::attribute(occurrence,"speechevent")?.ok_or("native speech name absent")?;
                let value=binding::float(occurrence,"speechvalue",Some(0.))?;
                use crate::reckoning_numeric::ReckoningNumeric;
                let value=crate::reckoning_numeric::HostReckoningNumeric.truncate_i32(value).map_err(|e|format!("speech enum: {e:?}"))?;
                if !(1..72).contains(&value){return Err("native speech state assertion826ed1cc".into());}
                Self::Speech(value as u32)
            }
            "ClearExhaustionTimer" => Self::ClearExhaustion,
            "StartInvestigateTimer" => Self::StartInvestigating,
            "EndInvestigateTimer" => Self::EndInvestigating,
            "SetTargetUnreachableTimer" => Self::SetUnreachableTimer,
            "UnsetWantOnEnd" => {
                // Constructor826edec0 requires a named, valid mood result.
                let name = binding::attribute(occurrence, "want")?
                    .ok_or("UnsetWantOnEnd requires want")?;
                Self::UnsetWantOnEnd { want: super::conditions::want_type(name)
                    .ok_or("UnsetWantOnEnd has invalid want")? }
            }
            "TakedownTargetableClearOnEnd" => Self::ClearTakeDownTargetOnEnd,
            // SetSimpleTimer ctor826ec700: defaults TimedRangeRandom and0.
            "SetSimpleTimer" => {
                let name = binding::attribute(occurrence, "timerName")?.unwrap_or("TimedRangeRandom");
                Self::SetTimer { timer: timer_type(name).ok_or("invalid native timer name")?,
                    length: binding::float(occurrence, "length", Some(0.))? }
            }
            "ClearWaitingToReactFlagOnBegin" => Self::ClearWaiting,
            "SetWaitingToReactFlagOnBegin" => Self::SetWaiting,
            "SetIsReactingToMoodEventFlagOnBegin" => Self::BeginReacting,
            "UnsetIsReactingToMoodEventFlagOnEnd" => Self::EndReacting,
            "AllowPedestrianJumping" => Self::AllowJumping,
            _ => return Ok(None),
        }))
    }

    /// Returns false for an unhandled phase; caller must resolve its actual
    /// vtable callback rather than treating that return as native success.
    pub(crate) fn begin(&self, entity: &mut PedestrianConstruction) -> Result<bool, String> {
        if let Self::RestFromChase = self { super::rest::begin(entity)?; return Ok(true); }
        if let Self::NativeNoop = self { return Ok(true); }
        //Begin826ed200 -> Pedestrian.SetAudioState82e94ca8. This is the
        //same StateOut field already consumed by presentation, no audio call.
        if let Self::Speech(value)=self{entity.state_out.state=*value;return Ok(true);}
        if matches!(self, Self::EndReacting | Self::EndInvestigating
            | Self::UnsetWantOnEnd { .. } | Self::ClearTakeDownTargetOnEnd) { return Ok(false); }
        if let Self::AllowJumping = self {
            //826ee898: mpNavRig+321, independent of entity's rig-enabled bit.
            entity.nav_rig.as_mut().ok_or("AllowPedestrianJumping requires rig")?
                .distances.do_jump_test = true;
            return Ok(true);
        }
        // Getters82e6f180/82e6f690 read live resources at Begin, not at bind.
        // A missing collection asserts; absent scalar uses DefaultDataArea(4).
        let authored_timer = match self {
            Self::StartInvestigating | Self::SetUnreachableTimer => {
                let resources = entity.resources.as_ref().ok_or("pedestrian timer resources absent")?;
                let (attributes, key) = if matches!(self, Self::StartInvestigating) {
                    (&resources.entity, 0x1ae2_9113_978b_e5aa)
                } else {
                    (&resources.chase, 0x5a3c_4a77_00dd_a13c)
                };
                if attributes.is_null() { return Err("pedestrian timer collection absent".into()); }
                Some(f32::from_bits(attributes.try_scalar_word(key, 0)
                    .map_err(|e| e.to_string())?.unwrap_or(0)))
            }
            _ => None,
        };
        let state = entity.state.as_mut().ok_or("pedestrian callback requires AI state")?;
        match *self {
            // IChaser adjusted receiver+192:1958 resolves entity1a18, then
            // tChaserData+10 at stateC74. ClearExhaustion82e6fd28.
            Self::ClearExhaustion => state.exhaustion = 0.,
            Self::StartInvestigating => state.set_timer(1, authored_timer.expect("read above"))
                .map_err(|e| e.to_string())?, //826ed9b0
            Self::SetUnreachableTimer => state.set_timer(40, authored_timer.expect("read above"))
                .map_err(|e| e.to_string())?, //8270bd68
            Self::SetTimer { timer, length } => state.set_timer(timer, length).map_err(|e| e.to_string())?,
            // Raw lwz1a18 then lbz/stbCAA = AI flag byte2, not entity flags.
            Self::ClearWaiting => state.flags[2] &= !0x20, //8270be10
            Self::SetWaiting => state.flags[2] |= 0x20, //8270beb0
            Self::BeginReacting => state.flags[2] |= 0x10, //826ee110
            Self::RestFromChase | Self::NativeNoop | Self::EndReacting | Self::EndInvestigating | Self::AllowJumping | Self::Speech(_)
            | Self::UnsetWantOnEnd { .. } | Self::ClearTakeDownTargetOnEnd => unreachable!("handled above"),
        }
        Ok(true)
    }
    pub(crate) fn end(&self, entity: &mut PedestrianConstruction) -> Result<bool, String> {
        match self {
            Self::RestFromChase => super::rest::end(entity)?,
            Self::NativeNoop => {},
            Self::UnsetWantOnEnd { want } => entity.state.as_mut()
                .ok_or("want callback requires AI state")?
                .set_want_active(*want, false).map_err(|e| e.to_string())?, //8270ad08
            Self::ClearTakeDownTargetOnEnd => {
                //8270a5b8 -> ILWTakeDownSource.SetTakeDownSinkId82e7aa50;
                //writes both words at state864, and has no other side effects.
                entity.state.as_mut().ok_or("takedown callback requires AI state")?
                    .take_down_sink = crate::living_world_chase::TargetableId::INVALID;
            }
            Self::EndInvestigating => entity.state.as_mut()
                .ok_or("investigation callback requires AI state")?
                .set_timer(1, 0.).map_err(|e| e.to_string())?, //826eda00
            Self::EndReacting => {
                entity.state.as_mut().ok_or("reaction callback requires AI state")?
                    .flags[2] &= !0x10; //826ee168
            }
            Self::AllowJumping => {
                entity.nav_rig.as_mut().ok_or("AllowPedestrianJumping requires rig")?
                    .distances.do_jump_test = false; //826ee8b0
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
