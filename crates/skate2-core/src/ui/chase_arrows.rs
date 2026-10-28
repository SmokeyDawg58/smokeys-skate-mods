//! `FE::ChaseArrowMan`, `FrontEndManager::mChaseArrowMan`: an arrow for each pedestrian whose
//! chase of the local skater a `cMsgChaseState` reports (warn, chase, tired, give up), drawn over
//! the pedestrian while it is on screen, else at the title-safe frame's edge toward it. `Update`
//! (0x82631328) counts and places the arrows in game; `Render` (0x82631638) draws them in screen
//! space before the APT levels. The tuning is the `FE_Icons` cvars' defaults.

/// `eFEChaserState`, the message's state.
pub const WARN: i32 = 0;
pub const CHASE: i32 = 1;
pub const TIRED: i32 = 2;
pub const GIVE_UP: i32 = 3;
pub const END: i32 = 4;

/// `GfxMan`'s front buffer, which the arrows are placed and drawn on.
pub const FRONT_BUFFER: [f32; 2] = [1280.0, 720.0];
/// `mSimArrows`' fixed capacity.
const CAPACITY: usize = 10;
/// `kFEArrowTimeWarn` and `kFEArrowTimeGiveUp`: how long a warn and a give-up arrow show, in
/// milliseconds.
const WARN_TIME: u32 = 6000;
const GIVE_UP_TIME: u32 = 3000;
/// `kFEChaseArrowSecurityGuardOffset`: the arrow's anchor above the pedestrian.
const SECURITY_GUARD_OFFSET: f32 = 0.72;
/// `kFEChaseArrowSkaterOffsetY`: the on-screen arrow's top above its anchor.
const SKATER_OFFSET_Y: f32 = 1.15;
/// `kFEChaseArrowMinScale`, `kFEChaseArrowMinScaleDistance` and `kFEChaseArrowMaxScaleDistance`.
const MIN_SCALE: f32 = 0.4;
const MIN_SCALE_DISTANCE: f32 = 1.0;
const MAX_SCALE_DISTANCE: f32 = 50.0;
/// `kFEChaseArrowRadiusOffsetX` and `kFEChaseArrowRadiusOffsetY`: the off-screen arrows' ellipse
/// past the title-safe frame.
const RADIUS_OFFSET: [f32; 2] = [20.0, 20.0];
/// `mTitleSafeFactor`, which the constructor (0x8262f238) sets.
const TITLE_SAFE_FACTOR: f32 = 0.85;
/// `HALF_TEXTURE_WIDTH` and `HALF_TEXTURE_HEIGHT`.
const HALF_TEXTURE: [f32; 2] = [32.0, 32.0];
/// `RenderOnScreenArrow`'s arrow height at scale 1.
const ON_SCREEN_HEIGHT: f32 = 64.0;
/// `STANDARD_UV_7`, the corners' texture coordinates.
const UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
/// The arrow textures `GetArrowAsset` (0x82618aa0) finds by state in the pres asset registry.
const TEXTURES: [&str; 4] = ["arrow_warn.Texture", "arrow_chase.Texture", "arrow_tired.Texture", "arrow_giveup.Texture"];

/// A `ChaseArrow`: the pedestrian, its state and how long the state has shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arrow {
    /// `mPedId`.
    pub pedestrian: u32,
    /// `mState`, an `eFEChaserState`.
    pub state: i32,
    /// `mTimeDisplayed`, in milliseconds.
    time_displayed: u32,
}

/// `mSimArrows`, in the order the arrows were added.
#[derive(Clone, Debug, Default)]
pub struct ChaseArrowMan {
    arrows: Vec<Arrow>,
}

impl ChaseArrowMan {
    #[must_use]
    pub fn arrows(&self) -> &[Arrow] {
        &self.arrows
    }

