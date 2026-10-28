//! `Skateboard::CalculateGroundPos` 82b2f1b0: four actual wheel body positions.
//! No terrain query, part-local frame, translation or normalization is involved.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "selected ground-position scheduling pending")
)]
use crate::skeleton_root_transforms::SkeletonRootVector;

pub(crate) trait SkateboardGroundPositionSource {
    type Ground;
    type Body;
    type Error;
    fn capture_ground(&mut self) -> Result<Self::Ground, Self::Error>;
    fn ground_column(
        &mut self,
        ground: &Self::Ground,
        column: usize,
    ) -> Result<SkeletonRootVector, Self::Error>;
    fn capture_body(&mut self) -> Result<Self::Body, Self::Error>;
    fn body_position(&mut self, body: &Self::Body, part: usize) -> Result<[f32; 3], Self::Error>;
}
fn fsel(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}
fn lowest_y(y: [f32; 4]) -> f32 {
    let chosen = if y[1] > y[0] { y[0] } else { y[1] };
    let lowest = fsel(chosen - y[2], y[2], chosen);
    fsel(lowest - y[3], y[3], lowest)
}
/// Rounded multiply then two FMAs, the source vector-scale/add graph.
fn rotate(columns: [SkeletonRootVector; 3], value: [f32; 3]) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| {
            let first = columns[0].xyz[lane] * value[0];
            let second = columns[1].xyz[lane].mul_add(value[1], first);
            columns[2].xyz[lane].mul_add(value[2], second)
        }),
        w: columns[0]
            .w
            .zip(columns[1].w)
            .zip(columns[2].w)
            .map(|((x, y), z)| {
                let first = x * value[0];
                let second = y.mul_add(value[1], first);
                z.mul_add(value[2], second)
            }),
    }
}
/// Source result publications f238 zero, f368 local average, f3d4 world value.
/// Callback failures preserve the already published prefix. Body W is absent
/// because no surviving source calculation consumes it; basis W is preserved.
pub(crate) fn try_calculate_ground_position<S: SkateboardGroundPositionSource>(
    source: &mut S,
    result: &mut SkeletonRootVector,
) -> Result<(), S::Error> {
    let ground = source.capture_ground()?;
    let right = source.ground_column(&ground, 0)?;
    let up = source.ground_column(&ground, 1)?;
    let at = source.ground_column(&ground, 2)?;
    let transpose = std::array::from_fn(|lane| SkeletonRootVector {
        xyz: [right.xyz[lane], up.xyz[lane], at.xyz[lane]],
        w: Some(0.0),
    });
    *result = SkeletonRootVector::known([0.0; 4]);
    let body = source.capture_body()?;
    let p0 = source.body_position(&body, 0)?;
    let p1 = source.body_position(&body, 1)?;
    let p2 = source.body_position(&body, 2)?;
    let p3 = source.body_position(&body, 3)?;
    let q0 = rotate(transpose, p0);
    let q1 = rotate(transpose, p1);
    let q2 = rotate(transpose, p2);
    let q3 = rotate(transpose, p3);
    let mean = |a: f32, b: f32, c: f32, d: f32| (((a + b) + c) + d) * 0.25;
    *result = SkeletonRootVector {
        xyz: std::array::from_fn(|lane| {
            mean(q0.xyz[lane], q1.xyz[lane], q2.xyz[lane], q3.xyz[lane])
        }),
        w: Some(mean(
            q0.w.expect("known transposed W"),
            q1.w.expect("known transposed W"),
            q2.w.expect("known transposed W"),
            q3.w.expect("known transposed W"),
        )),
    };
    let lowest = lowest_y([q0.xyz[1], q1.xyz[1], q2.xyz[1], q3.xyz[1]]);
    let local = [result.xyz[0], lowest, result.xyz[2]];
    *result = rotate([right, up, at], local);
    Ok(())
}
#[cfg(test)]
#[path = "skateboard_calculate_ground_position_tests.rs"]
mod tests;
