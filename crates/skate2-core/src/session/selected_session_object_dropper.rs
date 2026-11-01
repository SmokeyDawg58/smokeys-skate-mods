//! PC-only Object Dropper, laid out after Skate 3's park editor (Skate 2 has none) and placing
//! Skate 2's own props. LB + D-pad Right opens it: a free camera takes over while
//! the world keeps running (the skater's input is held neutral, so it stands where it was).
//! Left stick moves the camera across, right stick looks, D-pad Up/Down raise and lower it, and
//! a crosshair marks the middle of the screen. X opens the object list: LT/RT pick the tab
//! (Terrain, Other, Props; only Props has anything yet), LB/RB the subcategory, D-pad Up/Down
//! the item, A takes it into the editor's hands in front of the crosshair, B closes the list.
//! A held prop: left stick moves it, D-pad Up/Down raise and lower it, right stick orbits the
//! camera round it, D-pad Right/Left bring the camera closer or farther (closer moves it more
//! finely), RT + a left stick flick turns it 90 degrees, LT + RT + left stick turns it smoothly,
//! L3 stands it back up as it was taken, and A lets go of it so it falls. With nothing held, A
//! picks the placed prop under the crosshair back up and Y deletes it. B leaves. A placed prop
//! is kept as Skate 2 keeps its own (`try_drop_on_locator`): an ordinary object on a locator of
//! its own, which the skater can grab and drag; far away the census culls it and later respawns
//! it where it was left.

use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage, SelectedVisualSnapshot};
use crate::input_pad::XboxPadSample;
use crate::nis::manager::{FrozenFocus, NisCamera};
use crate::simulation_assembly_backend::constraint_session::skateboard_pool::riderless::RiderlessBoardHandle;
use crate::ui::object_dropper::{EditorSettings, MARQUEE, MISSION_BODY_BASE, MISSION_PROPS, MissionPlacement, MissionPropShape, PARK_PIECES, PROPS, ParkPiece, PropSettings, THEMES, YOUR_BOARD, YOUR_GRIND_BOARD, in_theme, is_board, marquee_entry, mission_prop, park_piece, prop_tab, row_name, theme_row};

const DPAD_UP: u16 = 1 << 0;
const DPAD_DOWN: u16 = 1 << 1;
const DPAD_LEFT: u16 = 1 << 2;
const DPAD_RIGHT: u16 = 1 << 3;

/// Object Dropper is for free skating, not active challenges or party play.
pub(super) fn object_dropper_mode_allowed(mode: u32, career_challenge: bool, online_freeskate: bool) -> bool {
    use crate::challenge::players::{GM_ONLINE_FREE_SKATE, GM_PROFILE_FREE_SKATE, GM_SINGLE_PLAYER};
    if career_challenge {
        return false;
    }
    mode == GM_SINGLE_PLAYER as u32 || mode == GM_PROFILE_FREE_SKATE as u32
        || (mode == GM_ONLINE_FREE_SKATE as u32 && online_freeskate)
}

#[cfg(test)]
#[test]
fn object_dropper_is_limited_to_free_roam() {
    use crate::challenge::players::{GM_ONLINE_CHALLENGE, GM_ONLINE_FREE_SKATE, GM_PARTY_PLAY, GM_PROFILE_FREE_SKATE, GM_SINGLE_PLAYER};
    assert!(object_dropper_mode_allowed(GM_SINGLE_PLAYER as u32, false, false));
    assert!(object_dropper_mode_allowed(GM_PROFILE_FREE_SKATE as u32, false, false));
    assert!(object_dropper_mode_allowed(GM_ONLINE_FREE_SKATE as u32, false, true));
    assert!(!object_dropper_mode_allowed(GM_SINGLE_PLAYER as u32, true, false));
    assert!(!object_dropper_mode_allowed(GM_ONLINE_FREE_SKATE as u32, false, false));
    assert!(!object_dropper_mode_allowed(GM_ONLINE_CHALLENGE as u32, false, true));
    assert!(!object_dropper_mode_allowed(GM_PARTY_PLAY as u32, false, false));
}
const START: u16 = 1 << 4;
const L3: u16 = 1 << 6;
const R3: u16 = 1 << 7;
const LEFT_BUMPER: u16 = 1 << 8;
const RIGHT_BUMPER: u16 = 1 << 9;
const A: u16 = 1 << 12;
const B: u16 = 1 << 13;
const X: u16 = 1 << 14;
const Y: u16 = 1 << 15;
/// Dropped props kept at once (their own locators; the census decides which are out).
const MAX_DROPPED: usize = 24;
/// Static pieces (mission props, Skate 3 park pieces) kept at once: no locator or census, only
/// a collision body and a draw each.
const MAX_STATIC: usize = 400;
/// Dropped props' locator IDs: a range no streamed data uses ("PCDR"), then a count.
const LOCATOR_BASE: u64 = 0x5043_4452_0000_0000;
/// How far the crosshair reaches for a surface (metres).
const AIM_REACH: f32 = 60.0;
/// A prop taken from the list appears this far along the crosshair, or on the first surface
/// the crosshair meets nearer than that (metres).
const SPAWN_DISTANCE: f32 = 6.0;
/// A prop taken from the list has its middle at least this many times its reach (half its
/// bounding box's diagonal) ahead of the camera, so the camera starts outside a big piece.
const SPAWN_CLEARANCE: f32 = 1.6;
/// The crosshair targets the placed prop whose middle passes nearest its line, within this
/// (metres) plus this much per metre away.
const TARGET_WIDTH: f32 = 1.0;
const TARGET_SPREAD: f32 = 0.06;
/// A prop's middle above its origin (its base), for aiming at it and orbiting it (metres).
const PROP_MIDDLE: f32 = 0.5;
/// The free camera: across and up/down (m/s), and looking (rad/s, Skate 3's manual camera's).
const FLY_SPEED: f32 = 8.0;
const RISE_SPEED: f32 = 6.0;
const LOOK_SPEED: f32 = 1.75;
/// A held prop moves across and up/down this fast per metre the camera is from it (1/s).
const MOVE_RATE: f32 = 0.8;
const LIFT_RATE: f32 = 0.4;
/// The orbit camera's distance from a held prop (metres), changed this fast (1/s, scaled).
const ORBIT_MIN: f32 = 1.5;
const ORBIT_MAX: f32 = 25.0;
/// The held prop's orbit camera stays this far in front of a surface between it and the prop
/// (metres), so it never looks from under the ground or inside a wall.
const CAMERA_CLEARANCE: f32 = 0.3;
const ZOOM_RATE: f32 = 1.2;
/// LT + RT + left stick turn a held prop at full tilt from this speed, rising to the top speed
/// over the ramp while the stick stays pushed (rad/s: 15 to 120 degrees; seconds).
const FINE_TURN_START: f32 = 15.0 * std::f32::consts::PI / 180.0;
const FINE_TURN_TOP: f32 = 120.0 * std::f32::consts::PI / 180.0;
const FINE_TURN_RAMP: f32 = 1.5;
/// An RT flick spins the prop 90 degrees at this speed (rad/s: a quarter turn in 0.2 s).
const SPIN_SPEED: f32 = 2.5 * std::f32::consts::PI;
/// A flick: the left stick leaves the middle (inside REST) and passes FLICK.
const STICK_REST: f32 = 0.3;
const STICK_FLICK: f32 = 0.7;
/// The editor holds B (the foot brake) for a riding skater from this many frames after opening.
const BRAKE_AFTER_FRAMES: u32 = 2;
/// A trigger counts as held from here (of 255).
const TRIGGER_HELD: u8 = 128;
/// Analog moves are cubed, as Skate 3's manual camera does; smaller results count as none.
const DEADZONE: f32 = 0.01;
const FIELD_OF_VIEW: f32 = 55.0;
const NEAR_PLANE: f32 = 0.2;
/// The object list's tabs, as Skate 3's: Terrain and Other hold its park editor pieces.
const TABS: [&str; 3] = ["Terrain", "Other", "Props"];
const PROPS_TAB: usize = 2;
/// Object list rows shown around the selected one.
const LIST_ROWS: usize = 10;
/// Under the Props tab, as Skate 3's says it.
const PROPS_INFO: [&str; 2] = ["Props always fall and settle to the ground.", "You can also knock them around while skating."];
/// Under the Terrain and Other tabs.
const PIECES_INFO: [&str; 2] = ["Skate 3 park pieces stay exactly where they are placed.", "Needs Skate 3's parkassets.big in data\\content."];

/// A held Skate 3 piece snaps when one of its faces' corners comes this near a corner of an
/// opposite face of a placed piece (metres), and squares its turn to that piece's when within
/// this (radians: 12 degrees) of a quarter turn of it.
const SNAP_REACH: f32 = 1.0;
const SNAP_SQUARE: f32 = 12.0 * std::f32::consts::PI / 180.0;
/// Placed pieces whose origins are farther than this from the held one's are not snapped to.
const SNAP_NEIGHBOURHOOD: f32 = 40.0;
/// Ground snapping probes a piece's footprint this often round its outline and inside (metres).
const GROUND_EDGE_STEP: f32 = 0.5;
const GROUND_GRID_STEP: f32 = 2.0;
/// Ground under a snapped piece this far below the level most of it stands on still takes it
/// down (metres); a deeper step is another level it overhangs.
const GROUND_DIP: f32 = 0.05;
/// A surface is a wall a side snaps flush to when its normal is within 25 degrees of the
/// side's way out (the dot product's size is above this).
const WALL_FACING: f32 = 0.906;
/// Two faces are opposite when their normals' dot product is below this.
const SNAP_OPPOSITE: f32 = -0.98;

/// The Skate 3 piece a catalog entry is, when it snaps: Terrain and Other pieces do, its
/// props do not.
fn snapping_piece(guid: u64) -> Option<usize> {
    park_piece(guid).filter(|&piece| PARK_PIECES[piece].tab < 2)
}

/// A model-space direction under a transform (axis rows).
fn direction(transform: &[f32; 16], v: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|lane| transform[lane] * v[0] + transform[4 + lane] * v[1] + transform[8 + lane] * v[2])
}

/// Skate 3 style snapping: the held piece at `held` squares its turn to the nearest placed
/// piece's when nearly square to it, then slides so the nearest pair of corners of opposite
/// faces (its and a placed piece's) meet, when they are within reach. None: nothing in reach.
fn snap_transform(held: &[f32; 16], faces: &[super::park_assets::SnapFace], placed: &[([f32; 16], &[super::park_assets::SnapFace])]) -> Option<[f32; 16]> {
    let at = origin(held);
    let near: Vec<&([f32; 16], &[super::park_assets::SnapFace])> = placed
        .iter()
        .filter(|(transform, _)| {
            let d = add(origin(transform), scale(at, -1.0));
            dot(d, d) < SNAP_NEIGHBOURHOOD * SNAP_NEIGHBOURHOOD
        })
        .collect();
    let mut transform = *held;
    let upright = |t: &[f32; 16]| t[5] > 0.999;
    if let Some((target, _)) = near.iter().min_by(|a, b| {
        let da = add(origin(&a.0), scale(at, -1.0));
        let db = add(origin(&b.0), scale(at, -1.0));
        dot(da, da).total_cmp(&dot(db, db))
    }) && upright(&transform)
        && upright(target)
    {
        let quarter = std::f32::consts::FRAC_PI_2;
        let off = heading(&transform) - heading(target);
        let off = off - (off / quarter).round() * quarter;
        if off.abs() < SNAP_SQUARE {
            turn(&mut transform, UP, -off);
        }
    }
    let mut best: Option<[f32; 3]> = None;
    for face in faces {
        let normal = direction(&transform, face.normal);
        for (target, target_faces) in &near {
            for target_face in target_faces.iter() {
                if dot(normal, direction(target, target_face.normal)) > SNAP_OPPOSITE {
                    continue;
                }
                for corner in &face.corners {
                    let from = place(&transform, *corner);
                    for target_corner in &target_face.corners {
                        let delta = add(place(target, *target_corner), scale(from, -1.0));
                        if dot(delta, delta) < SNAP_REACH * SNAP_REACH && best.is_none_or(|b| dot(delta, delta) < dot(b, b)) {
                            best = Some(delta);
                        }
                    }
                }
            }
        }
    }
    let delta = best?;
    for lane in 0..3 {
        transform[12 + lane] += delta[lane];
    }
    Some(transform)
}

/// A static piece's kind: `MISSION_PROPS` indexes, then `PARK_PIECES` ones after them.
fn static_kind(guid: u64) -> Option<usize> {
    if crate::ui::object_dropper::is_dynamic_park_prop(guid) {
        return None;
    }
    mission_prop(guid).or_else(|| park_piece(guid).map(|piece| MISSION_PROPS.len() + piece))
}

/// The tab's subcategories: each one's first catalog entry and length, in order.
fn tab_categories(tab: usize) -> Vec<(usize, usize)> {
    categories().into_iter().filter(|&(first, _)| prop_tab(first) == tab).collect()
}

/// Copies of the player's board out at once: each is a real board in the skater pool, which it
/// shares with AI and party skaters (16 members, two per skater).
const MAX_BOARDS: usize = 2;
/// A resting board's deck above the ground (the riderless board corpus: 0.087 m).
const BOARD_REST_DECK_HEIGHT: f32 = 0.087;
/// Prop-menu object IDs for dropped boards: living-world entity IDs carry their small entity
/// type in the top byte, so this type byte names no entity; the board's key follows.
const BOARD_PROP_ID: u32 = 0xB0 << 24;
/// An on-foot skater gets a dropped board's prop menu (the one Skate 2 shows by a grabbable
/// prop) within this of its deck, across the ground and up (metres).
const BOARD_MENU_REACH: f32 = 1.2;
const BOARD_MENU_HEIGHT: f32 = 1.5;
/// A deck whose up axis has at least this world-up component counts as upright.
const BOARD_UPRIGHT_UP_Y: f32 = 0.9;
/// Righting a board starts it this far above its deck and lets it fall (metres).
const BOARD_RIGHT_LIFT: f32 = 0.15;

#[derive(Clone)]
struct Dropped {
    /// A prop's own locator; unused for a board.
    locator: u64,
    /// A copy of the player's board: its riderless board in the skater pool.
    board: Option<RiderlessBoardHandle>,
    prop: usize,
    /// Where it was placed, then (in the editor) where it is now; a board's is its deck.
    transform: [f32; 16],
    /// A board's key in its prop-menu object ID (`BOARD_PROP_ID`).
    key: u32,
    /// A board's live deck as of the last frame.
    deck: Option<[f32; 16]>,
    /// A board added to the session marker: the deck transform the marker puts it back at.
    marker: Option<[f32; 16]>,
    /// A mission prop: its immovable Skate 3 body's ID (it has no locator or object).
    mission: Option<u64>,
    /// A marquee character or model: its key in `characters` (no locator or body).
    character: Option<u64>,
}

/// A placed (or held) marquee character: the viewer's clip on its recipe, and its model.
struct PlacedCharacter {
    character: crate::nis::characters::SceneCharacter,
    models: super::scene_characters::SceneCharacterModels,
    animated: bool,
    /// Seconds into its clip, which loops back to its start at its end.
    time: f32,
    /// Its collision once placed and drawn: a box over its drawn bounds (world space), an
    /// immovable body with this ID.
    shape: Option<std::sync::Arc<MissionPropShape>>,
    body: u64,
}

/// The clip a setting names, as its database orders them: the viewer's own file lists its clips
/// with its idle first (`Viewer::clip_names`), every other file in database order.
fn database_clip(
    viewer: &super::freeskate_viewer::Viewer,
    data: &crate::nis::characters::SceneAnimData,
    settings: PropSettings,
) -> usize {
    if settings.anim_file != 0 {
        return usize::from(settings.clip);
    }
    let names = viewer.clip_names();
    let name = names.get(usize::from(settings.clip)).or_else(|| names.first());
    name.and_then(|name| data.database.clips.iter().position(|clip| clip.name.eq_ignore_ascii_case(name))).unwrap_or(0)
}

/// How many clips the loaded animation file holds (the viewer's own, file 0, counts from it).
fn self_clip_count(
    animations: &std::collections::BTreeMap<u16, Option<std::sync::Arc<crate::nis::characters::SceneAnimData>>>,
    file: u16,
) -> usize {
    animations.get(&file).and_then(|data| data.as_ref()).map_or(0, |data| data.database.clips.len())
}

