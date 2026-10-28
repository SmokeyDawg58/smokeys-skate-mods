//! Original controls/skeleton, fingerprint7b44af51; class actions164a8/16bf8.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;
pub(super) const ASSET: usize = 82;

// SetupSkeleton1654c: both hips read element6. Array25 is supplied by the native
// translator but the shipped control never consumes it.
const PARTS: [(&str, usize, &str); 24] = [
    ("mSkull", 0, "SKULL"),
    ("neck", 1, "NECK"),
    ("mChestUpper", 2, "UPPERBODY"),
    ("mChestMid", 3, "MIDDLEBODY"),
    ("mChestLower", 4, "LOWERBODY"),
    ("mSpine", 5, "LOWERSPINE"),
    ("mHipLeft", 6, "HIPSRIGHT"),
    ("mHipRight", 6, "HIPSLEFT"),
    ("mUpperlegLeft", 7, "RIGHTUPLEG"),
    ("mLegLeft", 8, "RIGHTCALF"),
    ("mFootLeft", 9, "RIGHTFOOT"),
    ("mToeLeft", 10, "RIGHTTOE"),
    ("mShoulderLeft", 11, "RIGHTSHOULDER"),
    ("mArmLeft", 12, "RIGHTARM"),
    ("mForearmLeft", 13, "RIGHTFOREARM"),
    ("mHandLeft", 14, "RIGHTHAND"),
    ("mUpperlegRight", 15, "LEFTUPLEG"),
    ("mLegRight", 16, "LEFTCALF"),
    ("mFootRight", 17, "LEFTFOOT"),
    ("mToeRight", 18, "LEFTTOE"),
    ("mShoulderRight", 19, "LEFTSHOULDER"),
    ("mArmRight", 20, "LEFTARM"),
    ("mForearmRight", 21, "LEFTFOREARM"),
    ("mHandRight", 22, "LEFTHAND"),
];
const PARENT: [usize; 24] = [
    24, 0, 1, 2, 3, 4, 5, 5, 6, 8, 9, 10, 2, 12, 13, 14, 7, 16, 17, 18, 2, 20, 21, 22,
];
const ORDER: [usize; 24] = [
    6, 7, 16, 8, 2, 3, 4, 5, 12, 20, 13, 21, 1, 0, 14, 22, 17, 9, 15, 23, 18, 10, 11, 19,
];
const SOUNDS: [&str; 24] = [
    "FE_SOUND_HOM_SKULL",
    "FE_SOUND_HOM_NECK",
    "FE_SOUND_HOM_UPPERBODY",
    "FE_SOUND_HOM_MIDDLEBODY",
    "FE_SOUND_HOM_LOWERBODY",
    "FE_SOUND_HOM_LOWERSPINE",
    "FE_SOUND_HOM_HIPSRIGHT",
    "FE_SOUND_HOM_HIPSLEFT",
    "FE_SOUND_HOM_RIGHTUPLEG",
    "FE_SOUND_HOM_RIGHTCALF",
    "FE_SOUND_HOM_RIGHTFOOT",
    "FE_SOUND_HOM_RIGHTTOE",
    "FE_SOUND_HOM_RIGHTSHOULDER",
    "FE_SOUND_HOM_RIGHTARM",
    "FE_SOUND_HOM_RIGHTFOREARM",
    "FE_SOUND_HOM_RIGHTHAND",
    "FE_SOUND_HOM_LEFTUPLEG",
    "FE_SOUND_HOM_LEFTCALF",
    "FE_SOUND_HOM_LEFTFOOT",
    "FE_SOUND_HOM_LEFTTOE",
    "FE_SOUND_HOM_LEFTSHOULDER",
    "FE_SOUND_HOM_LEFTARM",
    "FE_SOUND_HOM_LEFTFOREARM",
    "FE_SOUND_HOM_LEFTHAND",
];

