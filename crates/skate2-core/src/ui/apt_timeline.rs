//! Authored APT display-list state, separate from scripts and GPU resources.
//!
//! `DoTemporaryFrameControls`829f9900 folds skipped frame placements without
//! executing their actions. `mergeState`82a1a688 preserves an existing child
//! when character and original placement frame match; character-less moves
//! update existing entries without requiring a matching placement frame.
//! This distinction prevents menu refreshes from restarting nested animation.

use skate2_formats::apt::{ActionStream, Control, EventAction, Matrix, Movie, Placement};
use std::collections::BTreeMap;

pub const IDENTITY: Matrix = Matrix {
    a: 1.,
    b: 0.,
    c: 0.,
    d: 1.,
    tx: 0.,
    ty: 0.,
};

/// The creation identity is a placement, not just a character definition:
/// two instances of the same symbol at the same depth can be different clips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacementIdentity {
    pub character: u32,
    pub frame: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayEntry {
    pub identity: PlacementIdentity,
    pub matrix: Matrix,
    pub colour_scale: u32,
    pub colour_bias: u32,
    pub ratio: f32,
    pub name: Option<String>,
    pub clip_depth: i32,
    pub events: Vec<EventAction>,
    pub blend_mode: Option<i32>,
}

impl DisplayEntry {
    fn new(placement: &Placement, frame: usize) -> Self {
        let mut entry = Self {
            identity: PlacementIdentity {
                character: placement.character_id,
                frame,
            },
            matrix: IDENTITY,
            colour_scale: u32::MAX,
            colour_bias: 0,
            ratio: 0.,
            name: placement.name.clone(),
            clip_depth: placement.clip_depth,
            events: Vec::new(),
            blend_mode: None,
        };
        entry.update(placement);
        entry
    }

    fn update(&mut self, placement: &Placement) {
        if placement.flags & Placement::MATRIX != 0 {
            self.matrix = placement.matrix;
        }
        if placement.flags & Placement::COLOUR != 0 {
            self.colour_scale = placement.colour_scale;
            self.colour_bias = placement.colour_bias;
        }
        if placement.flags & Placement::RATIO != 0 {
            self.ratio = placement.ratio;
        }
        if placement.flags & Placement::EVENTS != 0 {
            self.events.clone_from(&placement.events);
        }
        if let Some(mode) = placement.blend_mode {
            self.blend_mode = Some(mode);
        }
    }
}

/// Effects returned in native lifecycle order. A consumer must execute script
/// effects; this service deliberately does not treat an unhandled script as a
/// successful no-op. InitAction blocks precede frame placements; frame actions
/// follow placements (`doFrameControls`82a1ad80, `queueFrameActions`829e0518).
#[derive(Clone, Debug, PartialEq)]
pub enum FrameEffect {
    InitAction {
        character: u32,
        actions: ActionStream,
    },
    Remove {
        depth: i32,
        identity: PlacementIdentity,
    },
    Create {
        depth: i32,
        entry: DisplayEntry,
    },
    Update {
        depth: i32,
        entry: DisplayEntry,
    },
    Background(u32),
    Action(ActionStream),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimelineError {
    FrameOutOfRange { frame: usize, count: usize },
    UnknownLabel(String),
    MoveAfterRemoval { depth: i32 },
}
impl std::fmt::Display for TimelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "APT timeline: {self:?}")
    }
}
impl std::error::Error for TimelineError {}

#[derive(Clone, Debug, Default)]
pub struct Timeline {
    frame: Option<usize>,
    entries: BTreeMap<i32, DisplayEntry>,
}

#[derive(Clone, Debug)]
enum FoldedControl {
    Place { placement: Placement, frame: usize },
    Remove,
}

impl Timeline {
    pub fn frame(&self) -> Option<usize> {
        self.frame
    }
    pub fn entries(&self) -> &BTreeMap<i32, DisplayEntry> {
        &self.entries
    }

    pub fn goto_label(
        &mut self,
        movie: &Movie,
        label: &str,
    ) -> Result<Vec<FrameEffect>, TimelineError> {
        let frame = movie
            .label_frame(label)
            .ok_or_else(|| TimelineError::UnknownLabel(label.into()))?;
        self.goto(movie, frame)
    }

