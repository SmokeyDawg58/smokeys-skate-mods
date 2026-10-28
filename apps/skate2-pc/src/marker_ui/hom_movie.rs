//! Shipped hud2/homscoring ActionScript, admitted by FNV 65f4830a.
use super::{apt_random::AptRandom, assets::Assets, clips::Clip, score_movie::format_score};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::hom_scoring::{HomFrame, HomHudData, ScreenState};

pub(super) struct HomMovie {
    pub root: Clip,
    visible: bool,
    animating: bool,
    metrics: [Option<usize>; 5],
    next_metric: usize,
    pub sounds: Vec<&'static str>,
}

impl HomMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, 3, 0)?;
        root.playback.stop();
        root.named(&["mScore"])?.goto(assets, "endanim")?;
        root.named(&["mScore"])?.playback.stop();
        let mut this = Self {
            root, visible: true, animating: false, metrics: [None; 5], next_metric: 0,
            sounds: Vec::new(),
        };
        this.show(assets, false)?;
        this.callbacks()?;
        Ok(this)
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)?;
        self.callbacks()
    }

    fn callbacks(&mut self) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "HomAnimStarted" => self.animating = true,
                "HomAnimFinished" => self.animating = false,
                _ => return Err(format!("untranslated HoM callback {call}").into()),
            }
        }
        Ok(())
    }

    pub fn update(&mut self, assets: &Assets, frames: &[HomFrame], random: &mut AptRandom) -> RenderResult<()> {
        self.sounds.clear();
        for frame in frames {
            for state in &frame.commands {
                match state {
                    ScreenState::Show => self.show(assets, true)?,
                    ScreenState::Hide => self.show(assets, false)?,
                    ScreenState::Refresh => self.refresh(assets, &frame.data, random)?,
                    ScreenState::ChallengesCompleted => {
                        self.root.named(&["mScore", "mThrasher_bg"])?.goto(assets, "challenge")?;
                        self.sounds.push("FE_SOUND_HOM_SCORE_RED");
                    }
                    ScreenState::ChallengesCleared => {
                        self.root.named(&["mScore", "mThrasher_bg"])?.goto(assets, "normal")?;
                    }
                }
                self.callbacks()?;
            }
        }
        Ok(())
    }

    fn show(&mut self, assets: &Assets, visible: bool) -> RenderResult<()> {
        if self.visible == visible { return Ok(()); }
        self.visible = visible;
        self.root.visible = visible;
        self.root.goto(assets, if visible { "intro" } else { "outro" })?;
        if !visible { self.reset(assets)?; }
        Ok(())
    }

    fn reset(&mut self, assets: &Assets) -> RenderResult<()> {
        for i in 0..5 {
            let metric = self.root.named(&[&format!("mMetric{i}")])?;
            metric.goto(assets, "hide")?;
            metric.visible = false;
            metric.named(&["mPanel", "statText"])?.text = Some(Vec::new());
            for color in ["White", "Black"] {
                let sticker = self.root.named(&[&format!("mBonus{color}{i}")])?;
                sticker.goto(assets, "hide")?;
                sticker.visible = false;
            }
        }
        self.metrics = [None; 5];
        self.next_metric = 0;
        Ok(())
    }

    fn refresh(&mut self, assets: &Assets, data: &HomHudData, random: &mut AptRandom) -> RenderResult<()> {
        let label = self.root.named(&["mScore", "mActual", "label"])?;
        // AS slices the literal '#' prefix, then toInteger stops at grouping.
        // Clip stores rendered units without that prefix.
        let old = String::from_utf16(label.text.as_deref().unwrap_or(&[]))?;
        let old = old.chars().take_while(|c| c.is_ascii_digit()).collect::<String>()
            .parse::<i32>().unwrap_or(0);
        if old < data.score as i32 { self.sounds.push("FE_SOUND_HOM_SCORE_SCROLL"); }
        label.text = Some(signed_number(assets, data.score as i32)?);
        if !self.animating { self.root.named(&["mScore"])?.goto(assets, "startanim")?; }
        self.callbacks()?;
        self.bonus_metrics(assets, data)?;
        for (kind, count, scores) in [
            ("car", data.new_car_collisions, &data.car_collision_score),
            ("dmo", data.new_dmo_collisions, &data.dmo_collision_score),
        ] {
            for score in scores.iter().take(count as usize) { self.popup(assets, kind, *score, random)?; }
        }
        if data.new_body_tweak { self.popup(assets, "tweak", data.body_tweak_score, random)?; }
        for (kind, count, scores) in [
            ("ped", data.new_ped_collisions, &data.ped_collision_score),
            ("skater", data.new_skater_collisions, &data.skater_collision_score),
        ] {
            for score in scores.iter().take(count as usize) { self.popup(assets, kind, *score, random)?; }
        }
        Ok(())
    }

    fn bonus_metrics(&mut self, assets: &Assets, data: &HomHudData) -> RenderResult<()> {
        let scores = data.bonus_scores();
        let values = data.bonus_data();
        for kind in 0..5 {
            // Shipped script uses speed threshold for drop and duration for
            // rotation; preserve those actual references, not unused fields.
            let threshold = [1, 1, 5, 1, 1][kind];
            if scores[kind] > threshold {
                if let Some(index) = self.metrics[kind] {
                    self.root.named(&[&format!("mMetric{index}"), "mPanel", "scoreText"])?.text =
                        Some(signed_number(assets, scores[kind])?);
                } else {
                    let index = self.next_metric;
                    self.next_metric += 1;
                    self.metrics[kind] = Some(index);
                    let metric = self.root.named(&[&format!("mMetric{index}")])?;
                    metric.named(&["mPanel", "mIcon"])?.goto(assets, ["speed", "drop", "air", "duration", "rotation"][kind])?;
                    metric.visible = true;
                    metric.goto(assets, "scoring")?;
                    // Original first-activation branch does not set scoreText.
                }
            }
            if let Some(index) = self.metrics[kind] {
                let text = metric_text(assets, kind, values[kind])?;
                self.root.named(&[&format!("mMetric{index}"), "mPanel", "statText"])?.text =
                    Some(text.encode_utf16().collect());
            }
        }
        Ok(())
    }

    fn popup(&mut self, assets: &Assets, kind: &str, score: f32, random: &mut AptRandom) -> RenderResult<()> {
        let mut available = false;
        for i in 0..5 {
            for color in ["White", "Black"] {
                available |= !self.root.named(&[&format!("mBonus{color}{i}")])?.visible;
            }
        }
        if !available { return Ok(()); }
        // AS round(random()*5) also yields absent index5; retry that undefined
        // instance exactly as the authored visibility comparison does.
        let name = loop {
            let index = (random.next() * 5.).round() as usize;
            let color = if random.next().round() == 1. { "White" } else { "Black" };
            if index == 5 { continue; }
            let name = format!("mBonus{color}{index}");
            if !self.root.named(&[&name])?.visible { break name; }
        };
        let clip = self.root.named(&[&name])?;
        clip.named(&["mBonus", "mIcon"])?.goto(assets, kind)?;
        clip.named(&["mBonus", "scoreText"])?.text = Some(signed_number(assets, score.round() as i32)?);
        clip.visible = true;
        clip.goto(assets, "scoring")?;
        Ok(())
    }
}