/// A drawn character's posed mesh as world triangles: each vertex skinned on the CPU exactly
/// as the character vertex shader skins it (`skin_world`: weights / 255 with the fourth the
/// rest, indices times three into the published palette rows, the position stored halved and
/// lowered 0.4). None when the palette has not published or a mesh lacks its skinning.
fn posed_triangles(palette: &crate::character::palette::CharacterPalette) -> Option<Vec<[[f32; 3]; 3]>> {
    use skate2_formats::render::{NativeVertexAttributeValues, PRIMITIVE_TYPE_TRIANGLE_LIST, VertexSemantic};
    let scene = palette.scene();
    let mut triangles = Vec::new();
    for (part_index, part) in palette.skinning().parts.iter().enumerate() {
        let lod = &scene.components.get(part.component_index)?.models.get(part.model_index)?.lods.get(part.lod_index)?;
        let Some(model) = lod.arena.instances.iter().find_map(|instance| instance.render_model).and_then(|index| lod.arena.models.get(index)) else { continue };
        for (mesh_index, link) in model.meshes.iter().enumerate() {
            let geometry = &lod.arena.meshes.get(link.mesh_index)?.geometry;
            let rows = palette.mesh_rows(part_index, mesh_index).ok()?.rows;
            let bytes = |semantic| {
                geometry.attributes.iter().find(|a| a.semantic == semantic).and_then(|a| match &a.values {
                    NativeVertexAttributeValues::UnsignedByte4(values) => Some(values),
                    _ => None,
                })
            };
            let (Some(weights), Some(indices)) = (bytes(VertexSemantic::BlendWeight(0)), bytes(VertexSemantic::BlendIndices(0))) else { return None };
            let skinned: Vec<[f32; 3]> = geometry
                .positions
                .iter()
                .enumerate()
                .map(|(vertex, position)| {
                    let stored = weights.get(vertex).copied().unwrap_or([255, 0, 0, 0]);
                    let mut weight = stored.map(|w| f32::from(w) / 255.0);
                    weight[3] = 1.0 - weight[0] - weight[1] - weight[2];
                    let bone = indices.get(vertex).copied().unwrap_or([0; 4]).map(|i| usize::from(i) * 3);
                    let p = [position[0] * 2.0, position[1] * 2.0 + 0.8, position[2] * 2.0, 1.0];
                    std::array::from_fn(|row| {
                        (0..4)
                            .map(|k| {
                                let r = rows.get(bone[k] + row).copied().unwrap_or([0.0; 4]);
                                weight[k] * (p[0] * r[0] + p[1] * r[1] + p[2] * r[2] + p[3] * r[3])
                            })
                            .sum()
                    })
                })
                .collect();
            for draw in &geometry.draws {
                if draw.primitive_type != PRIMITIVE_TYPE_TRIANGLE_LIST {
                    continue;
                }
                let start = draw.start_index as usize;
                let end = (start + draw.index_count as usize).min(geometry.indices.len());
                for corners in geometry.indices[start..end].chunks_exact(3) {
                    let at = |i: u32| usize::try_from(i64::from(i) + i64::from(draw.base_vertex_index)).ok().and_then(|v| skinned.get(v).copied());
                    if let (Some(a), Some(b), Some(c)) = (at(corners[0]), at(corners[1]), at(corners[2])) {
                        triangles.push([a, b, c]);
                    }
                }
            }
        }
    }
    (!triangles.is_empty()).then_some(triangles)
}

/// World triangles as a mission prop shape (identity placement) with the native defaults a mesh
/// without explicit edges builds (`FLAG_TRIANGLE_DEFAULT`); degenerate ones left out.
fn triangles_shape(triangles: &[[[f32; 3]; 3]]) -> MissionPropShape {
    let mut shape = MissionPropShape::default();
    for &vertices in triangles {
        let [a, b, c] = vertices;
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = cross(u, v);
        if dot(n, n) < 1e-10 || vertices.iter().flatten().any(|value| !value.is_finite()) {
            continue;
        }
        shape.triangles.push(skate2_formats::collision::MeshTriangle { cluster_index: 0, unit_stream_offset: 0, vertices, group_id: None, surface_id: None });
        shape.features.push(crate::static_ground_pool::ResidentGroundFeatures { flags: 0x01e0, edge_cosines: [0.0; 3] });
    }
    shape
}

/// A box over `minimum`..`maximum` as twelve outward-facing triangles with the native defaults
/// a mesh without explicit edges builds (`FLAG_TRIANGLE_DEFAULT`).
const IDENTITY: [f32; 16] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];

fn box_shape(minimum: [f32; 3], maximum: [f32; 3]) -> MissionPropShape {
    let corner = |x: usize, y: usize, z: usize| [[minimum[0], maximum[0]][x], [minimum[1], maximum[1]][y], [minimum[2], maximum[2]][z]];
    // Each face's corners counter-clockwise seen from outside.
    let faces = [
        [corner(1, 0, 0), corner(1, 1, 0), corner(1, 1, 1), corner(1, 0, 1)],
        [corner(0, 0, 1), corner(0, 1, 1), corner(0, 1, 0), corner(0, 0, 0)],
        [corner(0, 1, 0), corner(0, 1, 1), corner(1, 1, 1), corner(1, 1, 0)],
        [corner(0, 0, 1), corner(0, 0, 0), corner(1, 0, 0), corner(1, 0, 1)],
        [corner(1, 0, 1), corner(1, 1, 1), corner(0, 1, 1), corner(0, 0, 1)],
        [corner(0, 0, 0), corner(0, 1, 0), corner(1, 1, 0), corner(1, 0, 0)],
    ];
    let mut shape = MissionPropShape::default();
    for [a, b, c, d] in faces {
        for vertices in [[a, b, c], [a, c, d]] {
            shape.triangles.push(skate2_formats::collision::MeshTriangle { cluster_index: 0, unit_stream_offset: 0, vertices, group_id: None, surface_id: None });
            shape.features.push(crate::static_ground_pool::ResidentGroundFeatures { flags: 0x01e0, edge_cosines: [0.0; 3] });
        }
    }
    shape
}

/// The editor's camera: where it is and the way it looks.
#[derive(Clone, Copy, Debug)]
struct View {
    position: [f32; 3],
    at: [f32; 3],
}

/// A prop in the editor's hands: drawn, not yet in the world.
#[derive(Clone, Copy, Debug)]
struct Held {
    prop: usize,
    /// Row-major (axis rows, translation row 3); a board's is its deck.
    transform: [f32; 16],
    /// The heading L3 stands it back up at.
    home: f32,
    /// Picked up from where it was placed: B puts it back there.
    picked: Option<[f32; 16]>,
}

#[derive(Default)]
pub(super) struct ObjectDropper {
    view: Option<View>,
    /// Leaving waits for an idle pad, so the B that left does not reach the skater.
    leaving: bool,
    list_open: bool,
    /// The settings page (Start), over whatever the editor was doing.
    settings_open: bool,
    settings_row: usize,
    settings: EditorSettings,
    /// The settings as last loaded or saved: closing the page saves only a change.
    settings_saved: EditorSettings,
    /// The list shows the selected subcategory's items, not the subcategories.
    list_items: bool,
    tab: usize,
    /// The catalog entry selected in the Props tab.
    selected: usize,
    held: Option<Held>,
    orbit_distance: f32,
    /// The left stick has been back in the middle since its last flick.
    flick_ready: bool,
    /// Frames since the editor opened.
    open_frames: u32,
    /// A flick's quarter turn still to spin: its axis and the angle left (radians).
    spin: Option<([f32; 3], f32)>,
    /// How long the stick has been turning the prop finely (seconds).
    fine_hold: f32,
    /// LT + RT fine-tuned the held piece: it does not snap (snapping would square its turn and
    /// move it) until the left stick moves it again.
    fine_tuned: bool,
    triggers: [u8; 2],
    /// The placed prop under the crosshair (index into `dropped`).
    target: Option<usize>,
    note: &'static str,
    dropped: Vec<Dropped>,
    buttons: u16,
    next_locator: u64,
    next_board_key: u32,
    /// The local board's deck (Part 6, world) and each dropped board's, as of the last frame.
    local_board_deck: Option<[f32; 16]>,
    dropped_board_decks: Vec<[f32; 16]>,
    /// The held prop's bank asset and RModel entry, for drawing it.
    preview_model: Option<(usize, u64, u32)>,
    /// Each looked-up prop's bounding box centre (model space), which it turns about, and half its
    /// diagonal, how far from that centre the crosshair still finds it.
    centers: std::collections::BTreeMap<usize, ([f32; 3], f32)>,
    /// Each mission prop's collision, cut from its hull once (by `MISSION_PROPS` index).
    mission_shapes: std::collections::BTreeMap<usize, std::sync::Arc<MissionPropShape>>,
    next_mission_body: u64,
    /// Each prop type's settings (static, add to marker), by `PROPS` index, and the page that
    /// changes the held one's (Start while holding).
    prop_settings: std::collections::BTreeMap<usize, PropSettings>,
    prop_settings_open: bool,
    prop_settings_row: usize,
    /// The character viewer's clip and faces, loaded with the first character; each placed or
    /// held character by key, and the held one's key.
    viewer: Option<super::freeskate_viewer::Viewer>,
    /// Animation databases loaded so far, by `ANIMATION_FILES` index (None: one that will not
    /// load).
    animations: std::collections::BTreeMap<u16, Option<std::sync::Arc<crate::nis::characters::SceneAnimData>>>,
    characters: std::collections::BTreeMap<u64, PlacedCharacter>,
    next_character: u64,
    held_character: Option<u64>,
    /// R3 while holding a character: the camera orbits the placement rather than the character's
    /// hips as drawn (which it follows through its clip by default).
    camera_on_placement: bool,
    /// B put the held prop back for another from the list: where it was (its transform, its
    /// heading for L3 and what it was), so the next one taken goes there with the camera as it is.
    swap_spot: Option<Held>,
    /// Each Skate 3 piece's snap faces, read the first time it is held or placed (by
    /// `PARK_PIECES` index).
    snap_faces: std::collections::BTreeMap<usize, std::sync::Arc<Vec<super::park_assets::SnapFace>>>,
    /// Where the held piece snaps this frame: it is drawn and placed there.
    snapped: Option<[f32; 16]>,
    /// What it snapped to, for the panel.
    snapped_to: &'static str,
    /// The last ground and wall snap: for the held transform (its bits), piece and placement
    /// publication it was found for, where it put the piece and what it snapped to. A piece
    /// held still keeps it instead of probing the world again every frame.
    ground_snapped: Option<([u32; 16], usize, u64, Option<[f32; 16]>, &'static str)>,
}

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::Visual, format!("object dropper: {e}"))
}

fn axis(value: i16) -> f32 {
    (f32::from(value) / 32767.0).clamp(-1.0, 1.0)
}