    /// `jumpToFrame`82a1b3d0. Same-frame jumps do not rerun frame actions.
    /// Adjacent frames execute ordinary controls. Other jumps first fold
    /// placements and merge them, then queue ONLY the target frame's actions.
    pub fn goto(
        &mut self,
        movie: &Movie,
        target: usize,
    ) -> Result<Vec<FrameEffect>, TimelineError> {
        if target >= movie.frames.len() {
            return Err(TimelineError::FrameOutOfRange {
                frame: target,
                count: movie.frames.len(),
            });
        }
        if self.frame == Some(target) {
            return Ok(Vec::new());
        }
        let mut effects = Vec::new();
        if self.frame.map_or(target == 0, |frame| frame + 1 == target) {
            self.apply_frame(movie, target, &mut effects);
        } else {
            let ahead = self.frame.is_some_and(|frame| frame < target);
            let start = self.frame.filter(|&frame| frame < target).unwrap_or(0);
            // Fold before mutating retained state, so unsupported/corrupt
            // source combinations cannot leave half a reconstructed display.
            let folded = fold(movie, start, target)?;
            let mut old = std::mem::take(&mut self.entries);
            let mut depths: Vec<i32> = old.keys().chain(folded.keys()).copied().collect();
            depths.sort_unstable();
            depths.dedup();
            for depth in depths {
                let previous = old.remove(&depth);
                match folded.get(&depth) {
                    None if ahead || depth >= 0x4000 => {
                        if let Some(entry) = previous {
                            self.entries.insert(depth, entry);
                        }
                    }
                    None | Some(FoldedControl::Remove) => {
                        if let Some(entry) = previous {
                            effects.push(FrameEffect::Remove {
                                depth,
                                identity: entry.identity,
                            });
                        }
                    }
                    Some(FoldedControl::Place { placement, frame }) => {
                        let identity = PlacementIdentity {
                            character: placement.character_id,
                            frame: *frame,
                        };
                        let preserve = previous.as_ref().is_some_and(|old| {
                            placement.character_id == u32::MAX || old.identity == identity
                        });
                        if preserve {
                            let mut entry = previous.unwrap();
                            // mergeState82a1a918 resets colour when a complete
                            // placement omits it; a move-only control retains it.
                            if placement.character_id != u32::MAX
                                && placement.flags & Placement::COLOUR == 0
                            {
                                entry.colour_scale = u32::MAX;
                                entry.colour_bias = 0;
                            }
                            // Retained children only receive matrix and colour
                            // from mergeState/ReplaceDisplyListItem82a1a4c8;
                            // clip events, names and morph ratios aren't replayed.
                            if placement.flags & Placement::MATRIX != 0 {
                                entry.matrix = placement.matrix;
                            }
                            if placement.flags & Placement::COLOUR != 0 {
                                entry.colour_scale = placement.colour_scale;
                                entry.colour_bias = placement.colour_bias;
                            }
                            effects.push(FrameEffect::Update {
                                depth,
                                entry: entry.clone(),
                            });
                            self.entries.insert(depth, entry);
                        } else {
                            if let Some(entry) = previous {
                                effects.push(FrameEffect::Remove {
                                    depth,
                                    identity: entry.identity,
                                });
                            }
                            if placement.character_id != u32::MAX {
                                let entry = DisplayEntry::new(placement, *frame);
                                effects.push(FrameEffect::Create {
                                    depth,
                                    entry: entry.clone(),
                                });
                                self.entries.insert(depth, entry);
                            }
                        }
                    }
                }
            }
        }
        self.frame = Some(target);
        for control in &movie.frames[target].controls {
            if let Control::Action(actions) = control {
                effects.push(FrameEffect::Action(*actions));
            }
        }
        Ok(effects)
    }