    /// `AddArrow` (0x826347e0), from `FrontEndManager::ReceiveChaseStateMsg` (0x82634dc4): a new
    /// pedestrian gets an arrow unless the state is `End` or ten show already; a known one's
    /// arrow goes at `End`, else takes a changed state with its time restarted.
    pub fn add_arrow(&mut self, pedestrian: u32, state: i32) {
        let Some(index) = self.arrows.iter().position(|arrow| arrow.pedestrian == pedestrian) else {
            if state != END && self.arrows.len() < CAPACITY {
                self.arrows.push(Arrow { pedestrian, state, time_displayed: 0 });
            }
            return;
        };
        if self.arrows[index].state == state {
            return;
        }
        if state == END {
            self.arrows.remove(index);
        } else {
            self.arrows[index] = Arrow { pedestrian, state, time_displayed: 0 };
        }
    }

    /// `Update`'s count, which `FrontEndManager::Update` (0x82640130) runs in game with its
    /// `dT / 100`: an arrow goes once its pedestrian has no pres pedestrian
    /// (`cLivingWorldPresEntityManager::GetPresPedestrian`), and a warn arrow at six seconds, a
    /// give-up arrow at three.
    pub fn update(&mut self, elapsed: u32, mut presented: impl FnMut(u32) -> bool) {
        self.arrows.retain_mut(|arrow| {
            if !presented(arrow.pedestrian) {
                return false;
            }
            let time = arrow.time_displayed.wrapping_add(elapsed);
            let limit = match arrow.state {
                WARN => Some(WARN_TIME),
                GIVE_UP => Some(GIVE_UP_TIME),
                _ => None,
            };
            if limit.is_some_and(|limit| time >= limit) {
                return false;
            }
            arrow.time_displayed = time;
            true
        });
    }

    /// `RemoveAllArrows` (0x8261cfb8), which `FrontEndManager::GameUnload` (0x82628c40) calls.
    pub fn remove_all_arrows(&mut self) {
        self.arrows.clear();
    }
}

/// Where `Update` places an arrow, in front-buffer pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// `mOnScreen`: the arrow's point lies in front of the camera and inside the title-safe frame.
    pub on_screen: bool,
    /// The anchor less the local skater's hips.
    pub displacement: [f32; 3],
    /// `GetScale` of the displacement's length.
    pub scale: f32,
    /// `mScreenX` and `mScreenY`.
    pub screen: [f32; 2],
}

/// `Update`'s placement of an arrow: its anchor `kFEChaseArrowSecurityGuardOffset` above the pres
/// pedestrian's local-to-world translation, measured from the local skater's hips
/// (`SkaterPresentation::GetHipsPos` of `GetLocalIndex`) for its scale, and the point
/// `kFEChaseArrowSkaterOffsetY` above the anchor on screen.
#[must_use]
pub fn placement(pedestrian: [f32; 3], hips: [f32; 3], clip_from_world: &[f32; 16]) -> Placement {
    let anchor = [pedestrian[0], pedestrian[1] + SECURITY_GUARD_OFFSET, pedestrian[2]];
    let displacement: [f32; 3] = std::array::from_fn(|axis| anchor[axis] - hips[axis]);
    let distance = displacement.iter().map(|lane| lane * lane).sum::<f32>().sqrt();
    let (depth, screen) = screen_position(clip_from_world, [anchor[0], anchor[1] + SKATER_OFFSET_Y, anchor[2]]);
    Placement { on_screen: depth > 0.0 && in_title_safe(screen), displacement, scale: scale(distance), screen }
}

/// `PostFX::CalculateScreenPos` (0x82828310) for `CAMERAIDX_GAMEVIEWPORT1`: the point through the
/// camera's view-projection, its viewport position on the front buffer, and its clip z, positive
/// in front of the near plane. The fisheye warp it works out is overwritten by the plain position
/// before it returns.
fn screen_position(clip_from_world: &[f32; 16], point: [f32; 3]) -> (f32, [f32; 2]) {
    let clip: [f32; 4] = std::array::from_fn(|row| {
        clip_from_world[12 + row]
            + clip_from_world[8 + row].mul_add(point[2], clip_from_world[4 + row].mul_add(point[1], clip_from_world[row] * point[0]))
    });
    let shifted = clip[0] / clip[3] + 1.0;
    let across = shifted * 0.5;
    let down = (-(clip[1] / clip[3] + 1.0)).mul_add(0.5, 1.0);
    (clip[2], [FRONT_BUFFER[0] * across, FRONT_BUFFER[1] * down])
}

