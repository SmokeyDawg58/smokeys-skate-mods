//! Mounted selected immutable Vault loading; no native export construction.
//!
//! Dependency path/order follows `LoadAttributeSystemVault82ab99f0`. The
//! existing reader's post-load graph and safe-host failure policy remain in
//! force: native early `VaultMap` publication and partial-load effects are not
//! reproduced. Nonasset cross-Vault resolution remains unsupported.
use super::{AttributeRuntimeDependency, AttributeRuntimeError, AttributeRuntimeVault};
use skate2_formats::{
    attrib::AttributeVaultMetadata,
    mount::{MountError, MountSet},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum MountedVaultError {
    Read { path: String, cause: MountError },
    Vault(AttributeRuntimeError),
    NonassetDependency { index: usize, name: String },
}
impl std::fmt::Display for MountedVaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, cause } => write!(f, "mounted Vault {path}: {cause}"),
            Self::Vault(cause) => write!(f, "mounted Vault: {cause}"),
            Self::NonassetDependency { index, name } => {
                write!(
                    f,
                    "Vault dependency {index} ({name}) needs cross-Vault resolution"
                )
            }
        }
    }
}

impl AttributeRuntimeVault {
    /// Read through the caller's same mounts, retaining one immutable graph.
    /// Native sibling paths and metadata order are preserved; allocation,
    /// constructor callbacks, unload and map publication are outside this API.
    pub(crate) fn try_load_mounted(
        mounts: &MountSet,
        path: &str,
        selected_classes: &[u64],
    ) -> Result<Arc<Self>, MountedVaultError> {
        let vlt: Arc<[u8]> = read(mounts, path)?.into();
        let metadata = AttributeVaultMetadata::parse(&vlt)
            .map_err(AttributeRuntimeError::from)
            .map_err(MountedVaultError::Vault)?;
        let directory = path.rfind(['/', '\\']).map_or("", |end| &path[..end]);
        let mut dependencies = Vec::with_capacity(metadata.dependencies.len());
        for (index, dependency) in metadata.dependencies.into_iter().enumerate() {
            if !dependency.is_asset {
                return Err(MountedVaultError::NonassetDependency {
                    index,
                    name: dependency.name,
                });
            }
            let dependency_path = format!("{directory}\\{}", dependency.name);
            let bytes = read(mounts, &dependency_path)?.into();
            dependencies.push(AttributeRuntimeDependency {
                id: dependency.id,
                name: dependency.name,
                is_asset: dependency.is_asset,
                bytes,
            });
        }
        Self::try_load(vlt, &dependencies, selected_classes).map_err(MountedVaultError::Vault)
    }
}

fn read(mounts: &MountSet, path: &str) -> Result<Vec<u8>, MountedVaultError> {
    mounts
        .read_path(path)
        .map_err(|cause| MountedVaultError::Read {
            path: path.to_owned(),
            cause,
        })
}