    fn apply_frame(&mut self, movie: &Movie, target: usize, effects: &mut Vec<FrameEffect>) {
        for control in &movie.frames[target].controls {
            if let Control::InitAction {
                character_id,
                actions,
            } = control
            {
                // The asset-level action owner performs ExecuteInitActions'
                // once-only bookkeeping across instances of this animation.
                effects.push(FrameEffect::InitAction {
                    character: *character_id,
                    actions: *actions,
                });
            }
        }
        for control in &movie.frames[target].controls {
            match control {
                Control::Place(p)
                    if p.flags & Placement::CHARACTER != 0 && p.character_id != u32::MAX =>
                {
                    if let Some(old) = self.entries.remove(&p.depth) {
                        effects.push(FrameEffect::Remove {
                            depth: p.depth,
                            identity: old.identity,
                        });
                    }
                    let entry = DisplayEntry::new(p, target);
                    effects.push(FrameEffect::Create {
                        depth: p.depth,
                        entry: entry.clone(),
                    });
                    self.entries.insert(p.depth, entry);
                }
                Control::Place(p) if p.flags & Placement::MOVE != 0 => {
                    if let Some(entry) = self.entries.get_mut(&p.depth) {
                        entry.update(p);
                        effects.push(FrameEffect::Update {
                            depth: p.depth,
                            entry: entry.clone(),
                        });
                    }
                }
                Control::Remove { depth } => {
                    if let Some(entry) = self.entries.remove(depth) {
                        effects.push(FrameEffect::Remove {
                            depth: *depth,
                            identity: entry.identity,
                        });
                    }
                }
                Control::Background { colour } => effects.push(FrameEffect::Background(*colour)),
                _ => {}
            }
        }
    }
}

fn fold(
    movie: &Movie,
    start: usize,
    target: usize,
) -> Result<BTreeMap<i32, FoldedControl>, TimelineError> {
    let mut state = BTreeMap::new();
    for frame in start..=target {
        for control in &movie.frames[frame].controls {
            match control {
                Control::Place(p) => {
                    if p.character_id == u32::MAX {
                        match state.get_mut(&p.depth) {
                            Some(FoldedControl::Place { placement: old, .. }) => {
                                if p.flags & Placement::MATRIX != 0 {
                                    old.matrix = p.matrix;
                                }
                                if p.flags & Placement::COLOUR != 0 {
                                    old.colour_scale = p.colour_scale;
                                    old.colour_bias = p.colour_bias;
                                }
                                if p.flags & Placement::EVENTS != 0 {
                                    old.events.clone_from(&p.events);
                                }
                                if p.flags & Placement::RATIO != 0 {
                                    old.ratio = p.ratio;
                                }
                                if let Some(mode) = p.blend_mode {
                                    old.blend_mode = Some(mode);
                                }
                                old.flags |= p.flags;
                                continue;
                            }
                            Some(FoldedControl::Remove) => {
                                return Err(TimelineError::MoveAfterRemoval { depth: p.depth });
                            }
                            None => {}
                        }
                    }
                    state.insert(
                        p.depth,
                        FoldedControl::Place {
                            placement: p.clone(),
                            frame,
                        },
                    );
                }
                Control::Remove { depth } => {
                    state.insert(*depth, FoldedControl::Remove);
                }
                // Skipped actions/backgrounds/labels are not executed by the
                // original pseudo display list. Target actions are queued above.
                _ => {}
            }
        }
    }
    Ok(state)
}

/// `AptRenderingContext::multMatrix`829dae70. `parent * child`, with the
/// original scalar single-precision multiply/FMA/add order. Raw instruction check:
/// 829daf1c..3c are fmadds; decompiler incorrectly expands them to two rounds.
pub fn concatenate(parent: Matrix, child: Matrix) -> Matrix {
    Matrix {
        a: parent.c.mul_add(child.b, parent.a * child.a),
        b: parent.d.mul_add(child.b, parent.b * child.a),
        c: child.d.mul_add(parent.c, child.c * parent.a),
        d: child.d.mul_add(parent.d, child.c * parent.b),
        tx: child.ty.mul_add(parent.c, child.tx * parent.a) + parent.tx,
        ty: child.ty.mul_add(parent.d, child.tx * parent.b) + parent.ty,
    }
}

#[cfg(test)]
#[path = "apt_timeline_tests.rs"]
mod tests;
