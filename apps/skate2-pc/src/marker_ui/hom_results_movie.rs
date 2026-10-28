//! Original hudComponents/hom_results, screen46 placement0; fingerprint5b4276e3.
use super::{
    assets::Assets, clips::Clip, hom_skeleton_movie::Skeleton, localizer as loc, map_movie,
};
use crate::gpu_renderer::RenderResult;
use skate2_core::{
    challenge::{hom_results::HomResultsView, mission::HomBail},
    ui::FeSoundRequest,
};
pub(super) const ASSET: usize = 81;

#[derive(Default)]
pub(super) struct HomResultsMovie {
    pub root: Option<Clip>,
    pub open: bool,
    pub sounds: Vec<FeSoundRequest>,
    previous: Option<HomResultsView>,
    skeleton: Option<Skeleton>,
    cash: f32,
    cash_goal: f32,
    cash_step: f32,
    cash_audio: bool,
    pub ready: bool,
}
impl HomResultsMovie {
    pub fn update(&mut self, assets: &Assets, view: Option<&HomResultsView>) -> RenderResult<()> {
        if let Some(view) = view {
            if !self.open {
                let mut root = Clip::new(assets, ASSET, 0)?;
                root.goto(assets, "intro")?;
                points(&mut root, assets, view.bail.as_ref())?;
                let mut broken = [false; 26];
                if let Some(bail) = view.bail {
                    const MAP: [usize; 24] = [
                        0, 1, 10, 11, 12, 13, 22, 21, 20, 19, 18, 9, 8, 7, 6, 17, 16, 15, 14, 5, 4,
                        3, 2, 24,
                    ];
                    for (i, j) in MAP.into_iter().enumerate() {
                        broken[i] = matches!(bail.bone_damage_levels[j], 4 | 5);
                    }
                    broken[24] = view.top_bail;
                    broken[25] = matches!(bail.bone_damage_levels[23], 4 | 5);
                }
                self.skeleton = Some(Skeleton::new(
                    root.named(&["mSkeletor"])?,
                    assets,
                    broken,
                    view.animation_seconds,
                )?);
                challenges(&mut root, assets, view)?;
                self.cash_goal = view.player_cash as f32;
                self.cash = (view.player_cash.wrapping_sub(view.total_cash) as f32).max(0.);
                self.cash_step = (self.cash_goal - self.cash) / 180.;
                if !(self.cash_step > 0.) {
                    self.cash_step = 0.2;
                }
                if !view.challenges.is_empty() {
                    cash_text(&mut root, assets, self.cash)?;
                    self.cash_audio = true;
                    self.sounds
                        .push(FeSoundRequest::LoopKey(0x5243_6b52_67fa_e4b4));
                    root.named(&["mPanelRight", "mChallenge"])?
                        .goto(assets, "intro")?;
                }
                root.named(&["mPanelRight", "mChallenge"])?.visible = !view.points_page;
                root.named(&["mPanelRight", "mPoints"])?.visible = view.points_page;
                if view.points_page {
                    show_points(&mut root, assets, false)?;
                }
                picker(&mut root, assets, view.selection)?;
                self.root = Some(root);
                self.open = true;
                self.ready = false;
                self.previous = Some(view.clone());
            } else if let Some(root) = self.root.as_mut() {
                if view.points_page && self.previous.as_ref().is_some_and(|v| !v.points_page) {
                    show_points(root, assets, true)?;
                }
                if self
                    .previous
                    .as_ref()
                    .is_none_or(|v| v.selection != view.selection)
                {
                    picker(root, assets, view.selection)?;
                }
                self.previous = Some(view.clone());
            }
        } else if self.open {
            self.open = false;
            self.ready = false;
            if self.cash_audio {
                self.sounds
                    .push(FeSoundRequest::StopKey(0x5243_6b52_67fa_e4b4));
                self.cash_audio = false;
            }
            if let Some(root) = self.root.as_mut() {
                root.goto(assets, "outro")?;
            }
        }
        self.calls()
    }
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if let Some(root) = self.root.as_mut() {
            if self.open {
                if let Some(skeleton) = self.skeleton.as_mut() {
                    let mut sounds = Vec::new();
                    skeleton.tick(root.named(&["mSkeletor"])?, assets, &mut sounds)?;
                    self.sounds
                        .extend(sounds.into_iter().map(FeSoundRequest::Play));
                }
                if self.cash_audio {
                    if self.cash < self.cash_goal {
                        self.cash = (self.cash + self.cash_step).min(self.cash_goal);
                        cash_text(root, assets, self.cash.floor())?;
                    } else {
                        self.cash_audio = false;
                        self.sounds
                            .push(FeSoundRequest::StopKey(0x5243_6b52_67fa_e4b4));
                    }
                }
            }
            root.tick(assets)?;
        }
        self.calls()
    }
    fn calls(&mut self) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else {
            return Ok(());
        };
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "hom_results" => {}
                "IntroComplete" => self.ready = true,
                "OutroComplete" => {
                    self.root = None;
                    self.skeleton = None;
                    self.previous = None;
                }
                "HOM_AMBIENCE_START" => self
                    .sounds
                    .push(FeSoundRequest::Loop("FE_SOUND_HOM_AMBIENCE")),
                "HOM_AMBIENCE_STOP" => self
                    .sounds
                    .push(FeSoundRequest::Stop("FE_SOUND_HOM_AMBIENCE")),
                "FE_SOUND_HOM_SCORE_TRANSITION"
                | "FE_SOUND_HOM_SCORE_FLICKER"
                | "FE_SOUND_HOM_GROIN"
                | "FE_SOUND_HOM_TOE_TAG" => self.sounds.push(FeSoundRequest::Play(call)),
                _ => return Err(format!("untranslated HoM results callback {call}").into()),
            }
        }
        Ok(())
    }
}

