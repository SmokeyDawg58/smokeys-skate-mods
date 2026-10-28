//! AntiFrustumManager's occluder quads, debug sk82_na_m.xex.
//!
//! `Presentation::RegArena` 0x827f1430 hands every `pegasus::tAntiFrustumData`
//! of a registered presentation arena to `AntiFrustumManager::AddAntiFustums`
//! 0x828487c0, which keeps its quads in a KD tree keyed by the midpoint of
//! corners 0 and 2. For each view, `Frustum::UpdateCullParams` 0x82853970
//! selects up to 32 quads within `fRadius` (100) of the camera that no view
//! plane wholly rejects (`Select_Antifrustums` 0x828536c0, `Frustum::TestQuad`
//! 0x8282c098) and gives each five planes (`GetAntiFrustumPlanes` 0x82829fb8):
//! the quad's own plane and one through the camera and each edge, all facing
//! the camera. `DoFrustumCulling` 0x828589c0 culls an entity or island whose
//! sphere lies wholly behind the five planes of one of them. The debug
//! `kbVisCurtainsExpand` (default off) is not modelled. The manager walks its
//! assets in hash-map order; callers pass them in collection order, which
//! differs only once more than 32 quads qualify. Host square roots stand in for
//! Xenon reciprocal square root estimates.

use crate::{presentation_shadow_culling::normalize_plane, reckoning_numeric::HostReckoningNumeric};
use skate2_formats::render::AxisAlignedBox;

/// `pegasus::RegisterArenaTypes` 0x829c90a4 registers `tAntiFrustumData` as this type.
pub const ANTI_FRUSTUM_DATA_TYPE_ID: u32 = 0x00eb_0025;

/// `fRadius` 0x82ff0238, which nothing writes.
const SEARCH_RADIUS: f32 = 100.0;
/// `mNearestAFQuads` and `FrustumParams::mAntiFrustums` both hold 32.
const MAX_SELECTED: usize = 32;

