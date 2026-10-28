//! Original ChaseMan registration/gating and shared Chaser/Chasee retained state.
//! Entities and navigation stay with the existing LivingWorld owners.
#[path = "living_world_chase_attributes.rs"]
pub(crate) mod attributes;
#[path = "living_world_chase_conditions.rs"]
pub(crate) mod conditions;
#[path = "living_world_chase_formation.rs"]
pub(crate) mod formation;
#[path = "living_world_chase_iterator.rs"]
pub(crate) mod iterator;
#[path = "living_world_chase_prediction.rs"]
pub(crate) mod prediction;
#[path = "living_world_chase_frame.rs"]
mod frame;
#[path = "living_world_chase_update.rs"]
pub(crate) mod update;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TargetableId {
    pub(crate) kind: i32,
    pub(crate) id: i32,
}
impl TargetableId {
    pub(crate) const INVALID: Self = Self { kind: -1, id: -1 };
    pub(crate) fn is_valid(self) -> bool {
        self.kind != -1 && self.id >= 0
    }
}
#[derive(Debug)]
pub(crate) enum ChaseError {
    Capacity,
    InvalidChaser,
    DuplicateChaser,
}
impl std::fmt::Display for ChaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "chase: {self:?}")
    }
}
impl std::error::Error for ChaseError {}

/// Side effects returned to the real challenge/mood/stats consumers. Callers
/// must dispatch them; returning these never claims those consumers have run.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ChaseEvent {
    LocalChaseStart,
    LocalChaseEnd,
    ChaserMoodStarted(TargetableId),
    ChaserMoodEnded(TargetableId),
    PersonalBestChaserCount(u32),
    ChaseFinished {
        captured: bool,
        duration: f32,
        offboard_duration: f32,
    },
    PreventManualRespawn(i32),
    SetJackedOverlay(bool),
    ShowJackedOverlay,
    RespawnActor {
        actor_id: i32,
        authored_override: Option<[f32; 16]>,
    },
}

/// Callbacks run at their original mutation boundaries, not after accumulating
/// an event vector. A checked queue failure retains only the completed prefix.
pub(crate) trait ChaseRegistrationServices {
    fn dispatch(&mut self, event: ChaseEvent) -> Result<(), String>;
    /// Resolve the actual pedestrian and read entity attribute79d412ca0b043f37.
    fn suppress_chaser_mood(&mut self, id: TargetableId) -> Result<bool, String>;
    fn local_capture(&mut self) -> Result<Option<bool>, String>;
}

