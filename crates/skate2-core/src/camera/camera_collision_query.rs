//! Actual camera collision adapter over the already-held world's line owner.
use super::{CameraCollision, CameraLineResult};
use crate::{
    collision_line_collection::CollisionLineInput,
    reckoning_numeric::ReckoningHostNumericError,
    simulation_assembly_backend::constraint_session::{
        physical_world_line::camera::CameraLineServices,
        world_line_query::{SimulationWorldLineError, SimulationWorldLineLeaseError},
    },
    skeleton_root_transforms::SkeletonRootVector as Vector,
};

#[derive(Debug)]
pub(crate) enum CameraQueryError {
    MissingQueries,
    MissingDmos,
    InterruptedBatch,
    Lease(SimulationWorldLineLeaseError),
    Query(SimulationWorldLineError<ReckoningHostNumericError>),
}
impl std::fmt::Display for CameraQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingQueries => f.write_str("camera world query owner absent"),
            Self::MissingDmos => f.write_str("camera world DMO owner absent"),
            Self::InterruptedBatch => f.write_str("camera batch interrupted before native Release"),
            Self::Lease(error) => write!(f, "{error}"),
            Self::Query(error) => write!(f, "{error}"),
        }
    }
}

impl CameraCollision {
    /// `TestLinesVsWorld82e39758`. Each result is appended only after Sync.
    /// A host failure retains the acquired lease instead of inventing Release.
    pub(crate) fn try_test_lines(
        &mut self,
        lines: &[CollisionLineInput],
        results: &mut Vec<CameraLineResult>,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), CameraQueryError> {
        let profile = std::env::var_os("SKATE2_PROFILE_REDRAW").map(|_| std::time::Instant::now());
        let _line_scope = crate::collision_line_collection::profile::begin("camera_positioner");
        use CameraQueryError as E;
        if self.pending_batch.is_some() {
            return Err(E::InterruptedBatch);
        }
        let queries = services.queries.as_mut().ok_or(E::MissingQueries)?;
        let dmos = services.dmos.ok_or(E::MissingDmos)?;
        self.pending_batch = Some(
            queries
                .try_acquire_line_batch(dmos, "CameraCollisionQueryServer::TestLinesVsWorld", 7)
                .map_err(E::Lease)?,
        );
        let batch = self.pending_batch.as_ref().ok_or(E::InterruptedBatch)?;
        queries.try_begin(batch).map_err(E::Lease)?;
        for line in lines {
            let _ = queries.try_add_line(batch, *line).map_err(E::Lease)?;
        }
        queries
            .try_end(batch, services.ground, dmos, services.numeric)
            .map_err(E::Query)?;
        queries.try_sync(batch).map_err(E::Lease)?;
        for (index, line) in lines.iter().enumerate() {
            let hit = queries
                .try_get_result(batch, u32::try_from(index).expect("native ten-line list"))
                .map_err(E::Lease)?;
            results.push(match hit {
                Some(hit) => CameraLineResult {
                    position: hit.position,
                    normal: hit.normal,
                    parameter: hit.line_parameter,
                    collided: true,
                },
                None => CameraLineResult {
                    position: line.end,
                    normal: Vector::known([0.0, 1.0, 0.0, 0.0]),
                    parameter: 1.0,
                    collided: false,
                },
            });
        }
        queries.try_release(batch).map_err(E::Lease)?;
        self.pending_batch = None;
        drop(_line_scope);
        if let Some(start) = profile {
            eprintln!(
                "{}",
                format!(
                    "diagnostic camera collision: batch_lines={} time={}ms",
                    lines.len(),
                    start.elapsed().as_secs_f64() * 1000.0
                )
            );
        }
        Ok(())
    }

    /// `TestFatLineVsWorld82e34068`. The actual Island vehicle wrapper delegates
    /// to `World.LineCollisionTest827b2538`; both branches use this same route.
    pub(crate) fn try_test_line(
        &mut self,
        line: CollisionLineInput,
        services: &mut CameraLineServices<'_>,
    ) -> Result<&CameraLineResult, CameraQueryError> {
        let profile = std::env::var_os("SKATE2_PROFILE_REDRAW").map(|_| std::time::Instant::now());
        let _line_scope = crate::collision_line_collection::profile::begin("camera_positioner");
        use CameraQueryError as E;
        let queries = services.queries.as_mut().ok_or(E::MissingQueries)?;
        let dmos = services.dmos.ok_or(E::MissingDmos)?;
        let hit = queries
            .try_line_test(
                services.ground,
                dmos,
                line,
                "CameraCollisionQueryServer::TestFatLineVsWorld",
                services.numeric,
            )
            .map_err(E::Query)?;
        if let Some(hit) = hit {
            self.result.position = hit.position;
            self.result.normal = hit.normal;
            self.result.collided = true;
            self.result.parameter = hit.line_parameter;
        } else {
            // A native miss only overwrites the flag; retained payload stays intact.
            self.result.collided = false;
        }
        drop(_line_scope);
        if let Some(start) = profile {
            eprintln!(
                "{}",
                format!(
                    "diagnostic camera collision: single time={}ms",
                    start.elapsed().as_secs_f64() * 1000.0
                )
            );
        }
        Ok(&self.result)
    }
}
