//! TrickDisplayManager8261d258/82619580. Authored APT calls are returned to
//! the platform movie owner; this is independent of scoring and glyph drawing.
use super::trick_backend::{DisplayEvent, TrickName, TricksBackend};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status { None, Start, Active, Landed, RestartFromBail }
#[derive(Clone, Debug, PartialEq)]
pub enum MovieCommand {
    RefreshTrick { text: String, new_trick: bool },
    UpdateStance,
    ActivateScoring,
    Close,
    Jolt { distance: f32 },
}
#[derive(Clone, Copy)]
pub struct DisplayTuning {
    pub allowed: bool,
    pub nollie_delay: i32,
    pub jolt_threshold: f32,
    pub jolt_distance: f32,
    pub jolt_speed: f32,
    pub jolt_recovery: f32,
}

pub struct TrickDisplay {
    status: Status,
    current: Option<TrickName>,
    last_index: i32,
    spins: i32,
    flips: i32,
    pub switched: bool,
    pub fakie: bool,
    pub can_update_stance: bool,
    stance_counter: i32,
    special_nollie: bool,
    refresh_stance: bool,
    refresh_count: u32,
    refresh_hud: bool,
    new_trick: bool,
    allow_context_update: bool,
    wipeout: bool,
}
impl Default for TrickDisplay {
    fn default() -> Self {
        // Inlined member constructor in FrontEndManager82635f18.
        Self {
            status: Status::None, current: None, last_index: 999, spins: 0, flips: 0,
            switched: false, fakie: false, can_update_stance: true, stance_counter: 0,
            special_nollie: false, refresh_stance: false, refresh_count: 0,
            refresh_hud: false, new_trick: false, allow_context_update: false, wipeout: false,
        }
    }
}
impl TrickDisplay {
    pub fn is_nollie(&self) -> bool {
        self.status != Status::None && self.current.as_ref().is_some_and(|t| t.valid && t.nollie)
    }
    pub fn event(&mut self, backend: &mut TricksBackend, event: DisplayEvent,
        tuning: DisplayTuning, commands: &mut Vec<MovieCommand>) {
        match event {
            DisplayEvent::Start => self.status = Status::Start,
            DisplayEvent::Landed => self.status = Status::Landed,
            DisplayEvent::EndSequence if tuning.allowed => {
                self.spins = 0; self.flips = 0; self.status = Status::None;
                commands.push(MovieCommand::ActivateScoring);
            }
            DisplayEvent::AirFinished(height) if height >= tuning.jolt_threshold => {
                commands.push(MovieCommand::Jolt { distance: tuning.jolt_distance });
            }
            // Scoring recovery is a one-tick simulation event. The PC owner
            // consumes it even while a challenge hides/unbinds the movie;
            // dropping it would leave mbWipedOut latched for later tricks.
            // This resets logical state and emits no movie command.
            DisplayEvent::ReadyToScore => {
                self.spins = 0; self.flips = 0; self.status = Status::None; self.wipeout = false;
                backend.wiped_out = false; backend.manual_length = 0.; backend.clear_data();
                self.stance_counter = 0; self.special_nollie = false;
                self.switched = backend.switched; self.fakie = backend.fakie;
            }
            _ => {}
        }
    }