pub(crate) struct ChaseMan {
    group_parameters: Option<crate::attribute_runtime::AttributeRuntimeInstance>,
    chasers: Vec<TargetableId>,
    /// Native separate mNumChasers+6304, changed after registration messages.
    num_chasers: u32,
    warners: Vec<TargetableId>,
    tazers: Vec<TargetableId>,
    flags: u8,
    local_enabled: bool,
    jacked_actor: Option<i32>,
    jacked_timer: f32,
    closest_jacked_spawn: Option<[f32; 16]>,
    pub(crate) current_chase_timer: f32,
    pub(crate) current_offboard_chase_timer: f32,
    pub(crate) prevent_local_chase_timer: f32,
}
impl Default for ChaseMan {
    /// ctor82715770 flags+6332=0x10, local flag+6333=0x80, timers0.
    fn default() -> Self {
        Self {
            group_parameters: None,
            chasers: Vec::new(),
            num_chasers: 0,
            warners: Vec::new(),
            tazers: Vec::new(),
            flags: 0x10,
            local_enabled: true,
            jacked_actor: None,
            jacked_timer: 0.0,
            closest_jacked_spawn: None,
            current_chase_timer: 0.0,
            current_offboard_chase_timer: 0.0,
            prevent_local_chase_timer: 0.0,
        }
    }
}
impl ChaseMan {
    pub(crate) fn chase_is_on(&self) -> bool {
        self.num_chasers != 0
    }
    pub(crate) fn threat_is_on(&self) -> bool {
        self.chase_is_on() || !self.warners.is_empty() || self.flags & 0x80 != 0
    }
    pub(crate) fn taze_is_on(&self) -> bool {
        !self.tazers.is_empty()
    }
    pub(crate) fn are_new_chases_allowed(&self) -> bool {
        self.flags & 0x10 != 0
    }
    pub(crate) fn are_chases_forced_to_terminate(&self) -> bool {
        self.flags & 8 != 0
    }
    pub(crate) fn are_local_chases_forced_to_terminate(&self) -> bool {
        self.flags & 4 != 0
    }
    /// Chasee.GetGroupEndChaseReason82de1de0. The local override returns
    /// eEndChaseReason_default (0) without changing the retained reason.
    /// The all-chases query at82de1df4 has no consumed result in this build.
    pub(crate) fn group_end_reason(&self, stored: i32, local: bool) -> i32 {
        if local && self.are_local_chases_forced_to_terminate() { 0 } else { stored }
    }
    pub(crate) fn is_skater_being_jacked(&self) -> bool {
        self.flags & 0x80 != 0
    }
    pub(crate) fn chasers(&self) -> &[TargetableId] {
        &self.chasers
    }
    pub(crate) fn warners(&self) -> &[TargetableId] {
        &self.warners
    }
    /// ReceiveDisallowLocalChaseMsg826f3e58; Allow826f3e80 does not clear termination.
    pub(crate) fn disallow_local_chase(&mut self) {
        self.flags = (self.flags & 0xeb) | 4;
        self.local_enabled = false;
    }
    pub(crate) fn allow_local_chase(&mut self) {
        self.flags |= 0x10;
        self.local_enabled = true;
    }
    /// Reset82702260 leaves registrations intact and asks active chases to end.
    pub(crate) fn reset(&mut self, vault: &std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>) -> Result<(), String> {
        let parameters = vault.instance(attributes::CHASE_CLASS, skate2_formats::attrib::string_hash64("global"));
        let missing = parameters.is_null();
        self.group_parameters = Some(parameters);
        if missing { return Err("ChaseMan.Reset global chase collection assertion827022a0".into()); }
        if self.chase_is_on() {
            self.flags |= 4;
        }
        self.flags = (self.flags & 0x67) | 0x10;
        self.local_enabled = true;
        self.jacked_timer = 0.0;
        Ok(())
    }
    /// GetLocalCaptureChaseSuppressionTime82702330 reads retained group layout+4.
    pub(crate) fn capture_suppression_time(&self) -> Result<f32, String> {
        let Some(parameters) = self.group_parameters.as_ref().filter(|p| !p.is_null()) else { return Ok(0.); };
        Ok(parameters.try_cached_layout_field(4, 4).map_err(|e|e.to_string())?
            .map_or(0., |v| f32::from_be_bytes(v.bytes().try_into().expect("four-byte chase field"))))
    }
    /// Lazy native ChaseMan getters used only by significant primary comparisons.
    pub(crate) fn primary_switch_thresholds(&self)->Result<(f32,f32),String>{
        Ok((self.group_scalar(0xf570_f5ed_bee3_b9d9)?,self.group_scalar(0x1fcd_3fdf_4c3b_5f06)?))
    }
    pub(crate) fn primary_auto_switch_time(&self)->Result<f32,String>{
        self.group_scalar(0xb21f_d5a3_4d5b_f776)
    }
    /// GetMinBehindChaseAngleToUseInterceptPrediction826f4110.
    pub(crate) fn min_behind_intercept_angle(&self)->Result<f32,String>{
        self.group_scalar(0x5d48_0f3f_fc02_e98a)
    }
    fn group_scalar(&self,key:u64)->Result<f32,String>{
        let parameters=self.group_parameters.as_ref().ok_or("ChaseMan group parameters absent")?;
        Ok(f32::from_bits(parameters.try_scalar_word(key,0).map_err(|e|e.to_string())?.unwrap_or(0)))
    }
    pub(crate) fn register_warner(&mut self, id: TargetableId) -> Result<(), ChaseError> {
        if !self.warners.contains(&id) && !self.chasers.contains(&id) {
            insert(&mut self.warners, id)?;
        }
        Ok(())
    }
    pub(crate) fn unregister_warner(&mut self, id: TargetableId) {
        remove(&mut self.warners, id);
    }
    pub(crate) fn register_tazer(&mut self, id: TargetableId) -> Result<(), ChaseError> {
        insert(&mut self.tazers, id).map(|_| ())
    }
    pub(crate) fn unregister_tazer(&mut self, id: TargetableId) {
        remove(&mut self.tazers, id);
    }
    /// RegisterChaser8271b9e0 emits start for EACH newly registered chaser.
    pub(crate) fn register_chaser(
        &mut self,
        id: TargetableId,
        services: &mut impl ChaseRegistrationServices,
    ) -> Result<(), String> {
        if self.chasers.contains(&id) { return Ok(()); }
        services.dispatch(ChaseEvent::LocalChaseStart)?;
        insert(&mut self.chasers, id).map_err(|e| e.to_string())?;
        if id.kind != 1 || !id.is_valid() {
            return Err(ChaseError::InvalidChaser.to_string());
        }
        if !services.suppress_chaser_mood(id)? {
            services.dispatch(ChaseEvent::ChaserMoodStarted(id))?;
        }
        self.num_chasers = self.num_chasers.wrapping_add(1);
        if self.local_enabled && !self.are_local_chases_forced_to_terminate() {
            if self.num_chasers == 1 {
                self.current_chase_timer = 0.0;
                self.current_offboard_chase_timer = 0.0;
            }
            services.dispatch(ChaseEvent::PersonalBestChaserCount(self.num_chasers))?;
        }
        Ok(())
    }
    /// UnRegisterChaser827186e8; local_capture=None means no local Actor.
    pub(crate) fn unregister_chaser(
        &mut self,
        id: TargetableId,
        services: &mut impl ChaseRegistrationServices,
    ) -> Result<(), String> {
        if !remove(&mut self.chasers, id) {
            return Ok(());
        }
        if self.num_chasers == 0 { return Err("UnRegisterChaser native count assertion".into()); }
        self.num_chasers -= 1;
        if id.kind != 1 || !id.is_valid() { return Err(ChaseError::InvalidChaser.to_string()); }
        if !services.suppress_chaser_mood(id)? {
            services.dispatch(ChaseEvent::ChaserMoodEnded(id))?;
        }
        if self.num_chasers == 0 {
            if let Some(captured) = services.local_capture()? {
                if self.local_enabled && !self.are_local_chases_forced_to_terminate() {
                    services.dispatch(ChaseEvent::ChaseFinished {
                        captured,
                        duration: self.current_chase_timer,
                        offboard_duration: self.current_offboard_chase_timer,
                    })?;
                }
                if captured {
                    self.flags |= 4;
                    self.prevent_local_chase_timer = self.capture_suppression_time()?;
                    self.flags |= 0x20;
                } else {
                    self.flags &= !4;
                }
            }
            services.dispatch(ChaseEvent::LocalChaseEnd)?;
        }
        Ok(())
    }

}
fn insert(list: &mut Vec<TargetableId>, id: TargetableId) -> Result<bool, ChaseError> {
    if list.contains(&id) {
        return Ok(false);
    }
    if list.len() == 256 {
        return Err(ChaseError::Capacity);
    }
    list.push(id);
    Ok(true)
}
fn remove(list: &mut Vec<TargetableId>, id: TargetableId) -> bool {
    if let Some(index) = list.iter().position(|value| *value == id) {
        list.remove(index);
        true
    } else {
        false
    }
}

