//! Original annalsofmeat.apt (b0e2d7f4), screen128, FE state77.
use super::{assets::Assets, clips::Clip, hom_skeleton_movie, localizer as loc};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::{hom::HomMetadata, hom_annals::AnnalsView, mission::HomBail};
pub(super) const ASSET: usize = 83;
#[derive(Default)]
pub(super) struct AnnalsMovie {
    pub root: Option<Clip>,
    pub open: bool,
    pub ready: bool,
    pub sounds: Vec<&'static str>,
    previous: Option<AnnalsView>,
}
impl AnnalsMovie {
    pub fn update(&mut self, assets: &Assets, view: Option<&AnnalsView>) -> RenderResult<()> {
        if let Some(view) = view {
            if !self.open {
                let mut root = Clip::new(assets, ASSET, 0)?;
                let main = root.named(&["mMain"])?;
                label(
                    main,
                    assets,
                    &["mTitle", "labeltext"],
                    "ID_ANNALSOFMEAT_TITLE",
                )?;
                for (name, id) in [
                    ("mTopBailsTab", "ID_ANNALSOFMEAT_TOP_BAILS"),
                    ("mBailStatsTab", "ID_ANNALSOFMEAT_BAIL_STATS"),
                    ("mChallengesTab", "ID_ANNALSOFMEAT_CHALLENGES"),
                ] {
                    label(main, assets, &[name, "labeltext"], id)?;
                }
                let top = super::pause_movie::child(main, "mTopBailsTab")?;
                top.entry.colour_scale = (top.entry.colour_scale & 0x00ff_ffff)
                    | if view.stats.top.is_empty() {
                        76 << 24
                    } else {
                        255 << 24
                    };
                top.clip.script_transform = true;
                label(
                    main,
                    assets,
                    &["mButtons", "mXButtonText", "buttonText"],
                    "ID_ANNALSOFMEAT_ONLINE_LEADERBOARD",
                )?;
                label(
                    main,
                    assets,
                    &["mButtons", "mBButtonText", "buttonText"],
                    "ID_COMMON_BACK",
                )?;
                state(main, assets, view)?;
                root.goto(assets, "intro")?;
                self.root = Some(root);
                self.open = true;
                self.ready = false;
            } else if let Some(root) = self.root.as_mut() {
                let main = root.named(&["mMain"])?;
                if self.previous.as_ref().is_none_or(|v| v.tab != view.tab) {
                    state(main, assets, view)?;
                } else if self
                    .previous
                    .as_ref()
                    .is_some_and(|v| v.bail_index != view.bail_index)
                {
                    top_bail(main, assets, view)?;
                } else if self
                    .previous
                    .as_ref()
                    .is_some_and(|v| v.path_index != view.path_index)
                {
                    challenge_selection(main, assets, view)?;
                }
            }
            self.previous = Some(view.clone());
        } else if self.open {
            self.open = false;
            self.ready = false;
            if let Some(root) = self.root.as_mut() {
                root.goto(assets, "outro")?;
            }
        }
        self.calls()
    }
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if let Some(root) = self.root.as_mut() {
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
                "annalsofmeat" => {}
                "IntroComplete" => self.ready = true,
                "OutroComplete" => {
                    self.root = None;
                    self.previous = None;
                }
                "FE_SOUND_HOM_GROIN" | "FE_SOUND_HOM_TOE_TAG" => self.sounds.push(call),
                _ => return Err(format!("untranslated Annals callback {call}").into()),
            }
        }
        Ok(())
    }
}
fn text(root: &mut Clip, path: &[&str], value: impl AsRef<str>) -> RenderResult<()> {
    root.named(path)?.text = Some(value.as_ref().encode_utf16().collect());
    Ok(())
}
fn label(root: &mut Clip, assets: &Assets, path: &[&str], id: &str) -> RenderResult<()> {
    root.named(path)?.text = Some(assets.initial_text(id));
    Ok(())
}
fn autosize(root: &mut Clip, path: &[&str], side: &str) -> RenderResult<()> {
    root.named(path)?
        .properties
        .insert("text_autosize".into(), side.into());
    Ok(())
}
fn title(assets: &Assets, t: &(String, String)) -> RenderResult<String> {
    Ok(String::from_utf16_lossy(&loc::format_custom(
        assets, &t.0, &t.1,
    )?))
}
fn state(main: &mut Clip, assets: &Assets, view: &AnnalsView) -> RenderResult<()> {
    main.goto(assets, ["topbails", "bailstats", "challenges"][view.tab])?;
    for (i, name) in ["mTopBailsTab", "mBailStatsTab", "mChallengesTab"]
        .into_iter()
        .enumerate()
    {
        main.named(&[name])?
            .goto(assets, if i == view.tab { "on" } else { "off" })?;
    }
    match view.tab {
        0 => {
            main.named(&["mTopBails"])?.visible = !view.stats.top.is_empty();
            if !view.stats.top.is_empty() {
                top_bail(main, assets, view)?;
            }
            for arrow in ["mArrowLeft", "mArrowRight"] {
                main.named(&["mTopBails", arrow])?.visible = view.stats.top.len() > 1;
            }
        }
        1 => stats(main, assets, view)?,
        2 => {
            main.named(&["mChallengeDetails"])?.visible = true;
            main.named(&["mChallengeStats"])?.visible = true;
            for field in ["labelText", "labelTextGlow"] {
                label(
                    main,
                    assets,
                    &["mChallengeDetails", "mChallengeTitle", field],
                    "ID_ANNALSOFMEAT_CHALLENGE_TITLE",
                )?;
            }
            for (i, path) in view.paths.iter().enumerate() {
                let row = main.named(&["mChallengeDetails", &format!("mChallenge{i}")])?;
                text(row, &["labelText"], title(assets, &path.current)?)?;
                text(
                    row,
                    &["mCompleted", "valueText"],
                    format!("{}/15", path.completed),
                )?;
            }
            challenge_selection(main, assets, view)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
fn broken(b: &HomBail) -> [bool; 26] {
    const MAP: [usize; 24] = [
        0, 1, 10, 11, 12, 13, 22, 21, 20, 19, 18, 9, 8, 7, 6, 17, 16, 15, 14, 5, 4, 3, 2, 24,
    ];
    let mut values = [false; 26];
    for (i, bone) in MAP.into_iter().enumerate() {
        values[i] = matches!(b.bone_damage_levels[bone], 4 | 5);
    }
    values[24] = true;
    values[25] = matches!(b.bone_damage_levels[23], 4 | 5);
    values
}
fn top_bail(main: &mut Clip, assets: &Assets, view: &AnnalsView) -> RenderResult<()> {
    let bail = &view.stats.top[view.bail_index];
    let metadata = &view.stats.top_metadata[view.bail_index];
    let root = main.named(&["mTopBails"])?;
    let heading = loc::localized_format_string(
        assets,
        "ID_ANNALSOFMEAT_BAIL_TITLE",
        &[loc::AptArgument::Integer(view.bail_index as i32 + 1)],
    )?;
    for field in ["labelText", "labelTextGlow"] {
        root.named(&["mBailTitle", field])?.text = Some(heading.clone());
    }
    let mut counts = [0i32; 5];
    for level in bail.bone_damage_levels {
        if (0..=5).contains(&level) {
            counts[4 - (level as usize).min(4)] += 1;
        }
    }
    for (i, (name, id)) in [
        ("Breaks", "BREAK"),
        ("Dislocated", "DISLOCATED"),
        ("Sprained", "SPRAINED"),
        ("Bruised", "BRUISED"),
        ("Scraped", "SCRAPED"),
    ]
    .into_iter()
    .enumerate()
    {
        root.named(&[&format!("mBail{name}"), "labelText"])?.text = Some(if counts[i] > 1 {
            loc::localized_format_string(
                assets,
                &format!("ID_HOM_{id}_MULT"),
                &[loc::AptArgument::Integer(counts[i])],
            )?
        } else {
            assets.initial_text(&format!("ID_HOM_{id}"))
        });
    }
    for (name, id) in [
        ("Date", "BAIL_DATE"),
        ("Time", "BAIL_TIME"),
        ("Location", "BAIL_LOCATION"),
        ("Damage", "DAMAGE"),
        ("Bonuses", "BONUSES"),
        ("Total", "TOTAL"),
    ] {
        label(
            root,
            assets,
            &[&format!("mBail{name}"), "labelText"],
            &format!("ID_ANNALSOFMEAT_{id}"),
        )?;
    }
    let (date, time) = calendar(assets, metadata)?;
    let values = [
        date,
        time,
        String::from_utf16_lossy(&assets.initial_text(&metadata.location)),
        loc::format_int(
            assets,
            bail.bone_scores[4]
                .wrapping_add(bail.bone_scores[5])
                .cast_signed(),
        ),
        loc::format_int(assets, bail.bone_scores[3].cast_signed()),
        loc::format_int(assets, bail.bone_scores[2].cast_signed()),
        loc::format_int(assets, bail.bone_scores[1].cast_signed()),
        loc::format_int(assets, bail.bone_scores[0].cast_signed()),
        loc::format_int(assets, bail.bone_score.cast_signed()),
        loc::format_int(
            assets,
            bail.score.wrapping_sub(bail.bone_score).cast_signed(),
        ),
        loc::format_int(assets, bail.score.cast_signed()),
    ];
    for (name, value) in [
        "Date",
        "Time",
        "Location",
        "Breaks",
        "Dislocated",
        "Sprained",
        "Bruised",
        "Scraped",
        "Damage",
        "Bonuses",
        "Total",
    ]
    .into_iter()
    .zip(values)
    {
        let path = [format!("mBail{name}Text"), "labelText".into()];
        let path = [path[0].as_str(), path[1].as_str()];
        text(root, &path, value)?;
        autosize(root, &path, "right")?;
    }
    hom_skeleton_movie::highlight(root.named(&["mSkeleton"])?, assets, broken(bail))
}
/// FormatDate82981ad0: date-format runs, zero padded. LanguageManager82d66a78
/// registers only the date pattern; SetupClockFormat82982088 uses HH:mm.
fn calendar(assets: &Assets, metadata: &HomMetadata) -> RenderResult<(String, String)> {
    let Some([year, month, day, hour, minute, _]) = metadata.calendar else {
        return Err("HoM record has no host local calendar".into());
    };
    let pattern = assets
        .localized
        .get(&skate2_formats::localization::string_hash(
            b"ID_COMMON_DATE_FORMAT",
        ))
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf16_lossy(s))
        .unwrap_or_else(|| "dd/MM/yyyy".into());
    let chars = pattern.as_bytes();
    let mut date = String::new();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        let mut end = i + 1;
        while end < chars.len() && chars[end] == ch {
            end += 1;
        }
        let value = match ch {
            b'd' => Some(day),
            b'M' => Some(month),
            b'y' => Some(year),
            b'/' | b'.' | b'-' => None,
            _ => return Err("unsupported native date pattern".into()),
        };
        if let Some(value) = value {
            let width = end - i;
            let s = format!("{value:0width$}");
            date.push_str(&s[s.len().saturating_sub(width)..]);
        } else {
            date.push_str(&pattern[i..end]);
        }
        i = end;
    }
    Ok((date, format!("{hour:02}:{minute:02}")))
}
fn stats(main: &mut Clip, assets: &Assets, view: &AnnalsView) -> RenderResult<()> {
    let s = &view.stats;
    let values = [
        s.injuries[4],
        s.injuries[3],
        s.injuries[2],
        s.injuries[1],
        s.injuries[0],
        s.tweaks,
        s.entity_hits[1],
        s.entity_hits[0],
        0,
        s.total_bails,
    ];
    for (i, (name, id)) in [
        ("Breaks", "BAIL_BREAKS"),
        ("Dislocated", "DISLOCATED"),
        ("Sprained", "SPRAINED"),
        ("Bruised", "BRUISED"),
        ("Scraped", "SCRAPED"),
        ("Tweaks", "TWEAKS"),
        ("CarsHit", "CARS_HIT"),
        ("PedsHit", "PEDS_HIT"),
        ("TimeBailing", "TIME_BAILING"),
        ("TotalBails", "TOTAL_BAILS"),
    ]
    .into_iter()
    .enumerate()
    {
        let row = main.named(&["mStats", &format!("m{name}")])?;
        label(
            row,
            assets,
            &["titleText"],
            &format!("ID_ANNALSOFMEAT_{id}"),
        )?;
        let value = if i == 8 {
            format!(
                "{} {}",
                loc::format_float(assets, s.time_bailing, 1),
                String::from_utf16_lossy(&assets.initial_text("ID_COMMON_METRIC_TIME"))
            )
        } else {
            loc::format_int(assets, values[i].cast_signed())
        };
        text(row, &["valueText"], value)?;
    }
    // FrontEndState_AnnalsOfMeat.GetOnlineStatus8268fd88: the offline backend.
    let online = main.named(&["mOnlineStats"])?;
    label(
        online,
        assets,
        &["mTitle", "labelText"],
        "ID_ANNALSOFMEAT_ONLINE_STATS",
    )?;
    autosize(online, &["mTitle", "labelText"], "left")?;
    online.goto(assets, "disconnected")?;
    label(
        online,
        assets,
        &["mConnect", "labelText"],
        "ID_ANNALSOFMEAT_DISCONNECTED",
    )?;
    let title = super::pause_movie::child(online.named(&["mTitle"])?, "labelText")?;
    let x = title.entry.matrix.tx + super::hud_leaderboard_movie::width(title, assets)?;
    let connect = super::pause_movie::child(online, "mConnect")?;
    connect.entry.matrix.tx = x;
    connect.clip.script_transform = true;
    Ok(())
}
fn challenge_selection(main: &mut Clip, assets: &Assets, view: &AnnalsView) -> RenderResult<()> {
    for i in 0..4 {
        main.named(&["mChallengeDetails", &format!("mChallenge{i}")])?
            .goto(assets, if i == view.path_index { "on" } else { "off" })?;
    }
    let root = main.named(&["mChallengeStats"])?;
    let path = &view.paths[view.path_index];
    for field in ["labelText", "labelTextGlow"] {
        label(
            root,
            assets,
            &["mCompleted", field],
            [
                "ID_ANNALSOFMEAT_POINTS",
                "ID_ANNALSOFMEAT_DAMAGE_PATH",
                "ID_ANNALSOFMEAT_STYLE",
                "ID_ANNALSOFMEAT_GRAB_BAG",
            ][view.path_index],
        )?;
    }
    root.named(&["mPathIcons"])?.goto(
        assets,
        ["score", "damage", "style", "grab"][view.path_index],
    )?;
    text(
        root,
        &["mMoney", "labelText"],
        loc::format_currency(assets, path.cash),
    )?;
    for (i, row) in path.rows.iter().enumerate() {
        let clip = root.named(&[&format!("mDetail{i}")])?;
        clip.goto(assets, row.state)?;
        text(clip, &["mIndex", "labelText"], format!("{}.", i + 1))?;
        text(
            clip,
            &["mDescription", "labelText"],
            title(assets, &row.description)?,
        )?;
        autosize(clip, &["mDescription", "labelText"], "left")?;
        if let Some(cash) = row.cash {
            text(
                clip,
                &["mCash", "labelText"],
                loc::format_currency(assets, cash as f32),
            )?;
        } else {
            label(
                clip,
                assets,
                &["mCash", "labelText"],
                "ID_ANNALSOFMEAT_UNKNOWN_CASH",
            )?;
        }
        autosize(clip, &["mCash", "labelText"], "right")?;
    }
    Ok(())
}
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x9754 => {
            clip.calls.push("annalsofmeat");
            clip.playback.stop();
        }
        0x9770 | 0xa6a0 => {}
        0xa6bc => {
            clip.playback.stop();
            clip.calls.push("IntroComplete");
        }
        0xa6e8 => {
            clip.playback.stop();
            clip.calls.push("OutroComplete");
        }
        _ => return Err(format!("untranslated Annals action {offset:#x}").into()),
    }
    Ok(())
}

/// Guarded annalsofmeat class definition in the shipped APT.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    asset == ASSET && character == 189 && offset == 0x9770
}
