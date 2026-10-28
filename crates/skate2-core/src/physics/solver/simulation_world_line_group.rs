//! LineBatchGroup827bc248/370, retaining existing World query leases.
use super::*;

pub(crate) struct WorldLineGroup {
    batches: Vec<SimulationWorldLineBatch>,
    offsets: Vec<u32>,
    current: usize,
}
impl WorldLineGroup {
    pub(crate) fn new() -> Self {
        Self { batches: Vec::new(), offsets: Vec::new(), current: 0 }
    }
    /// Caller retains this owner if an acquisition fails part-way through.
    pub(crate) fn acquire(&mut self, queries: &mut SimulationWorldLineQueries,
        dmos: &SimulationDmoAssemblyPool, label: &str, flags: u32, count: usize,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        while self.batches.len() < count {
            self.batches.push(queries.try_acquire_line_batch(dmos, label, flags)?);
            self.offsets.push(0);
        }
        Ok(())
    }
    pub(crate) fn begin(&mut self, queries: &mut SimulationWorldLineQueries) -> Result<(), String> {
        self.current = 0;
        let first = self.batches.first().ok_or("line group has no acquired batch")?;
        queries.try_begin(first).map_err(|e| format!("line group begin: {e:?}"))?;
        self.offsets[0] = 0;
        Ok(())
    }
    pub(crate) fn add<N: AssemblyGroundNumeric + ReckoningNumeric>(
        &mut self, queries: &mut SimulationWorldLineQueries, input: CollisionLineInput,
        ground: &StaticGroundPool, dmos: &SimulationDmoAssemblyPool, numeric: &mut N,
    ) -> Result<bool, String>
    where N::Error: fmt::Debug,
    {
        while self.current < self.batches.len() {
            let batch = &self.batches[self.current];
            if queries.try_add_line(batch, input).map_err(|e| format!("line group add: {e:?}"))? {
                self.offsets[self.current] += 1;
                return Ok(true);
            }
            queries.try_end(batch, ground, dmos, numeric).map_err(|e| format!("line group end: {e:?}"))?;
            self.current += 1;
            if let Some(next) = self.batches.get(self.current) {
                queries.try_begin(next).map_err(|e| format!("line group next: {e:?}"))?;
                self.offsets[self.current] = self.offsets[self.current - 1];
            }
        }
        Ok(false)
    }
    pub(crate) fn end<N: AssemblyGroundNumeric + ReckoningNumeric>(
        &mut self, queries: &mut SimulationWorldLineQueries,
        ground: &StaticGroundPool, dmos: &SimulationDmoAssemblyPool, numeric: &mut N,
    ) -> Result<(), String>
    where N::Error: fmt::Debug,
    {
        if let Some(batch) = self.batches.get(self.current) {
            queries.try_end(batch, ground, dmos, numeric).map_err(|e| format!("line group end: {e:?}"))?;
        }
        Ok(())
    }
    pub(crate) fn sync(&self, queries: &mut SimulationWorldLineQueries) -> Result<(), SimulationWorldLineLeaseError> {
        for batch in &self.batches { queries.try_sync(batch)?; }
        Ok(())
    }
    pub(crate) fn result(&self, queries: &mut SimulationWorldLineQueries, index: u32)
        -> Result<Option<SimulationWorldLineHit>, SimulationWorldLineLeaseError> {
        for batch in 0..self.batches.len() {
            if batch > self.current { break; }
            if self.offsets[batch] > index {
                let local = index - if batch == 0 { 0 } else { self.offsets[batch - 1] };
                return queries.try_get_result(&self.batches[batch], local);
            }
        }
        Ok(None)
    }
    pub(crate) fn release(&self, queries: &mut SimulationWorldLineQueries) -> Result<(), SimulationWorldLineLeaseError> {
        for batch in &self.batches { queries.try_release(batch)?; }
        Ok(())
    }
}