fn cubic(value: f32) -> f32 {
    let cubed = value * value * value;
    if cubed.abs() > DEADZONE { cubed } else { 0.0 }
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(v: [f32; 3], s: f32) -> [f32; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt();
    if length > 1e-6 { scale(v, 1.0 / length) } else { v }
}

/// `v` turned `angle` radians about the unit `axis` (right-handed, as the manual camera turns).
fn rotate(v: [f32; 3], axis: [f32; 3], angle: f32) -> [f32; 3] {
    let (sin, cos) = angle.sin_cos();
    add(add(scale(v, cos), scale(cross(axis, v), sin)), scale(axis, dot(axis, v) * (1.0 - cos)))
}

const UP: [f32; 3] = [0.0, 1.0, 0.0];

/// The camera's way turned by `yaw` about the vertical, then `pitch` about its own side, as Skate
/// 3's manual camera looks (never straight up or down).
fn look(at: [f32; 3], yaw: f32, pitch: f32) -> [f32; 3] {
    let at = rotate(at, UP, yaw);
    let side = normalize(cross(UP, at));
    let pitched = normalize(rotate(at, side, pitch));
    if pitched[1].abs() < 0.97 { pitched } else { normalize(at) }
}

/// The camera's way across the ground, and the way the left stick's right is (Skate 3's manual
/// camera strafes along `at x up`).
fn ground_axes(at: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let flat = [at[0], 0.0, at[2]];
    let flat = if dot(flat, flat) > 1e-8 { normalize(flat) } else { [0.0, 0.0, 1.0] };
    (flat, normalize(cross(flat, UP)))
}

fn heading(transform: &[f32; 16]) -> f32 {
    if transform[8].hypot(transform[10]) > 1e-4 { transform[8].atan2(transform[10]) } else { 0.0 }
}

/// A transform's axis rows turned `angle` about `axis`, then made orthonormal again.
fn turn(transform: &mut [f32; 16], axis: [f32; 3], angle: f32) {
    let row = |t: &[f32; 16], i: usize| [t[i * 4], t[i * 4 + 1], t[i * 4 + 2]];
    let x = rotate(row(transform, 0), axis, angle);
    let z = normalize(rotate(row(transform, 2), axis, angle));
    let y = normalize(cross(z, x));
    let x = normalize(cross(y, z));
    for (i, axis) in [x, y, z].into_iter().enumerate() {
        transform[i * 4..i * 4 + 3].copy_from_slice(&axis);
    }
}

fn origin(transform: &[f32; 16]) -> [f32; 3] {
    [transform[12], transform[13], transform[14]]
}

/// `local` (model space) placed by a row-major transform.
fn place(transform: &[f32; 16], local: [f32; 3]) -> [f32; 3] {
    let mut point = origin(transform);
    for (row, value) in local.into_iter().enumerate() {
        point = add(point, scale([transform[row * 4], transform[row * 4 + 1], transform[row * 4 + 2]], value));
    }
    point
}

/// A prop's middle in its own model space: its bounding box's centre once its model has been
/// looked up, else `PROP_MIDDLE` above its origin; a board's deck is its middle.
fn local_middle(centers: &std::collections::BTreeMap<usize, ([f32; 3], f32)>, prop: usize) -> [f32; 3] {
    if is_board(PROPS[prop].2) {
        return [0.0; 3];
    }
    centers.get(&prop).map_or([0.0, PROP_MIDDLE, 0.0], |center| center.0)
}

/// How far out the held prop's orbit camera may zoom: `ORBIT_MAX`, or for a big piece far
/// enough to see all of it (`SPAWN_CLEARANCE` times its reach).
fn orbit_max(centers: &std::collections::BTreeMap<usize, ([f32; 3], f32)>, prop: usize) -> f32 {
    ORBIT_MAX.max(centers.get(&prop).map_or(0.0, |center| center.1 * SPAWN_CLEARANCE))
}

/// How far from its middle the crosshair finds a placed prop: half its bounding box's diagonal,
/// at least `TARGET_WIDTH`.
fn target_reach(centers: &std::collections::BTreeMap<usize, ([f32; 3], f32)>, prop: usize) -> f32 {
    centers.get(&prop).map_or(TARGET_WIDTH, |center| center.1.max(TARGET_WIDTH))
}

/// Where a placed or held thing is aimed at, orbited and turned about: its middle.
fn middle(centers: &std::collections::BTreeMap<usize, ([f32; 3], f32)>, prop: usize, transform: &[f32; 16]) -> [f32; 3] {
    place(transform, local_middle(centers, prop))
}

/// `turn` about the prop's middle (`local`, model space) rather than its origin.
fn turn_about(transform: &mut [f32; 16], axis: [f32; 3], angle: f32, local: [f32; 3]) {
    let fixed = place(transform, local);
    turn(transform, axis, angle);
    let moved = place(transform, local);
    for lane in 0..3 {
        transform[12 + lane] += fixed[lane] - moved[lane];
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// Whether the editor's camera holds the screen (the skater's input is held neutral).
    #[must_use]
    pub fn object_dropper_open(&self) -> bool {
        self.object_dropper.view.is_some()
    }

    /// Whether to hold B for the local skater: on its board (the foot brake), from the editor's
    /// third frame, so the editor receives a fresh B press.
    pub(super) fn object_dropper_skater_riding(&self) -> bool {
        if self.object_dropper.open_frames < BRAKE_AFTER_FRAMES {
            return false;
        }
        let Some(actor) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_ref()) else {
            return false;
        };
        let Ok(physical) = actor.cached_physical_allocation() else {
            return false;
        };
        let Ok(output) = physical.owner().phys_out().lock() else {
            return false;
        };
        output.try_state_output().is_ok_and(|state| !state.is_offboard)
    }

    /// The board decks the visual snapshot draws copies of the player's board with.
    pub(super) fn object_dropper_board_poses(&self) -> (Option<[f32; 16]>, Vec<[f32; 16]>) {
        (self.object_dropper.local_board_deck, self.object_dropper.dropped_board_decks.clone())
    }

    /// Once a frame after the simulation: where the local board's deck is now (physics Part 6),
    /// the frame of reference for drawing dropped copies of the player's board.
    pub(super) fn refresh_object_dropper_boards(&mut self) -> Result<(), SelectedLiveError> {
        let deck = self
            .inner
            .try_with_selected_queued_frame(|frame, _, _| {
                frame
                    .world
                    .board(frame.board)
                    .ok()
                    .and_then(|board| board.try_live_part_transform_host(frame.world.backend(), 6).ok())
            })
            .map_err(|e| error(format!("{e:?}")))?;
        self.object_dropper.local_board_deck = deck;
        let mut decks = Vec::new();
        let mut grind = Vec::new();
        if self.object_dropper.dropped.iter().any(|dropped| dropped.board.is_some()) {
            let owner = self.inner.world.as_ref().ok_or_else(|| error("world absent"))?;
            let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
            for dropped in &mut self.object_dropper.dropped {
                dropped.deck = dropped
                    .board
                    .as_ref()
                    .and_then(|board| world.riderless_board_part_transforms(board).ok().map(|parts| parts[6]));
                decks.extend(dropped.deck);
                if PROPS[dropped.prop].2 == YOUR_GRIND_BOARD {
                    grind.extend(dropped.deck.map(|deck| (BOARD_PROP_ID | dropped.key, deck)));
                }
            }
        }
        self.object_dropper.dropped_board_decks = decks;
        // A grind board's deck edges are ground where its deck is now.
        crate::ui::object_dropper::publish_grind_boards(grind);
        Ok(())
    }

    /// Opens the editor on LB + D-pad Left, when play has the game's focus.
    /// True when it opened (the caller skips this frame's simulation).
    pub(super) fn try_open_object_dropper(&mut self, pad: Option<XboxPadSample>, allowed: bool) -> bool {
        let buttons = pad.map_or(0, |pad| pad.buttons);
        let pressed = buttons & !self.object_dropper.buttons;
        self.object_dropper.buttons = buttons;
        if !allowed || buttons & LEFT_BUMPER == 0 || pressed & DPAD_LEFT == 0 {
            return false;
        }
        let Some(matrix) = self.game_camera_matrix() else {
            return false;
        };
        let dropper = &mut self.object_dropper;
        dropper.view = Some(View {
            position: [matrix[12], matrix[13], matrix[14]],
            at: look(normalize([matrix[8], matrix[9], matrix[10]]), 0.0, 0.0),
        });
        dropper.leaving = false;
        dropper.list_open = false;
        dropper.list_items = false;
        dropper.swap_spot = None;
        dropper.tab = PROPS_TAB;
        dropper.held = None;
        dropper.target = None;
        dropper.flick_ready = false;
        dropper.open_frames = 0;
        dropper.triggers = [255; 2];
        dropper.settings_open = false;
        dropper.prop_settings = crate::ui::object_dropper::load_prop_settings();
        dropper.prop_settings_open = false;
        dropper.settings = EditorSettings::load();
        dropper.settings.publish();
        dropper.settings_saved = dropper.settings;
        dropper.note = "LB + D-pad Left opened the park editor";
        self.show_object_dropper_menu();
        true
    }

    /// One frame of the editor, after the world's (held neutral) simulation tick.
    pub(super) fn update_object_dropper(&mut self, pad: Option<XboxPadSample>, dt: f32) -> Result<(), SelectedLiveError> {
        let pad = pad.unwrap_or(XboxPadSample {
            buttons: 0,
            left_trigger: 0,
            right_trigger: 0,
            left_x: 0,
            left_y: 0,
            right_x: 0,
            right_y: 0,
        });
        let dropper = &mut self.object_dropper;
        dropper.open_frames = dropper.open_frames.saturating_add(1);
        let pressed = pad.buttons & !dropper.buttons;
        dropper.buttons = pad.buttons;
        let held = [pad.left_trigger >= TRIGGER_HELD, pad.right_trigger >= TRIGGER_HELD];
        let triggers_pressed = [held[0] && dropper.triggers[0] < TRIGGER_HELD, held[1] && dropper.triggers[1] < TRIGGER_HELD];
        dropper.triggers = [pad.left_trigger, pad.right_trigger];
        let stick = [axis(pad.left_x), axis(pad.left_y)];
        let flick = if stick[0].hypot(stick[1]) < STICK_REST {
            dropper.flick_ready = true;
            None
        } else if dropper.flick_ready && (stick[0].abs() > STICK_FLICK || stick[1].abs() > STICK_FLICK) {
            dropper.flick_ready = false;
            Some(if stick[0].abs() > stick[1].abs() { [stick[0].signum(), 0.0] } else { [0.0, stick[1].signum()] })
        } else {
            None
        };
        if dropper.leaving {
            let idle = pad.buttons == 0 && pad.left_trigger < 30 && pad.right_trigger < 30;
            if idle {
                dropper.view = None;
                dropper.leaving = false;
                crate::ui::object_dropper::set_menu(None);
            }
            return Ok(());
        }
        if dropper.settings_open {
            self.update_editor_settings(pressed);
        } else if dropper.prop_settings_open {
            self.update_prop_settings(pressed);
            self.orbit_held()?;
        } else if dropper.list_open {
            self.update_object_list(pressed, triggers_pressed, flick)?;
        } else if pressed & START != 0 && dropper.held.is_some() {
            dropper.prop_settings_open = true;
            dropper.prop_settings_row = 0;
            dropper.note = "";
        } else if pressed & START != 0 {
            dropper.settings_open = true;
            dropper.note = "";
        } else if dropper.held.is_some() {
            self.update_held_prop(&pad, pressed, held, flick, dt)?;
        } else {
            self.update_free_camera(&pad, pressed, dt)?;
        }
        self.refresh_dropper_preview_model()?;
        self.show_object_dropper_menu();
        Ok(())
    }

    /// The object list, as Skate 3's: LT/RT tab; the subcategories (D-pad Up/Down, A opens one,
    /// B or X closes the list), then its items (D-pad Up/Down or a stick flick, LB/RB the next
    /// subcategory's, A takes it, B back to the subcategories). A Skate 3 piece's themes are one
    /// row, taken in the settings' theme (Start opens the settings over the list).
    fn update_object_list(&mut self, pressed: u16, triggers: [bool; 2], flick: Option<[f32; 2]>) -> Result<(), SelectedLiveError> {
        let dropper = &mut self.object_dropper;
        dropper.note = "";
        if pressed & START != 0 {
            dropper.settings_open = true;
            return Ok(());
        }
        let take = navigate_object_list(dropper, pressed, triggers, flick);
        dropper.selected = in_theme(dropper.selected, dropper.settings.theme());
        if take {
            self.take_from_list()?;
        }
        Ok(())
    }
}

/// The object list's moves (`update_object_list`); whether A takes the selected item.
fn navigate_object_list(dropper: &mut ObjectDropper, pressed: u16, triggers: [bool; 2], flick: Option<[f32; 2]>) -> bool {
    {
        if triggers[0] || triggers[1] {
            let step = if triggers[0] { TABS.len() - 1 } else { 1 };
            dropper.tab = (dropper.tab + step) % TABS.len();
            dropper.list_items = false;
            if let Some(&(first, _)) = tab_categories(dropper.tab).first() {
                dropper.selected = first;
            }
        }
        let step = flick.map_or(0.0, |flick| -flick[1]);
        let down = pressed & DPAD_DOWN != 0 || step > 0.0;
        let up = pressed & DPAD_UP != 0 || step < 0.0;
        let (start, len) = category_run(dropper.selected);
        let runs = tab_categories(dropper.tab);
        let run = runs.iter().position(|&(first, _)| first == start);
        let next_run = |forward: bool| run.map_or(runs.first(), |at| runs.get((at + if forward { 1 } else { runs.len() - 1 }) % runs.len())).map(|&(first, _)| first);
        if !dropper.list_items {
            if pressed & (B | X) != 0 {
                dropper.list_open = false;
                dropper.swap_spot = None;
            } else if !runs.is_empty() {
                if down && let Some(first) = next_run(true) {
                    dropper.selected = first;
                }
                if up && let Some(first) = next_run(false) {
                    dropper.selected = first;
                }
                if pressed & A != 0 && prop_tab(dropper.selected) == dropper.tab {
                    dropper.list_items = true;
                }
            }
            return false;
        }
        if pressed & B != 0 {
            dropper.list_items = false;
            dropper.selected = start;
            return false;
        }
        if pressed & RIGHT_BUMPER != 0 && let Some(first) = next_run(true) {
            dropper.selected = first;
        }
        if pressed & LEFT_BUMPER != 0 && let Some(first) = next_run(false) {
            dropper.selected = first;
        }
        let rows = category_rows(start, len);
        let row = rows.iter().position(|&first| first == theme_row(dropper.selected).0).unwrap_or(0);
        if down {
            dropper.selected = rows[(row + 1) % rows.len()];
        }
        if up {
            dropper.selected = rows[(row + rows.len() - 1) % rows.len()];
        }
        pressed & A != 0
    }
}

impl<T, C> SelectedLiveSession<T, C> {

    /// Nothing held: fly and look, X opens the list, A picks up and Y deletes the placed prop
    /// under the crosshair, B leaves.
    fn update_free_camera(&mut self, pad: &XboxPadSample, pressed: u16, dt: f32) -> Result<(), SelectedLiveError> {
        let dropper = &mut self.object_dropper;
        let Some(mut view) = dropper.view else {
            return Ok(());
        };
        let (flat, side) = ground_axes(view.at);
        let speed = FLY_SPEED * dropper.settings.scale(EditorSettings::CAMERA_MOVE) * dt;
        view.position = add(view.position, scale(flat, cubic(axis(pad.left_y)) * speed));
        view.position = add(view.position, scale(side, cubic(axis(pad.left_x)) * speed));
        let rise = f32::from(pad.buttons & DPAD_UP != 0) - f32::from(pad.buttons & DPAD_DOWN != 0);
        view.position[1] += rise * RISE_SPEED * dropper.settings.scale(EditorSettings::CAMERA_MOVE) * dt;
        let look_speed = LOOK_SPEED * dropper.settings.scale(EditorSettings::CAMERA_LOOK) * dt;
        view.at = look(view.at, cubic(-axis(pad.right_x)) * look_speed, cubic(-axis(pad.right_y)) * look_speed);
        dropper.view = Some(view);
        self.refresh_dropper_target()?;
        let dropper = &mut self.object_dropper;
        if pressed & B != 0 {
            dropper.leaving = true;
            crate::ui::object_dropper::set_menu(Some(leaving_menu()));
            return Ok(());
        }
        if pressed & X != 0 {
            dropper.list_open = true;
            dropper.list_items = false;
            dropper.note = "";
        } else if pressed & A != 0 {
            let note = match dropper.target {
                Some(index) => {
                    self.pick_up(index)?;
                    "picked it up"
                }
                None => "nothing under the crosshair to pick up",
            };
            self.object_dropper.note = note;
        } else if pressed & Y != 0 {
            let note = match dropper.target.take() {
                Some(index) => {
                    let dropped = self.object_dropper.dropped.remove(index);
                    self.remove_dropped(&dropped)?;
                    "deleted it"
                }
                None => "nothing under the crosshair to delete",
            };
            self.object_dropper.note = note;
        }
        Ok(())
    }

    /// The camera round the held prop along its look, at the orbit distance: on a character's
    /// hips as drawn (unless R3 put it on the placement), so it follows the clip, also while the
    /// prop settings page picks clips; and short of any surface between them.
    fn orbit_held(&mut self) -> Result<(), SelectedLiveError> {
        let dropper = &self.object_dropper;
        let (Some(mut view), Some(held)) = (dropper.view, dropper.held) else {
            return Ok(());
        };
        let hips = dropper
            .held_character
            .filter(|_| !dropper.camera_on_placement)
            .and_then(|key| dropper.characters.get(&key))
            .and_then(|placed| placed.models.visuals().first().map(|visual| visual.hips_position));
        let centre = hips.unwrap_or_else(|| place(&held.transform, local_middle(&dropper.centers, held.prop)));
        view.position = add(centre, scale(view.at, -dropper.orbit_distance));
        // A surface between the prop and the camera: the camera comes in to just before it.
        if let Some(hit) = self.first_hit(centre, view.position, "ObjectDropper::Orbit")? {
            let reach = dot(add(hit, scale(centre, -1.0)), scale(view.at, -1.0));
            view.position = add(centre, scale(view.at, -(reach - CAMERA_CLEARANCE).max(0.0)));
        }
        // And at least that far above what is under it (looking along the ground).
        let [x, y, z] = view.position;
        if let Some(under) = self.first_hit([x, y.max(centre[1]), z], [x, y - CAMERA_CLEARANCE, z], "ObjectDropper::OrbitFloor")? {
            view.position[1] = y.max(under[1] + CAMERA_CLEARANCE);
        }
        self.object_dropper.view = Some(view);
        Ok(())
    }

    /// A prop held: move, raise, orbit, zoom, turn; A places it, B places it and leaves.
    fn update_held_prop(
        &mut self,
        pad: &XboxPadSample,
        pressed: u16,
        triggers: [bool; 2],
        flick: Option<[f32; 2]>,
        dt: f32,
    ) -> Result<(), SelectedLiveError> {
        let dropper = &mut self.object_dropper;
        let (Some(mut view), Some(mut held)) = (dropper.view, dropper.held) else {
            return Ok(());
        };
        let (flat, side) = ground_axes(view.at);
        // Turning "up" tips the prop's near side up: about the camera's left across the ground.
        let tip = scale(side, -1.0);
        let settings = dropper.settings;
        let stick = [axis(pad.left_x), axis(pad.left_y)];
        let local = local_middle(&dropper.centers, held.prop);
        if let Some((axis, left)) = dropper.spin {
            let step = left.signum() * (SPIN_SPEED * settings.scale(EditorSettings::SPIN) * dt).min(left.abs());
            turn_about(&mut held.transform, axis, step, local);
            dropper.spin = (left - step).abs().gt(&1e-5).then_some((axis, left - step));
        }
        let fine = triggers == [true, true] && stick[0].hypot(stick[1]) > STICK_REST;
        dropper.fine_hold = if fine { dropper.fine_hold + dt } else { 0.0 };
        if triggers == [true, true] {
            dropper.fine_tuned = true;
        }
        match triggers {
            [true, true] => {
                let ramp = (dropper.fine_hold / FINE_TURN_RAMP).min(1.0);
                let speed = (FINE_TURN_START + (FINE_TURN_TOP - FINE_TURN_START) * ramp * ramp)
                    * settings.scale(EditorSettings::FINE_TURN) * dt;
                turn_about(&mut held.transform, UP, stick[0] * speed, local);
                turn_about(&mut held.transform, tip, stick[1] * speed, local);
            }
            [false, true] => {
                if let Some([x, y]) = flick.filter(|_| dropper.spin.is_none()) {
                    let quarter = std::f32::consts::FRAC_PI_2;
                    dropper.spin = Some(if x != 0.0 { (UP, x * quarter) } else { (tip, y * quarter) });
                }
            }
            _ => {
                let speed = dropper.orbit_distance * MOVE_RATE * settings.scale(EditorSettings::PROP_MOVE) * dt;
                let across = add(scale(flat, cubic(stick[1]) * speed), scale(side, cubic(stick[0]) * speed));
                held.transform[12] += across[0];
                held.transform[14] += across[2];
                // Moved again: it snaps again.
                if stick[0].hypot(stick[1]) > STICK_REST {
                    dropper.fine_tuned = false;
                }
            }
        }
        let rise = f32::from(pad.buttons & DPAD_UP != 0) - f32::from(pad.buttons & DPAD_DOWN != 0);
        held.transform[13] += rise * dropper.orbit_distance * LIFT_RATE * settings.scale(EditorSettings::PROP_MOVE) * dt;
        let zoom = f32::from(pad.buttons & DPAD_LEFT != 0) - f32::from(pad.buttons & DPAD_RIGHT != 0);
        let orbit_max = orbit_max(&dropper.centers, held.prop);
        dropper.orbit_distance = (dropper.orbit_distance * (zoom * ZOOM_RATE * dt).exp()).clamp(ORBIT_MIN, orbit_max);
        if pressed & R3 != 0 && dropper.held_character.is_some() {
            dropper.camera_on_placement = !dropper.camera_on_placement;
            dropper.note = if dropper.camera_on_placement { "camera on the placement" } else { "camera on the character" };
        }
        if pressed & L3 != 0 {
            dropper.spin = None;
            let fixed = place(&held.transform, local);
            held.transform = crate::ui::object_dropper::prop_transform(origin(&held.transform), held.home);
            let moved = place(&held.transform, local);
            for lane in 0..3 {
                held.transform[12 + lane] += fixed[lane] - moved[lane];
            }
        }
        let look_speed = LOOK_SPEED * settings.scale(EditorSettings::CAMERA_LOOK) * dt;
        view.at = look(view.at, cubic(-axis(pad.right_x)) * look_speed, cubic(-axis(pad.right_y)) * look_speed);
        dropper.view = Some(view);
        dropper.held = Some(held);
        self.orbit_held()?;
        self.refresh_held_snap()?;
        if pressed & A != 0 {
            // A snapped piece goes where it snapped.
            if let (Some(snapped), Some(held)) = (self.object_dropper.snapped.take(), self.object_dropper.held.as_mut()) {
                held.transform = snapped;
            }
            let Some(held) = self.object_dropper.held else {
                return Ok(());
            };
            // LB held: the placed prop's copy (same turn and height) stays in hand.
            let copy = (pad.buttons & LEFT_BUMPER != 0).then_some(held);
            self.object_dropper.note = self.place_held()?;
            if let Some(copy) = copy
                && self.object_dropper.held.is_none()
            {
                self.hold(Held { picked: None, ..copy })?;
                self.object_dropper.note = "placed; a copy is in hand";
            }
        } else if pressed & B != 0 {
            self.cancel_held()?;
        } else if pressed & Y != 0 {
            let dropper = &mut self.object_dropper;
            dropper.held = None;
            dropper.spin = None;
            if let Some(key) = dropper.held_character.take() {
                dropper.characters.remove(&key);
            }
            dropper.note = "deleted it";
        }
        Ok(())
    }

    /// Restore editor props after challenge/game resets in any mode. The native
    /// census keeps its ordinary distance/capacity rules when enabled; modes that
    /// disable it use the same explicit drop path as the editor.
    pub(super) fn refresh_dropped_props(&mut self) -> Result<(), SelectedLiveError> {
        let locators: Vec<u64> = self.object_dropper.dropped.iter().filter(|dropped| dropped.board.is_none() && dropped.mission.is_none()).map(|dropped| dropped.locator).collect();
        if locators.is_empty() {
            return Ok(());
        }
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let (Some(living), Some(recipes)) = (frame.living_world, frame.living_world_recipes.as_ref()) else {
                    return Ok(());
                };
                living
                    .try_respawn_dropped_locators(frame.world, frame.actor, frame.cvars, recipes, &locators)
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)
    }

    /// Explicitly leave the editor, returning a picked-up prop to its last placement.
    pub(super) fn close_object_dropper(&mut self) -> Result<(), SelectedLiveError> {
        self.cancel_held()?;
        let dropper = &mut self.object_dropper;
        dropper.view = None;
        dropper.leaving = false;
        dropper.list_open = false;
        dropper.settings_open = false;
        crate::ui::object_dropper::set_menu(None);
        Ok(())
    }

    fn cancel_held(&mut self) -> Result<(), SelectedLiveError> {
        let Some(held) = self.object_dropper.held.take() else {
            return Ok(());
        };
        if let Some(transform) = held.picked {
            if is_board(PROPS[held.prop].2) {
                self.drop_board(held.prop, transform)?;
            } else {
                self.drop_prop(held.prop, transform)?;
            }
        }
        if let Some(key) = self.object_dropper.held_character.take() {
            self.object_dropper.characters.remove(&key);
        }
        let dropper = &mut self.object_dropper;
        dropper.swap_spot = held.picked.is_none().then_some(held);
        dropper.spin = None;
        dropper.list_open = true;
        dropper.list_items = true;
        dropper.tab = prop_tab(held.prop);
        dropper.selected = held.prop;
        dropper.note = "";
        Ok(())
    }

    /// The settings page: D-pad Up/Down pick a setting, Left/Right change it, X sets it back to
    /// its default, B (or Start) closes the page, saving any change. The board weight takes
    /// effect at once.
    fn update_editor_settings(&mut self, pressed: u16) {
        let dropper = &mut self.object_dropper;
        let rows = EditorSettings::ROWS.len();
        if pressed & DPAD_DOWN != 0 {
            dropper.settings_row = (dropper.settings_row + 1) % rows;
        }
        if pressed & DPAD_UP != 0 {
            dropper.settings_row = (dropper.settings_row + rows - 1) % rows;
        }
        let (min, max, step) = EditorSettings::limits(dropper.settings_row);
        let value = &mut dropper.settings.percent[dropper.settings_row];
        if dropper.settings_row == EditorSettings::THEME {
            // The theme goes round.
            let count = max + 1;
            if pressed & DPAD_RIGHT != 0 {
                *value = (*value + 1) % count;
            }
            if pressed & DPAD_LEFT != 0 {
                *value = (*value + count - 1) % count;
            }
        } else if pressed & DPAD_RIGHT != 0 {
            *value = (*value + step).min(max);
        }
        if pressed & DPAD_LEFT != 0 {
            *value = value.saturating_sub(step).max(min);
        }
        if pressed & X != 0 {
            *value = EditorSettings::DEFAULTS[dropper.settings_row];
        }
        dropper.settings.publish();
        if pressed & (B | START) != 0 {
            dropper.settings_open = false;
            dropper.note = "";
            if dropper.settings != dropper.settings_saved {
                dropper.note = match dropper.settings.save() {
                    Ok(()) => "settings saved",
                    Err(_) => "could not save park-editor.json",
                };
                dropper.settings_saved = dropper.settings;
            }
        }
    }

    /// The held prop type's settings page: D-pad Up/Down pick a row, Left/Right or A change it,
    /// B (or Start) closes it, saving any change. A mission prop is always static; a copy of the
    /// player's board always dynamic.
    fn update_prop_settings(&mut self, pressed: u16) {
        let dropper = &mut self.object_dropper;
        let Some(held) = dropper.held else {
            dropper.prop_settings_open = false;
            return;
        };
        if pressed & (DPAD_DOWN | DPAD_UP) != 0 {
            dropper.prop_settings_row ^= 1;
        }
        let guid = PROPS[held.prop].2;
        let before = dropper.prop_settings.get(&held.prop).copied().unwrap_or_default();
        let mut after = before;
        if pressed & (DPAD_LEFT | DPAD_RIGHT | A) != 0 {
            let fixed_kind = static_kind(guid).is_some() || marquee_entry(guid).is_some();
            let animated = marquee_entry(guid).is_some_and(|entry| MARQUEE[entry].1);
            let back = pressed & DPAD_LEFT != 0;
            match dropper.prop_settings_row {
                0 if animated => {
                    let files = crate::ui::object_dropper::ANIMATION_FILES.len();
                    let step = if back { files - 1 } else { 1 };
                    after.anim_file = u16::try_from((usize::from(after.anim_file) + step) % files).unwrap_or(0);
                    after.clip = 0;
                }
                1 if animated => {
                    let clips = if after.anim_file == 0 { dropper.viewer.as_ref().map_or(1, |viewer| viewer.data().database.clips.len()) } else { self_clip_count(&dropper.animations, after.anim_file) }.max(1);
                    let step = if back { clips - 1 } else { 1 };
                    after.clip = u16::try_from((usize::from(after.clip) + step) % clips).unwrap_or(0);
                }
                0 if !fixed_kind && !is_board(guid) => after.fixed = !after.fixed,
                1 if !fixed_kind => after.marker = !after.marker,
                _ => {}
            }
        }
        if after != before {
            dropper.prop_settings.insert(held.prop, after);
            dropper.note = match crate::ui::object_dropper::save_prop_settings(&dropper.prop_settings) {
                Ok(()) => "prop settings saved",
                Err(_) => "could not save park-editor-props.json",
            };
        }
        if pressed & (B | START) != 0 {
            dropper.prop_settings_open = false;
        }
        if (after.anim_file, after.clip) != (before.anim_file, before.clip) {
            self.restart_held_animation(held, after);
        }
    }

    /// An animation file's database, loaded the first time (None: one that will not load). Files
    /// no character plays any more are let go first (a character keeps its own reference), so
    /// trying clips in the prop settings does not keep every file tried loaded.
    fn animation_data(&mut self, file: u16) -> Option<std::sync::Arc<crate::nis::characters::SceneAnimData>> {
        if file == 0 {
            return self.object_dropper.viewer.as_ref().map(|viewer| std::sync::Arc::clone(viewer.data()));
        }
        if let Some(loaded) = self.object_dropper.animations.get(&file) {
            return loaded.clone();
        }
        self.object_dropper.animations.retain(|_, data| data.as_ref().is_none_or(|data| std::sync::Arc::strong_count(data) > 1));
        let loaded = (|| {
            let path = crate::ui::object_dropper::ANIMATION_FILES.get(usize::from(file))?;
            let bytes = self.inner.assets.as_ref()?.mounts.read_path(path).ok()?;
            crate::nis::characters::SceneAnimData::parse(bytes).ok().map(std::sync::Arc::new)
        })();
        self.object_dropper.animations.insert(file, loaded.clone());
        loaded
    }

    /// The held character plays its type's new animation at once, from its start; its model
    /// stays.
    fn restart_held_animation(&mut self, held: Held, settings: PropSettings) {
        let Some(entry) = marquee_entry(PROPS[held.prop].2) else { return };
        let data = self.animation_data(settings.anim_file);
        let dropper = &mut self.object_dropper;
        let (Some(key), Some(viewer)) = (dropper.held_character, dropper.viewer.as_ref()) else { return };
        let animation = data.as_ref().map(|data| (data, database_clip(viewer, data, settings)));
        let Ok((mut character, models)) = viewer.placed_character(MARQUEE[entry].0, origin(&held.transform), heading(&held.transform), animation) else { return };
        character.in_place = true;
        let _ = character.set_viewer_matrix(held.transform);
        if let Some(placed) = dropper.characters.get_mut(&key) {
            // Its model reloads on the new clip's skeleton (bone order differs per database).
            placed.character = character;
            placed.models = models;
            placed.time = 0.0;
            placed.shape = None;
        }
    }

    /// The held prop type's settings as its page shows them.
    fn prop_settings_rows(&self, prop: usize) -> Vec<(String, Option<usize>)> {
        let settings = self.object_dropper.prop_settings.get(&prop).copied().unwrap_or_default();
        let guid = PROPS[prop].2;
        if marquee_entry(guid).is_some_and(|entry| MARQUEE[entry].1) {
            let files = crate::ui::object_dropper::ANIMATION_FILES.len();
            let file = format!(
                "Animation file   {}  ({}/{files})",
                crate::ui::object_dropper::animation_file_name(usize::from(settings.anim_file)),
                settings.anim_file + 1
            );
            let clip = match self.object_dropper.animations.get(&settings.anim_file) {
                Some(None) => "this file will not load".to_owned(),
                _ if settings.anim_file == 0 => {
                    let names = self.object_dropper.viewer.as_ref().map(super::freeskate_viewer::Viewer::clip_names).unwrap_or_default();
                    format!("{}  ({}/{})", names.get(usize::from(settings.clip)).cloned().unwrap_or_default(), settings.clip + 1, names.len())
                }
                Some(Some(data)) => {
                    let count = data.database.clips.len();
                    let name = data.database.clips.get(usize::from(settings.clip)).map(|c| c.name.clone()).unwrap_or_default();
                    format!("{name}  ({}/{count})", settings.clip + 1)
                }
                None => "loads when chosen".to_owned(),
            };
            return vec![(file, None), (format!("Clip   {clip}"), None)];
        }
        let physics = if marquee_entry(guid).is_some() {
            "Static (marquee model)"
        } else if mission_prop(guid).is_some() {
            "Static (mission prop)"
        } else if park_piece(guid).is_some() && static_kind(guid).is_some() {
            // A Skate 3 prop registered as a living-world model is Dynamic or Static as set.
            "Static (Skate 3 park piece)"
        } else if is_board(guid) {
            "Dynamic (your board)"
        } else if settings.fixed {
            "Static"
        } else {
            "Dynamic"
        };
        let marker = if static_kind(guid).is_some() || marquee_entry(guid).is_some() {
            "n/a (it never moves)"
        } else if settings.marker {
            "On"
        } else {
            "Off"
        };
        vec![(format!("Physics   {physics}"), None), (format!("Add to marker on place   {marker}"), None)]
    }

    fn show_object_dropper_menu(&self) {
        use crate::ui::object_dropper::Menu;
        let dropper = &self.object_dropper;
        if dropper.leaving {
            return;
        }
        // Static pieces (mission props, Skate 3 pieces) have their own budget.
        let fixed = dropper.dropped.iter().filter(|dropped| dropped.mission.is_some()).count();
        let placed = format!("{} of {MAX_DROPPED} props, {fixed} of {MAX_STATIC} pieces placed", dropper.dropped.len() - fixed);
        let mut menu = if let (true, Some(held)) = (dropper.prop_settings_open, dropper.held) {
            Menu {
                title: format!("{} Settings", PROPS[held.prop].1),
                rows: self.prop_settings_rows(held.prop),
                selected: Some(dropper.prop_settings_row),
                info: vec![
                    "D-pad Up/Down: choose".into(),
                    "D-pad Left/Right or A: change".into(),
                    "Static props stay exactly where they are placed.".into(),
                ],
                hints: vec![("A", "Change".into()), ("B", "Back".into())],
                ..Menu::default()
            }
        } else if dropper.settings_open {
            Menu {
                title: "Editor Settings".into(),
                rows: EditorSettings::ROWS
                    .iter()
                    .enumerate()
                    .map(|(row, (_, label))| (format!("{label}   {}", dropper.settings.value_text(row)), None))
                    .collect(),
                selected: Some(dropper.settings_row),
                info: vec!["D-pad Up/Down: choose".into(), "D-pad Left/Right: slower / faster".into()],
                hints: vec![("X", "Default".into()), ("B", "Back".into())],
                ..Menu::default()
            }
        } else if dropper.list_open {
            let tabs = Some((TABS.iter().map(|tab| (*tab).to_owned()).collect(), dropper.tab));
            let (start, len) = category_run(dropper.selected);
            let info: Vec<String> = if dropper.tab == PROPS_TAB { PROPS_INFO.iter() } else { PIECES_INFO.iter() }.map(|line| (*line).to_owned()).collect();
            if !dropper.list_items {
                let runs = tab_categories(dropper.tab);
                Menu {
                    title: "New Object".into(),
                    tabs,
                    rows: runs.iter().map(|&(first, len)| (PROPS[first].0.to_owned(), Some(category_rows(first, len).len()))).collect(),
                    selected: runs.iter().position(|&(first, _)| first == start),
                    info,
                    hints: vec![("A", "Select".into()), ("B", "Back".into())],
                    ..Menu::default()
                }
            } else {
                let all = category_rows(start, len);
                let row = all.iter().position(|&first| first == theme_row(dropper.selected).0).unwrap_or(0);
                let rows = LIST_ROWS.min(all.len());
                let first = row.saturating_sub(rows / 2).min(all.len() - rows);
                let mut info = info;
                if theme_row(dropper.selected).1 > 1 {
                    info.insert(0, format!("Theme: {}   (Start: Settings to change it)", THEMES[dropper.settings.theme()]));
                }
                Menu {
                    title: format!("{}  ({}/{})", PROPS[start].0, row + 1, all.len()),
                    tabs,
                    rows: all[first..first + rows].iter().map(|&index| (row_name(index), None)).collect(),
                    selected: Some(row - first),
                    info,
                    hints: vec![("A", "Take".into()), ("B", "Back".into()), ("RB", "Next group".into()), ("Start", "Theme".into())],
                    ..Menu::default()
                }
            }
        } else if let Some(held) = dropper.held {
            Menu {
                title: PROPS[held.prop].1.into(),
                info: vec![
                    "Left stick: move      D-pad Up/Down: raise / lower".into(),
                    "Right stick: orbit      D-pad Right/Left: closer / farther".into(),
                    "RT + flick left stick: turn 90 degrees".into(),
                    "LT + RT + left stick: fine turn      L3: reset turn".into(),
                    "LB + A: place and keep a copy in hand".into(),
                ]
                .into_iter()
                .chain(dropper.held_character.map(|_| {
                    format!("R3: camera on the {}", if dropper.camera_on_placement { "character" } else { "placement" })
                }))
                .chain(dropper.snapped.map(|_| dropper.snapped_to.to_owned()))
                .chain(dropper.held.filter(|held| dropper.fine_tuned && snapping_piece(PROPS[held.prop].2).is_some()).map(|_| "Snapping off while fine-tuned (LT + RT); move it to snap again".to_owned()))
                .collect(),
                hints: vec![("A", "Place".into()), ("B", "Back".into()), ("Y", "Delete".into()), ("Start", "Prop settings".into())],
                crosshair: true,
                ..Menu::default()
            }
        } else {
            let target = dropper.target.and_then(|index| dropper.dropped.get(index));
            Menu {
                title: "Park Editor".into(),
                info: vec![
                    match target {
                        Some(target) => format!("Crosshair on: {}", PROPS[target.prop].1),
                        None => "Crosshair on: nothing".into(),
                    },
                    placed,
                    "Left stick: move      Right stick: look".into(),
                    "D-pad Up/Down: raise / lower".into(),
                ],
                hints: vec![("X", "Objects".into()), ("A", "Pick up".into()), ("Y", "Delete".into()), ("B", "Exit".into()), ("Start", "Settings".into())],
                crosshair: true,
                ..Menu::default()
            }
        };
        if !dropper.note.is_empty() {
            menu.info.push(dropper.note.into());
        }
        crate::ui::object_dropper::set_menu(Some(menu));
    }

    /// How far along the crosshair the first surface is, within reach.
    fn crosshair_hit(&mut self) -> Result<Option<f32>, SelectedLiveError> {
        let Some(view) = self.object_dropper.view else {
            return Ok(None);
        };
        let end = add(view.position, scale(view.at, AIM_REACH));
        Ok(self.first_hit(view.position, end, "ObjectDropper::Aim")?.map(|p| dot(add(p, scale(view.position, -1.0)), view.at)))
    }

    /// The world line test the editor uses (Skate 2 ground, DMOs and placed static pieces): the
    /// first surface from `start` to `end`.
    fn first_hit(&mut self, start: [f32; 3], end: [f32; 3], label: &'static str) -> Result<Option<[f32; 3]>, SelectedLiveError> {
        Ok(self.first_surface(start, end, label)?.map(|(position, _)| position))
    }

    /// `first_hit`, with the surface's normal.
    fn first_surface(&mut self, start: [f32; 3], end: [f32; 3], label: &'static str) -> Result<Option<([f32; 3], [f32; 3])>, SelectedLiveError> {
        Ok(self.first_surfaces(&[(start, end)], label)?.pop().flatten())
    }

    /// `first_surface` for each line, in one visit to the world.
    fn first_surfaces(&mut self, lines: &[([f32; 3], [f32; 3])], label: &'static str) -> Result<Vec<Option<([f32; 3], [f32; 3])>>, SelectedLiveError> {
        self.inner
            .try_with_selected_queued_frame(|frame, ground, _| -> Result<Vec<Option<([f32; 3], [f32; 3])>>, String> {
                use crate::skeleton_root_transforms::SkeletonRootVector;
                lines
                    .iter()
                    .map(|(start, end)| {
                        let hit = frame
                            .world
                            .try_world_line_test(
                                ground,
                                crate::collision_line_collection::CollisionLineInput {
                                    start: SkeletonRootVector::known([start[0], start[1], start[2], 1.0]),
                                    end: SkeletonRootVector::known([end[0], end[1], end[2], 1.0]),
                                    fatness: 0.0,
                                },
                                label,
                                &mut crate::collision_line_numeric::HostCollisionLineNumeric,
                            )
                            .map_err(|e| format!("{e:?}"))?;
                        Ok(hit.and_then(|hit| {
                            let p = hit.position.known_lanes()?;
                            let n = hit.normal.known_lanes()?;
                            Some(([p[0], p[1], p[2]], [n[0], n[1], n[2]]))
                        }))
                    })
                    .collect()
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)
    }

    /// The selected prop into the editor's hands, in front of the crosshair: on the first
    /// surface it meets, or `SPAWN_DISTANCE` along it.
    fn take_from_list(&mut self) -> Result<(), SelectedLiveError> {
        let Some(view) = self.object_dropper.view else {
            return Ok(());
        };
        if let Some(spot) = self.object_dropper.swap_spot.take() {
            return self.take_into_spot(spot);
        }
        let distance = self.crosshair_hit()?.map_or(SPAWN_DISTANCE, |hit| hit.clamp(0.0, SPAWN_DISTANCE));
        let mut at = add(view.position, scale(view.at, distance));
        let prop = self.object_dropper.selected;
        if is_board(PROPS[prop].2) {
            at[1] += BOARD_REST_DECK_HEIGHT;
        }
        let home = view.at[0].atan2(view.at[2]);
        self.hold(Held { prop, transform: crate::ui::object_dropper::prop_transform(at, home), home, picked: None })?;
        // A piece too big for the crosshair's spawn point (a mega ramp) is pushed on across the
        // ground until the camera is clear of it, and the orbit taken out to see all of it.
        let dropper = &mut self.object_dropper;
        if let (Some(held), Some(view)) = (dropper.held.as_mut(), dropper.view.as_mut()) {
            let reach = dropper.centers.get(&prop).map_or(0.0, |center| center.1);
            let clear = reach * SPAWN_CLEARANCE;
            let middle = middle(&dropper.centers, prop, &held.transform);
            let (flat, _) = ground_axes(view.at);
            let ahead = dot(add(middle, scale(view.position, -1.0)), flat);
            if ahead < clear {
                for lane in [0, 2] {
                    held.transform[12 + lane] += flat[lane] * (clear - ahead);
                }
            }
            let middle = self::middle(&dropper.centers, prop, &held.transform);
            let toward = add(middle, scale(view.position, -1.0));
            let distance = dot(toward, toward).sqrt();
            if distance > 1e-3 {
                view.at = look(normalize(toward), 0.0, 0.0);
            }
            dropper.orbit_distance = distance.clamp(ORBIT_MIN, orbit_max(&dropper.centers, prop));
            view.position = add(middle, scale(view.at, -dropper.orbit_distance));
        }
        dropper.list_open = false;
        dropper.note = "";
        Ok(())
    }

    /// The selected prop into the spot the one B put back was in: the same turn and base height,
    /// its middle over the last one's (so it sits where that one did on screen), and the camera
    /// looking the same way, backed off only as far as a bigger one needs to be seen whole.
    fn take_into_spot(&mut self, spot: Held) -> Result<(), SelectedLiveError> {
        let prop = self.object_dropper.selected;
        let look = self.object_dropper.view.map(|view| view.at);
        // A board's transform is its deck, the rest props' their base.
        let lift = |prop: usize| if is_board(PROPS[prop].2) { BOARD_REST_DECK_HEIGHT } else { 0.0 };
        let mut transform = spot.transform;
        transform[13] += lift(prop) - lift(spot.prop);
        self.hold(Held { prop, transform, home: spot.home, picked: None })?;
        let dropper = &mut self.object_dropper;
        if let (Some(held), Some(view), Some(look)) = (dropper.held.as_mut(), dropper.view.as_mut(), look) {
            // `hold` looked up its middle.
            let (was, now) = (middle(&dropper.centers, spot.prop, &spot.transform), middle(&dropper.centers, prop, &held.transform));
            held.transform[12] += was[0] - now[0];
            held.transform[14] += was[2] - now[2];
            view.at = look;
            let reach = dropper.centers.get(&prop).map_or(0.0, |center| center.1);
            dropper.orbit_distance = dropper.orbit_distance.max(reach * SPAWN_CLEARANCE).clamp(ORBIT_MIN, orbit_max(&dropper.centers, prop));
            view.position = add(middle(&dropper.centers, prop, &held.transform), scale(view.at, -dropper.orbit_distance));
        }
        dropper.list_open = false;
        dropper.note = "";
        Ok(())
    }

    /// `held` into the editor's hands, the camera turned onto it from where it is.
    fn hold(&mut self, held: Held) -> Result<(), SelectedLiveError> {
        self.object_dropper.held = Some(held);
        if let Some(entry) = marquee_entry(PROPS[held.prop].2)
            && self.object_dropper.held_character.is_none()
        {
            self.object_dropper.held_character = self.spawn_character(entry, &held.transform)?;
        }
        self.refresh_dropper_preview_model()?;
        let dropper = &mut self.object_dropper;
        let Some(view) = dropper.view.as_mut() else {
            return Ok(());
        };
        let middle = middle(&dropper.centers, held.prop, &held.transform);
        let toward = [middle[0] - view.position[0], middle[1] - view.position[1], middle[2] - view.position[2]];
        let distance = dot(toward, toward).sqrt();
        if distance > 1e-3 {
            view.at = look(normalize(toward), 0.0, 0.0);
        }
        dropper.orbit_distance = distance.clamp(ORBIT_MIN, orbit_max(&dropper.centers, held.prop));
        view.position = add(middle, scale(view.at, -dropper.orbit_distance));
        dropper.held = Some(held);
        dropper.fine_tuned = false;
        dropper.target = None;
        dropper.flick_ready = false;
        dropper.spin = None;
        Ok(())
    }

    /// Takes the placed prop `index` out of the world and into the editor's hands.
    fn pick_up(&mut self, index: usize) -> Result<(), SelectedLiveError> {
        let mut dropped = self.object_dropper.dropped.remove(index);
        self.object_dropper.target = None;
        // A character goes into the editor's hands as it is, already loaded.
        self.object_dropper.held_character = dropped.character.take();
        if let Some(placed) = self.object_dropper.held_character.and_then(|key| self.object_dropper.characters.get_mut(&key)) {
            placed.shape = None;
        }
        self.remove_dropped(&dropped)?;
        let home = heading(&dropped.transform);
        self.hold(Held { prop: dropped.prop, transform: dropped.transform, home, picked: Some(dropped.transform) })?;
        Ok(())
    }

    /// Lets go of the held prop where it is: it joins the world and falls. Kept in hand if the
    /// world takes no more.
    fn place_held(&mut self) -> Result<&'static str, SelectedLiveError> {
        let Some(held) = self.object_dropper.held else {
            return Ok("");
        };
        // The oldest of the held prop's sort makes room: static pieces have their own budget.
        let is_static = |dropped: &Dropped| dropped.mission.is_some();
        let held_static = static_kind(PROPS[held.prop].2).is_some();
        let (count, cap) = (self.object_dropper.dropped.iter().filter(|dropped| is_static(dropped) == held_static).count(), if held_static { MAX_STATIC } else { MAX_DROPPED });
        if count >= cap
            && let Some(at) = self.object_dropper.dropped.iter().position(|dropped| is_static(dropped) == held_static)
        {
            let oldest = self.object_dropper.dropped.remove(at);
            self.remove_dropped(&oldest)?;
        }
        let note = if is_board(PROPS[held.prop].2) {
            self.drop_board(held.prop, held.transform)?
        } else {
            self.drop_prop(held.prop, held.transform)?
        };
        if note.starts_with("placed") {
            self.object_dropper.held = None;
        }
        Ok(note)
    }

    fn drop_prop(&mut self, prop: usize, transform: [f32; 16]) -> Result<&'static str, SelectedLiveError> {
        let guid = PROPS[prop].2;
        if let Some(kind) = static_kind(guid) {
            return self.place_mission_prop(prop, kind, transform);
        }
        if let Some(entry) = marquee_entry(guid) {
            return self.place_character(prop, entry, transform);
        }
        let settings = self.object_dropper.prop_settings.get(&prop).copied().unwrap_or_default();
        let locator = LOCATOR_BASE + self.object_dropper.next_locator;
        self.object_dropper.next_locator += 1;
        let added = self
            .inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let (Some(living), Some(recipes)) = (frame.living_world, frame.living_world_recipes.as_ref()) else {
                    return Ok(false);
                };
                living
                    .try_drop_on_locator_as(frame.world, frame.actor, frame.cvars, recipes, locator, guid, &transform, settings.fixed)
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)?;
        Ok(if added {
            self.object_dropper.dropped.push(Dropped { locator, board: None, prop, transform, key: 0, deck: None, marker: None, mission: None, character: None });
            if settings.marker {
                self.add_placed_to_marker(locator)?;
            }
            if settings.fixed { "placed (static)" } else { "placed" }
        } else {
            "could not place it there"
        })
    }

    /// Where every placed prop is now (the world moves them), and which one the crosshair is on.
    fn refresh_dropper_target(&mut self) -> Result<(), SelectedLiveError> {
        let Some(view) = self.object_dropper.view else {
            return Ok(());
        };
        let locators: Vec<u64> = self.object_dropper.dropped.iter().map(|dropped| dropped.locator).collect();
        let now = {
            let owner = self.inner.world.as_ref().ok_or_else(|| error("world absent"))?;
            let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
            let mut now = crate::selected_living_world_dmo::SelectedLivingWorldDmo::dropped_locator_transforms(&world, &locators)
                .map_err(error)?;
            for (dropped, now) in self.object_dropper.dropped.iter().zip(now.iter_mut()) {
                if let Some(board) = &dropped.board {
                    *now = world.riderless_board_part_transforms(board).ok().map(|parts| parts[6]);
                }
            }
            now
        };
        let dropper = &mut self.object_dropper;
        for (dropped, now) in dropper.dropped.iter_mut().zip(&now) {
            if let Some(transform) = now {
                dropped.transform = *transform;
            }
        }
        dropper.target = dropper
            .dropped
            .iter()
            .enumerate()
            .filter_map(|(index, dropped)| {
                let middle = middle(&dropper.centers, dropped.prop, &dropped.transform);
                let toward = [middle[0] - view.position[0], middle[1] - view.position[1], middle[2] - view.position[2]];
                let along = dot(toward, view.at);
                let off = (dot(toward, toward) - along * along).max(0.0).sqrt();
                let reach = target_reach(&dropper.centers, dropped.prop) + TARGET_SPREAD * along;
                (along > 0.0 && along < AIM_REACH && off <= reach).then_some((index, off / reach))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index);
        Ok(())
    }

    /// A copy of the player's board: a real riderless board with its deck at `transform`.
    fn drop_board(&mut self, prop: usize, transform: [f32; 16]) -> Result<&'static str, SelectedLiveError> {
        let boards: Vec<usize> = (0..self.object_dropper.dropped.len())
            .filter(|&index| self.object_dropper.dropped[index].board.is_some())
            .collect();
        if boards.len() >= MAX_BOARDS {
            let oldest = self.object_dropper.dropped.remove(boards[0]);
            self.remove_dropped(&oldest)?;
        }
        let board = self
            .inner
            .try_with_selected_queued_frame(|frame, _, _| -> Result<_, String> {
                let vault = std::sync::Arc::clone(
                    &frame.actor.cached_physical_allocation().map_err(|e| format!("{e:?}"))?.owner().inputs().vault,
                );
                Ok(frame.world.try_spawn_riderless_board(&vault, transform).ok())
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)?;
        Ok(match board {
            Some(board) => {
                let key = self.object_dropper.next_board_key & 0x00ff_ffff;
                self.object_dropper.next_board_key = self.object_dropper.next_board_key.wrapping_add(1);
                self.object_dropper.dropped.push(Dropped {
                    locator: 0,
                    board: Some(board),
                    prop,
                    transform,
                    key,
                    deck: None,
                    marker: None,
                    mission: None,
                    character: None,
                });
                "placed your board"
            }
            None => "no room for another board (AI or party skaters use the slots)",
        })
    }

    /// The prop just placed on `locator` joins the session marker, as the prop menu's add does.
    fn add_placed_to_marker(&mut self, locator: u64) -> Result<(), SelectedLiveError> {
        let object = {
            let owner = self.inner.world.as_ref().ok_or_else(|| error("world absent"))?;
            let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
            crate::selected_living_world_dmo::SelectedLivingWorldDmo::dropped_locator_object(&world, locator).map_err(error)?
        };
        if let (Some(object), Some(marker)) = (object, self.inner.session_marker.as_mut()) {
            marker.prop_actions.push((object, 2));
        }
        Ok(())
    }

    /// A marquee character standing at `transform` (its origin and heading), on the character
    /// viewer's clip; its model streams in. The viewer's data loads with the first.
    fn spawn_character(&mut self, entry: usize, transform: &[f32; 16]) -> Result<Option<u64>, SelectedLiveError> {
        if self.object_dropper.viewer.is_none() {
            let db = self.freeskate_attrib_db()?;
            let mounts = std::sync::Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts);
            match super::freeskate_viewer::Viewer::load(&db, &mounts, super::freeskate_viewer::FREESKATE) {
                Ok(viewer) => self.object_dropper.viewer = Some(viewer),
                Err(e) => {
                    crate::diagnostics::test_log::event("park_editor_character", format_args!("viewer failed: {e}"));
                    return Ok(None);
                }
            }
        }
        let (schema, animated) = MARQUEE[entry];
        let prop = PROPS.iter().position(|row| marquee_entry(row.2) == Some(entry)).unwrap_or(0);
        let settings = self.object_dropper.prop_settings.get(&prop).copied().unwrap_or_default();
        let data = if animated { self.animation_data(settings.anim_file) } else { None };
        let viewer = self.object_dropper.viewer.as_ref().expect("viewer loaded");
        let animation = data.as_ref().map(|data| (data, database_clip(viewer, data, settings)));
        let (character, models) = match viewer.placed_character(schema, origin(transform), heading(transform), animation) {
            Ok((mut character, models)) => {
                character.in_place = true;
                character.set_viewer_matrix(*transform).map_err(error)?;
                (character, models)
            }
            Err(e) => {
                crate::diagnostics::test_log::event("park_editor_character", format_args!("{schema} failed: {e}"));
                return Ok(None);
            }
        };
        let key = self.object_dropper.next_character;
        self.object_dropper.next_character += 1;
        let body = MISSION_BODY_BASE + self.object_dropper.next_mission_body;
        self.object_dropper.next_mission_body += 1;
        self.object_dropper.characters.insert(key, PlacedCharacter { character, models, animated, time: 0.0, shape: None, body });
        Ok(Some(key))
    }

    /// The held character (or a new one) stays where it is let go.
    fn place_character(&mut self, prop: usize, entry: usize, transform: [f32; 16]) -> Result<&'static str, SelectedLiveError> {
        let key = match self.object_dropper.held_character.take() {
            Some(key) => Some(key),
            None => self.spawn_character(entry, &transform)?,
        };
        let Some(key) = key else {
            return Ok("could not load that character");
        };
        if let Some(placed) = self.object_dropper.characters.get_mut(&key) {
            placed.character.set_viewer_matrix(transform).map_err(error)?;
            // Its box follows once it is drawn here.
            placed.shape = None;
        }
        self.object_dropper.dropped.push(Dropped { locator: 0, board: None, prop, transform, key: 0, deck: None, marker: None, mission: None, character: Some(key) });
        Ok("placed")
    }

    /// Each tick: the held character follows the editor's hands, every character plays its
    /// clip (a `z_` model stays on its first frame), and models stream in.
    pub(super) fn update_placed_characters(&mut self, dt: f32) -> Result<(), SelectedLiveError> {
        let dropper = &mut self.object_dropper;
        if dropper.characters.is_empty() {
            return Ok(());
        }
        if let (Some(key), Some(held)) = (dropper.held_character, dropper.held)
            && let Some(placed) = dropper.characters.get_mut(&key)
        {
            placed.character.set_viewer_matrix(held.transform).map_err(error)?;
        }
        let assets = self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?;
        let held = self.object_dropper.held_character;
        let mut boxed = false;
        for (key, placed) in &mut self.object_dropper.characters {
            // A placed model standing frozen (a `z_` model, its pose and box made) has nothing
            // left to pose: skip it rather than rebuild its palette every tick.
            if !placed.animated && Some(*key) != held && placed.shape.is_some() {
                continue;
            }
            if placed.animated {
                // Every clip loops (a NIS scene's plays once): back to its start at its end.
                placed.time += dt;
                if placed.time >= placed.character.anim.length() {
                    placed.time = 0.0;
                    placed.character.set_time(0.0).map_err(error)?;
                } else {
                    placed.character.add_time(dt).map_err(error)?;
                }
            }
            placed.models.present_streamed(std::slice::from_mut(&mut placed.character), &assets.mounts, &assets.vault, &assets.cvars);
            // Placed and drawn: solid over its drawn bounds.
            // Placed and drawn: solid over its posed mesh (its drawn bounds' box if the mesh
            // cannot be posed here).
            if placed.shape.is_none()
                && Some(*key) != held
                && let Some(visual) = placed.models.visuals().first()
            {
                let shape = posed_triangles(visual.palette)
                    .map(|triangles| triangles_shape(&triangles))
                    .filter(|shape| !shape.triangles.is_empty())
                    .unwrap_or_else(|| {
                        let [minimum, maximum] = visual.world_bounds;
                        box_shape(minimum, maximum)
                    });
                placed.shape = Some(std::sync::Arc::new(shape));
                boxed = true;
            }
        }
        if boxed {
            self.publish_mission_placements();
        }
        Ok(())
    }

    /// Every placed and held character's renderable.
    pub(super) fn placed_character_visuals(&self) -> Vec<super::SelectedSceneCharacterVisual<'_>> {
        self.object_dropper.characters.values().flat_map(|placed| placed.models.visuals()).collect()
    }

    /// A static piece (a mission prop, or a Skate 3 park piece: `static_kind`) at `transform`:
    /// its collision (cut from its hull, or read from the park archive, the first time) goes to
    /// Skate 3 physics as an immovable body. Nothing spawns; it stays exactly where it is put.
    fn place_mission_prop(&mut self, prop: usize, kind: usize, transform: [f32; 16]) -> Result<&'static str, SelectedLiveError> {
        if !self.object_dropper.mission_shapes.contains_key(&kind) {
            let mounts = std::sync::Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts);
            let loaded = match kind.checked_sub(MISSION_PROPS.len()) {
                None => super::mission_props::load_mission_prop_shape(&mounts, &MISSION_PROPS[kind]),
                Some(piece) => super::park_assets::load_park_piece_shape(&mounts, &PARK_PIECES[piece]),
            };
            match loaded {
                Ok(shape) => {
                    self.object_dropper.mission_shapes.insert(kind, std::sync::Arc::new(shape));
                }
                Err(e) => {
                    crate::diagnostics::test_log::event("park_editor_mission_prop", format_args!("{} failed: {e}", PROPS[prop].1));
                    return Ok(if kind < MISSION_PROPS.len() { "could not load that mission prop" } else { "could not load that piece (is parkassets.big in data\\content?)" });
                }
            }
        }
        let id = MISSION_BODY_BASE + self.object_dropper.next_mission_body;
        self.object_dropper.next_mission_body += 1;
        self.object_dropper.dropped.push(Dropped { locator: 0, board: None, prop, transform, key: 0, deck: None, marker: None, mission: Some(id), character: None });
        self.publish_mission_placements();
        Ok("placed")
    }

    /// Hands every placed mission prop to Skate 3 physics.
    fn publish_mission_placements(&self) {
        let dropper = &self.object_dropper;
        let placements = dropper
            .dropped
            .iter()
            .filter_map(|dropped| {
                let kind = static_kind(PROPS[dropped.prop].2)?;
                Some(MissionPlacement { id: dropped.mission?, shape: std::sync::Arc::clone(dropper.mission_shapes.get(&kind)?), transform: dropped.transform })
            })
            .chain(dropper.characters.iter().filter(|(key, _)| Some(**key) != dropper.held_character).filter_map(|(_, placed)| {
                Some(MissionPlacement { id: placed.body, shape: std::sync::Arc::clone(placed.shape.as_ref()?), transform: IDENTITY })
            }))
            .collect::<Vec<_>>();
        crate::ui::object_dropper::publish_mission_placements(placements);
    }

    /// Where the held Skate 3 piece snaps this frame: to the placed ones (`snap_transform`, its
    /// and their snap faces read the first time each is needed), or else down or up onto the
    /// map surface under it (`ground_snap`).
    fn refresh_held_snap(&mut self) -> Result<(), SelectedLiveError> {
        self.object_dropper.snapped = None;
        let Some(held) = self.object_dropper.held else {
            return Ok(());
        };
        if self.object_dropper.fine_tuned {
            return Ok(());
        }
        let Some(piece) = snapping_piece(PROPS[held.prop].2) else {
            return Ok(());
        };
        let pieces: Vec<(usize, [f32; 16])> = self
            .object_dropper
            .dropped
            .iter()
            .filter_map(|dropped| Some((snapping_piece(PROPS[dropped.prop].2)?, dropped.transform)))
            .collect();
        if !pieces.is_empty() {
            for wanted in std::iter::once(piece).chain(pieces.iter().map(|(piece, _)| *piece)) {
                if self.object_dropper.snap_faces.contains_key(&wanted) {
                    continue;
                }
                let mounts = std::sync::Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts);
                // A piece whose faces cannot be read snaps to nothing.
                let faces = super::park_assets::load_park_piece_snap_faces(&mounts, &PARK_PIECES[wanted]).unwrap_or_default();
                self.object_dropper.snap_faces.insert(wanted, std::sync::Arc::new(faces));
            }
            let dropper = &mut self.object_dropper;
            let placed: Vec<([f32; 16], &[super::park_assets::SnapFace])> =
                pieces.iter().filter_map(|(piece, transform)| Some((*transform, dropper.snap_faces.get(piece)?.as_slice()))).collect();
            if let Some(snapped) = dropper.snap_faces.get(&piece).and_then(|faces| snap_transform(&held.transform, faces, &placed)) {
                dropper.snapped = Some(snapped);
                dropper.snapped_to = "Snapped to a placed piece";
                return Ok(());
            }
        }
        let key = (held.transform.map(f32::to_bits), piece, crate::ui::object_dropper::mission_generation());
        if let Some((transform, cached, generation, snapped, to)) = self.object_dropper.ground_snapped
            && (transform, cached, generation) == key
        {
            self.object_dropper.snapped = snapped;
            self.object_dropper.snapped_to = to;
            return Ok(());
        }
        let mut snapped = held.transform;
        let grounded = self.ground_snap(&snapped, &PARK_PIECES[piece])?.inspect(|rise| snapped[13] += rise).is_some();
        let walled = self.wall_snap(&mut snapped, &PARK_PIECES[piece])?;
        let to = match (grounded, walled) {
            (true, true) => "Snapped to the ground and a wall",
            (true, false) => "Snapped to the ground",
            (false, true) => "Snapped to a wall",
            (false, false) => "",
        };
        let snapped = (!to.is_empty()).then_some(snapped);
        self.object_dropper.ground_snapped = Some((key.0, key.1, key.2, snapped, to));
        self.object_dropper.snapped = snapped;
        self.object_dropper.snapped_to = to;
        Ok(())
    }

    /// An upright piece beside a wall (a surface within 25 degrees of facing one of its sides
    /// square on) squares its turn to the wall when within `SNAP_SQUARE`, then slides so that
    /// side is flush with it, when the wall is within `SNAP_REACH` of the side (or that far
    /// into it). Lines out of each side, at two heights and three places across, find the walls;
    /// the side nearest one snaps. False: no wall in reach.
    fn wall_snap(&mut self, transform: &mut [f32; 16], piece: &ParkPiece) -> Result<bool, SelectedLiveError> {
        if transform[5] < 0.999 {
            return Ok(false);
        }
        let middle: [f32; 3] = std::array::from_fn(|axis| (piece.min[axis] + piece.max[axis]) * 0.5);
        let mut nearest: Option<(f32, [f32; 3], [f32; 3])> = None;
        for (axis, sign) in [(0, 1.0), (0, -1.0), (2, 1.0), (2, -1.0)] {
            let Some((gap, normal)) = self.wall_gap(transform, piece, axis, sign)? else {
                continue;
            };
            if nearest.is_none_or(|(best, _, _)| gap.abs() < best.abs()) {
                let mut out = [0.0; 3];
                out[axis] = sign;
                nearest = Some((gap, out, normal));
            }
        }
        let Some((_, out, normal)) = nearest else {
            return Ok(false);
        };
        // Square to the wall: the side's way out turned onto the wall's (horizontal) normal.
        let side = direction(transform, out);
        let wall = normalize([normal[0], 0.0, normal[2]]);
        let wall = if dot(wall, side) < 0.0 { scale(wall, -1.0) } else { wall };
        let off = (side[2] * wall[0] - side[0] * wall[2]).atan2(dot(side, wall));
        if off.abs() < SNAP_SQUARE {
            let mut turned = *transform;
            turn_about(&mut turned, UP, off, middle);
            let mut back = *transform;
            turn_about(&mut back, UP, -off, middle);
            // Whichever way lines the side up with the wall.
            *transform = if dot(direction(&turned, out), wall) >= dot(direction(&back, out), wall) { turned } else { back };
        }
        let (axis, sign) = if out[0] != 0.0 { (0, out[0]) } else { (2, out[2]) };
        let Some((gap, _)) = self.wall_gap(transform, piece, axis, sign)? else {
            return Ok(true);
        };
        let side = direction(transform, out);
        for lane in 0..3 {
            transform[12 + lane] += side[lane] * gap;
        }
        Ok(true)
    }

    /// How far the nearest wall is out of one side of a piece (model `axis`, `sign` the way
    /// out; negative: that far into it), and its normal, if one is within `SNAP_REACH`.
    fn wall_gap(&mut self, transform: &[f32; 16], piece: &ParkPiece, axis: usize, sign: f32) -> Result<Option<(f32, [f32; 3])>, SelectedLiveError> {
        let across = 2 - axis;
        let face = if sign > 0.0 { piece.max[axis] } else { piece.min[axis] };
        let mut out = [0.0; 3];
        out[axis] = sign;
        let way = direction(transform, out);
        let mut nearest: Option<(f32, [f32; 3])> = None;
        for height in [0.4, 0.8] {
            for spot in [0.1, 0.5, 0.9] {
                let mut local = [0.0; 3];
                local[axis] = face;
                local[1] = piece.min[1] + (piece.max[1] - piece.min[1]) * height;
                local[across] = piece.min[across] + (piece.max[across] - piece.min[across]) * spot;
                let at = place(transform, local);
                let Some((hit, normal)) = self.first_surface(add(at, scale(way, -SNAP_REACH)), add(at, scale(way, SNAP_REACH)), "ObjectDropper::Wall")? else {
                    continue;
                };
                if dot(normal, way).abs() < WALL_FACING {
                    continue;
                }
                let gap = dot(add(hit, scale(at, -1.0)), way);
                if nearest.is_none_or(|(best, _)| gap < best) {
                    nearest = Some((gap, normal));
                }
            }
        }
        Ok(nearest)
    }

    /// How far to raise (or lower) a held piece at `transform` so its base rests on the map
    /// surface under it: vertical lines through points of its footprint find the surface within
    /// `SNAP_REACH` above or below each, and the piece goes down to the level most of them find,
    /// or to the lowest within `GROUND_DIP` below it (its edge meets or sinks into the ground
    /// there, with no lip; higher ground under it passes into it). None: no surface in reach (held
    /// higher, or over a drop).
    fn ground_snap(&mut self, transform: &[f32; 16], piece: &ParkPiece) -> Result<Option<f32>, SelectedLiveError> {
        // Skate 3 builds a piece on its model's y = 0 plane (a ramp's toe is there; a ledge runs
        // on below it): that plane goes on the ground. It is probed every `GROUND_EDGE_STEP`
        // round its footprint's outline and every `GROUND_GRID_STEP` inside, so no stretch of
        // its edge is left over a dip as a lip.
        let [low, high] = [piece.min, piece.max];
        let steps = |from: f32, to: f32, step: f32| -> Vec<f32> {
            let count = (((to - from) / step).ceil() as usize).clamp(1, 64);
            (0..=count).map(|i| from + (to - from) * i as f32 / count as f32).collect()
        };
        let (edge_x, edge_z) = (steps(low[0], high[0], GROUND_EDGE_STEP), steps(low[2], high[2], GROUND_EDGE_STEP));
        let (grid_x, grid_z) = (steps(low[0], high[0], GROUND_GRID_STEP), steps(low[2], high[2], GROUND_GRID_STEP));
        let outline = edge_x.iter().flat_map(|&x| [[x, low[2]], [x, high[2]]]).chain(edge_z.iter().flat_map(|&z| [[low[0], z], [high[0], z]]));
        let inside = grid_x.iter().flat_map(|&x| grid_z.iter().map(move |&z| [x, z]));
        let probes: Vec<[f32; 3]> = outline.chain(inside).map(|[x, z]| place(transform, [x, 0.0, z])).collect();
        let heights = self.surface_heights(&probes, SNAP_REACH)?;
        let mut rises: Vec<f32> = probes.iter().zip(heights).filter_map(|(probe, height)| Some(height? - probe[1])).collect();
        if rises.is_empty() {
            return Ok(None);
        }
        // The level most of it stands on, down to its lowest dip within `GROUND_DIP` of that (so
        // no edge floats over camber or a rough patch); a step down further than that is another
        // level the piece overhangs.
        rises.sort_by(f32::total_cmp);
        let level = rises[rises.len() / 2];
        Ok(rises.into_iter().find(|rise| *rise >= level - GROUND_DIP))
    }

    /// The first surface's height down a vertical line through each point, from `reach` above it
    /// to `reach` below, if any.
    fn surface_heights(&mut self, points: &[[f32; 3]], reach: f32) -> Result<Vec<Option<f32>>, SelectedLiveError> {
        let lines: Vec<([f32; 3], [f32; 3])> = points.iter().map(|p| ([p[0], p[1] + reach, p[2]], [p[0], p[1] - reach, p[2]])).collect();
        Ok(self.first_surfaces(&lines, "ObjectDropper::Ground")?.into_iter().map(|hit| hit.map(|(position, _)| position[1])).collect())
    }

    /// Every placed Skate 3 park piece as the DMO renderer draws it (its catalog ID is its
    /// asset there).
    pub(super) fn park_piece_visuals(&self) -> Vec<crate::SelectedDynamicObjectVisual> {
        self.object_dropper
            .dropped
            .iter()
            .filter_map(|dropped| {
                let piece = park_piece(PROPS[dropped.prop].2)?;
                // Entity IDs from the body's count, in a range living-world entities do not use.
                let entity = PARK_PIECE_ENTITY | (dropped.mission? as u32 & 0x0fff_ffff);
                Some(park_piece_visual(piece, entity, dropped.transform))
            })
            .collect()
    }

    /// Every placed mission prop (its `MISSION_PROPS` index and transform), for drawing.
    pub(super) fn mission_prop_poses(&self) -> Vec<(usize, [f32; 16])> {
        self.object_dropper
            .dropped
            .iter()
            .filter(|dropped| dropped.mission.is_some())
            .filter_map(|dropped| Some((mission_prop(PROPS[dropped.prop].2)?, dropped.transform)))
            .collect()
    }

    /// Takes one dropped thing back: a board leaves the skater pool, a prop its locator.
    fn remove_dropped(&mut self, dropped: &Dropped) -> Result<(), SelectedLiveError> {
        if let Some(key) = dropped.character {
            self.object_dropper.characters.remove(&key);
            self.publish_mission_placements();
            return Ok(());
        }
        if dropped.character.is_none() && dropped.board.is_none() && dropped.mission.is_none() && dropped.locator == 0 {
            // A picked-up character: its body left with its box.
            self.publish_mission_placements();
            return Ok(());
        }
        if dropped.mission.is_some() {
            self.publish_mission_placements();
            return Ok(());
        }
        if let Some(board) = &dropped.board {
            return self
                .inner
                .try_with_selected_queued_frame(|frame, _, _| frame.world.try_remove_riderless_board(board).map_err(|e| format!("{e:?}")))
                .map_err(|e| error(format!("{e:?}")))?
                .map_err(error);
        }
        self.remove_dropped_objects(&[dropped.locator])
    }

    fn remove_dropped_objects(&mut self, locators: &[u64]) -> Result<(), SelectedLiveError> {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let (Some(living), Some(recipes)) = (frame.living_world, frame.living_world_recipes.as_ref()) else {
                    return Ok(());
                };
                living
                    .try_remove_dropped_locators(frame.world, frame.actor, frame.cvars, recipes, locators)
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)
    }

    /// The world through the editor's camera, with the held prop drawn where it is.
    ///
    /// # Errors
    ///
    /// Returns [`SelectedLiveError`] when the editor is not open or the world cannot be drawn.
    pub fn try_object_dropper_visual_snapshot(&self) -> Result<SelectedVisualSnapshot<'_>, SelectedLiveError> {
        let view = self.object_dropper.view.ok_or_else(|| error("not open"))?;
        let right = normalize(cross(UP, view.at));
        let up = normalize(cross(view.at, right));
        let right = normalize(cross(up, view.at));
        let (p, at) = (view.position, view.at);
        let matrix = [
            right[0], right[1], right[2], 0.0,
            up[0], up[1], up[2], 0.0,
            at[0], at[1], at[2], 0.0,
            p[0], p[1], p[2], 1.0,
        ];
        let mut visual = self.visual_snapshot(
            Some(NisCamera { matrix: Some(matrix), fov: FIELD_OF_VIEW, near_plane: NEAR_PLANE }),
            Some(FrozenFocus::At(p)),
        )?;
        let Some(held) = self.object_dropper.held else {
            return Ok(visual);
        };
        if is_board(PROPS[held.prop].2) {
            visual.dropped_boards.push(held.transform);
        } else if let Some(kind) = mission_prop(PROPS[held.prop].2) {
            visual.held_mission_prop = Some((kind, held.transform));
        } else if let Some(piece) = park_piece(PROPS[held.prop].2) {
            visual.dynamic_objects.push(park_piece_visual(piece, u32::MAX, self.object_dropper.snapped.unwrap_or(held.transform)));
        } else if let Some((model_prop, asset_id, render_model_entry)) = self.object_dropper.preview_model
            && model_prop == held.prop
        {
            // A spawned prop's draw settings; with none out (an online game spawns nothing until
            // a prop is placed), fully visible with no fade.
            let template = visual.dynamic_objects.first().copied().unwrap_or(crate::SelectedDynamicObjectVisual {
                entity_id: u32::MAX,
                asset_id,
                render_model_entry,
                part_transform: held.transform,
                opacity_parameters: Default::default(),
                opacity_settings: crate::LivingWorldOpacitySettings { enabled: false, fade_in_frames: 0.0 },
                simulation_center_velocity: [0.0; 4],
            });
            visual.dynamic_objects.push(crate::SelectedDynamicObjectVisual {
                entity_id: u32::MAX,
                asset_id,
                render_model_entry,
                part_transform: held.transform,
                ..template
            });
        }
        Ok(visual)
    }

    /// Looks up the held prop's model when it changes.
    fn refresh_dropper_preview_model(&mut self) -> Result<(), SelectedLiveError> {
        let Some(held) = self.object_dropper.held else {
            return Ok(());
        };
        if self.object_dropper.preview_model.is_some_and(|(cached, _, _)| cached == held.prop) || is_board(PROPS[held.prop].2) {
            return Ok(());
        }
        // A mission prop turns about its box's middle (its origin is the box's base).
        if let Some(kind) = mission_prop(PROPS[held.prop].2) {
            let prop = &MISSION_PROPS[kind];
            let size: [f32; 3] = std::array::from_fn(|axis| prop.max[axis] - prop.min[axis]);
            let reach = 0.5 * dot(size, size).sqrt();
            self.object_dropper.centers.insert(held.prop, ([0.0, size[1] * 0.5, 0.0], reach));
            return Ok(());
        }
        // A park piece turns about its bounds' middle (its origin is where Skate 3 places it).
        if let Some(piece) = park_piece(PROPS[held.prop].2) {
            let piece = &PARK_PIECES[piece];
            let size: [f32; 3] = std::array::from_fn(|axis| piece.max[axis] - piece.min[axis]);
            let middle = std::array::from_fn(|axis| (piece.min[axis] + piece.max[axis]) * 0.5);
            self.object_dropper.centers.insert(held.prop, (middle, 0.5 * dot(size, size).sqrt()));
            return Ok(());
        }
        let owner = self.inner.world.as_ref().ok_or_else(|| error("world absent"))?;
        let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
        let model = crate::selected_living_world_dmo::SelectedLivingWorldDmo::dropped_preview_model(&world, PROPS[held.prop].2)
            .map_err(error)?;
        drop(world);
        if let Some((_, _, center, reach)) = model {
            self.object_dropper.centers.insert(held.prop, (center, reach));
        }
        self.object_dropper.preview_model = model.map(|(asset_id, entry, _, _)| (held.prop, asset_id, entry));
        Ok(())
    }
}