/// One `tAntiFrustumData`: its quads, and their KD tree in insertion order.
#[derive(Clone, Debug, PartialEq)]
pub struct AntiFrustumSet {
    quads: Vec<[[f32; 4]; 4]>,
    nodes: Vec<KdNode>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct KdNode {
    coord: [f32; 3],
    left: Option<usize>,
    right: Option<usize>,
    parent: Option<usize>,
}

impl AntiFrustumSet {
    /// `tAntiFrustumData::Fixup` 0x829c7c50 rebases `m_pAntiFrustums` (+0x24) on
    /// the object; `m_uiNumAntiFrustums` (+0x20) counts its 64-byte quads.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let word = |offset: usize| {
            bytes
                .get(offset..offset + 4)
                .map(|word| u32::from_be_bytes([word[0], word[1], word[2], word[3]]))
                .ok_or_else(|| format!("anti-frustum data field +{offset:#x} truncated"))
        };
        let count = usize::try_from(word(0x20)?).map_err(|e| e.to_string())?;
        let start = usize::try_from(word(0x24)?).map_err(|e| e.to_string())?;
        let table = count
            .checked_mul(64)
            .and_then(|size| bytes.get(start..start.checked_add(size)?))
            .ok_or("anti-frustum quads outside their object")?;
        let quads = table
            .chunks_exact(64)
            .map(|quad| {
                std::array::from_fn(|corner| {
                    std::array::from_fn(|lane| {
                        let at = corner * 16 + lane * 4;
                        f32::from_be_bytes([quad[at], quad[at + 1], quad[at + 2], quad[at + 3]])
                    })
                })
            })
            .collect::<Vec<[[f32; 4]; 4]>>();
        let mut set = Self {
            nodes: Vec::with_capacity(quads.len()),
            quads,
        };
        for index in 0..set.quads.len() {
            set.insert(index);
        }
        Ok(set)
    }

    /// `KDTree::Insert` 0x82837998 and `KDNode::Insert` 0x8282d2d8: from the
    /// root, a larger coordinate on the node's axis goes right and any other
    /// left, the axis cycling x, y, z.
    fn insert(&mut self, index: usize) {
        let [first, _, third, _] = self.quads[index];
        // AddAntiFustums 828488f4..90c: (corner 0 + corner 2) * 0.5.
        let coord = std::array::from_fn(|lane| (first[lane] + third[lane]) * 0.5);
        self.nodes.push(KdNode {
            coord,
            left: None,
            right: None,
            parent: None,
        });
        if index == 0 {
            return;
        }
        let (mut node, mut axis) = (0, 0);
        loop {
            let right = coord[axis] > self.nodes[node].coord[axis];
            let child = if right {
                self.nodes[node].right
            } else {
                self.nodes[node].left
            };
            match child {
                Some(next) => {
                    node = next;
                    axis = (axis + 1) % 3;
                }
                None => {
                    if right {
                        self.nodes[node].right = Some(index);
                    } else {
                        self.nodes[node].left = Some(index);
                    }
                    self.nodes[index].parent = Some(node);
                    return;
                }
            }
        }
    }

    /// `KDNode::FindNear` 0x828519d8 then `KDTree::FindNear` 0x82853500: the
    /// first `limit` quads within `radius` of `point` in the tree walk, nearest
    /// first (ties keep walk order).
    fn find_near(&self, point: [f32; 3], radius: f32, limit: usize) -> Vec<usize> {
        let mut found = Vec::new();
        let mut node = (!self.nodes.is_empty()).then_some(0);
        let (mut previous, mut axis, mut returned) = (None, 0, false);
        while let Some(current) = node {
            if found.len() >= limit {
                break;
            }
            let entry = self.nodes[current];
            if !returned {
                let distance = length(sub(entry.coord, point));
                if distance <= radius {
                    found.push((distance, current));
                }
            }
            if let Some(left) = entry.left
                && !returned
                && entry.coord[axis] >= point[axis] - radius
            {
                (previous, node, axis, returned) = (Some(current), Some(left), (axis + 1) % 3, false);
                continue;
            }
            if let Some(right) = entry.right
                && (!returned || previous == entry.left)
                && point[axis] + radius > entry.coord[axis]
            {
                (previous, node, axis, returned) = (Some(current), Some(right), (axis + 1) % 3, false);
                continue;
            }
            (previous, node, axis, returned) = (Some(current), entry.parent, (axis + 2) % 3, true);
        }
        found.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        found.into_iter().map(|(_, index)| index).collect()
    }
}

/// The anti-frustums one view culls with: five planes each, a point and a
/// normal facing the camera.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AntiFrustums {
    planes: Vec<[([f32; 3], [f32; 3]); 5]>,
}

impl AntiFrustums {
    /// `Select_Antifrustums` 0x828536c0 for a camera at `camera` whose clip
    /// rows (the final row producing clip W) give the view planes, then
    /// `GetAntiFrustumPlanes` 0x82829fb8 for each quad selected.
    pub fn try_select<'a>(
        sets: impl IntoIterator<Item = &'a AntiFrustumSet>,
        camera: [f32; 3],
        clip_rows: [[f32; 4]; 4],
    ) -> Result<Self, String> {
        let view = view_planes(clip_rows)?;
        let mut planes = Vec::new();
        'sets: for set in sets {
            for index in set.find_near(camera, SEARCH_RADIUS, MAX_SELECTED) {
                if planes.len() >= MAX_SELECTED {
                    break 'sets;
                }
                let quad = &set.quads[index];
                let code = test_quad(&view, quad);
                // 828537e4..800: no plane rejects all four corners.
                if (code >> 24) & (code >> 16) & (code >> 8) & code == 0 {
                    planes.push(quad_planes(quad, camera));
                }
            }
        }
        Ok(Self { planes })
    }

    pub fn is_empty(&self) -> bool {
        self.planes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.planes.len()
    }

    /// `DoFrustumCulling`'s anti-frustum loop (82858ff4..82859228): the box's
    /// sphere, at its centre with the authored min.W radius, lies at least that
    /// radius behind every plane of one anti-frustum.
    pub fn occludes(&self, bounds: &AxisAlignedBox) -> bool {
        let centre: [f32; 3] =
            std::array::from_fn(|lane| (bounds.minimum[lane] + bounds.maximum[lane]) * 0.5);
        let reach = -bounds.minimum[3];
        self.planes.iter().any(|planes| {
            planes.iter().all(|(point, normal)| {
                let distance = (centre[0] - point[0]).mul_add(
                    normal[0],
                    (centre[1] - point[1]).mul_add(normal[1], (centre[2] - point[2]) * normal[2]),
                );
                reach >= distance
            })
        })
    }
}