pub(super) struct Skeleton {
    animations: [&'static str; 24],
    delay: i32,
    index: usize,
    frames_per_bone: f32,
    timer: f32,
}
impl Skeleton {
    pub fn new(
        root: &mut Clip,
        assets: &Assets,
        broken: [bool; 26],
        seconds: f32,
    ) -> RenderResult<Self> {
        let mut status = PARTS.map(|(_, i, _)| if broken[i] { 3i32 } else { 0 });
        if broken[24] {
            status[0] = 4;
        }
        // HighlightAnimSetup's second parameter is omitted by its callers. AptValue
        // toFloat829dcc48 converts undefined to0, so no parent proximity propagates.
        let animations = std::array::from_fn(|i| {
            let child = PARENT
                .iter()
                .enumerate()
                .filter(|(_, p)| **p == i)
                .map(|(c, _)| status[c])
                .max()
                .unwrap_or(0)
                - 1;
            let grand = PARENT
                .iter()
                .enumerate()
                .filter(|(_, p)| **p == i)
                .filter_map(|(c, _)| {
                    PARENT
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| **p == c)
                        .map(|(g, _)| status[g])
                        .max()
                        .map(|v| v - 1)
                })
                .max()
                .unwrap_or(0)
                .max(0)
                - 1;
            match status[i].max(child).max(grand) {
                4 => "broken_face",
                3 => "Broken",
                2 => "Normal3",
                1 => "Normal2",
                _ => "Normal1",
            }
        });
        root.named(&["mGroin"])?
            .goto(assets, if broken[23] { "on" } else { "off" })?;
        root.named(&["mToetag"])?
            .goto(assets, if broken[24] { "on" } else { "off" })?;
        // Native APT16a5a..6c counts the entries whose animation is NOT Broken.
        let count = animations.iter().filter(|&&a| a != "Broken").count();
        let frames_per_bone = seconds / count as f32 * 60.;
        Ok(Self {
            animations,
            delay: 30,
            index: 0,
            frames_per_bone,
            timer: frames_per_bone,
        })
    }
    pub fn tick(
        &mut self,
        root: &mut Clip,
        assets: &Assets,
        sounds: &mut Vec<&'static str>,
    ) -> RenderResult<()> {
        if self.delay > 0 {
            self.delay -= 1;
            return Ok(());
        }
        if self.index == 0 {
            for i in ORDER {
                if self.animations[i] != "Broken" {
                    root.named(&[PARTS[i].0])?
                        .goto(assets, self.animations[i])?;
                }
            }
        }
        let Some(&i) = ORDER.get(self.index) else {
            return Ok(());
        };
        if self.animations[i] != "Broken" {
            self.index += 1;
        } else {
            self.timer -= 1.;
            if self.timer < 0. {
                root.named(&[PARTS[i].0])?
                    .goto(assets, self.animations[i])?;
                sounds.push(SOUNDS[i]);
                self.timer = self.frames_per_bone;
                self.index += 1;
            }
        }
        Ok(())
    }
}

pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x164a8 | 0x16bf8 | 0x17350 => {}
        0x17810 => clip.calls.push("FE_SOUND_HOM_GROIN"),
        0x17844 => clip.calls.push("FE_SOUND_HOM_TOE_TAG"),
        _ => return Err(format!("untranslated skeleton action {offset:#x}").into()),
    }
    Ok(())
}

/// skeleton.HighLight169d9 / skeleton_node.Highlight16f85: unlike the
/// animated method, parent proximity is the first argument and propagates.
pub(super) fn highlight(root: &mut Clip, assets: &Assets, broken: [bool; 26]) -> RenderResult<()> {
    let mut status = PARTS.map(|(_, i, _)| if broken[i] { 3i32 } else { 0 });
    if broken[24] {
        status[0] = 4;
    }
    root.named(&["mGroin"])?
        .goto(assets, if broken[23] { "on" } else { "off" })?;
    root.named(&["mToetag"])?
        .goto(assets, if broken[24] { "on" } else { "off" })?;
    fn visit(
        i: usize,
        parent: i32,
        status: &[i32; 24],
        root: &mut Clip,
        assets: &Assets,
    ) -> RenderResult<()> {
        let current = status[i].max(parent);
        for (child, _) in PARENT.iter().enumerate().filter(|(_, p)| **p == i) {
            visit(child, current - 1, status, root, assets)?;
        }
        let child = PARENT
            .iter()
            .enumerate()
            .filter(|(_, p)| **p == i)
            .map(|(c, _)| status[c])
            .max()
            .unwrap_or(0)
            - 1;
        let grand = PARENT
            .iter()
            .enumerate()
            .filter(|(_, p)| **p == i)
            .filter_map(|(c, _)| {
                PARENT
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| **p == c)
                    .map(|(g, _)| status[g])
                    .max()
                    .map(|v| v - 1)
            })
            .max()
            .unwrap_or(0)
            .max(0)
            - 1;
        let animation = match current.max(child).max(grand) {
            4 => "broken_face",
            3 => "Broken",
            2 => "Normal3",
            1 => "Normal2",
            _ => "Normal1",
        };
        root.named(&[PARTS[i].0])?.goto(assets, animation)
    }
    visit(0, 0, &status, root, assets)
}

/// skeleton, skeleton_node and the exported skeleton symbol's guarded classes.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    asset == 82
        && matches!(
            (character, offset),
            (207, 0x164a8) | (208, 0x16bf8) | (206, 0x17350)
        )
}
