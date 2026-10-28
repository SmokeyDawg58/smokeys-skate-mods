//! Translated shipped hud2/trickdisplay2 ActionScript (fingerprint58c4b20b).
//! Uses the same retained APT instances, fonts and GPU batches as the phone.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;
#[path = "score_context.rs"]
mod context_movie;
use skate2_core::ui::{
    trick_backend::{PhysicsFrame, TricksBackend},
    trick_display::{DisplayTuning, MovieCommand, TrickDisplay},
    trick_display_hud::{HudValues, ScreenState, TrickDisplayHud},
};

pub(super) struct ScoreMovie {
    pub root: Clip,
    pub(super) landing_notice: super::landing_notice::LandingNotice,
    backend: TricksBackend,
    // Finished points remain readable through the original trick-name outro.
    // This is presentation state; native scoring and online facts are unchanged.
    banked_score: Option<u32>,
    display: TrickDisplay,
    hud: TrickDisplayHud,
    force_show: bool,
    open: bool,
    stance_initted: bool,
    stance_dirty: bool,
    last_switch: bool,
    last_fakie: bool,
    active_index: usize,
    target_index: usize,
    displayed: usize,
    outro: bool,
    // Native audio commands retained for the future frontend audio consumer.
    pub sounds: Vec<&'static str>,
    pub jolt: Option<f32>,
    context: skate2_core::ui::trick_context::ContextDisplay,
    context_visible: [bool; 7],
    context_millisecond_remainder: f64,
}
impl ScoreMovie {
    pub fn take_jacked_request(&mut self)->bool{
        if !self.backend.is_jacked||!self.backend.can_show_jacked_overlay{return false;}
        self.backend.is_jacked=false;self.backend.can_show_jacked_overlay=false;true
    }
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, 2, 0)?;
        for i in 0..4 { root.named(&[&format!("mcTrickAnim{i}")])?.visible = false; }
        // Constructor still calls bigIcon_mc.gotoAndPlay("iconOff"), but
        // this shipped movie has no such instance: native undefined call has
        // no display effect. Do not manufacture the removed icon.
        root.named(&["mcScoreModes", "mcLinescore", "mcLinetitle"])?.text =
            Some(assets.localize("ID_SCORING_HUD_LINE")?.encode_utf16().collect());
        root.named(&["mcScoreModes", "mcLinescore", "labelText"])?.text = Some(vec![48]);
        align_line_score(assets, &mut root)?;
        root.goto(assets, "empty")?;
        root.defer_frame_actions();
        Ok(Self {
            root, landing_notice: super::landing_notice::LandingNotice::new()?,
            backend: Default::default(), banked_score: None, display: Default::default(), hud: Default::default(),
            force_show: true, open: false, stance_initted: false, stance_dirty: false,
            last_switch: false, last_fakie: false, active_index: 0, target_index: 0,
            displayed: 0, outro: false, sounds: Vec::new(), jolt: None,
            context: Default::default(), context_visible: [false; 7], context_millisecond_remainder: 0.,
        })
    }
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.landing_notice.tick();
        self.root.tick(assets)?;
        // Apt tick82a21c10 advances the display list BEFORE RunActions.
        // HUD SetScreenState calls after AptIntegration::Update82640220
        // leave goto frame actions queued until this next authored step.
        // Executing a goto's stop immediately lets consecutive x1 requests
        // (reset 0 -> 1) run the ring through later multiplier colours.
        while self.root.run_next_frame_action(assets)? {
            self.callbacks(assets)?;
        }
        self.callbacks(assets)
    }
    fn callbacks(&mut self, assets: &Assets) -> RenderResult<()> {
        let mut calls = Vec::new(); self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "TrickDisplayTrickNameOutroDone" => {
                    if self.outro { self.banked_score = None; }
                    self.outro = false; self.displayed = 0;
                }
                "TrickDisplayLineTimerOutroDone" => self.root.named(&["multiplier_mc"])?.goto(assets,"clear")?,
                _ => return Err(format!("untranslated scoring callback {call}").into()),
            }
        }
        Ok(())
    }
    /// `shown` is the bound TrickDisplayHUD instance's visibility, none while unbound.
    pub fn update(&mut self, assets: &Assets, frames: &[PhysicsFrame], tuning: DisplayTuning, shown: Option<bool>) -> RenderResult<()> {
        self.sounds.clear(); // Per-update commands, never an unconsumed history.
        let mut commands = Vec::new();
        for frame in frames {
            self.landing_notice.observe(frame,tuning.allowed && shown == Some(true));
            if frame.sequence_begun || frame.begun.is_some() || frame.converted.is_some()
                || frame.just_wiped_out || frame.reactivated {
                self.banked_score = None;
            }
            if frame.sequence_completed && !frame.just_wiped_out && !frame.interrupted
                && frame.finished_sequence_score > 0 && self.open
                && tuning.allowed && shown == Some(true) {
                self.banked_score = Some(frame.finished_sequence_score);
            }
            for entry in &frame.context_entries { self.context.add(entry.clone()); }
            let milliseconds = self.context_millisecond_remainder + f64::from(frame.simulation_rate) * 1000.;
            self.context_millisecond_remainder = milliseconds.fract();
            self.context.update(milliseconds as u32);
            let mut events = Vec::new();
            self.backend.consume(frame, &mut events)?;
            self.backend.update_timers(frame.simulation_rate, false);
            for event in events { self.display.event(&mut self.backend, event, tuning, &mut commands); }
        }
        let mut states = Vec::new();
        // HUDManager2::Update (0x82cee4f0) shows or hides a bound TrickDisplayHUD to its
        // instance's visibility, then updates it, which does nothing while it is hidden
        // (0x826bda40); an unbound one is neither shown nor updated.
        if let Some(shown) = shown {
            if self.hud.visible() != shown { self.hud.show(shown, &mut states); }
            self.hud.update(HudValues {
                multiplier: self.backend.multiplier, line_score: self.backend.line_score,
                line_timer_points: self.backend.line_points, line_timer_max_points: self.backend.line_max_points,
                sequence_score: self.display_score(), total_tricks: self.backend.total_tricks,
                regular_stance: !self.display.switched, fakie: self.display.fakie,
                nollie: self.display.is_nollie(), can_update_stance: self.display.can_update_stance,
            }, &mut states);
        }
        for state in states {
            match state {
                ScreenState::Show | ScreenState::Hide => {
                    self.stance_initted = false; self.update_stance(assets)?;
                    self.force_show = state == ScreenState::Show;
                    self.root.visible = self.force_show;
                    if self.force_show { self.update_line(assets)?; }
                }
                ScreenState::UpdateMultiplier => self.update_multiplier(assets)?,
                ScreenState::UpdateLineDisplay => self.update_line(assets)?,
                ScreenState::UpdateTrickScoring => { self.update_score(assets)?; self.update_stance(assets)?; }
            }
        }
        // FrontEndManager82640130 updates HUDManager2 before TrickDisplayManager.
        self.display.update(&mut self.backend, tuning, &mut |label| assets.localize(label), &mut commands)?;
        if self.context.take_changed() && tuning.allowed {
            context_movie::refresh(assets, &mut self.root, &self.context, &mut self.context_visible)?;
        }
        for command in commands {
            match command {
                MovieCommand::RefreshTrick { text, new_trick } => {
                    if new_trick { self.refresh_name(assets, &text)?; }
                    else { self.modify_name(assets, &text)?; }
                }
                MovieCommand::Close => {
                    if self.open { self.root.goto(assets, "bail")?; }
                    self.open = false;
                }
                MovieCommand::Jolt { distance } => self.jolt = Some(distance),
                // gActionScriptCalls82fdb148 entries5/6 target obsolete
                // HUDHooks.TrickDisplay. Shipped hudbase never assigns that
                // object; the new HUD sends SetState7 for stance/score itself.
                // EndSequence also starts the retained names' authored outro.
                MovieCommand::UpdateStance => {},
                MovieCommand::ActivateScoring => {
                    if self.banked_score.is_some() { self.clear(assets)?; }
                },
            }
        }
        self.callbacks(assets)
    }
    fn update_multiplier(&mut self, assets: &Assets) -> RenderResult<()> {
        if !self.force_show { return Ok(()); }
        let (label, sound) = match self.backend.multiplier {
            1.5 => ("x1.5", Some("FE_SOUND_MULTIPLYER_15")),
            2.0 => ("x2", Some("FE_SOUND_MULTIPLYER_2")),
            3.0 => ("x3", Some("FE_SOUND_MULTIPLYER_3")),
            _ => ("x1", None),
        };
        if let Some(sound) = sound { self.sounds.push(sound); }
        // The AS body also addresses x1Left/x1Right/TL2/BL/BR, but those
        // instances were removed from the shipped timer symbol56. Calls on
        // undefined have no visual effect; retain the three authored clips.
        // Authored call order: background, TL, TR, glow, score backers.
        self.root.named(&["multiplier_mc", "multiBG_mc"])?.goto(assets,label)?;
        for child in ["mcSpecialTL", "mcSpecialTR", "glow_mc"] {
            self.root.named(&["multiplier_mc", "multiTimer_mc", child])?.goto(assets,label)?;
        }
        for child in ["scoreBack_mc", "backerHighlight_mc"] {
            self.root.named(&["mcScoreModes", child])?.goto(assets,label)?;
        }
        Ok(())
    }
    fn update_line(&mut self, assets: &Assets) -> RenderResult<()> {
        if !self.force_show { return Ok(()); }
        // Native translator82605e50 returns an INTEGER, before AS multiplies
        // by60. maxFrames500 is authored class data at0x23000.
        let seconds = self.backend.line_seconds as i32;
        let frames = (seconds as i64 * 60 - 30).clamp(0,500);
        if seconds > 0 {
            self.root.named(&["multiplier_mc"])?.goto(assets,"all")?;
            self.root.named(&["multiplier_mc", "multiTimer_mc"])?.goto_number(assets,(500-frames) as usize)?;
        }
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn test_banked_score(&self) -> Option<u32> { self.banked_score }
    fn display_score(&self) -> u32 { self.banked_score.unwrap_or(self.backend.sequence_score) }
    fn update_score(&mut self, assets: &Assets) -> RenderResult<()> {
        if !self.force_show { return Ok(()); }
        let score = self.display_score();
        if score > 0 {
            let score_clip = self.root.named(&["mcScoreModes", "mcScore"])?;
            score_clip.goto(assets, "largetext")?;
            if score >= 10000 { score_clip.goto(assets,"smalltext")?; }
            let text = format_score(assets,score)?;
            for label in ["labelText", "labelTextGlow"] { score_clip.named(&[label])?.text = Some(text.clone()); }
            self.root.named(&["mcScoreModes"])?.goto(assets,"set")?;
            if self.backend.line_seconds as i32 <= 0 {
                self.root.named(&["multiplier_mc"])?.goto(assets,"clear")?;
                self.root.named(&["multiplier_mc", "multiTimer_mc"])?.goto_number(assets,500)?;
            }
        } else {
            self.clear(assets)?;
            self.root.named(&["mcScoreModes"])?.goto(assets,"hide")?;
        }
        let line = self.root.named(&["mcScoreModes", "mcLinescore"])?;
        let visible = self.backend.line_score > 0;
        line.named(&["mcLinetitle"])?.visible = visible;
        line.named(&["labelText"])?.visible = visible;
        if visible {
            line.goto(assets,"large")?;
            // The retained assets/localizer currently select English.
            if self.backend.line_score >= 10000 { line.goto(assets,"small")?; }
            line.named(&["labelText"])?.text = Some(format_score(assets,self.backend.line_score)?);
            align_line_score(assets,&mut self.root)?;
        }
        Ok(())
    }
    fn update_stance(&mut self, assets: &Assets) -> RenderResult<()> {
        if !self.force_show && self.stance_initted { return Ok(()); }
        let (switched,fakie,nollie) = (self.display.switched,self.display.fakie,self.display.is_nollie());
        if self.last_switch == switched && self.last_fakie == fakie && !self.stance_dirty && self.stance_initted { return Ok(()); }
        self.stance_dirty = false; self.stance_initted = true;
        let clip = self.root.named(&["multiplier_mc", "mcSwitch"])?;
        let label = if switched && !fakie { if nollie { "fakie" } else { "switch" } }
            else if !switched && fakie { if nollie { "switch" } else { "fakie" } } else { "empty" };
        clip.named(&["mcTextSwitch"])?.goto(assets,label)?;
        if switched != fakie {
            if nollie { self.stance_dirty = true; }
            clip.visible = true; clip.goto(assets,"intro")?;
        }
        self.last_switch = switched; self.last_fakie = fakie;
        Ok(())
    }
    fn clear(&mut self, assets: &Assets) -> RenderResult<()> {
        if !self.force_show { return Ok(()); }
        for offset in 0..3 {
            let i = (self.target_index+3-offset)%3;
            let clip = self.root.named(&[&format!("mcTrickAnim{i}")])?;
            if clip.visible { clip.goto(assets,&format!("{offset}Outro"))?; }
        }
        self.outro = true;
        Ok(())
    }
    fn modify_name(&mut self, assets: &Assets, text: &str) -> RenderResult<()> {
        if !self.force_show { return Ok(()); }
        self.set_name(assets,(self.active_index+2)%3,text)
    }
    fn set_name(&mut self, assets: &Assets, i: usize, text: &str) -> RenderResult<()> {
        let clip = self.root.named(&[&format!("mcTrickAnim{i}"),"mcTrickName","mcText"])?;
        let units: Vec<u16> = assets.localize(text)?.encode_utf16().collect();
        for label in ["labelText","labelTextglow"] { clip.named(&[label])?.text = Some(units.clone()); }
        Ok(())
    }
    fn refresh_name(&mut self, assets: &Assets, text: &str) -> RenderResult<()> {
        if !self.force_show { return Ok(()); }
        if !self.open { self.root.goto(assets,"intro")?; }
        self.open = true;
        if self.backend.wiped_out { return self.clear(assets); }
        if self.backend.sequence_sketchy { self.root.goto(assets,"sketchy")?; }
        // RefreshTrickName immediately queues then calls UpdateTrickNames;
        // no delayed consumer or artificial lifetime for the authored names.
        self.target_index = self.active_index;
        self.set_name(assets,self.active_index,text)?;
        self.root.named(&[&format!("mcTrickAnim{}",self.active_index)])?.goto(assets,"0")?;
        if self.displayed > 0 {
            let previous = (self.active_index+2)%3;
            self.root.named(&[&format!("mcTrickAnim{previous}")])?.goto(assets,"0-1")?;
            if self.displayed > 1 {
                self.root.named(&[&format!("mcTrickAnim{}",(previous+2)%3)])?.goto(assets,"1-2")?;
            }
        }
        self.displayed = (self.displayed+1).min(2);
        self.active_index = (self.active_index+1)%3;
        Ok(())
    }
}