/// `IsInTitleSafe` (0x82618b68): inside the middle `mTitleSafeFactor` of the front buffer.
fn in_title_safe(screen: [f32; 2]) -> bool {
    let middle = FRONT_BUFFER.map(|size| size * 0.5);
    let half = FRONT_BUFFER.map(|size| TITLE_SAFE_FACTOR * size * 0.5);
    if screen[0] < middle[0] - half[0] || screen[0] > half[0] + middle[0] || screen[1] < middle[1] - half[1] {
        return false;
    }
    screen[1] <= half[1] + middle[1]
}

/// `GetScale` (0x82607440): full size within `kFEChaseArrowMinScaleDistance`, shrinking evenly to
/// `kFEChaseArrowMinScale` at `kFEChaseArrowMaxScaleDistance` and beyond.
fn scale(distance: f32) -> f32 {
    if distance < MIN_SCALE_DISTANCE || distance.is_nan() {
        return 1.0;
    }
    if distance >= MAX_SCALE_DISTANCE {
        return MIN_SCALE;
    }
    ((MAX_SCALE_DISTANCE - distance) / (MAX_SCALE_DISTANCE - MIN_SCALE_DISTANCE)).mul_add(1.0 - MIN_SCALE, MIN_SCALE)
}

/// One arrow's quad: its texture and four corners (x, y, u, v), in front-buffer pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub texture: &'static str,
    pub corners: [[f32; 4]; 4],
    /// The colour's alpha; the colour is white.
    pub alpha: f32,
}

/// `Render`'s quad for an arrow, drawn as `QuadListTextured` with alpha blending and no depth:
/// none for a state past give-up, which has no texture. An arrow on screen hangs from its point;
/// one off screen sits on the ellipse past the title-safe frame toward the pedestrian, found from
/// the displacement and the camera's view direction (`CameraPresentation::GetMatrix`'s row 2).
#[must_use]
pub fn quad(state: i32, placement: &Placement, view_direction: [f32; 3]) -> Option<Quad> {
    let texture = *usize::try_from(state).ok().and_then(|index| TEXTURES.get(index))?;
    let points = if placement.on_screen {
        on_screen_corners(placement.screen, placement.scale)
    } else {
        off_screen_corners(off_screen_angle(placement.displacement, view_direction), placement.scale)
    };
    let corners = std::array::from_fn(|index| [points[index][0], points[index][1], UV[index][0], UV[index][1]]);
    Some(Quad { texture, corners, alpha: if placement.scale == 0.0 { 0.0 } else { 1.0 } })
}

/// `Render`'s turn for an arrow off screen: the displacement and view direction flattened to the
/// ground and normalised, the angle between them, negative where their cross product points up
/// (view direction first), less a quarter turn.
fn off_screen_angle(displacement: [f32; 3], view_direction: [f32; 3]) -> f32 {
    let normalised = |vector: [f32; 3]| {
        let inverse = 1.0 / vector[0].mul_add(vector[0], vector[2] * vector[2]).sqrt();
        [vector[0] * inverse, 0.0, vector[2] * inverse]
    };
    let towards = normalised(displacement);
    let forward = normalised(view_direction);
    let cosine = forward[0] * towards[0] + forward[1] * towards[1] + forward[2] * towards[2];
    let cross_y = (-forward[0]).mul_add(towards[2], forward[2] * towards[0]);
    let sign = if cross_y <= 0.0 { 1.0 } else { -1.0 };
    cosine.clamp(-1.0, 1.0).acos().mul_add(sign, -std::f32::consts::FRAC_PI_2)
}

/// `RenderOnScreenArrow` (0x8262f740): the arrow's half width either side of the point and its
/// height below, turned half a turn about the point.
fn on_screen_corners(screen: [f32; 2], scale: f32) -> [[f32; 2]; 4] {
    let half_width = HALF_TEXTURE[0] * scale;
    let height = scale * -ON_SCREEN_HEIGHT;
    [[half_width, 0.0], [-half_width, 0.0], [-half_width, height], [half_width, height]].map(|corner| {
        let turned = rotate_point(corner, std::f32::consts::PI);
        [turned[0] + screen[0], turned[1] + screen[1]]
    })
}

