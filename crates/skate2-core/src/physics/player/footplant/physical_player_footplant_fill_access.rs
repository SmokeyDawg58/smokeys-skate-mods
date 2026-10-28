//! Access the actual retained `FootPlant` fields for Fill82db6890.
use super::{
    Arc, FootPlantError, FootPlantFields, PhysicalPlayerAssemblyOwner,
    PhysicalPlayerFootPlantOwner, SimulationSkaterAssemblyHandle,
};
impl PhysicalPlayerFootPlantOwner {
    pub(crate) fn with_fill_fields<T>(
        &self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &SimulationSkaterAssemblyHandle,
        read: impl FnOnce(&FootPlantFields) -> T,
    ) -> Result<T, FootPlantError> {
        let value = self.value.as_ref().ok_or(FootPlantError::NotConstructed)?;
        if !value
            .owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, owner))
            || !value.skeleton.same_as(skeleton)
        {
            return Err(FootPlantError::ForeignOwner);
        }
        #[cfg(feature = "skate3-runtime")]
        if let Some(source)=value.source3.as_ref().filter(|source|source.active) {
            return Ok(read(&source.fill_fields()));
        }
        Ok(read(&value.fields))
    }
}