fn signed_number(assets: &Assets, number: i32) -> RenderResult<Vec<u16>> {
    let mut out = format_score(assets, number.unsigned_abs())?;
    if number < 0 { out.insert(0, u16::from(b'-')); }
    Ok(out)
}

fn metric_text(assets: &Assets, kind: usize, value: f32) -> RenderResult<String> {
    match kind {
        0 | 1 => {
            // ProcessSpeed8255a3b8 / ProcessDistance8255a2b8. Current retained
            // settings use the original Imperial default (825644d8).
            let (label, token, factor, units) = if kind == 0 {
                ("ID_HOM_SPEED_VALUE", "{0:speed}", 2.25, "ID_COMMON_MPH")
            } else {
                ("ID_COMMON_METRIC_METERS", "{0:distance}", 3., "ID_COMMON_METRIC_NEW_FEET")
            };
            let pattern = assets.localize(label)?;
            if pattern != token { return Err(format!("unsupported HoM metric pattern {pattern}").into()); }
            let number = String::from_utf16(&signed_number(assets, (value * factor).round() as i32)?)?;
            Ok(format!("{number} {}", assets.localize(units)?))
        }
        2 | 3 => {
            // GetLocalizedTime82604280 + BuildTimeFormatString82980e30:
            // truncate hundredths, omit zero minutes, retain two second digits.
            if assets.localize("ID_COMMON_TIME_FORMAT")? != "hh:mm:ss.ff" {
                return Err("unsupported HoM time pattern".into());
            }
            let value = value.max(0.);
            let hours = (value * f32::from_bits(0x3991a2b4)) as i32;
            let minutes = value.mul_add(f32::from_bits(0x3c888889), -(hours * 60) as f32) as i32;
            let seconds = (value - (hours * 3600) as f32 - (minutes * 60) as f32) as i32;
            let hundredths = ((value - value.trunc()) * 100.) as u32;
            let time = if minutes > 0 { format!("{minutes:02}:{seconds:02}.{hundredths:02}") }
                else if seconds > 0 || hundredths > 0 { format!("{seconds:02}.{hundredths:02}") }
                else { String::new() };
            Ok(format!("{time}{}", assets.localize("ID_COMMON_METRIC_TIME")?))
        }
        _ => Ok(String::new()), // Authored rotation statText = '#'.
    }
}
