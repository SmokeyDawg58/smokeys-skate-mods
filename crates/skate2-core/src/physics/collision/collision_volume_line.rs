//! VolumeLineBatch constructor827c0ac8 and request/volume admission.
//! Native LineTest827c05d8 filtering/escape policy and rounded-box queries.
use crate::animation::AnimationMatrix;
use crate::physical_player_grab_spline::geometry::inverse_transform_point;
use crate::reckoning_numeric::ReckoningNumeric;

pub(crate) struct VolumeBoxHit {
    pub(crate) position: [f32; 4],
    pub(crate) normal: [f32; 4],
    pub(crate) parameter: f32,
}

pub(crate) struct VolumeData {
    pub(crate) transform: AnimationMatrix,
    pub(crate) dimensions: [f32; 4],
    pub(crate) mask: u32,
}
pub(crate) struct VolumeLineResult {
    pub(crate) position: [f32; 4],
    pub(crate) normal: [f32; 4],
    pub(crate) line_parameter: f32,
    pub(crate) surface_id: u32,
    pub(crate) owner: u32,
    pub(crate) intersect: bool,
}
pub(crate) struct VolumeLineTest {
    pub(crate) start: [f32; 4],
    pub(crate) end: [f32; 4],
    pub(crate) fatness: f32,
    owner_mask: u32,
    ignore_mask: u32,
    has_owner: bool,
    use_mask: bool,
    pub(crate) result: VolumeLineResult,
}
impl VolumeLineTest {
    /// LineTest827c0694..06c8: ignored owner first, then querying owner.
    pub(crate) fn excludes(&self, volume: &VolumeData) -> bool {
        (self.use_mask && volume.mask == self.ignore_mask)
            || (self.has_owner && volume.mask == self.owner_mask)
    }