fn align_line_score(assets: &Assets, root: &mut Clip) -> RenderResult<()> {
    use skate2_formats::apt::Character;
    let line = root.named(&["mcScoreModes", "mcLinescore"])?;
    let title = line.children.values().find(|c| c.entry.name.as_deref()==Some("mcLinetitle"))
        .ok_or("line title absent")?;
    let Character::Text(text) = title.clip.definition(assets)? else { return Err("line title is not text".into()); };
    let Some(Character::Font { name, .. }) = &assets.movies[2].movie.characters[text.font_id as usize] else { return Err("line title font absent".into()); };
    let font = assets.apt_font(name)?;
    let initial:Vec<u16> = text.initial_text.encode_utf16().collect();
    let units = title.clip.text.as_ref().unwrap_or(&initial);
    let width = units.iter().filter_map(|&u| font.metrics.mapped_glyph(u))
        .map(|g| g.advance * (font.scale[0]*text.height)).sum::<f32>();
    let x = title.entry.matrix.tx + width * title.entry.matrix.a;
    line.children.values_mut().find(|c| c.entry.name.as_deref()==Some("labelText"))
        .ok_or("line score text absent")?.entry.matrix.tx = x;
    Ok(())
}
pub(super) fn format_score(assets: &Assets, score: u32) -> RenderResult<Vec<u16>> {
    // SetupNumberFormat829808e0 reads the original language format; its
    // fallback is XXX,XXX.XX. FormatNumber82980468 inserts that grouping.
    let pattern = assets.localized.get(&skate2_formats::localization::string_hash(b"ID_COMMON_NUMBER_FORMAT"));
    let pattern = pattern.map(|u| String::from_utf16(u)).transpose()?.unwrap_or_else(|| "XXX,XXX.XX".into());
    let grouping = pattern.chars().enumerate().find(|(_,c)| matches!(*c,'.'|','|' '|'\u{a0}'));
    let digits = score.to_string(); let mut out = String::new();
    for (i,c) in digits.chars().enumerate() {
        if let Some((size,separator)) = grouping {
            if size>0 && i>0 && (digits.len()-i)%size==0 { out.push(separator); }
        }
        out.push(c);
    }
    Ok(out.encode_utf16().collect())
}
