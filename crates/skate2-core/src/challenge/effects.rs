//! `Sk8::Challenge::cChallengeEffectsManager`: the out-of-bounds tint and the
//! challenge fades. Each update posts the fade amount the presentation draws
//! (`cMsgFadeAmount`); a finished effect posts `cMsgEffectFinished`.

use super::{
    hud::VISIBILITY_CHALLENGE_EFFECTS,
    system::{Event, Field, Message, World},
};

/// `eChallengeEffect`.
pub const OOB: usize = 0;
pub const FADE_IN: usize = 1;
pub const FADE_OUT: usize = 2;
pub const FADE_OUT_FADE_IN: usize = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EffectParams {
    pub active: bool,
    pub current: f32,
    pub total: f32,
    pub stay_persistent: bool,
}

#[derive(Debug, Default)]
pub struct EffectsManager {
    pub params: [EffectParams; 4],
    /// This update's `cMsgFadeAmount` (screen black amount, 0..1), sent only
    /// while a fade runs; `VDWriter::WriteData` (`0x827ff8c8`) passes it on
    /// once and forgets it.
    pub fade_amount: Option<f32>,
    /// The last out-of-bounds amount (`cMsgSetSessionMarker`-typed message).
    pub out_of_bounds_amount: f32,
}

fn effect_finished(effect: usize) -> Event {
    #[expect(clippy::cast_precision_loss, reason = "enum value")]
    Event::Challenge(Message::new("cMsgEffectFinished", vec![("Effect", Field::Number(effect as f32))]))
}

/// `StartEffect` (`0x82da81d0`).
pub fn start_effect(world: &mut World, effect: usize, total: f32) {
    if effect >= 4 {
        return;
    }
    if effect != OOB {
        world.pause_menu_disabled = true;
        world.hud.set_visible(false, VISIBILITY_CHALLENGE_EFFECTS);
    }
    if world.effects.params[FADE_OUT].stay_persistent {
        if effect == FADE_OUT {
            world.send(effect_finished(FADE_OUT));
        } else if effect != OOB {
            world.effects.params[FADE_OUT] = EffectParams::default();
        }
    }
    let p = &mut world.effects.params[effect];
    p.current = total;
    p.active = true;
    p.total = total;
}

/// `ResetEffect` (`0x82da82d0`).
pub fn reset_effect(world: &mut World, effect: usize) {
    if effect >= 4 {
        return;
    }
    if effect != OOB {
        world.pause_menu_disabled = false;
        world.hud.set_visible(true, VISIBILITY_CHALLENGE_EFFECTS);
    }
    if world.effects.params[effect].active {
        world.effects.params[effect] = EffectParams::default();
        world.send(effect_finished(effect));
    }
}

/// `cChallengeEffectsManager::Update` (`0x82daa1a0`).
pub fn update(world: &mut World, dt: f32) {
    world.effects.fade_amount = None;
    for i in 0..4 {
        let p = world.effects.params[i];
        if !p.active {
            continue;
        }
        let current = p.current - dt;
        world.effects.params[i].current = current;
        match i {
            OOB => world.effects.out_of_bounds_amount = (p.total - current) / p.total,
            FADE_IN => world.effects.fade_amount = Some(current / p.total),
            FADE_OUT => {
                world.effects.fade_amount = Some(if p.stay_persistent { 1.0 } else { (p.total - current) / p.total });
            }
            _ => {
                let half = p.total * 0.5;
                world.effects.fade_amount = Some(if current <= half { current / half } else { (p.total - current) / half });
            }
        }
        if current < 0.001 {
            if i == FADE_OUT {
                if !world.effects.params[FADE_OUT].stay_persistent {
                    world.effects.params[FADE_OUT].stay_persistent = true;
                    world.send(effect_finished(FADE_OUT));
                }
            } else {
                reset_effect(world, i);
            }
        }
    }
}