fn picker(root: &mut Clip, assets: &Assets, selected: usize) -> RenderResult<()> {
    let rows = [
        "ID_COMMON_CONTINUE",
        "ID_HOM_ANNALS_OF_MEAT",
        "ID_PAUSEMENU_REPLAY",
    ]
    .map(|s| assets.initial_text(s))
    .map(|s| String::from_utf16_lossy(&s));
    let rows = rows.iter().map(|s| (s.as_str(), true)).collect::<Vec<_>>();
    map_movie::update_picker(
        root.named(&["mMenuPanel", "mMenuPicker", "mMenuPicker"])?,
        assets,
        &rows,
        selected,
        0,
        3,
        false,
    )
}
fn show_points(root: &mut Clip, assets: &Assets, transition: bool) -> RenderResult<()> {
    root.named(&["mMenuPanel"])?.goto(assets, "points")?;
    if transition {
        root.named(&["mPanelRight", "mChallenge"])?
            .goto(assets, "outro")?;
    }
    root.named(&["mPanelRight", "mPoints"])?.visible = true;
    root.named(&["mPanelRight", "mPoints"])?
        .goto(assets, "intro")
}
fn text(clip: &mut Clip, path: &[&str], s: String) -> RenderResult<()> {
    clip.named(path)?.text = Some(s.encode_utf16().collect());
    Ok(())
}
fn points(root: &mut Clip, assets: &Assets, bail: Option<&HomBail>) -> RenderResult<()> {
    let points = root.named(&["mPanelRight", "mPoints"])?;
    let mut counts = [0i32; 5];
    if let Some(bail) = bail {
        for level in bail.bone_damage_levels {
            if (0..=5).contains(&level) {
                counts[4 - (level as usize).min(4)] += 1;
            }
        }
    }
    let damages = bail.map(|b| {
        [
            b.bone_scores[4].wrapping_add(b.bone_scores[5]),
            b.bone_scores[3],
            b.bone_scores[2],
            b.bone_scores[1],
            b.bone_scores[0],
        ]
    });
    for (i, label) in [
        "ID_HOM_BREAK",
        "ID_HOM_DISLOCATED",
        "ID_HOM_SPRAINED",
        "ID_HOM_BRUISED",
        "ID_HOM_SCRAPED",
    ]
    .into_iter()
    .enumerate()
    {
        let score = damages.map_or_else(
            || "#NO LAST BAIL!".into(),
            |d| loc::format_int(assets, d[i].cast_signed()),
        );
        let state = if score == "0" { "off" } else { "on" };
        points
            .named(&["mDamageIcons", &format!("mIcon{i}")])?
            .goto(assets, state)?;
        let row = points.named(&[&format!("mDamage{i}")])?;
        row.goto(assets, state)?;
        text(row, &["mData", "pointText"], score)?;
        row.named(&["mData", "injuryText"])?.text = Some(if counts[i] > 0 {
            loc::localized_format_string(
                assets,
                &format!("{label}_MULT"),
                &[loc::AptArgument::Integer(counts[i])],
            )?
        } else {
            loc::localized_format_string(assets, label, &[])?
        });
    }
    let measures = bail.map(|b| measures(assets, b)).transpose()?;
    for (i, label) in [
        "ID_HOM_TIME",
        "ID_HOM_DROP",
        "ID_HOM_AIR_TIME",
        "ID_HOM_ROTATION",
        "ID_HOM_SPEED",
        "ID_HOM_BODY_TWEAK",
        "ID_HOM_PEDESTRIANS",
        "ID_HOM_ROAD_MEAT",
        "ID_HOM_DMO_HITS",
    ]
    .into_iter()
    .enumerate()
    {
        let score = bail.map_or_else(
            || "#NO LAST BAIL!".into(),
            |b| loc::format_int(assets, b.bonus_scores[i].cast_signed()),
        );
        let state = if score == "0" { "off" } else { "on" };
        points
            .named(&["mBonusIcons", &format!("mIcon{i}")])?
            .goto(assets, state)?;
        let row = points.named(&[&format!("mBonus{i}")])?;
        row.goto(assets, state)?;
        text(row, &["mData", "pointText"], score)?;
        let mut label =
            String::from_utf16_lossy(&loc::localized_format_string(assets, label, &[])?);
        if let Some(measures) = &measures {
            if measures[i] != "#" {
                label.push_str(&format!(" ({})", measures[i]));
            }
        }
        text(row, &["mData", "injuryText"], label)?;
    }
    for path in [
        ["mTextTotal", "totalText"],
        ["mTextTotal", "totalText_glow"],
    ] {
        points.named(&path)?.text = Some(assets.initial_text("ID_HOM_TOTAL_POINTS"));
    }
    points
        .named(&["mTextTotal", "totalText"])?
        .properties
        .insert("text_autosize".into(), "left".into());
    // mTextAnnals/mTextRecord are absent from the shipped timeline; its assignments have no target.
    let total = bail.map_or(0, |b| b.score);
    let bone = bail.map_or(0, |b| b.bone_score);
    for (path, value) in [
        (["mBoneScore", "labeltext"], bone),
        (["mBonusScore", "labeltext"], total.wrapping_sub(bone)),
        (["mTextScore", "totalText"], total),
    ] {
        text(points, &path, loc::format_int(assets, value.cast_signed()))?;
    }
    Ok(())
}
fn measures(assets: &Assets, b: &HomBail) -> RenderResult<[String; 9]> {
    let time = |v: f32| {
        format!(
            "{} {}",
            loc::format_time(
                assets,
                0,
                0,
                v as i32,
                ((v - v.trunc()) * 100.) as i32,
                false,
                0
            ),
            String::from_utf16_lossy(&assets.initial_text("ID_COMMON_METRIC_TIME"))
        )
    };
    let metric = |label: &str, v: f32| -> RenderResult<String> {
        Ok(String::from_utf16_lossy(&loc::format_custom(
            assets,
            label,
            &loc::format_float(assets, v, 0),
        )?))
    };
    Ok([
        time(b.moving_duration),
        metric("ID_COMMON_METRIC_METERS", b.accumulated_air_drop_height)?,
        time(b.accumulated_air_drop_frames as f32 * (1. / 60.)),
        "#".into(),
        metric("ID_HOM_SPEED_VALUE", b.bail_speed)?,
        loc::format_int(assets, b.num_body_tweaks_performed.cast_signed()),
        loc::format_int(assets, b.entity_counts[0].cast_signed()),
        loc::format_int(assets, b.entity_counts[1].cast_signed()),
        loc::format_int(assets, b.entity_counts[2].cast_signed()),
    ])
}
fn challenges(root: &mut Clip, assets: &Assets, view: &HomResultsView) -> RenderResult<()> {
    if view.challenges.is_empty() {
        return Ok(());
    }
    let panel = root.named(&["mPanelRight", "mChallenge"])?;
    for i in 0..4 {
        let row = panel.named(&[&format!("mItem{i}")])?;
        if let Some(challenge) = view.challenges.get(i) {
            // The shipped character147 has no mCheckMark instance; the
            // original 10086/100ca assignments therefore have no target.
            for name in ["mIcon", "mProgress"] {
                row.named(&[name])?.visible = challenge.icon != "none";
            }
            if challenge.icon != "none" {
                row.named(&["mIcon"])?.goto(assets, challenge.icon)?;
                row.named(&["mProgress", "labeltext"])?.text = Some(loc::localized_format_string(
                assets,
                "ID_HOM_CHALLENGES_COMPLETE_FRACTION",
                &challenge.progress.map(loc::AptArgument::Integer),
                )?);
            }
            row.named(&["mObjective", "labeltext"])?.text = Some(loc::format_custom(
                assets,
                &challenge.title.0,
                &challenge.title.1,
            )?);
            // Character146 contains labeltext only; 10164 targets undefined.
            for label in ["labeltext"] {
                text(
                    row,
                    &["mCash", label],
                    loc::format_currency(assets, challenge.cash as f32),
                )?;
            }
            if super::tutorial_movie::max_scroll(row.named(&["mObjective", "labeltext"])?, assets)?
                == 1
            {
                let child = super::pause_movie::child(row, "mObjective")?;
                child.entry.matrix.ty += 17.;
                child.clip.script_transform = true;
            }
        } else {
            for name in ["mIcon", "mProgress", "mObjective", "mCash"] {
                row.named(&[name])?.visible = false;
            }
            row.goto(assets, "off")?;
        }
    }
    text(
        panel,
        &["mTotal", "labeltext"],
        loc::format_currency(assets, view.total_cash as f32),
    )
}
fn cash_text(root: &mut Clip, assets: &Assets, cash: f32) -> RenderResult<()> {
    let panel = root.named(&["mPanelRight", "mChallenge", "mCash"])?;
    for label in ["labeltext", "labeltext_glow"] {
        text(panel, &[label], loc::format_currency(assets, cash))?;
    }
    Ok(())
}
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0xfe38 => clip.calls.push("hom_results"),
        0xfe70 => {}
        0x109f8 => {
            clip.calls.push("FE_SOUND_HOM_SCORE_TRANSITION");
            clip.calls.push("HOM_AMBIENCE_START");
        }
        0x10a24 => {
            clip.calls.push("IntroComplete");
            clip.playback.stop();
        }
        0x10a50 => {
            clip.calls.push("HOM_AMBIENCE_STOP");
            clip.calls.push("OutroComplete");
            clip.playback.stop();
        }
        0x10d54 | 0x10ecc => clip.calls.push("FE_SOUND_HOM_SCORE_FLICKER"),
        _ => return Err(format!("untranslated HoM results action {offset:#x}").into()),
    }
    Ok(())
}

/// Guarded hom_results class definition in the shipped APT.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    asset == ASSET && character == 175 && offset == 0xfe70
}
