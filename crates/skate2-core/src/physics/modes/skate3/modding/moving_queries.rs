//! Solid dual-world boundary. Native local bones participate in BoardStep;
//! remote actor mirrors participate in Rapier. Never solve the local actor in
//! both worlds, and never feed reactions back to another peer's body.
//!
//! A moving body with an authored collision mesh (Object Dropper props, city
//! DMOs) answers line and nearby-triangle queries with that mesh's native
//! triangles at its current placement, exactly as static ground answers them:
//! the same triangle dispatcher, GP features and packed surface. Other bodies
//! answer through their solid Parry shapes.

use crate::skate3_native::math_host::*;
use skate3_core::{math::Vector3, physics::{
    board_world::{ExternalLineHit, ExternalQueries, PreparedTriangles, WorldLineHit, query_metadata::Bounds},
    triangle_query::{TriangleLineHit, triangle_segment},
}};
use skate_dynamics::{SolidBody, solid::{collider_triangles, sweep_sphere}};
use std::sync::{Arc, Mutex, Weak};

fn xyz(v: Vector3) -> [f32;3] { [v.x,v.y,v.z] }
fn native(v: [f32;3]) -> Vector3 { Vector3::new(v[0],v[1],v[2]) }

/// Nearby-triangle answers per query. Every consumer keeps at most 64 after
/// its own exact triangle/box test, so this only bounds a pathological query.
const NEARBY_CAPACITY: usize = 512;

fn body_frame(body: &SolidBody) -> [[f32; 4]; 4] {
    let q = body.pose.rotation;
    Mat4::from_rotation_translation(
        Quat::from_xyzw(q.x,q.y,q.z,q.w), Vec3::from_array(body.pose.translation.to_array()))
        .to_cols_array_2d()
}
fn geometry_id(id: u64) -> u32 { 0x8000_0000 | (id as u32 & 0x7fff_ffff) }

/// Triangles per run of a mesh's index.
const RUN: usize = 32;

/// A placed mesh's triangles in runs of `RUN` (native order), each run's bounds, and the mesh as
/// a contact world whose query meshes are those runs: queries and contacts skip the runs away
/// from them instead of testing every triangle of a big piece. Built once per placed triangle
/// set (a placement's triangles stay the same `Arc` while it does not move).
pub(crate) struct MeshIndex {
    bounds: Bounds,
    runs: Vec<(std::ops::Range<usize>, Bounds)>,
    pub world: Mutex<skate3_core::physics::board_world::BoardWorld>,
}

/// The index of `triangles`, built the first time they are asked for; None when they are
/// not all finite.
fn mesh_index(triangles: &Arc<PreparedTriangles>) -> Option<Arc<MeshIndex>> {
    use skate3_core::physics::{board_world::{BoardWorld, query_metadata::{QueryMesh, QueryMetadata, QueryPool}}, drive_frames::RetailAffineTransform};
    static INDEXES: Mutex<Vec<(Weak<PreparedTriangles>, Arc<MeshIndex>)>> = Mutex::new(Vec::new());
    let mut indexes = INDEXES.lock().ok()?;
    if let Some((_, index)) = indexes.iter().find(|(weak, _)| weak.strong_count() > 0 && std::ptr::eq(weak.as_ptr(), Arc::as_ptr(triangles))) {
        return Some(Arc::clone(index));
    }
    indexes.retain(|(weak, _)| weak.strong_count() > 0);
    let bounds = Bounds::from_points(triangles.triangle_bounds().iter().flat_map(|bounds| [bounds.min, bounds.max]))?;
    let runs: Vec<_> = (0..triangles.triangles().len())
        .step_by(RUN)
        .map(|start| {
            let range = start..(start + RUN).min(triangles.triangles().len());
            let run = Bounds::from_points(triangles.triangle_bounds()[range.clone()].iter().flat_map(|bounds| [bounds.min, bounds.max]));
            run.map(|run| (range, run))
        })
        .collect::<Option<_>>()?;
    let metadata = QueryMetadata {
        packed_surfaces: vec![0; triangles.triangles().len()],
        meshes: runs
            .iter()
            .map(|(range, bounds)| QueryMesh {
                triangle_range: range.clone(),
                local_to_world: RetailAffineTransform::IDENTITY,
                world_to_local: RetailAffineTransform::IDENTITY,
                local_bounds: *bounds,
                matching_group: 0,
                rejection_flags: 0,
                geometry: 0,
                pool: QueryPool::Ground,
            })
            .collect(),
        static_edges: Vec::new(),
        island_flags: 0,
    };
    // The runs only narrow which triangles are tried (a superset of every pair the
    // per-triangle test keeps, in native order), so contacts are the same.
    let world = BoardWorld::with_query_metadata(triangles.triangles().to_vec(), metadata)
        .unwrap_or_else(|_| BoardWorld::new(triangles.triangles().to_vec()));
    let index = Arc::new(MeshIndex { bounds, runs, world: Mutex::new(world) });
    indexes.push((Arc::downgrade(triangles), Arc::clone(&index)));
    Some(index)
}