/// Draw entity IDs of placed park pieces: this type byte plus a count.
const PARK_PIECE_ENTITY: u32 = 0xB1 << 24;

/// A park piece at `transform`, fully visible with no fade, as a DMO draw: its catalog ID is
/// its asset in the renderer's bank, its RModel entry the catalog's.
fn park_piece_visual(piece: usize, entity_id: u32, transform: [f32; 16]) -> crate::SelectedDynamicObjectVisual {
    crate::SelectedDynamicObjectVisual {
        entity_id,
        asset_id: PARK_PIECES[piece].id,
        render_model_entry: PARK_PIECES[piece].model,
        part_transform: transform,
        opacity_parameters: Default::default(),
        opacity_settings: crate::LivingWorldOpacitySettings { enabled: false, fade_in_frames: 0.0 },
        simulation_center_velocity: [0.0; 4],
    }
}

/// The panel while leaving waits for the buttons to be let go.
fn leaving_menu() -> crate::ui::object_dropper::Menu {
    crate::ui::object_dropper::Menu {
        title: "Park Editor".into(),
        info: vec!["Let go of the buttons to skate.".into()],
        ..Default::default()
    }
}

/// Every subcategory: its first catalog entry and length, in order.
fn categories() -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut index = 0;
    while index < PROPS.len() {
        let run = category_run(index);
        runs.push(run);
        index = run.0 + run.1;
    }
    runs
}