/// `RenderOffScreenArrow` (0x8262f2e8): the point at `angle` on the ellipse
/// `kFEChaseArrowRadiusOffsetX`/`Y` past the title-safe frame, kept half a texture inside the
/// frame, and the arrow about it turned a quarter turn back from `angle`.
fn off_screen_corners(angle: f32, scale: f32) -> [[f32; 2]; 4] {
    let [width, height] = FRONT_BUFFER;
    let safe = [TITLE_SAFE_FACTOR * width, TITLE_SAFE_FACTOR * height];
    let radius = [safe[0].mul_add(0.5, RADIUS_OFFSET[0]), safe[1].mul_add(0.5, RADIUS_OFFSET[1])];
    let middle = [width * 0.5, height * 0.5];
    let (sin, cos) = angle.sin_cos();
    let point = [cos.mul_add(radius[0], middle[0]), sin.mul_add(radius[1], middle[1])];
    let centre: [f32; 2] = std::array::from_fn(|axis| {
        let low = (-safe[axis]).mul_add(0.5, middle[axis]) + HALF_TEXTURE[axis];
        let high = safe[axis].mul_add(0.5, middle[axis]) - HALF_TEXTURE[axis];
        let raised = if low - point[axis] >= 0.0 { low } else { point[axis] };
        if high - raised >= 0.0 { raised } else { high }
    });
    let half = [HALF_TEXTURE[0] * scale, HALF_TEXTURE[1] * scale];
    let turn = angle - std::f32::consts::FRAC_PI_2;
    [[-half[0], -half[1]], [half[0], -half[1]], [half[0], half[1]], [-half[0], half[1]]].map(|corner| {
        let turned = rotate_point(corner, turn);
        [turned[0] + centre[0], turned[1] + centre[1]]
    })
}

