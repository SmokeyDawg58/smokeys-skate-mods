//! Original TrickDisplayHUD826c04b0/826bd9d8/826bda40 change notifications.
//! Values come from the completed TricksBackEnd/TrickDisplayManager update;
//! this owner never calculates trick points or advances simulation meters.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum ScreenState {
    UpdateTrickScoring = 7,
    UpdateMultiplier = 10,
    UpdateLineDisplay = 11,
    Show = 18,
    Hide = 19,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HudValues {
    pub multiplier: f32,
    pub line_score: u32,
    pub line_timer_points: i32,
    pub line_timer_max_points: i32,
    pub sequence_score: u32,
    pub total_tricks: u32,
    // Native826bdb24 compares TrickDisplayManager stance to Regular(0).
    pub regular_stance: bool,
    pub fakie: bool,
    pub nollie: bool,
    pub can_update_stance: bool,
}

#[derive(Debug, Default)]
pub struct TrickDisplayHud {
    visible: bool,
    last_multiplier: f32,
    last_score: u32,
    last_line_score: u32,
    last_switch_stance: bool,
    last_fakie: bool,
    line_score_surpassed_max_display: bool,
    last_nollie: bool,
    last_total_tricks: u32,
}
impl TrickDisplayHud {
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Native Show always sends the state, even when visibility is unchanged.
    pub fn show(&mut self, visible: bool, commands: &mut Vec<ScreenState>) {
        self.visible = visible;
        commands.push(if visible {
            ScreenState::Show
        } else {
            ScreenState::Hide
        });
    }

    pub fn update(&mut self, value: HudValues, commands: &mut Vec<ScreenState>) {
        if !self.visible {
            return;
        }
        if self.last_multiplier != value.multiplier {
            commands.push(ScreenState::UpdateMultiplier);
        }
        self.last_multiplier = value.multiplier;
        if self.last_line_score != value.line_score || self.line_score_surpassed_max_display {
            self.line_score_surpassed_max_display = false;
            commands.push(ScreenState::UpdateLineDisplay);
        }
        self.last_line_score = value.line_score;
        // Signed comparison in native826bdafc. The remembered flag causes a
        // refresh on the NEXT update, including when the meter just fell below.
        if value.line_timer_points >= value.line_timer_max_points {
            self.line_score_surpassed_max_display = true;
        }
        if self.last_score != value.sequence_score
            || self.last_total_tricks != value.total_tricks
            || self.last_fakie != value.fakie
            || (value.can_update_stance && self.last_switch_stance != value.regular_stance)
            || self.last_nollie != value.nollie
        {
            self.last_switch_stance = value.regular_stance;
            self.last_fakie = value.fakie;
            self.last_nollie = value.nollie;
            commands.push(ScreenState::UpdateTrickScoring);
            self.last_total_tricks = value.total_tricks;
        }
        self.last_score = value.sequence_score;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn values() -> HudValues {
        HudValues {
            multiplier: 1.0,
            line_timer_max_points: 400,
            ..Default::default()
        }
    }
    #[test]
    fn hidden_updates_do_not_consume_changes_and_notifications_preserve_order() {
        let mut hud = TrickDisplayHud::default();
        let mut commands = Vec::new();
        let value = HudValues {
            multiplier: 2.,
            line_score: 750,
            sequence_score: 50,
            total_tricks: 3,
            ..values()
        };
        hud.update(value, &mut commands);
        assert!(commands.is_empty());
        hud.show(true, &mut commands);
        hud.update(value, &mut commands);
        assert_eq!(
            commands,
            [
                ScreenState::Show,
                ScreenState::UpdateMultiplier,
                ScreenState::UpdateLineDisplay,
                ScreenState::UpdateTrickScoring
            ]
        );
        commands.clear();
        hud.update(value, &mut commands);
        assert!(commands.is_empty());
    }
    #[test]
    fn line_overflow_refresh_is_delayed_and_survives_leaving_maximum() {
        let mut hud = TrickDisplayHud::default();
        let mut commands = Vec::new();
        hud.show(true, &mut commands);
        hud.update(values(), &mut commands);
        commands.clear();
        hud.update(
            HudValues {
                line_timer_points: 400,
                ..values()
            },
            &mut commands,
        );
        assert!(commands.is_empty());
        hud.update(values(), &mut commands);
        assert_eq!(commands, [ScreenState::UpdateLineDisplay]);
        commands.clear();
        hud.update(values(), &mut commands);
        assert!(commands.is_empty());
    }
    #[test]
    fn stance_gate_blocks_only_stance_changes_and_does_not_lose_them() {
        let mut hud = TrickDisplayHud::default();
        let mut commands = Vec::new();
        hud.show(true, &mut commands);
        hud.update(values(), &mut commands);
        commands.clear();
        let switched = HudValues {
            regular_stance: true,
            ..values()
        };
        hud.update(switched, &mut commands);
        assert!(commands.is_empty());
        hud.update(
            HudValues {
                can_update_stance: true,
                ..switched
            },
            &mut commands,
        );
        assert_eq!(commands, [ScreenState::UpdateTrickScoring]);
    }
}