/// A subcategory's object list rows (from its first entry and length): each row's first entry.
fn category_rows(start: usize, len: usize) -> Vec<usize> {
    let mut rows = Vec::new();
    let mut index = start;
    while index < start + len {
        rows.push(index);
        index += theme_row(index).1;
    }
    rows
}

/// The subcategory holding catalog entry `index`: its first entry and length.
fn category_run(index: usize) -> (usize, usize) {
    let category = PROPS[index].0;
    let start = PROPS[..index].iter().rposition(|entry| entry.0 != category).map_or(0, |last| last + 1);
    let len = PROPS[start..].iter().take_while(|entry| entry.0 == category).count();
    (start, len)
}

// ---- Dropped boards in the session marker ---------------------------------------------------
// Skate 2 adds a grabbable prop to the session marker through the prop menu an on-foot skater
// gets beside it (`PropHudSnapshot`); using the marker puts every member back where it was
// added. A dropped board is no living-world object, so it gets that same menu under an object
// ID of its own and keeps its membership here.

/// Whether a prop-menu object ID names a dropped board rather than a living-world object.
pub(super) fn is_board_prop(object: u32) -> bool {
    object & 0xff00_0000 == BOARD_PROP_ID
}

/// The deck turned upright about its own heading, lifted to fall back onto its wheels.
fn upright_deck(deck: [f32; 16]) -> [f32; 16] {
    // The deck's Z row is its length; keep it level, or use X when the board stands on end.
    let (mut x, mut z) = (deck[8], deck[10]);
    if x.hypot(z) < 1e-3 {
        (x, z) = (-deck[2], deck[0]);
    }
    let length = x.hypot(z).max(1e-6);
    let (zx, zz) = (x / length, z / length);
    [
        zz, 0.0, -zx, 0.0,
        0.0, 1.0, 0.0, 0.0,
        zx, 0.0, zz, 0.0,
        deck[12], deck[13] + BOARD_RIGHT_LIFT, deck[14], 1.0,
    ]
}