/// Fields owned by tPedestrianStateStorage, not an extra target registry.
#[derive(Debug)]
pub(crate) struct ChaserState {
    pub(crate) chasee: TargetableId,
    pub(crate) end_reason: i32,
}
impl Default for ChaserState {
    fn default() -> Self {
        Self {
            chasee: TargetableId::INVALID,
            end_reason: -1,
        }
    }
}
impl ChaserState {
    /// SetChasee82e9e458 returns whether caller must clear angrychase want.
    pub(crate) fn set_chasee(&mut self, id: TargetableId) -> bool {
        let clear_angry_chase = !id.is_valid() && self.chasee.is_valid();
        self.chasee = id;
        self.end_reason = -1;
        clear_angry_chase
    }
    pub(crate) fn end_chase(&mut self, reason: i32) {
        self.end_reason = reason;
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ChaserInfo {
    pub(crate) id: TargetableId,
    pub(crate) ideal_position: [f32; 3],
    pub(crate) recommended_position: [f32; 3],
    /// tChaserInfo82de1fa8 initializes this vector to zero before prediction.
    pub(crate) intercept_direction: Option<[f32; 3]>,
    pub(crate) intercept_time: f32,
}
#[derive(Debug)]
pub(crate) struct ChaseeState {
    pub(crate) chasers: Vec<ChaserInfo>,
    pub(crate) group_end_reason: i32,
    pub(crate) scared_timer: f32,
    pub(crate) nearby_event_timer: f32,
    pub(crate) block_radius: f32,
    pub(crate) formation_min_radius: f32,
    pub(crate) formation_max_radius: f32,
    pub(crate) formation_velocity_scalar: f32,
}
impl ChaseeState {
    /// ChaseeReset82de2310 reads these three values from authored global chase attributes.
    pub(crate) fn new(min_radius: f32, max_radius: f32, velocity_scalar: f32) -> Self {
        Self {
            chasers: Vec::new(),
            group_end_reason: -1,
            scared_timer: 0.0,
            nearby_event_timer: 0.0,
            block_radius: min_radius,
            formation_min_radius: min_radius,
            formation_max_radius: max_radius,
            formation_velocity_scalar: velocity_scalar,
        }
    }
    /// AddChaser82de4bf0. Entity validation precedes this call. The boolean
    /// tells the caller to register with ChaseMan iff this chasee is local.
    pub(crate) fn add_chaser(
        &mut self,
        id: TargetableId,
        max_chasers: usize,
        chasee_position: [f32; 3],
        chaser_position: [f32; 3],
    ) -> Result<bool, ChaseError> {
        if !id.is_valid() {
            return Err(ChaseError::InvalidChaser);
        }
        if self.chasers.len() >= max_chasers {
            return Ok(false);
        }
        if self.chasers.iter().any(|entry| entry.id == id) {
            return Err(ChaseError::DuplicateChaser);
        }
        if self.chasers.len() == 5 {
            return Err(ChaseError::Capacity);
        }
        let delta = std::array::from_fn(|i| chaser_position[i] - chasee_position[i]);
        self.chasers.push(ChaserInfo {
            id,
            ideal_position: delta,
            recommended_position: delta,
            //tChaserInfo ctor82de1fa8 stores zero into all three vectors;
            //raw stfs sites omitted from Hex-Rays' VMX reconstruction.
            intercept_direction: Some([0.;3]),
            intercept_time: f32::MAX,
        });
        if self.chasers.len() == 1 {
            self.group_end_reason = -1;
        }
        Ok(true)
    }
    /// RemoveChaser82de4990 prefix. The caller unregisters from ChaseMan
    /// before finish_chaser_removal82de49f8 clears the captured end reason.
    pub(crate) fn erase_chaser(&mut self, id: TargetableId) -> bool {
        let Some(index) = self.chasers.iter().position(|entry| entry.id == id) else { return false; };
        self.chasers.remove(index);
        true
    }
    pub(crate) fn finish_chaser_removal(&mut self) {
        if self.chasers.is_empty() { self.group_end_reason = -1; }
    }
}

/// Source IChaser getters used in ShouldSwapChaser82de3b98. Values come from
/// the live entity's motion/state storage and native interception calculation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChaserAssessment {
    pub(crate) id: TargetableId,
    pub(crate) primary_timeout: f32,
    pub(crate) wiped_out: bool,
    pub(crate) colliding: bool,
    pub(crate) resting: bool,
    pub(crate) failed_takedown: bool,
    pub(crate) intercept_time: f32,
}
fn should_swap(
    left: ChaserAssessment,
    right: ChaserAssessment,
    timeout_improvement: f32,
    intercept_improvement: f32,
) -> bool {
    let l = if left.primary_timeout <= 0.0 {
        0.0
    } else {
        left.primary_timeout
    };
    let r = if right.primary_timeout <= 0.0 {
        0.0
    } else {
        right.primary_timeout
    };
    if (l - r).abs() > intercept_improvement {
        return l > r + intercept_improvement;
    }
    for (l, r) in [
        (left.wiped_out, right.wiped_out),
        (left.colliding, right.colliding),
        (left.resting, right.resting),
        (left.failed_takedown, right.failed_takedown),
    ] {
        if l != r {
            return l && !r;
        }
    }
    left.intercept_time - right.intercept_time > timeout_improvement
        && r >= l + intercept_improvement
}
impl ChaseeState {
    /// EvaluateBestChasers82de3fc0 sorts secondaries only. Intercept times have
    /// already been written individually by the concrete prediction pass.
    pub(crate) fn rank_secondaries(
        &mut self,
        assessments: &[ChaserAssessment],
    ) -> Result<(), ChaseError> {
        let mut values = Vec::with_capacity(self.chasers.len());
        for chaser in &self.chasers {
            let value = assessments
                .iter()
                .find(|entry| entry.id == chaser.id)
                .ok_or(ChaseError::InvalidChaser)?;
            values.push(*value);
        }
        let count = self.chasers.len();
        for i in 1..count.saturating_sub(1) {
            for j in (i + 1..count).rev() {
                if should_swap(values[j - 1], values[j], 0.0, 0.0) {
                    values.swap(j - 1, j);
                    self.chasers.swap(j - 1, j);
                }
            }
        }
        Ok(())
    }
    /// AutoSwitchPrimaryChaser82de50f8. Return the old primary's timeout write;
    /// caller must complete it BEFORE finish_primary_switch mutates the roster.
    pub(crate) fn primary_switch_candidate(&self,assessments:&[ChaserAssessment],
        timeout_improvement:f32,intercept_improvement:f32)->Result<Option<TargetableId>,ChaseError>{
        if self.chasers.len()<2 {return Ok(None);}
        let read=|index:usize|assessments.iter().find(|v|v.id==self.chasers[index].id)
            .copied().ok_or(ChaseError::InvalidChaser);
        let left=read(0)?;let right=read(1)?;
        if !should_swap(left,right,timeout_improvement,intercept_improvement){return Ok(None);}
        Ok(Some(left.id))
    }
    /// InternalChangePrimaryChaser82de5018..50e0, after SetTimeUntilPrimary.
    pub(crate) fn finish_primary_switch(&mut self,old:TargetableId)->Result<(),ChaseError>{
        if self.chasers.len()<2 || self.chasers[0].id!=old {return Err(ChaseError::InvalidChaser);}
        self.chasers.swap(0,1);
        let previous=self.chasers.remove(1);
        self.chasers.push(previous);
        Ok(())
    }
}