/// `ComputeFrustum` 0x8282bd28's normalized planes: w+x, w-x, w-y, w+y, w+z, w-z.
fn view_planes(clip_rows: [[f32; 4]; 4]) -> Result<[[f32; 4]; 6], String> {
    let [x, y, z, w] = clip_rows;
    let add = |a: [f32; 4], b: [f32; 4]| std::array::from_fn(|i| a[i] + b[i]);
    let sub = |a: [f32; 4], b: [f32; 4]| std::array::from_fn(|i| a[i] - b[i]);
    let mut planes = [add(w, x), sub(w, x), sub(w, y), add(w, y), add(w, z), sub(w, z)];
    for plane in &mut planes {
        *plane = normalize_plane(*plane, &mut HostReckoningNumeric)
            .map_err(|error| format!("anti-frustum view plane: {error}"))?;
    }
    Ok(planes)
}

/// `Frustum::TestQuad` 0x8282c098: bit 8k+p is set when corner k lies outside
/// view plane p, the four-lane dot product plus the plane's W.
fn test_quad(planes: &[[f32; 4]; 6], quad: &[[f32; 4]; 4]) -> u32 {
    let mut code = 0;
    for (corner_index, corner) in quad.iter().enumerate() {
        for (plane_index, plane) in planes.iter().enumerate() {
            let dot = (plane[0] * corner[0] + plane[1] * corner[1])
                + (plane[2] * corner[2] + plane[3] * corner[3]);
            if dot + plane[3] < 0.0 {
                code |= 1 << (corner_index * 8 + plane_index);
            }
        }
    }
    code
}

/// `GetAntiFrustumPlanes` 0x82829fb8: through corner 0 the quad's plane, then
/// for each edge i to i+1 the plane through corner i containing the camera
/// ray, every normal negated when corner 3 lies on its positive side of the
/// quad's plane as seen from the camera (the `fsel` at 8282a170).
fn quad_planes(quad: &[[f32; 4]; 4], camera: [f32; 3]) -> [([f32; 3], [f32; 3]); 5] {
    let corners = quad.map(|corner| [corner[0], corner[1], corner[2]]);
    let near = normalize(cross(
        sub(corners[1], corners[0]),
        sub(corners[2], corners[1]),
    ));
    let sign = if dot(sub(corners[3], camera), near) >= 0.0 {
        -1.0
    } else {
        1.0
    };
    let side = |i: usize| {
        let normal = normalize(cross(
            sub(corners[i], camera),
            sub(corners[(i + 1) % 4], corners[i]),
        ));
        (corners[i], normal.map(|lane| lane * sign))
    };
    [
        side(0),
        side(1),
        side(2),
        side(3),
        (corners[0], near.map(|lane| lane * sign)),
    ]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[2].mul_add(b[2], a[0].mul_add(b[0], a[1] * b[1]))
}

fn length(v: [f32; 3]) -> f32 {
    dot(v, v).sqrt()
}

