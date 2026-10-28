//! ContextSensitiveUIDData82ded198/d250/8a00: fixed capacity, UID-only identity.
use super::{
    super::scoring::{ContextGroups, ContextUid, ContextUids},
    scoring_attributes::ScoreError,
};

impl ContextUids {
    fn active(&self) -> Result<&[ContextUid], ScoreError> {
        self.uids
            .get(..self.count as usize)
            .ok_or(ScoreError::Invalid("context UID count"))
    }
    pub(super) fn add_uid(&mut self, unique_id: u32, entity_type: i32) -> Result<(), ScoreError> {
        if self.active()?.iter().any(|v| v.unique_id == unique_id) {
            return Err(ScoreError::Invalid("duplicate context UID"));
        }
        if self.count < 8 {
            self.uids[self.count as usize] = ContextUid {
                unique_id,
                entity_type,
            };
            self.count += 1;
        }
        Ok(())
    }
    pub(super) fn add(&mut self, other: &Self) -> Result<(), ScoreError> {
        for uid in other.active()? {
            if !self.active()?.iter().any(|v| v.unique_id == uid.unique_id) {
                self.add_uid(uid.unique_id, uid.entity_type)?;
            }
        }
        Ok(())
    }
    pub(super) fn remove(&mut self, other: &Self) -> Result<(), ScoreError> {
        for uid in other.active()? {
            if let Some(index) = self
                .active()?
                .iter()
                .position(|v| v.unique_id == uid.unique_id)
            {
                self.count -= 1;
                if index < self.count as usize {
                    self.uids[index] = self.uids[self.count as usize].clone();
                }
            }
        }
        Ok(())
    }
}
impl ContextGroups {
    pub(super) fn add(&mut self, other: &Self) -> Result<(), ScoreError> {
        for (dst, src) in self
            .context_sensitive_data
            .iter_mut()
            .zip(&other.context_sensitive_data)
        {
            for (dst, src) in dst.uid_data.iter_mut().zip(&src.uid_data) {
                dst.add(src)?;
            }
        }
        Ok(())
    }
    pub(super) fn remove(&mut self, other: &Self) -> Result<(), ScoreError> {
        for (dst, src) in self
            .context_sensitive_data
            .iter_mut()
            .zip(&other.context_sensitive_data)
        {
            for (dst, src) in dst.uid_data.iter_mut().zip(&src.uid_data) {
                dst.remove(src)?;
            }
        }
        Ok(())
    }
}