impl ObjectDropper {
    /// The prop menu for the dropped board nearest an on-foot skater at `skater`, if one is
    /// in reach. `marker_exists`: the session marker has been set (the menu's third value).
    pub(super) fn board_prop_hud(
        &self,
        skater: [f32; 3],
        marker_exists: bool,
    ) -> Option<crate::ui::phonelist::PropHudSnapshot> {
        // Like a living-world prop, only while RB (interact) is held, and not in the editor.
        if self.view.is_some() || self.buttons & RIGHT_BUMPER == 0 {
            return None;
        }
        self.dropped
            .iter()
            .filter(|dropped| dropped.board.is_some())
            .filter_map(|dropped| {
                let deck = dropped.deck?;
                let across = (deck[12] - skater[0]).hypot(deck[14] - skater[2]);
                (across <= BOARD_MENU_REACH && (deck[13] - skater[1]).abs() <= BOARD_MENU_HEIGHT)
                    .then_some((dropped, deck, across))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(dropped, deck, _)| crate::ui::phonelist::PropHudSnapshot {
                object: BOARD_PROP_ID | dropped.key,
                // Upright disabled, reset disabled, marker valid, in the marker, maximum reached.
                params: [deck[5] >= BOARD_UPRIGHT_UP_Y, false, marker_exists, dropped.marker.is_some(), false],
            })
    }

    /// `ClearDynamicObjectLocatorsInSessionMarker`'s counterpart: no board stays a member.
    pub(super) fn clear_board_markers(&mut self) {
        for dropped in &mut self.dropped {
            dropped.marker = None;
        }
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// The prop menu's actions on dropped boards (0 upright, 1 reset, 2 add to the marker,
    /// 3 remove), then, when the marker is being used (`restore`), every member board put back
    /// at rest where it was added. `has_marker`: a session marker exists to add to.
    pub(super) fn apply_board_prop_actions(
        &mut self,
        actions: &[(u32, u8)],
        restore: bool,
        has_marker: bool,
    ) -> Result<(), SelectedLiveError> {
        for &(object, action) in actions {
            let key = object & 0x00ff_ffff;
            let Some(index) = self
                .object_dropper
                .dropped
                .iter()
                .position(|dropped| dropped.board.is_some() && dropped.key == key)
            else {
                continue;
            };
            let board = self.object_dropper.dropped[index].board.clone().expect("filtered board");
            let deck = self.board_deck(&board)?;
            match action {
                0 if deck[5] < BOARD_UPRIGHT_UP_Y => self.place_board(&board, upright_deck(deck))?,
                1 => {
                    let start = self.object_dropper.dropped[index].transform;
                    self.place_board(&board, start)?;
                }
                2 if has_marker => {
                    self.object_dropper.dropped[index].marker =
                        Some(if deck[5] >= BOARD_UPRIGHT_UP_Y { deck } else { upright_deck(deck) });
                }
                3 => self.object_dropper.dropped[index].marker = None,
                _ => {}
            }
        }
        if restore {
            let members: Vec<_> = self
                .object_dropper
                .dropped
                .iter()
                .filter_map(|dropped| Some((dropped.board.clone()?, dropped.marker?)))
                .collect();
            for (board, saved) in members {
                self.place_board(&board, saved)?;
            }
        }
        Ok(())
    }

    fn board_deck(&mut self, board: &RiderlessBoardHandle) -> Result<[f32; 16], SelectedLiveError> {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                frame.world.riderless_board_part_transforms(board).map(|parts| parts[6]).map_err(|e| format!("{e:?}"))
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)
    }

    /// Teleports a dropped board (deck at `transform`) at rest.
    fn place_board(&mut self, board: &RiderlessBoardHandle, transform: [f32; 16]) -> Result<(), SelectedLiveError> {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                frame.world.try_place_riderless_board(board, transform).map_err(|e| format!("{e:?}"))
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)
    }
}