/// The source's `vpermwi` form of `b x a`: `b * a.yzx - b.yzx * a`, rotated.
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    let lanes = [
        (-b[1]).mul_add(a[0], b[0] * a[1]),
        (-b[2]).mul_add(a[1], b[1] * a[2]),
        (-b[0]).mul_add(a[2], b[2] * a[0]),
    ];
    [lanes[1], lanes[2], lanes[0]]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let inverse = 1.0 / length(v);
    v.map(|lane| lane * inverse)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit quad facing -Z at depth `z`, wound counter-clockwise seen from the origin.
    fn quad_at(x: f32, z: f32) -> [[f32; 4]; 4] {
        [
            [x - 1.0, -1.0, z, 1.0],
            [x + 1.0, -1.0, z, 1.0],
            [x + 1.0, 1.0, z, 1.0],
            [x - 1.0, 1.0, z, 1.0],
        ]
    }

    fn set_of(quads: &[[[f32; 4]; 4]]) -> AntiFrustumSet {
        let mut bytes = vec![0; 48];
        bytes[0x20..0x24].copy_from_slice(&u32::try_from(quads.len()).unwrap().to_be_bytes());
        bytes[0x24..0x28].copy_from_slice(&48_u32.to_be_bytes());
        for quad in quads {
            for lane in quad.iter().flatten() {
                bytes.extend_from_slice(&lane.to_be_bytes());
            }
        }
        AntiFrustumSet::parse(&bytes).unwrap()
    }

    /// A camera at the origin looking down +Z, near 0.1 and far 1000: clip W is Z.
    const FORWARD: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0001, -0.10001],
        [0.0, 0.0, 1.0, 0.0],
    ];

    fn sphere(centre: [f32; 3], radius: f32) -> AxisAlignedBox {
        AxisAlignedBox {
            minimum: [centre[0] - radius, centre[1] - radius, centre[2] - radius, radius],
            maximum: [centre[0] + radius, centre[1] + radius, centre[2] + radius, 0.0],
        }
    }

    #[test]
    fn a_quad_hides_spheres_wholly_behind_it() {
        let set = set_of(&[quad_at(0.0, 10.0)]);
        let occluders = AntiFrustums::try_select([&set], [0.0; 3], FORWARD).unwrap();
        assert_eq!(occluders.len(), 1);
        assert!(occluders.occludes(&sphere([0.0, 0.0, 30.0], 0.5)));
        // In front of the quad, or reaching past its silhouette, stays visible.
        assert!(!occluders.occludes(&sphere([0.0, 0.0, 5.0], 0.5)));
        assert!(!occluders.occludes(&sphere([0.0, 0.0, 10.2], 0.5)));
        assert!(!occluders.occludes(&sphere([2.5, 0.0, 30.0], 0.5)));
    }

    #[test]
    fn winding_does_not_matter() {
        let mut quad = quad_at(0.0, 10.0);
        quad.reverse();
        let set = set_of(&[quad]);
        let occluders = AntiFrustums::try_select([&set], [0.0; 3], FORWARD).unwrap();
        assert!(occluders.occludes(&sphere([0.0, 0.0, 30.0], 0.5)));
        assert!(!occluders.occludes(&sphere([0.0, 0.0, 5.0], 0.5)));
    }

    #[test]
    fn far_or_unseen_quads_are_not_selected() {
        let set = set_of(&[quad_at(0.0, 150.0), quad_at(0.0, -10.0), quad_at(0.0, 20.0)]);
        let occluders = AntiFrustums::try_select([&set], [0.0; 3], FORWARD).unwrap();
        assert_eq!(occluders.len(), 1);
    }

    #[test]
    fn selection_is_nearest_first_and_capped() {
        let quads: Vec<_> = (0..40_u16)
            .map(|i| quad_at(f32::from(i) * 0.01, 10.0 + f32::from(i)))
            .collect();
        let set = set_of(&quads);
        let found = set.find_near([0.0; 3], SEARCH_RADIUS, MAX_SELECTED);
        assert_eq!(found.len(), 32);
        assert!(found.windows(2).all(|pair| pair[0] < pair[1]));
        let occluders = AntiFrustums::try_select([&set], [0.0; 3], FORWARD).unwrap();
        assert_eq!(occluders.len(), 32);
    }

    #[test]
    fn kd_walk_finds_what_a_scan_finds() {
        let mut quads = Vec::new();
        for i in 0..60_u16 {
            let x = f32::from((i * 37) % 23) * 9.0 - 100.0;
            let z = f32::from((i * 53) % 29) * 8.0 - 110.0;
            quads.push(quad_at(x, z));
        }
        let set = set_of(&quads);
        let point = [3.0, 0.0, -7.0];
        let mut expected: Vec<usize> = (0..quads.len())
            .filter(|&i| length(sub(set.nodes[i].coord, point)) <= 40.0)
            .collect();
        let mut found = set.find_near(point, 40.0, 1000);
        expected.sort_unstable();
        found.sort_unstable();
        assert_eq!(found, expected);
    }
}