/// One moving body's authored mesh: world-space native triangles at the
/// body's current placement, with their union bounds and index.
#[derive(Clone)]
pub(crate) struct MeshQuery {
    pub id: u64,
    pub frame: [[f32; 4]; 4],
    /// The body's linear velocity (contacts use the volume's relative motion).
    pub velocity: Vector3,
    pub triangles: Arc<PreparedTriangles>,
    pub bounds: Bounds,
    pub index: Arc<MeshIndex>,
}
impl MeshQuery {
    pub(crate) fn new(body: &SolidBody, triangles: Arc<PreparedTriangles>) -> Option<Self> {
        let index = mesh_index(&triangles)?;
        Some(Self { id: body.id, frame: body_frame(body), velocity: native(body.linvel.to_array()), triangles, bounds: index.bounds, index })
    }
    /// Native triangles whose bounds overlap `bounds`, in native order.
    pub(crate) fn candidates(&self, bounds: Bounds)
        -> impl Iterator<Item = &skate3_core::physics::board_world::WorldTriangle> {
        let overlaps = self.bounds.overlaps(bounds);
        let (triangles, triangle_bounds) = (self.triangles.triangles(), self.triangles.triangle_bounds());
        self.index
            .runs
            .iter()
            .filter(move |(_, run)| overlaps && run.overlaps(bounds))
            .flat_map(|(range, _)| range.clone())
            .filter(move |&index| triangle_bounds[index].overlaps(bounds))
            .map(move |index| &triangles[index])
    }
}

pub(crate) struct MovingQueries {
    /// Bodies answered through their solid Parry shapes.
    solids: Vec<SolidBody>,
    meshes: Vec<MeshQuery>,
}
impl MovingQueries {
    /// `meshes` name bodies of `bodies`; those answer only through their mesh.
    pub(crate) fn new(bodies: Vec<SolidBody>, meshes: Vec<MeshQuery>) -> Self {
        let solids = bodies.into_iter().filter(|body| !meshes.iter().any(|mesh| mesh.id == body.id)).collect();
        Self { solids, meshes }
    }
}
impl ExternalQueries for MovingQueries {
    fn line(&self, start: Vector3, end: Vector3, radius: f32) -> Option<ExternalLineHit> {
        let mut nearest = sweep_sphere(&self.solids, xyz(start), xyz(end), radius).and_then(|(id, c)| {
            let body = self.solids.iter().find(|b| b.id == id)?;
            Some(ExternalLineHit {
                hit: WorldLineHit { geometry: TriangleLineHit {
                    position: native(c.point_b.to_array()), normal: native(c.normal.to_array()),
                    fraction: c.time_of_impact, volume_parameter: [0.;3],
                }, tag: 0 }, surface: 0, geometry_id: geometry_id(id),
                frame: body_frame(body),
            })
        });
        if self.meshes.is_empty() || !radius.is_finite() || radius < 0. { return nearest; }
        let Some(segment) = Bounds::from_points([start, end]) else { return nearest };
        // Thin-triangle leaves accept slightly negative barycentrics; pad as
        // the static world's line candidates do.
        let segment = segment.expanded(radius + 0.01);
        let direction = Vector3::new(end.x - start.x, end.y - start.y, end.z - start.z);
        for mesh in &self.meshes {
            for entry in mesh.candidates(segment) {
                let mut geometry = TriangleLineHit {
                    position: Vector3::ZERO, normal: Vector3::ZERO, fraction: 0., volume_parameter: [0.; 3],
                };
                if triangle_segment(&mut geometry, start, direction, entry.triangle.vertices, radius, entry.triangle.fatness)
                    && nearest.as_ref().is_none_or(|hit| geometry.fraction < hit.hit.geometry.fraction)
                {
                    nearest = Some(ExternalLineHit {
                        hit: WorldLineHit { geometry, tag: entry.tag },
                        surface: entry.tag as u16,
                        geometry_id: geometry_id(mesh.id),
                        frame: mesh.frame,
                    });
                }
            }
        }
        nearest
    }
    fn nearby(&self, center: Vector3, radius: f32) -> Vec<[Vector3;3]> {
        let point = Vec3::from_array(xyz(center));
        if !point.is_finite() || !radius.is_finite() || radius < 0. { return Vec::new(); }
        let mut triangles = Vec::new();
        let within = |min: Vec3, max: Vec3| point.distance_squared(point.clamp(min, max)) <= radius * radius;
        let query = Bounds { min: center, max: center }.expanded(radius);
        for mesh in &self.meshes {
            for entry in mesh.candidates(query) {
                let verts = entry.triangle.vertices.map(|v| Vec3::new(v.x, v.y, v.z));
                if within(verts[0].min(verts[1]).min(verts[2]), verts[0].max(verts[1]).max(verts[2])) {
                    triangles.push(entry.triangle.vertices);
                    if triangles.len() >= NEARBY_CAPACITY { return triangles; }
                }
            }
        }
        for body in &self.solids {
            for collider in &body.colliders {
                let aabb = collider.shape.compute_aabb(&collider.pose);
                if !within(Vec3::from_array(aabb.mins.to_array()), Vec3::from_array(aabb.maxs.to_array())) { continue; }
                for t in collider_triangles(collider) {
                    let verts = t.map(Vec3::from_array);
                    if within(verts[0].min(verts[1]).min(verts[2]), verts[0].max(verts[1]).max(verts[2])) {
                        triangles.push(t.map(native));
                        if triangles.len() >= NEARBY_CAPACITY { return triangles; }
                    }
                }
            }
        }
        triangles
    }
}
