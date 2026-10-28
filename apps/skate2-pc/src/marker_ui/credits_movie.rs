//! credits.apt (1402d4b0): original 30-slot roll, source file and APT clock.
use super::{assets::Assets, clips::Clip, edit_skater_movie::attach};
use crate::gpu_renderer::RenderResult;
pub(super) const ASSET: usize = 86;
struct Entry {
    size: f32,
    spacing: f32,
    label: String,
}
pub(super) struct CreditsMovie {
    pub root: Clip,
    entries: Vec<Entry>,
    active: [bool; 30],
    next: usize,
    next_line: f32,
    running: bool,
    pub intro: bool,
    pub done: bool,
}
impl CreditsMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, ASSET, 0)?;
        root.visible = false;
        let mut entries = Vec::new();
        // cCreditsManager::ParseCreditEntries82d6ade0 / ExtractCreditEntry82d6ac28.
        for line in assets
            .credits_source
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            let (values, quoted) = line.split_once('"').ok_or("credit entry missing quote")?;
            let (label, _) = quoted
                .rsplit_once('"')
                .ok_or("credit entry missing closing quote")?;
            let mut values = values.split_whitespace();
            let _font: u32 = values.next().ok_or("credit font absent")?.parse()?;
            let size = values.next().ok_or("credit size absent")?.parse()?;
            let spacing = values.next().ok_or("credit spacing absent")?.parse()?;
            // GetLine826230e0 strips # only from #ID..., before the text localizer.
            let label = if label.starts_with("#ID") {
                &label[1..]
            } else {
                label
            };
            if !label.is_empty() {
                entries.push(Entry {
                    size,
                    spacing,
                    label: assets.localize(label)?,
                });
            }
        }
        Ok(Self {
            root,
            entries,
            active: [false; 30],
            next: 0,
            next_line: 0.,
            running: false,
            intro: false,
            done: false,
        })
    }
    pub fn open(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root = Clip::new(assets, ASSET, 0)?;
        self.root.goto(assets, "intro")?;
        self.active = [false; 30];
        self.next = 0;
        self.next_line = 0.;
        self.running = true;
        self.intro = false;
        self.done = false;
        for i in 0..30 {
            let mut field = Clip::new(assets, ASSET, 2)?;
            field.visible = false;
            for (k, v) in [
                ("text_font", "Helvetica LT Std"),
                ("text_alignment", "2"),
                ("text_left", "0"),
                ("text_top", "0"),
                ("text_width", "1"),
                ("text_multiline", "false"),
            ] {
                field.properties.insert(k.into(), v.into());
            }
            field.properties.insert(
                "text_colour".into(),
                (0xff00_0000u32 | 12046808).to_string(),
            );
            attach(&mut self.root, i + 1, field, 960., 750.);
        }
        self.grab(assets)
    }
    pub fn close(&mut self, assets: &Assets) -> RenderResult<()> {
        self.running = false;
        self.root.goto(assets, "outro")
    }
    fn grab(&mut self, assets: &Assets) -> RenderResult<()> {
        let Some(slot) = self.active.iter().position(|a| !a) else {
            return Ok(());
        };
        let Some(entry) = self.entries.get(self.next) else {
            return Ok(());
        };
        let child = self
            .root
            .children
            .get_mut(&((slot + 1) as i32))
            .ok_or("credit slot missing")?;
        child.clip.text = Some(entry.label.encode_utf16().collect());
        child
            .clip
            .properties
            .insert("text_height".into(), entry.size.to_string());
        child.clip.visible = true;
        child.entry.matrix.ty = 750.;
        self.active[slot] = true;
        // SkateAptString82a70d60 textHeight: GetMaxCharHeight82849a80 * size.
        let font = assets.apt_font("Helvetica LT Std")?;
        self.next_line = font.metrics.ascent * font.scale[1] * entry.size * entry.spacing;
        self.next += 1;
        Ok(())
    }
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if !self.root.visible {
            return Ok(());
        }
        if self.running {
            // UpdateCredits06a0: each active slot moves 1.5 pixels per APT update.
            for i in 0..30 {
                if self.active[i] {
                    let child = self
                        .root
                        .children
                        .get_mut(&((i + 1) as i32))
                        .ok_or("credit slot missing")?;
                    child.entry.matrix.ty -= 1.5;
                    if child.entry.matrix.ty < -20. {
                        self.active[i] = false;
                    }
                    // Authored alphadir is 0 below y100 and 1 elsewhere; starting alpha100 stays100.
                }
            }
            if self.next < self.entries.len() {
                self.next_line -= 1.5;
                if self.next_line < 0. {
                    self.grab(assets)?;
                }
            } else if !self.active.iter().any(|active| *active) {
                self.done = true;
            }
        }
        self.root.tick(assets)?;
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "CreditsIntro" => self.intro = true,
                "CreditsOutro" => self.root.visible = false,
                _ => return Err(format!("unknown credits callback {call}").into()),
            }
        }
        Ok(())
    }
}
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x300 | 0x314 => clip.playback.stop(),
        0x9c4 | 0xa04 => {} // Owner disables letterbox / onEnterFrame.
        0x9e0 => {
            clip.playback.stop();
            clip.calls.push("CreditsIntro");
        }
        0xa18 => {
            clip.playback.stop();
            clip.calls.push("CreditsOutro");
        }
        _ => return Err(format!("untranslated credits APT action {offset:#x}").into()),
    }
    Ok(())
}