/// Test access for the facade corpus (which drives the whole live session).
#[cfg(test)]
impl<T, C> SelectedLiveSession<T, C> {
    /// Every animation file tried on Chris Cole: (files that load and play, clips in them,
    /// files that do not).
    pub(crate) fn test_animation_census(&mut self) -> (usize, usize, Vec<&'static str>) {
        let entry = MARQUEE.iter().position(|(name, _)| *name == "chris_cole").unwrap();
        let at = crate::ui::object_dropper::prop_transform([0.0; 3], 0.0);
        self.spawn_character(entry, &at).unwrap();
        let viewer = self.object_dropper.viewer.as_ref().unwrap();
        let _ = viewer;
        let (mut ok, mut clips, mut bad) = (0, 0, Vec::new());
        for file in 0..crate::ui::object_dropper::ANIMATION_FILES.len() {
            let file16 = u16::try_from(file).unwrap();
            let data = self.animation_data(file16);
            let viewer = self.object_dropper.viewer.as_ref().unwrap();
            let marquee = &viewer.data().database.hierarchy.bone_names;
            let played = data.as_ref().and_then(|data| {
                let name = &data.database.clips.first()?.name;
                crate::nis::characters::SceneCharacterAnim::viewer(std::sync::Arc::clone(data), name).ok()?;
                // The share of the marquee skeleton's bones this database's skeleton names.
                let own = &data.database.hierarchy.bone_names;
                let matched = marquee.iter().filter(|bone| own.iter().any(|b| b.eq_ignore_ascii_case(bone))).count();
                eprintln!("park editor animation COVERAGE {} {matched}/{} bones, {} clips", crate::ui::object_dropper::animation_file_name(file), marquee.len(), data.database.clips.len());
                Some(data.database.clips.len())
            });
            let _ = viewer;
            match played {
                Some(count) => {
                    ok += 1;
                    clips += count;
                }
                None => bad.push(crate::ui::object_dropper::animation_file_name(file)),
            }
            // Keep memory down: the census drops each file after trying it.
            if file != 0 {
                self.object_dropper.animations.remove(&file16);
            }
        }
        (ok, clips, bad)
    }
    /// The character viewer's clips (loading it), for the test log.
    pub(crate) fn test_character_clips(&mut self) -> Vec<String> {
        let db = self.freeskate_attrib_db().unwrap();
        let mounts = std::sync::Arc::clone(&self.inner.assets.as_ref().unwrap().mounts);
        super::freeskate_viewer::Viewer::load(&db, &mounts, super::freeskate_viewer::FREESKATE).unwrap().clip_names()
    }
    /// Every drawn character's first palette rows (its posed bones), to see a clip play.
    pub(crate) fn test_character_pose(&self) -> Vec<Vec<f32>> {
        self.placed_character_visuals()
            .iter()
            .map(|visual| visual.palette.mesh_rows(0, 0).map(|rows| rows.rows.iter().flatten().copied().collect()).unwrap_or_default())
            .collect()
    }
    /// Every character's hips as drawn now (empty until its model has streamed in).
    pub(crate) fn test_character_hips(&self) -> Vec<[f32; 3]> {
        self.placed_character_visuals().iter().map(|visual| visual.hips_position).collect()
    }
    /// Sets the catalog entry named `name`'s animation to the first file whose name holds
    /// `file` (its first clip).
    pub(crate) fn test_set_animation(&mut self, name: &str, file: &str) -> &'static str {
        let prop = PROPS.iter().position(|entry| entry.1 == name).expect("catalog entry");
        let index = (0..crate::ui::object_dropper::ANIMATION_FILES.len())
            .find(|&i| crate::ui::object_dropper::animation_file_name(i).to_ascii_lowercase().contains(&file.to_ascii_lowercase()))
            .expect("animation file");
        let settings = self.object_dropper.prop_settings.entry(prop).or_default();
        settings.anim_file = u16::try_from(index).unwrap();
        settings.clip = 0;
        crate::ui::object_dropper::animation_file_name(index)
    }
    /// Takes the catalog entry named `name` from the list as A on it would (the editor open).
    pub(crate) fn test_take(&mut self, name: &str) {
        self.object_dropper.selected = PROPS.iter().position(|entry| entry.1 == name).expect("catalog entry");
        self.take_from_list().unwrap();
    }
    /// Lets go of whatever the editor holds (not placed).
    pub(crate) fn test_let_go(&mut self) {
        self.object_dropper.held = None;
        self.object_dropper.snapped = None;
    }
    /// Takes every placed prop away, as Y would one by one.
    pub(crate) fn test_remove_all(&mut self) {
        let dropped = std::mem::take(&mut self.object_dropper.dropped);
        for dropped in &dropped {
            self.remove_dropped(dropped).unwrap();
        }
    }
    /// Skate 2 physics' grind query over a box: each primitive's world start and end.
    pub(crate) fn test_skate2_grind_query(&mut self, minimum: [f32; 3], maximum: [f32; 3]) -> Vec<([f32; 4], [f32; 4])> {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let mut result = crate::grind_data_runtime::GrindDataQueryResult::new();
                let bounds = crate::grind_data_octree::OctreeBounds { minimum: [minimum[0], minimum[1], minimum[2], 0.0], maximum: [maximum[0], maximum[1], maximum[2], 0.0] };
                frame.world.grind_services.try_box_test(bounds, &mut result, false, frame.cvars).unwrap();
                result.results().map(|primitive| (primitive.start, primitive.end)).collect()
            })
            .unwrap()
    }
    /// Skate 3 physics' grind world: every primitive's world start and end (None: no Skate 3
    /// player).
    pub(crate) fn test_skate3_grind_segments(&mut self) -> Option<Vec<([f32; 4], [f32; 4])>> {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let physical = frame.actor.cached_physical_allocation().ok()?;
                let player = physical.owner().skate3.lock().unwrap();
                player.as_ref().map(|player| player.test_grind_segments())
            })
            .unwrap()
    }
    /// The skater's physics state (Skate 3's numbering: 4xx grinding) and board deck position.
    pub(crate) fn test_physics_state(&mut self) -> (i32, [f32; 3]) {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let physical = frame.actor.cached_physical_allocation().unwrap();
                let output = physical.owner().phys_out().lock().unwrap();
                (output.try_state_output().unwrap().physics_state, output.try_camera_board_output().unwrap().deck_position.xyz)
            })
            .unwrap()
    }
    /// How many placed characters the editor keeps.
    pub(crate) fn test_placed_characters(&self) -> usize {
        self.object_dropper.characters.len()
    }
    /// How many animation files the editor keeps loaded.
    pub(crate) fn test_animation_files_loaded(&self) -> usize {
        self.object_dropper.animations.values().filter(|data| data.is_some()).count()
    }
    /// Sets the catalog entry named `name`'s prop settings (not saved).
    pub(crate) fn test_set_prop_settings(&mut self, name: &str, fixed: bool, marker: bool) {
        let prop = PROPS.iter().position(|entry| entry.1 == name).expect("catalog entry");
        self.object_dropper.prop_settings.insert(prop, PropSettings { fixed, marker, anim_file: 0, clip: 0 });
    }
    /// Takes the catalog entry named `name` into the editor's hands at `transform` (the editor
    /// open), and where it snaps there.
    pub(crate) fn test_hold_snapped(&mut self, name: &str, transform: [f32; 16]) -> Option<[f32; 16]> {
        let prop = PROPS.iter().position(|entry| entry.1 == name).expect("catalog entry");
        self.hold(Held { prop, transform, home: heading(&transform), picked: None }).unwrap();
        if let Some(held) = self.object_dropper.held.as_mut() {
            held.transform = transform;
        }
        self.refresh_held_snap().unwrap();
        self.object_dropper.snapped
    }
    /// Places the catalog entry named `name` at `transform` as the editor's A would.
    pub(crate) fn test_drop_prop(&mut self, name: &str, transform: [f32; 16]) -> &'static str {
        let prop = PROPS.iter().position(|entry| entry.1 == name).expect("catalog entry");
        self.drop_prop(prop, transform).unwrap()
    }
    /// The editor's world line test from `start` to `end`: the first surface and its normal.
    pub(crate) fn test_first_surface(&mut self, start: [f32; 3], end: [f32; 3]) -> Option<([f32; 3], [f32; 3])> {
        self.first_surface(start, end, "ObjectDropper::Test").unwrap()
    }
    /// The world line test the editor's crosshair uses (Skate 2 ground and DMOs): the hit's
    /// height along a vertical line at `x`, `z` from `top` down to `bottom`.
    pub(crate) fn test_line_down(&mut self, x: f32, z: f32, top: f32, bottom: f32) -> Option<f32> {
        self.inner
            .try_with_selected_queued_frame(|frame, ground, _| -> Result<Option<f32>, String> {
                use crate::skeleton_root_transforms::SkeletonRootVector;
                let hit = frame
                    .world
                    .try_world_line_test(
                        ground,
                        crate::collision_line_collection::CollisionLineInput {
                            start: SkeletonRootVector::known([x, top, z, 1.0]),
                            end: SkeletonRootVector::known([x, bottom, z, 1.0]),
                            fatness: 0.0,
                        },
                        "ObjectDropper::TestDown",
                        &mut crate::collision_line_numeric::HostCollisionLineNumeric,
                    )
                    .map_err(|e| format!("{e:?}"))?;
                Ok(hit.and_then(|hit| hit.position.known_lanes()).map(|p| p[1]))
            })
            .unwrap()
            .unwrap()
    }
    /// The living world reset an online game's entry runs (`try_reset_game` as a network game).
    pub(crate) fn test_network_game_reset(&mut self) {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let (Some(living), Some(recipes)) = (frame.living_world, frame.living_world_recipes.as_ref()) else {
                    return Err("no living world".to_string());
                };
                living.try_reset_game(frame.world, frame.actor, frame.cvars, recipes, true).map_err(|e| e.to_string())
            })
            .unwrap()
            .unwrap();
    }
    pub(crate) fn test_respawn_online_props(&mut self) {
        self.refresh_dropped_props().unwrap();
    }
    /// Whether each placed prop has its object in the world.
    pub(crate) fn test_dropped_objects_out(&mut self) -> Vec<bool> {
        let locators: Vec<u64> = self.object_dropper.dropped.iter().map(|dropped| dropped.locator).collect();
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                crate::selected_living_world_dmo::SelectedLivingWorldDmo::dropped_locator_objects_out(frame.world, &locators).map_err(|e| e.to_string())
            })
            .unwrap()
            .unwrap()
    }
    pub(crate) fn test_drop_board(&mut self, transform: [f32; 16]) -> bool {
        let board = PROPS.iter().position(|entry| entry.2 == YOUR_BOARD).expect("Your board entry");
        self.drop_board(board, transform).expect("drop board") == "placed your board"
    }
    pub(crate) fn test_drop_grind_board(&mut self, transform: [f32; 16]) -> bool {
        let board = PROPS.iter().position(|entry| entry.2 == YOUR_GRIND_BOARD).expect("Your board (grind) entry");
        self.drop_board(board, transform).expect("drop board") == "placed your board"
    }
    /// The local skater's root (world XYZ, then its forward Z axis XYZ).
    pub(crate) fn test_skater_root(&mut self) -> ([f32; 3], [f32; 3]) {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let physical = frame.actor.cached_physical_allocation().unwrap();
                let output = physical.owner().phys_out().lock().unwrap();
                let root = &output.try_skeleton_output().unwrap().skeleton_root_matrix;
                (root.columns()[3].xyz, root.columns()[2].xyz)
            })
            .unwrap()
    }
    /// Whether each dropped board is asleep (all its Parts frozen).
    pub(crate) fn test_boards_frozen(&self) -> Vec<bool> {
        let owner = self.inner.world.as_ref().expect("world");
        let world = owner.world.lock().unwrap();
        self.object_dropper.dropped.iter().filter_map(|dropped| dropped.board.as_ref()).map(|board| world.riderless_board_frozen(board).unwrap_or(false)).collect()
    }
    /// Each dropped board's (prop-menu object ID, live deck, marker transform).
    pub(crate) fn test_dropped_boards(&self) -> Vec<(u32, Option<[f32; 16]>, Option<[f32; 16]>)> {
        self.object_dropper
            .dropped
            .iter()
            .filter(|dropped| dropped.board.is_some())
            .map(|dropped| (BOARD_PROP_ID | dropped.key, dropped.deck, dropped.marker))
            .collect()
    }
    pub(crate) fn test_place_dropped_board(&mut self, object: u32, transform: [f32; 16]) {
        let board = self
            .object_dropper
            .dropped
            .iter()
            .find(|dropped| dropped.board.is_some() && BOARD_PROP_ID | dropped.key == object)
            .and_then(|dropped| dropped.board.clone())
            .expect("dropped board");
        self.place_board(&board, transform).unwrap();
    }
    /// The prop menu the HUD shows now, if any.
    pub(crate) fn test_prop_menu(&self) -> Option<crate::ui::phonelist::PropHudSnapshot> {
        self.inner.session_marker.as_ref().and_then(|marker| marker.hud.prop)
    }
    /// Queues a prop-menu action as the pad would (0 upright, 1 reset, 2 add, 3 remove).
    pub(crate) fn test_queue_prop_action(&mut self, object: u32, action: u8) {
        self.inner.session_marker.as_mut().expect("session marker").prop_actions.push((object, action));
    }
    /// Uses the marker's prop restore, as a marker teleport request does.
    pub(crate) fn test_restore_marker_props(&mut self) {
        self.inner.session_marker.as_mut().expect("session marker").restore_props = true;
    }
    /// The park editor as the test pad sees it: (open, list open, items shown, held prop name and
    /// transform, every placed prop's live transform, the crosshair's target, camera position).
    pub(crate) fn test_editor(&mut self) -> TestEditor {
        self.refresh_dropper_target().unwrap();
        let dropper = &self.object_dropper;
        TestEditor {
            open: dropper.view.is_some(),
            list_open: dropper.list_open,
            list_items: dropper.list_items,
            held: dropper.held.map(|held| (PROPS[held.prop].1, held.transform)),
            held_middle: dropper.held.map(|held| (local_middle(&dropper.centers, held.prop), middle(&dropper.centers, held.prop, &held.transform))),
            placed: dropper.dropped.iter().map(|dropped| dropped.transform).collect(),
            target: dropper.target,
            camera: dropper.view.map(|view| view.position),
            look: dropper.view.map(|view| view.at),
            settings_open: dropper.settings_open,
            prop_settings_open: dropper.prop_settings_open,
            settings: dropper.settings.percent.to_vec(),
            note: dropper.note,
            snapped: dropper.snapped,
        }
    }
    pub(crate) fn test_has_session_marker(&self) -> bool {
        self.inner.session_marker.as_ref().is_some_and(|marker| marker.marker.is_some())
    }
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct TestEditor {
    pub open: bool,
    pub list_open: bool,
    pub list_items: bool,
    pub held: Option<(&'static str, [f32; 16])>,
    /// The held prop's middle in model space and in the world.
    pub held_middle: Option<([f32; 3], [f32; 3])>,
    pub placed: Vec<[f32; 16]>,
    pub target: Option<usize>,
    pub camera: Option<[f32; 3]>,
    /// Where the camera looks (unit).
    pub look: Option<[f32; 3]>,
    pub settings_open: bool,
    pub prop_settings_open: bool,
    pub settings: Vec<u16>,
    /// The panel's last note ("placed", "could not ...").
    pub note: &'static str,
    /// Where the held piece snaps this frame.
    pub snapped: Option<[f32; 16]>,
}

#[cfg(test)]
mod snap_tests {
    use super::super::park_assets::SnapFace;
    use super::*;

    /// A 6 x 3 x 6 m box's faces as Skate 3's street pad lists them (x 0..6, y 0..3, z -6..0).
    fn pad_faces() -> Vec<SnapFace> {
        let corners = |points: [[f32; 3]; 4]| points.to_vec();
        vec![
            SnapFace { normal: [0.0, -1.0, 0.0], corners: corners([[0.0, 0.0, 0.0], [0.0, 0.0, -6.0], [6.0, 0.0, -6.0], [6.0, 0.0, 0.0]]) },
            SnapFace { normal: [0.0, 0.0, 1.0], corners: corners([[0.0, 3.0, 0.0], [0.0, 0.0, 0.0], [6.0, 3.0, 0.0], [6.0, 0.0, 0.0]]) },
            SnapFace { normal: [1.0, 0.0, 0.0], corners: corners([[6.0, 0.0, -6.0], [6.0, 3.0, -6.0], [6.0, 3.0, 0.0], [6.0, 0.0, 0.0]]) },
            SnapFace { normal: [0.0, 0.0, -1.0], corners: corners([[0.0, 3.0, -6.0], [0.0, 0.0, -6.0], [6.0, 0.0, -6.0], [6.0, 3.0, -6.0]]) },
            SnapFace { normal: [-1.0, 0.0, 0.0], corners: corners([[0.0, 0.0, 0.0], [0.0, 0.0, -6.0], [0.0, 3.0, -6.0], [0.0, 3.0, 0.0]]) },
            SnapFace { normal: [0.0, 1.0, 0.0], corners: corners([[0.0, 3.0, 0.0], [0.0, 3.0, -6.0], [6.0, 3.0, -6.0], [6.0, 3.0, 0.0]]) },
        ]
    }

    fn at(position: [f32; 3], yaw_degrees: f32) -> [f32; 16] {
        crate::ui::object_dropper::prop_transform(position, yaw_degrees.to_radians())
    }

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|lane| (a[lane] - b[lane]).abs() < 1e-3)
    }

    #[test]
    fn pieces_snap_side_by_side_and_stacked() {
        let faces = pad_faces();
        let placed = [(at([10.0, 2.0, 20.0], 0.0), faces.as_slice())];
        // Beside it, a little off: it slides flush, edge to edge.
        let side = snap_transform(&at([16.4, 2.1, 20.3], 0.0), &faces, &placed).expect("no side snap");
        assert!(close(origin(&side), [16.0, 2.0, 20.0]), "side snap at {:?}", origin(&side));
        // Above it: it sits on its top.
        let top = snap_transform(&at([10.3, 5.4, 19.8], 0.0), &faces, &placed).expect("no stack snap");
        assert!(close(origin(&top), [10.0, 5.0, 20.0]), "stack snap at {:?}", origin(&top));
        // Out of reach: nothing.
        assert!(snap_transform(&at([19.0, 2.0, 20.0], 0.0), &faces, &placed).is_none());
        // Nearly square (5 degrees off): squared, then flush.
        let turned = snap_transform(&at([16.3, 2.0, 20.2], 5.0), &faces, &placed).expect("no squared snap");
        assert!(heading(&turned).abs() < 1e-3, "heading {}", heading(&turned).to_degrees());
        assert!(close(origin(&turned), [16.0, 2.0, 20.0]), "squared snap at {:?}", origin(&turned));
        // A quarter turn round is square too: faces still meet.
        assert!(snap_transform(&at([16.2, 2.0, 26.1], 90.0), &faces, &placed).is_some());
    }
}