    /// 827c05d8. The intersector must supply BoxVolume82ad7170 results;
    /// ordinary slab intersection does not preserve the rounded corners.
    pub(crate) fn process<N: ReckoningNumeric>(
        &mut self, volumes: &[VolumeData], numeric: &mut N,
        intersect: &mut impl FnMut(&VolumeData, [f32; 4], [f32; 4], f32,
            &mut N) -> Result<Option<VolumeBoxHit>, N::Error>,
    ) -> Result<bool, N::Error> {
        // EPSILON_31-4 and EPSILON_31 read from821043a8/ac.
        let segment_epsilon = f32::from_bits(0x37800000);
        let parameter_epsilon = f32::from_bits(0x34000000);
        if !(0..3).any(|i| (self.start[i] - self.end[i]).abs() > segment_epsilon) {
            return Ok(false);
        }
        for volume in volumes {
            if self.excludes(volume) { continue; }
            let Some(hit) = intersect(volume, self.start, self.end, self.fatness, numeric)? else {
                continue;
            };
            if !(hit.parameter.abs() > parameter_epsilon) {
                let reverse = intersect(volume, self.end, self.start, self.fatness, numeric)?;
                let escaping = if let Some(reverse) = reverse.filter(|h| h.parameter.abs() > parameter_epsilon) {
                    let rows = std::array::from_fn(|r| std::array::from_fn(|c| volume.transform[r * 4 + c]));
                    // Transpose/negative translation Z,Y,X followed by point
                    // X,Y,Z: same instruction graph as shared grab geometry.
                    let start = inverse_transform_point(&rows, self.start);
                    let exit = inverse_transform_point(&rows, reverse.position);
                    (0..3).all(|i| start[i] * exit[i] >= 0.0)
                } else {
                    let mut direction = std::array::from_fn(|i| self.end[i] - self.start[i]);
                    let mut toward_centre = std::array::from_fn(|i| volume.transform[12 + i] - self.start[i]);
                    direction[1] = 0.0;
                    toward_centre[1] = 0.0;
                    numeric.dot3(toward_centre, direction)? < 0.0
                };
                if escaping { continue; }
            }
            if !self.result.intersect || hit.parameter < self.result.line_parameter {
                self.result.intersect = true;
                self.result.position = hit.position;
                self.result.normal = hit.normal;
                self.result.line_parameter = hit.parameter;
                // Native copies only position/normal/parameter, not volumeMask
                // into result owner or surface. Preserve AddTest's zeros.
                if !(hit.parameter.abs() > parameter_epsilon) { break; }
            }
        }
        Ok(self.result.intersect)
    }
}
pub(crate) struct VolumeLineBatch {
    pub(crate) volumes: Vec<VolumeData>,
    pub(crate) tests: Vec<VolumeLineTest>,
    capacity: usize,
    pub(crate) null_mask: u32,
}
impl VolumeLineBatch {
    pub(crate) fn new(capacity: usize, null_mask: u32) -> Self {
        Self { volumes: Vec::with_capacity(512), tests: Vec::with_capacity(capacity),
            capacity, null_mask }
    }
    /// AddVolume82d16060 copies transform/dimensions/mask without conversion.
    pub(crate) fn add_volume(&mut self, volume: VolumeData) -> bool {
        if self.volumes.len() >= 512 { return false; }
        self.volumes.push(volume);
        true
    }
    /// JobBatch Begin resets test count; volume snapshot has separate lifetime.
    pub(crate) fn begin(&mut self) { self.tests.clear(); }
    pub(crate) fn clear_volumes(&mut self) { self.volumes.clear(); }
    pub(crate) fn end<N: ReckoningNumeric>(&mut self, numeric: &mut N) -> Result<(), N::Error> {
        self.process(numeric, crate::collision_box_line::intersect)
    }
    pub(crate) fn process<N: ReckoningNumeric>(&mut self, numeric: &mut N,
        mut intersect: impl FnMut(&VolumeData, [f32; 4], [f32; 4], f32,
            &mut N) -> Result<Option<VolumeBoxHit>, N::Error>,
    ) -> Result<(), N::Error> {
        for test in &mut self.tests {
            test.process(&self.volumes, numeric, &mut intersect)?;
        }
        Ok(())
    }
    pub(crate) fn result(&self, index: usize) -> Option<&VolumeLineResult> {
        self.tests.get(index).and_then(|test| test.result.intersect.then_some(&test.result))
    }
    /// AddTest827bdaf0 initializes output before retaining masks and endpoints.
    pub(crate) fn add_test(&mut self, start: [f32; 4], end: [f32; 4],
        fatness: f32, owner: u32, ignore: u32,
    ) -> bool {
        if self.tests.len() == self.capacity { return false; }
        self.tests.push(VolumeLineTest {
            start, end, fatness, owner_mask: owner, ignore_mask: ignore,
            has_owner: owner != self.null_mask, use_mask: ignore != self.null_mask,
            result: VolumeLineResult { position: [0.; 4], normal: [0., 1., 0., 0.],
                line_parameter: f32::MAX, surface_id: 0, owner: 0, intersect: false },
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;

    fn volume(mask: u32) -> VolumeData {
        VolumeData { transform: [1.,0.,0.,0., 0.,1.,0.,0., 0.,0.,1.,0., 0.,0.,0.,1.],
            dimensions: [1.,1.,1.,0.], mask }
    }
    fn hit(parameter: f32, x: f32) -> VolumeBoxHit {
        VolumeBoxHit { position: [x,0.,0.,1.], normal: [1.,0.,0.,0.], parameter }
    }
    #[test]
    fn escape_on_same_side_is_ignored_but_crossing_is_blocked() {
        for (reverse_x, blocked) in [(1., false), (-1., true)] {
            let mut batch = VolumeLineBatch::new(1, 0);
            batch.add_volume(volume(7));
            batch.add_test([0.5,0.,0.,1.], [2.,0.,0.,1.], 0.3, 0, 0);
            let mut calls = 0;
            batch.process(&mut HostReckoningNumeric, |_, _, _, _, _| {
                calls += 1;
                Ok(Some(if calls == 1 { hit(0., 0.5) } else { hit(0.5, reverse_x) }))
            }).unwrap();
            assert_eq!(calls, 2);
            assert_eq!(batch.result(0).is_some(), blocked);
        }
    }
    #[test]
    fn masks_are_filtered_before_queries_and_equal_hits_keep_first() {
        let mut batch = VolumeLineBatch::new(1, 0);
        for mask in [1,2,3,4] { batch.add_volume(volume(mask)); }
        batch.add_test([-2.,0.,0.,1.], [2.,0.,0.,1.], 0.3, 1, 2);
        let mut visited = Vec::new();
        batch.process(&mut HostReckoningNumeric, |v, _, _, _, _| {
            visited.push(v.mask);
            Ok(Some(hit(0.25, v.mask as f32)))
        }).unwrap();
        assert_eq!(visited, [3,4]);
        let result = batch.result(0).unwrap();
        assert_eq!(result.position[0], 3.);
        assert_eq!(result.owner, 0);
    }
    #[test]
    fn overlap_fallback_uses_horizontal_motion_and_stops_on_zero_hit() {
        for (end_x, blocked) in [(2., false), (-2., true)] {
            let mut batch = VolumeLineBatch::new(1, 0);
            batch.add_volume(volume(7));
            batch.add_test([0.5,10.,0.,1.], [end_x,100.,0.,1.], 0.3, 0, 0);
            let mut calls = 0;
            batch.process(&mut HostReckoningNumeric, |_, _, _, _, _| {
                calls += 1;
                Ok(if calls == 1 { Some(hit(0., 0.5)) } else { None })
            }).unwrap();
            assert_eq!(batch.result(0).is_some(), blocked);
        }
    }
}