/// `RotatePoint` (0x82618998).
fn rotate_point(point: [f32; 2], angle: f32) -> [f32; 2] {
    let (sin, cos) = angle.sin_cos();
    [point[0] * cos - point[1] * sin, point[1].mul_add(cos, point[0] * sin)]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A camera at the origin looking down +z, +x on its left, with a 90 degree, 16:9 projection.
    fn projection() -> [f32; 16] {
        let mut clip = [0.0; 16];
        clip[0] = -1.0 / (16.0 / 9.0);
        clip[5] = 1.0;
        clip[10] = 1.0;
        clip[14] = -0.1;
        clip[11] = 1.0;
        clip
    }

    #[test]
    fn a_state_change_restarts_the_arrow_and_the_end_removes_it() {
        let mut arrows = ChaseArrowMan::default();
        arrows.add_arrow(7, END);
        assert!(arrows.arrows().is_empty());
        arrows.add_arrow(7, WARN);
        for _ in 0..200 {
            arrows.update(16, |_| true);
        }
        arrows.add_arrow(7, WARN);
        assert_eq!(arrows.arrows()[0].time_displayed, 3200);
        arrows.add_arrow(7, CHASE);
        assert_eq!(arrows.arrows()[0], Arrow { pedestrian: 7, state: CHASE, time_displayed: 0 });
        arrows.add_arrow(7, END);
        assert!(arrows.arrows().is_empty());
    }

    #[test]
    fn ten_arrows_at_most_and_one_per_pedestrian() {
        let mut arrows = ChaseArrowMan::default();
        for pedestrian in 0..12 {
            arrows.add_arrow(pedestrian, CHASE);
            arrows.add_arrow(pedestrian, CHASE);
        }
        assert_eq!(arrows.arrows().len(), 10);
        assert_eq!(arrows.arrows()[9].pedestrian, 9);
    }

    #[test]
    fn a_warning_shows_six_seconds_a_give_up_three_and_a_chase_until_it_changes() {
        let mut arrows = ChaseArrowMan::default();
        arrows.add_arrow(1, WARN);
        arrows.add_arrow(2, GIVE_UP);
        arrows.add_arrow(3, CHASE);
        arrows.add_arrow(4, TIRED);
        let shown = |arrows: &ChaseArrowMan| arrows.arrows().iter().map(|arrow| arrow.pedestrian).collect::<Vec<_>>();
        for _ in 0..187 {
            arrows.update(16, |_| true);
        }
        assert_eq!(shown(&arrows), [1, 2, 3, 4]);
        arrows.update(16, |_| true);
        assert_eq!(shown(&arrows), [1, 3, 4]);
        for _ in 188..374 {
            arrows.update(16, |_| true);
        }
        assert_eq!(shown(&arrows), [1, 3, 4]);
        arrows.update(16, |_| true);
        assert_eq!(shown(&arrows), [3, 4]);
        arrows.update(16, |pedestrian| pedestrian != 3);
        assert_eq!(shown(&arrows), [4]);
        arrows.remove_all_arrows();
        assert!(arrows.arrows().is_empty());
    }

    #[test]
    fn arrows_shrink_from_one_metre_to_fifty() {
        assert!((scale(0.5) - 1.0).abs() < f32::EPSILON);
        assert!((scale(1.0) - 1.0).abs() < f32::EPSILON);
        assert!((scale(25.5) - 0.7).abs() < 1e-6);
        assert!((scale(50.0) - 0.4).abs() < f32::EPSILON);
        assert!((scale(f32::NAN) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_pedestrian_ahead_has_its_arrow_hanging_over_it() {
        let placed = placement([0.0, -1.87, 10.0], [0.0, 0.0, 0.0], &projection());
        assert!(placed.on_screen);
        assert!((placed.screen[0] - 640.0).abs() < 1e-3 && (placed.screen[1] - 360.0).abs() < 1e-3);
        let arrow = quad(CHASE, &placed, [0.0, 0.0, 1.0]).expect("chase texture");
        assert_eq!(arrow.texture, "arrow_chase.Texture");
        let size = placed.scale;
        let expected = [[640.0 - 32.0 * size, 360.0], [640.0 + 32.0 * size, 360.0],
            [640.0 + 32.0 * size, 360.0 + 64.0 * size], [640.0 - 32.0 * size, 360.0 + 64.0 * size]];
        for (corner, (point, uv)) in arrow.corners.iter().zip(expected.iter().zip(UV)) {
            assert!((corner[0] - point[0]).abs() < 1e-3 && (corner[1] - point[1]).abs() < 1e-3);
            assert_eq!([corner[2].to_bits(), corner[3].to_bits()], uv.map(f32::to_bits));
        }
        assert!(quad(END, &placed, [0.0, 0.0, 1.0]).is_none());
    }

    #[test]
    fn off_screen_arrows_point_from_the_title_safe_edge_toward_the_pedestrian() {
        let centre = |placed: &Placement| {
            let arrow = quad(WARN, placed, [0.0, 0.0, 1.0]).expect("warn texture");
            let sum = arrow.corners.iter().fold([0.0, 0.0], |sum, corner| [sum[0] + corner[0], sum[1] + corner[1]]);
            let tip = [f32::midpoint(arrow.corners[2][0], arrow.corners[3][0]), f32::midpoint(arrow.corners[2][1], arrow.corners[3][1])];
            ([sum[0] * 0.25, sum[1] * 0.25], tip)
        };
        let behind = placement([0.0, -0.72, -10.0], [0.0, 0.0, 0.0], &projection());
        assert!(!behind.on_screen);
        let (middle, tip) = centre(&behind);
        assert!((middle[0] - 640.0).abs() < 1e-2 && (middle[1] - (360.0 + 306.0 - 32.0)).abs() < 1e-2);
        assert!(tip[1] > middle[1]);
        let left = placement([30.0, -0.72, 0.5], [0.0, 0.0, 0.0], &projection());
        let (middle, tip) = centre(&left);
        assert!((middle[0] - (96.0 + 32.0)).abs() < 1.0 && tip[0] < middle[0]);
        let right = placement([-30.0, -0.72, 0.5], [0.0, 0.0, 0.0], &projection());
        let (middle, tip) = centre(&right);
        assert!((middle[0] - (1280.0 - 96.0 - 32.0)).abs() < 1.0 && tip[0] > middle[0]);
    }
}