    pub fn update<E>(&mut self, b: &mut TricksBackend, tuning: DisplayTuning,
        localize: &mut impl FnMut(&str) -> Result<String, E>, commands: &mut Vec<MovieCommand>) -> Result<(), E> {
        // The folded IsGameInProgress82a738d0 actually loads backend+184
        // (mbWipedOut), confirmed in raw assembly, not networking state.
        if self.status != Status::RestartFromBail && b.wiped_out {
            self.status = Status::RestartFromBail;
            if tuning.allowed {
                self.wipeout = true;
                commands.push(MovieCommand::Close);
                b.clear_data();
            }
            return Ok(());
        }
        match self.status {
            Status::None => self.last_index = 999,
            Status::Start => {
                if !self.can_update_stance { self.refresh_stance = true; }
                self.can_update_stance = true; self.stance_counter = 0; self.special_nollie = false;
                if self.last_index == 999 { b.spins = 0; b.flips = 0; b.manual_length = 0.; }
                self.current = b.current.clone();
                let index = self.current.as_ref().map_or(-1, |t| t.index);
                self.new_trick = index != self.last_index && !b.converted;
                if self.new_trick { self.allow_context_update = true; }
                self.refresh_hud = true; self.status = Status::Active; self.last_index = index;
            }
            Status::Active => {
                if self.spins != b.spins { self.spins = b.spins; self.refresh_hud = true; }
                if self.flips != b.flips { self.flips = b.flips; self.refresh_hud = true; }
            }
            Status::Landed => self.status = Status::None,
            Status::RestartFromBail => return Ok(()),
        }
        if self.switched != b.switched { self.switched = b.switched; self.refresh_stance = true; }
        if self.fakie != b.fakie { self.fakie = b.fakie; self.refresh_stance = true; }
        if !self.can_update_stance {
            self.stance_counter = self.stance_counter.wrapping_add(1);
            if self.stance_counter >= tuning.nollie_delay {
                self.stance_counter = 0; self.can_update_stance = true; self.refresh_stance = true;
            }
        }
        if self.refresh_hud && tuning.allowed {
            self.refresh_name(localize, commands)?;
        }
        if self.refresh_stance && tuning.allowed {
            self.refresh_count = self.refresh_count.wrapping_add(1);
            if self.refresh_count > 2 || self.special_nollie {
                self.refresh_stance = false; self.refresh_count = 0;
                if self.can_update_stance { commands.push(MovieCommand::UpdateStance); }
                if self.special_nollie { self.special_nollie = false; self.can_update_stance = false; }
            }
        }
        Ok(())
    }
    fn refresh_name<E>(&mut self, localize: &mut impl FnMut(&str) -> Result<String, E>,
        commands: &mut Vec<MovieCommand>) -> Result<(), E> {
        let mut cab = false;
        let (mut name, accepts_spin) = if let Some(trick) = &self.current {
            (localize(trick.label.as_deref().unwrap_or("N/A"))?, trick.accepts_spin)
        } else {
            let key = if self.spins == 0 { "ID_TRICK_AIR" }
                else if self.switched == (self.spins < 0) { "ID_TRICK_AIR_BS_SPIN" }
                else { "ID_TRICK_AIR_FS_SPIN" };
            (localize(key)?, self.spins != 0)
        };
        if let Some(trick) = &self.current {
            if trick.nollie {
                if let Some((variant, _)) = &trick.fakie_variant {
                    if self.switched != self.fakie {
                        self.switched = !self.switched; self.fakie = !self.fakie;
                        self.special_nollie = true; self.refresh_stance = true;
                        name = localize(variant)?;
                    }
                }
            }
            let nollie = if self.special_nollie {
                trick.fakie_variant.as_ref().is_some_and(|(_, n)| *n)
            } else { trick.nollie };
            if accepts_spin && self.fakie && !nollie && self.spins != 0 {
                cab = true;
                let key = match (self.spins.unsigned_abs() == 1, self.spins > 0) {
                    (true, true) => "ID_TRICK_AUTHENTIC_FS_HALFCAB",
                    (true, false) => "ID_TRICK_AUTHENTIC_BS_HALFCAB",
                    (false, true) => "ID_TRICK_AUTHENTIC_FS_CAB",
                    (false, false) => "ID_TRICK_AUTHENTIC_BS_CAB",
                };
                name.push(' '); name.push_str(&localize(key)?);
            }
        }
        let show_degrees = if self.spins != 0 && accepts_spin {
            if cab { self.fakie = false; self.spins.unsigned_abs() >= 2 } else { true }
        } else { false };
        let mut text = if show_degrees {
            format!("#{name} {} ", self.spins.unsigned_abs().wrapping_mul(180))
        } else { format!("#{name} ") };
        if self.flips != 0 {
            text.push_str(&localize(if self.flips < 0 { "ID_TRICK_AIR_METRICS_BACKFLIP" }
                else { "ID_TRICK_AIR_METRICS_FRONTFLIP" })?);
        }
        self.refresh_hud = false;
        commands.push(MovieCommand::RefreshTrick { text, new_trick: self.new_trick });
        self.new_trick = false;
        Ok(())
    }
}


#[cfg(test)]
mod recovery_tests {
    use super::*;
    use crate::ui::trick_backend::PhysicsFrame;
    #[test]
    fn scoring_reactivation_while_challenge_hides_display_does_not_latch_bail() {
        let visible = DisplayTuning { allowed: true, nollie_delay: 140,
            jolt_threshold: 0.73, jolt_distance: 50., jolt_speed: 9.98, jolt_recovery: 0.3258 };
        let hidden = DisplayTuning { allowed: false, ..visible };
        for recovery_tuning in [hidden, visible] {
            let mut display=TrickDisplay::default();
            let mut backend=TricksBackend::default();
            let mut commands=Vec::new();
            let mut events=Vec::new();
            backend.consume(&PhysicsFrame { just_wiped_out:true, interrupted:true,
                ..Default::default() }, &mut events).unwrap();
            display.update(&mut backend,visible,&mut |s:&str|Ok::<_,()>(s.to_owned()),&mut commands).unwrap();
            assert_eq!(display.status,Status::RestartFromBail);
            events.clear(); commands.clear();
            backend.consume(&PhysicsFrame { reactivated:true, ..Default::default() },&mut events).unwrap();
            for event in events.drain(..) { display.event(&mut backend,event,recovery_tuning,&mut commands); }
            backend.consume(&PhysicsFrame { begun:Some(TrickName {index:12,valid:true,
                label:Some("ID_TRICK_AIR".into()),..Default::default()}),
                sequence_begun:true,hud_score:100,..Default::default() },&mut events).unwrap();
            for event in events { display.event(&mut backend,event,visible,&mut commands); }
            display.update(&mut backend,visible,&mut |s:&str|Ok::<_,()>(s.to_owned()),&mut commands).unwrap();
            assert!(!backend.wiped_out,"hidden scoring recovery kept the previous bail latched");
            assert!(commands.iter().any(|c|matches!(c,MovieCommand::RefreshTrick {..})),
                "a scored post-reset trick never reached the native HUD");
            assert_eq!(backend.sequence_score,100);
        }
    }
}
