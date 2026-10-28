//! Weak, owner-qualified model routes for one resident main-DMO bank.
//!
//! Native `Sk8::Dmo::Registration::RegDmoModel` (`0x825eab98`) keys its
//! model table by `tInstance + 0x68`, and `Sk8::Dmo::SimManager::SpawnStatic`
//! (`0x825eb0a0`) looks up the locator's model ID in that table before following
//! the model's Assembly relationship. This module reconstructs only that
//! immutable identity route. It does not create a DMO, source live transforms,
//! or retain a generic resident-asset ownership capability.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "opaque resolution awaits the production DMO occurrence caller"
    )
)]

use super::{
    SimulationDmoAssemblyPartOwner, SimulationDmoAssemblyPartOwnerError,
    SimulationDmoModelCurrentWinner, SourceBackend,
};
use crate::{
    collection_resident_streamer::CollectionId,
    dmo_bank_processed_asset::{DmoBankProcessedArenaAsset, DmoSerialisedAssemblySelectError},
    owned_resident_collection::{
        OwnedResidentCollection, OwnedResidentCollectionError, OwnedResidentCollectionView,
    },
    simulation_normal_arena_plan::{SimulationNormalOperation, SimulationRecordSourceKind},
};
use skate2_formats::pegasus::{C_MODEL_DATA_TYPE_ID, ObjectId};
use std::{error::Error, fmt, sync::Arc, sync::Weak};

const DMO_MODEL_RECORD_MINIMUM_BYTES: usize = 136;
const DMO_MODEL_PRIMARY_GUID_OFFSET: usize = 96;
const DMO_MODEL_LOCAL_GUID_OFFSET: usize = 104;
const DMO_MODEL_COLLISION_OBJECT_OFFSET: usize = 132;
const DMO_MODEL_ROUTE_CAPACITY: usize = 256;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Physical position of one model route in the resident bank.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmoBankResidentModelRoutePosition {
    pub asset_index: usize,
    pub operation_index: usize,
    pub asset_id: CollectionId,
}

/// Safe-host refusal while sealing one immutable resident model-route catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DmoBankResidentModelCatalogError {
    ResidentSnapshot(OwnedResidentCollectionError),
    RouteStorageAllocation {
        requested: usize,
    },
    AssetExpiredDuringBuild {
        position: usize,
        asset_id: CollectionId,
    },
    MissingRecordSource(DmoBankResidentModelRoutePosition),
    WrongRecordSourceKind(DmoBankResidentModelRoutePosition),
    ShortModelRecord {
        position: DmoBankResidentModelRoutePosition,
        actual: usize,
    },
    PrimaryGuidMismatch {
        position: DmoBankResidentModelRoutePosition,
        operation: u64,
        record: u64,
    },
    LocalGuidMismatch {
        position: DmoBankResidentModelRoutePosition,
        operation: u64,
        record: u64,
    },
    MissingCollisionModel {
        position: DmoBankResidentModelRoutePosition,
        model_id: u64,
    },
    NonLocalCollisionModel {
        position: DmoBankResidentModelRoutePosition,
        object_id: ObjectId,
        section: u16,
    },
    MissingCollisionModelObject {
        position: DmoBankResidentModelRoutePosition,
        object_id: ObjectId,
    },
    WrongCollisionModelObjectType {
        position: DmoBankResidentModelRoutePosition,
        object_id: ObjectId,
        actual: u32,
    },
    MissingProjectedCollisionModel {
        position: DmoBankResidentModelRoutePosition,
        dictionary_index: u32,
    },
    DuplicateProjectedCollisionModel {
        position: DmoBankResidentModelRoutePosition,
        dictionary_index: u32,
    },
    MissingAssembly {
        position: DmoBankResidentModelRoutePosition,
        dictionary_index: u32,
    },
    NonLocalAssembly {
        position: DmoBankResidentModelRoutePosition,
        object_id: ObjectId,
        section: u16,
    },
    MissingAssemblyTemplate {
        position: DmoBankResidentModelRoutePosition,
        object_id: ObjectId,
    },
    DuplicateAssemblyTemplate {
        position: DmoBankResidentModelRoutePosition,
        object_id: ObjectId,
        first_ordinal: usize,
        second_ordinal: usize,
    },
    DuplicateModelId {
        model_id: u64,
        first: DmoBankResidentModelRoutePosition,
        second: DmoBankResidentModelRoutePosition,
    },
    ModelCapacityExceeded {
        capacity: usize,
        position: DmoBankResidentModelRoutePosition,
    },
}

impl fmt::Display for DmoBankResidentModelCatalogError {
    #[allow(
        clippy::too_many_lines,
        reason = "the exhaustive typed refusal formatter remains a single mechanical match"
    )]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResidentSnapshot(source) => {
                write!(formatter, "resident snapshot failed: {source}")
            }
            Self::RouteStorageAllocation { requested } => {
                write!(formatter, "could not reserve {requested} DMO model routes")
            }
            Self::AssetExpiredDuringBuild { position, asset_id } => write!(
                formatter,
                "resident DMO asset {position} ({:#018x}) expired during catalog construction",
                asset_id.0
            ),
            Self::MissingRecordSource(position) => write!(
                formatter,
                "DMO operation {} in asset {} has no retained record source",
                position.operation_index, position.asset_index
            ),
            Self::WrongRecordSourceKind(position) => write!(
                formatter,
                "DMO operation {} in asset {} has the wrong retained record kind",
                position.operation_index, position.asset_index
            ),
            Self::ShortModelRecord { position, actual } => write!(
                formatter,
                "DMO operation {} in asset {} has a {actual}-byte record",
                position.operation_index, position.asset_index
            ),
            Self::PrimaryGuidMismatch {
                position,
                operation,
                record,
            } => write!(
                formatter,
                "DMO operation {} in asset {} primary GUID {operation:#018x} differs from record {record:#018x}",
                position.operation_index, position.asset_index
            ),
            Self::LocalGuidMismatch {
                position,
                operation,
                record,
            } => write!(
                formatter,
                "DMO operation {} in asset {} local GUID {operation:#018x} differs from record {record:#018x}",
                position.operation_index, position.asset_index
            ),
            Self::MissingCollisionModel { position, model_id } => write!(
                formatter,
                "DMO model {model_id:#018x} at asset {}/operation {} has no collision model",
                position.asset_index, position.operation_index
            ),
            Self::NonLocalCollisionModel {
                position,
                object_id,
                section,
            } => write!(
                formatter,
                "DMO operation {} in asset {} names collision model {:#010x} in section {section}",
                position.operation_index, position.asset_index, object_id.0
            ),
            Self::MissingCollisionModelObject {
                position,
                object_id,
            } => write!(
                formatter,
                "DMO operation {} in asset {} names missing collision model {:#010x}",
                position.operation_index, position.asset_index, object_id.0
            ),
            Self::WrongCollisionModelObjectType {
                position,
                object_id,
                actual,
            } => write!(
                formatter,
                "DMO operation {} in asset {} collision object {:#010x} has type {actual:#010x}",
                position.operation_index, position.asset_index, object_id.0
            ),
            Self::MissingProjectedCollisionModel {
                position,
                dictionary_index,
            } => write!(
                formatter,
                "DMO operation {} in asset {} has no projected collision model for dictionary {dictionary_index}",
                position.operation_index, position.asset_index
            ),
            Self::DuplicateProjectedCollisionModel {
                position,
                dictionary_index,
            } => write!(
                formatter,
                "DMO operation {} in asset {} has duplicate projected collision models for dictionary {dictionary_index}",
                position.operation_index, position.asset_index
            ),
            Self::MissingAssembly {
                position,
                dictionary_index,
            } => write!(
                formatter,
                "DMO operation {} in asset {} collision model {dictionary_index} has no Assembly",
                position.operation_index, position.asset_index
            ),
            Self::NonLocalAssembly {
                position,
                object_id,
                section,
            } => write!(
                formatter,
                "DMO operation {} in asset {} names Assembly {:#010x} in section {section}",
                position.operation_index, position.asset_index, object_id.0
            ),
            Self::MissingAssemblyTemplate {
                position,
                object_id,
            } => write!(
                formatter,
                "DMO operation {} in asset {} names missing Assembly template {:#010x}",
                position.operation_index, position.asset_index, object_id.0
            ),
            Self::DuplicateAssemblyTemplate {
                position,
                object_id,
                first_ordinal,
                second_ordinal,
            } => write!(
                formatter,
                "DMO operation {} in asset {} names Assembly {:#010x} at template ordinals {first_ordinal} and {second_ordinal}",
                position.operation_index, position.asset_index, object_id.0
            ),
            Self::DuplicateModelId {
                model_id,
                first,
                second,
            } => write!(
                formatter,
                "DMO model {model_id:#018x} occurs first at asset {}/operation {} and again at asset {}/operation {}",
                first.asset_index,
                first.operation_index,
                second.asset_index,
                second.operation_index
            ),
            Self::ModelCapacityExceeded { capacity, position } => write!(
                formatter,
                "DMO model capacity {capacity} is exceeded at asset {}/operation {}",
                position.asset_index, position.operation_index
            ),
        }
    }
}

impl Error for DmoBankResidentModelCatalogError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ResidentSnapshot(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct DmoBankResidentModelRoute {
    position: DmoBankResidentModelRoutePosition,
    primary_guid: u64,
    model_id: u64,
    collision_model_object_id: ObjectId,
    assembly_object_id: ObjectId,
    asset: Weak<DmoBankProcessedArenaAsset>,
}

/// Opaque strong route minted only by one still-resident exact catalog row.
pub(crate) struct DmoBankResidentStrongModelSource {
    asset: Arc<DmoBankProcessedArenaAsset>,
    assembly_object_id: ObjectId,
}

impl DmoBankResidentStrongModelSource {
    pub(super) fn try_bind_part_owner(
        self,
        backend: &SourceBackend,
    ) -> Result<Arc<SimulationDmoAssemblyPartOwner>, SimulationDmoAssemblyPartOwnerError> {
        let Self {
            asset,
            assembly_object_id,
        } = self;
        SimulationDmoAssemblyPartOwner::try_new_from_validated_route(
            asset,
            assembly_object_id,
            backend,
        )
    }
}

#[cfg(test)]
impl DmoBankResidentStrongModelSource {
    pub(super) fn from_parts_for_test(
        _model_id: u64,
        asset: Arc<DmoBankProcessedArenaAsset>,
        assembly_object_id: ObjectId,
    ) -> Self {
        Self {
            asset,
            assembly_object_id,
        }
    }
}

/// Resolution refusal for one already-sealed weak catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DmoBankResidentModelResolveError {
    ResidentExpired,
    ResidentAccess(OwnedResidentCollectionError),
    MissingModel,
    RegisteredWinnerMismatch,
    AssetExpired,
}

impl fmt::Display for DmoBankResidentModelResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResidentExpired => formatter.write_str("resident DMO model catalog expired"),
            Self::ResidentAccess(source) => {
                write!(
                    formatter,
                    "resident DMO model catalog access failed: {source}"
                )
            }
            Self::MissingModel => formatter.write_str("DMO model is absent from the catalog"),
            Self::RegisteredWinnerMismatch => {
                formatter.write_str("registered DMO model winner differs from catalog route")
            }
            Self::AssetExpired => formatter.write_str("DMO model lost its exact resident asset"),
        }
    }
}

impl Error for DmoBankResidentModelResolveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ResidentAccess(source) => Some(source),
            _ => None,
        }
    }
}

/// Immutable, weak catalog for one exact resident main-DMO collection.
///
/// The reference model table keeps the first duplicate registration. This
/// safe-host catalog instead rejects duplicate local-GUID keys before
/// publication so a malformed bank cannot silently choose an owner.
pub struct DmoBankResidentModelCatalog {
    collection: OwnedResidentCollectionView<DmoBankProcessedArenaAsset>,
    asset_count: usize,
    routes: Vec<DmoBankResidentModelRoute>,
    signature: u64,
}

impl fmt::Debug for DmoBankResidentModelCatalog {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DmoBankResidentModelCatalog")
            .field("asset_count", &self.asset_count)
            .field("route_count", &self.routes.len())
            .field("resident", &self.collection.is_alive())
            .finish_non_exhaustive()
    }
}

impl DmoBankResidentModelCatalog {
    /// Validates all weak resident assets and publishes no partial catalog.
    /// Structural record lookup establishes the raw local-GUID key first;
    /// duplicate-key and 256-row capacity refusals therefore precede deeper
    /// identity, collision-model, and Assembly validation for that row.
    ///
    /// # Errors
    ///
    /// Returns a typed refusal if the resident snapshot cannot be acquired or
    /// any ordered model route disagrees with its retained source objects.
    pub fn try_new(
        collection: &OwnedResidentCollection<DmoBankProcessedArenaAsset>,
    ) -> Result<Self, DmoBankResidentModelCatalogError> {
        let assets = collection
            .weak_assets_in_order()
            .map_err(DmoBankResidentModelCatalogError::ResidentSnapshot)?;
        let asset_count = assets.len();
        let mut routes: Vec<DmoBankResidentModelRoute> = Vec::new();
        routes
            .try_reserve_exact(DMO_MODEL_ROUTE_CAPACITY)
            .map_err(
                |_| DmoBankResidentModelCatalogError::RouteStorageAllocation {
                    requested: DMO_MODEL_ROUTE_CAPACITY,
                },
            )?;

        for (asset_index, weak_asset) in assets.iter().enumerate() {
            let asset_id = weak_asset.id();
            let asset = weak_asset.weak().upgrade().ok_or(
                DmoBankResidentModelCatalogError::AssetExpiredDuringBuild {
                    position: asset_index,
                    asset_id,
                },
            )?;
            for (operation_index, operation) in asset.plan().operations().iter().enumerate() {
                let SimulationNormalOperation::DmoInstance {
                    asset_id: primary_guid,
                    local_guid,
                    ..
                } = *operation
                else {
                    continue;
                };
                let position = DmoBankResidentModelRoutePosition {
                    asset_index,
                    operation_index,
                    asset_id,
                };
                let record_model_id = validate_route_key(&asset, position)?;
                if let Some(first) = routes.iter().find(|row| row.model_id == record_model_id) {
                    return Err(DmoBankResidentModelCatalogError::DuplicateModelId {
                        model_id: record_model_id,
                        first: first.position,
                        second: position,
                    });
                }
                if routes.len() == DMO_MODEL_ROUTE_CAPACITY {
                    return Err(DmoBankResidentModelCatalogError::ModelCapacityExceeded {
                        capacity: DMO_MODEL_ROUTE_CAPACITY,
                        position,
                    });
                }
                let route = validate_route(&asset, position, primary_guid, local_guid)?;
                routes.push(route);
            }
        }

        let signature = route_signature(&routes);
        Ok(Self {
            collection: collection.weak_view(),
            asset_count,
            routes,
            signature,
        })
    }

    #[must_use]
    pub const fn asset_count(&self) -> usize {
        self.asset_count
    }

    #[must_use]
    pub fn model_count(&self) -> usize {
        self.routes.len()
    }

    #[must_use]
    pub const fn signature(&self) -> u64 {
        self.signature
    }

    #[must_use]
    pub fn is_resident(&self) -> bool {
        self.collection.is_alive()
    }

    pub(super) fn try_resolve_current_winner(
        &self,
        winner: &SimulationDmoModelCurrentWinner<'_>,
    ) -> Result<DmoBankResidentStrongModelSource, DmoBankResidentModelResolveError> {
        if !self.collection.is_alive() {
            return Err(DmoBankResidentModelResolveError::ResidentExpired);
        }
        let route = self
            .routes
            .iter()
            .find(|route| winner.matches_key(route.model_id))
            .ok_or(DmoBankResidentModelResolveError::MissingModel)?;
        if !winner.matches_catalog_route(&route.asset, route.position.operation_index) {
            return Err(DmoBankResidentModelResolveError::RegisteredWinnerMismatch);
        }
        let asset = self
            .collection
            .try_upgrade_exact_asset(route.position.asset_id, &route.asset)
            .map_err(|source| match source {
                OwnedResidentCollectionError::NotAlive { .. } => {
                    DmoBankResidentModelResolveError::ResidentExpired
                }
                source => DmoBankResidentModelResolveError::ResidentAccess(source),
            })?
            .ok_or(DmoBankResidentModelResolveError::AssetExpired)?;
        Ok(DmoBankResidentStrongModelSource {
            asset,
            assembly_object_id: route.assembly_object_id,
        })
    }
}

fn validate_route_key(
    asset: &DmoBankProcessedArenaAsset,
    position: DmoBankResidentModelRoutePosition,
) -> Result<u64, DmoBankResidentModelCatalogError> {
    let source = asset.plan().record_source(position.operation_index).ok_or(
        DmoBankResidentModelCatalogError::MissingRecordSource(position),
    )?;
    if source.kind() != SimulationRecordSourceKind::DmoInstance {
        return Err(DmoBankResidentModelCatalogError::WrongRecordSourceKind(
            position,
        ));
    }
    let bytes = source.bytes();
    if bytes.len() < DMO_MODEL_RECORD_MINIMUM_BYTES {
        return Err(DmoBankResidentModelCatalogError::ShortModelRecord {
            position,
            actual: bytes.len(),
        });
    }
    Ok(read_u64(bytes, DMO_MODEL_LOCAL_GUID_OFFSET))
}

#[allow(
    clippy::too_many_lines,
    reason = "one side-effect-free ordered validator preserves the exact refusal precedence"
)]
fn validate_route(
    asset: &Arc<DmoBankProcessedArenaAsset>,
    position: DmoBankResidentModelRoutePosition,
    primary_guid: u64,
    model_id: u64,
) -> Result<DmoBankResidentModelRoute, DmoBankResidentModelCatalogError> {
    let source = asset.plan().record_source(position.operation_index).ok_or(
        DmoBankResidentModelCatalogError::MissingRecordSource(position),
    )?;
    if source.kind() != SimulationRecordSourceKind::DmoInstance {
        return Err(DmoBankResidentModelCatalogError::WrongRecordSourceKind(
            position,
        ));
    }
    let bytes = source.bytes();
    if bytes.len() < DMO_MODEL_RECORD_MINIMUM_BYTES {
        return Err(DmoBankResidentModelCatalogError::ShortModelRecord {
            position,
            actual: bytes.len(),
        });
    }
    let record_primary = read_u64(bytes, DMO_MODEL_PRIMARY_GUID_OFFSET);
    if record_primary != primary_guid {
        return Err(DmoBankResidentModelCatalogError::PrimaryGuidMismatch {
            position,
            operation: primary_guid,
            record: record_primary,
        });
    }
    let record_local = read_u64(bytes, DMO_MODEL_LOCAL_GUID_OFFSET);
    if record_local != model_id {
        return Err(DmoBankResidentModelCatalogError::LocalGuidMismatch {
            position,
            operation: model_id,
            record: record_local,
        });
    }
    let collision_model_object_id = ObjectId(read_u32(bytes, DMO_MODEL_COLLISION_OBJECT_OFFSET));
    if collision_model_object_id.0 == 0 {
        return Err(DmoBankResidentModelCatalogError::MissingCollisionModel { position, model_id });
    }
    let section = collision_model_object_id.section_index();
    if section != 0 {
        return Err(DmoBankResidentModelCatalogError::NonLocalCollisionModel {
            position,
            object_id: collision_model_object_id,
            section,
        });
    }
    let dictionary_index = collision_model_object_id.entry_index();
    let dictionary_entry = asset
        .plan()
        .static_plan()
        .asset()
        .decoded
        .arena
        .dictionary
        .get(usize::try_from(dictionary_index).expect("u32 dictionary index fits usize"))
        .ok_or(
            DmoBankResidentModelCatalogError::MissingCollisionModelObject {
                position,
                object_id: collision_model_object_id,
            },
        )?;
    if dictionary_entry.type_id != C_MODEL_DATA_TYPE_ID {
        return Err(
            DmoBankResidentModelCatalogError::WrongCollisionModelObjectType {
                position,
                object_id: collision_model_object_id,
                actual: dictionary_entry.type_id,
            },
        );
    }
    let mut projected = asset
        .plan()
        .static_plan()
        .asset()
        .collision
        .collision_models
        .iter()
        .filter(|model| model.dictionary_index == dictionary_index);
    let model = projected.next().ok_or(
        DmoBankResidentModelCatalogError::MissingProjectedCollisionModel {
            position,
            dictionary_index,
        },
    )?;
    if projected.next().is_some() {
        return Err(
            DmoBankResidentModelCatalogError::DuplicateProjectedCollisionModel {
                position,
                dictionary_index,
            },
        );
    }
    let assembly_object_id = model.assembly_object_id.map(ObjectId).ok_or(
        DmoBankResidentModelCatalogError::MissingAssembly {
            position,
            dictionary_index,
        },
    )?;
    asset
        .try_serialised_assembly_by_object_id(assembly_object_id)
        .map_err(|source| match source {
            DmoSerialisedAssemblySelectError::NonLocal { object_id, section } => {
                DmoBankResidentModelCatalogError::NonLocalAssembly {
                    position,
                    object_id,
                    section,
                }
            }
            DmoSerialisedAssemblySelectError::Missing { object_id } => {
                DmoBankResidentModelCatalogError::MissingAssemblyTemplate {
                    position,
                    object_id,
                }
            }
            DmoSerialisedAssemblySelectError::Duplicate {
                object_id,
                first_ordinal,
                second_ordinal,
            } => DmoBankResidentModelCatalogError::DuplicateAssemblyTemplate {
                position,
                object_id,
                first_ordinal,
                second_ordinal,
            },
        })?;
    Ok(DmoBankResidentModelRoute {
        position,
        primary_guid,
        model_id,
        collision_model_object_id,
        assembly_object_id,
        asset: Arc::downgrade(asset),
    })
}

fn route_signature(routes: &[DmoBankResidentModelRoute]) -> u64 {
    let mut hash = FNV_OFFSET;
    hash_bytes(&mut hash, b"SKATE2/DMO/RESIDENT_MODEL_ROUTES\0V1\0");
    for route in routes {
        hash_u64(&mut hash, route.position.asset_id.0);
        hash_u64(
            &mut hash,
            u64::try_from(route.position.asset_index).expect("asset ordinal fits u64"),
        );
        hash_u64(
            &mut hash,
            u64::try_from(route.position.operation_index).expect("operation ordinal fits u64"),
        );
        hash_u64(&mut hash, route.primary_guid);
        hash_u64(&mut hash, route.model_id);
        hash_u32(&mut hash, route.collision_model_object_id.0);
        hash_u32(&mut hash, route.assembly_object_id.0);
    }
    hash
}

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

fn hash_u32(hash: &mut u64, value: u32) {
    hash_bytes(hash, &value.to_be_bytes());
}

fn hash_u64(hash: &mut u64, value: u64) {
    hash_bytes(hash, &value.to_be_bytes());
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("validated record span"),
    )
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_be_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("validated record span"),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        collection_load_pump::LoadedAssetCollection,
        collection_resident_streamer::ResidentCollectionStorage,
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        dynamic_assembly_session::DynamicAssemblySessionCreateError,
        owned_resident_collection::{
            OwnedResidentCollection, OwnedResidentCollectionStorage, ResidentAssetIdentity,
        },
        serialised_assembly_template::{
            SerialisedAssemblyPartInertiaSource, SerialisedAssemblyTemplate,
            SerialisedAssemblyTemplatePart,
        },
        simulation_assembly_backend::{
            SerialisedAssemblyPartAttachmentBindError, SerialisedAssemblySimulationBindError,
        },
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_dmo_arena_plan::SimulationDmoArenaPlan,
        simulation_dmo_assembly_part_session::{
            SimulationDmoAssemblyPartAttachError, SimulationDmoLocatorRegistryInitError,
            SimulationDmoModelRegistrationSource, SimulationDmoModelRegistry,
            SimulationDmoModelRegistryInitError, SimulationDmoRegisteredLocatorOccurrenceError,
            SimulationDmoRegistrationManager, SimulationDmoRegistrationManagerInitError,
            SimulationDmoRegistrationManagerResolveError, SimulationDmoResolvedLocatorPartOwner,
            SimulationDmoResolvedLocatorPartOwnerError,
            SimulationDmoResolvedLocatorPartSessionCreateError,
        },
        simulation_normal_arena_plan::SimulationNormalArenaPlan,
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        simulation_xenon_address_image::{
            SimulationArenaXenonAddressBinding, SimulationXenonAddressImage,
        },
        world::{CollisionArena, CollisionModel},
    };
    use skate2_formats::{
        collision::{AssemblyPartInertia, AssemblyPartMaterial},
        pegasus::{DMO_DATA_TYPE_ID, TABLE_OF_CONTENTS_TYPE_ID},
        rw4::{Arena, DictionaryEntry, FileHeader, ResourceDescriptor},
        stream::{ArenaResources, BaseResourceDescriptor},
    };

    #[derive(Clone, Copy)]
    struct RouteFixture {
        operation_primary: u64,
        operation_local: u64,
        record_primary: u64,
        record_local: u64,
        collision_object: ObjectId,
        source_kind: Option<SimulationRecordSourceKind>,
        record_len: usize,
    }

    impl RouteFixture {
        fn valid(primary: u64, local: u64, collision_index: u32) -> Self {
            Self {
                operation_primary: primary,
                operation_local: local,
                record_primary: primary,
                record_local: local,
                collision_object: ObjectId(collision_index),
                source_kind: Some(SimulationRecordSourceKind::DmoInstance),
                record_len: 0xa0,
            }
        }
    }

    struct AssetFixture {
        asset_id: u64,
        routes: Vec<RouteFixture>,
        dictionary_types: Vec<u32>,
        collision_models: Vec<CollisionModel>,
        assembly_ids: Vec<ObjectId>,
        part_inertia_offsets: Vec<Option<u32>>,
    }

    type LocatorRecord = (u64, u64, u64);
    type LocatorGroup<'a> = (u64, &'a [LocatorRecord]);

    type TestCollection = OwnedResidentCollection<DmoBankProcessedArenaAsset>;
    type TestStorage = OwnedResidentCollectionStorage<
        DmoBankProcessedArenaAsset,
        fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
        OwnedResidentCollectionError,
    >;

    fn identity_error(error: OwnedResidentCollectionError) -> OwnedResidentCollectionError {
        error
    }

    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ];

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn locator_dmo_data(records: &[(u64, u64, u64)]) -> Vec<u8> {
        let records_end = 20 + records.len() * 128;
        let mut bytes = vec![0; records_end + records.len() * 2 + 1];
        put_u32(&mut bytes, 0, DMO_DATA_TYPE_ID);
        put_u32(&mut bytes, 4, u32::try_from(records.len()).unwrap());
        put_u32(&mut bytes, 12, 20);
        put_u32(&mut bytes, 16, u32::try_from(records_end).unwrap());
        for (index, &(guid, local_guid, model_key)) in records.iter().enumerate() {
            let start = 20 + index * 128;
            put_u64(&mut bytes, start + 0x60, guid);
            put_u64(&mut bytes, start + 0x68, local_guid);
            put_u64(&mut bytes, start + 0x70, model_key);
            let name_offset = records_end + index * 2;
            put_u32(
                &mut bytes,
                start + 0x7c,
                u32::try_from(name_offset).unwrap(),
            );
            bytes[name_offset] = b'x';
        }
        bytes
    }

    fn locator_toc(entries: &[(u64, ObjectId)]) -> Vec<u8> {
        let names = 20 + entries.len() * 24;
        let mut bytes = vec![0; names + 1];
        put_u32(&mut bytes, 0, u32::try_from(entries.len()).unwrap());
        put_u32(&mut bytes, 4, 20);
        put_u32(&mut bytes, 8, u32::try_from(names).unwrap());
        put_u32(&mut bytes, 16, u32::try_from(names).unwrap());
        for (index, &(container_id, object_id)) in entries.iter().enumerate() {
            let offset = 20 + index * 24;
            put_u32(&mut bytes, offset, u32::try_from(names).unwrap());
            put_u64(&mut bytes, offset + 8, container_id);
            put_u32(&mut bytes, offset + 16, DMO_DATA_TYPE_ID);
            put_u32(&mut bytes, offset + 20, object_id.0);
        }
        bytes
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one coherent parsed-locator fixture preserves object and TOC identity"
    )]
    fn locator_plan(groups: &[LocatorGroup<'_>]) -> SimulationDmoArenaPlan {
        placed_locator_plan(groups, [0.0; 16])
    }

    #[allow(
        clippy::too_many_lines,
        reason = "shared complete parsed-locator fixture"
    )]
    pub(crate) fn placed_locator_plan(
        groups: &[LocatorGroup<'_>],
        transform: [f32; 16],
    ) -> SimulationDmoArenaPlan {
        let mut resource_zero = Vec::new();
        let mut dictionary = Vec::new();
        let mut toc_entries = Vec::new();
        for &(container_id, records) in groups {
            let mut object = locator_dmo_data(records);
            for index in 0..records.len() {
                for (lane, value) in transform.iter().enumerate() {
                    put_u32(&mut object, 20 + index * 128 + lane * 4, value.to_bits());
                }
            }
            let start = resource_zero.len();
            resource_zero.extend_from_slice(&object);
            let end = resource_zero.len();
            let object_id = ObjectId(u32::try_from(dictionary.len()).unwrap());
            dictionary.push(DictionaryEntry {
                resource_index: 0,
                object_offset: u32::try_from(start).unwrap(),
                relocation_offset: 0,
                size: u32::try_from(object.len()).unwrap(),
                alignment: 1,
                type_index: 0,
                type_id: DMO_DATA_TYPE_ID,
                object_range: std::ops::Range { start, end },
            });
            toc_entries.push((container_id, object_id));
        }
        let toc = locator_toc(&toc_entries);
        let toc_start = resource_zero.len();
        resource_zero.extend_from_slice(&toc);
        let toc_end = resource_zero.len();
        dictionary.push(DictionaryEntry {
            resource_index: 0,
            object_offset: u32::try_from(toc_start).unwrap(),
            relocation_offset: 0,
            size: u32::try_from(toc.len()).unwrap(),
            alignment: 1,
            type_index: 0,
            type_id: TABLE_OF_CONTENTS_TYPE_ID,
            object_range: std::ops::Range {
                start: toc_start,
                end: toc_end,
            },
        });
        let resources = [
            resource_zero,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];
        let resource_descriptor = std::array::from_fn(|index| ResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: 1,
        });
        let arena = Arena {
            file_header: FileHeader {
                magic: *b"\x89RW4xb2\0\r\n\x1a\n",
                is_big_endian: true,
                pointer_size_bits: 32,
                pointer_alignment: 4,
                unused: 0,
                major_version: *b"454\0",
                minor_version: *b"000\0",
                build_number: 0,
            },
            id: 0x1234_5678,
            entry_count: u32::try_from(dictionary.len()).unwrap(),
            used_entry_count: u32::try_from(dictionary.len()).unwrap(),
            alignment: 1,
            virtual_size: resource_descriptor[0].size,
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor,
            resources_used: resource_descriptor,
            resource_offsets: [u32::MAX; 5],
            arena_group_offset: 0,
            dictionary,
        };
        let asset_resource_descriptors = std::array::from_fn(|index| BaseResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: 1,
        });
        let collision = SimulationCollisionAsset {
            decoded: DecodedArenaAsset {
                asset_id: 0x1357_9bdf_2468_ace0,
                asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
                asset_resource_descriptors,
                resources: ArenaResources { resources },
                arena,
            },
            collision: CollisionArena {
                id: 0x1234_5678,
                grind_assets: Vec::new(),
                aggregate_roots: Vec::new(),
                primitive_sets: Vec::new(),
                simple_primitive_sets: Vec::new(),
                meshes: Vec::new(),
                collision_models: Vec::new(),
                instances: Vec::new(),
            },
        };
        let static_plan = Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
            collision,
            Vec::new(),
        ));
        let normal_plan =
            Arc::new(SimulationNormalArenaPlan::from_static_plan(static_plan).unwrap());
        SimulationDmoArenaPlan::from_normal_plan(normal_plan).unwrap()
    }

    fn resident_collection(
        assets: Vec<DmoBankProcessedArenaAsset>,
    ) -> (TestStorage, TestCollection) {
        let mut storage = TestStorage::new(identity_error);
        let reserve = i32::try_from(assets.len()).unwrap();
        let collection = storage.create_empty(reserve).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        for asset in assets {
            writer.add_asset(asset);
        }
        drop(writer);
        (storage, collection)
    }

    fn valid_fixture(asset_id: u64, identities: &[(u64, u64)]) -> AssetFixture {
        let dictionary_types = std::iter::once(0)
            .chain(std::iter::repeat_n(C_MODEL_DATA_TYPE_ID, identities.len()))
            .collect();
        let routes = identities
            .iter()
            .enumerate()
            .map(|(ordinal, &(primary, local))| {
                RouteFixture::valid(primary, local, u32::try_from(ordinal + 1).unwrap())
            })
            .collect();
        let collision_models = identities
            .iter()
            .enumerate()
            .map(|(ordinal, _)| CollisionModel {
                dictionary_index: u32::try_from(ordinal + 1).unwrap(),
                bounding_root: None,
                collision_roots: Vec::new(),
                assembly_object_id: Some(u32::try_from(ordinal + 16).unwrap()),
            })
            .collect();
        let assembly_ids = identities
            .iter()
            .enumerate()
            .map(|(ordinal, _)| ObjectId(u32::try_from(ordinal + 16).unwrap()))
            .collect();
        AssetFixture {
            asset_id,
            routes,
            dictionary_types,
            collision_models,
            assembly_ids,
            part_inertia_offsets: vec![None; identities.len()],
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one coherent arena owner fixture keeps every positional source correlated"
    )]
    fn build_asset(fixture: AssetFixture) -> DmoBankProcessedArenaAsset {
        const RECORD_STRIDE: usize = 0x100;
        let mut resource_zero = vec![0; fixture.routes.len().saturating_mul(RECORD_STRIDE)];
        let mut operations = Vec::with_capacity(fixture.routes.len());
        let mut sources = Vec::with_capacity(fixture.routes.len());
        for (ordinal, route) in fixture.routes.iter().copied().enumerate() {
            let start = ordinal * RECORD_STRIDE;
            if route.record_len >= DMO_MODEL_RECORD_MINIMUM_BYTES {
                resource_zero[start + DMO_MODEL_PRIMARY_GUID_OFFSET
                    ..start + DMO_MODEL_PRIMARY_GUID_OFFSET + 8]
                    .copy_from_slice(&route.record_primary.to_be_bytes());
                resource_zero
                    [start + DMO_MODEL_LOCAL_GUID_OFFSET..start + DMO_MODEL_LOCAL_GUID_OFFSET + 8]
                    .copy_from_slice(&route.record_local.to_be_bytes());
                resource_zero[start + DMO_MODEL_COLLISION_OBJECT_OFFSET
                    ..start + DMO_MODEL_COLLISION_OBJECT_OFFSET + 4]
                    .copy_from_slice(&route.collision_object.0.to_be_bytes());
            }
            operations.push(SimulationNormalOperation::DmoInstance {
                asset_id: route.operation_primary,
                local_guid: route.operation_local,
                source_toc_index: 0,
                source_dictionary_index: u32::try_from(ordinal).unwrap(),
                source_inline_index: 0,
                source_record_offset: u32::try_from(start).unwrap(),
                raw_object_id: u32::try_from(ordinal).unwrap(),
            });
            sources.push(
                route
                    .source_kind
                    .map(|kind| (kind, 0, start, route.record_len)),
            );
        }
        let resources = [
            resource_zero,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];
        let resource_descriptor = std::array::from_fn(|index| ResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: 0x10,
        });
        let dictionary = fixture
            .dictionary_types
            .into_iter()
            .enumerate()
            .map(|(index, type_id)| DictionaryEntry {
                resource_index: 0,
                object_offset: 0,
                relocation_offset: 0,
                size: 0,
                alignment: 1,
                type_index: u32::try_from(index).unwrap(),
                type_id,
                object_range: 0..0,
            })
            .collect::<Vec<_>>();
        let arena = Arena {
            file_header: FileHeader {
                magic: *b"\x89RW4xb2\0\r\n\x1a\n",
                is_big_endian: true,
                pointer_size_bits: 32,
                pointer_alignment: 4,
                unused: 0,
                major_version: *b"454\0",
                minor_version: *b"000\0",
                build_number: 0,
            },
            id: 0x8646_a1f7,
            entry_count: u32::try_from(dictionary.len()).unwrap(),
            used_entry_count: u32::try_from(dictionary.len()).unwrap(),
            alignment: 16,
            virtual_size: resource_descriptor[0].size,
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor,
            resources_used: resource_descriptor,
            resource_offsets: [u32::MAX; 5],
            arena_group_offset: 0,
            dictionary,
        };
        let asset_resource_descriptors = std::array::from_fn(|index| BaseResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: 0x10,
        });
        let decoded = DecodedArenaAsset {
            asset_id: fixture.asset_id,
            asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
            asset_resource_descriptors,
            resources: ArenaResources { resources },
            arena,
        };
        let collision = SimulationCollisionAsset {
            decoded,
            collision: CollisionArena {
                id: 0x8646_a1f7,
                grind_assets: Vec::new(),
                aggregate_roots: Vec::new(),
                primitive_sets: Vec::new(),
                simple_primitive_sets: Vec::new(),
                meshes: Vec::new(),
                collision_models: fixture.collision_models,
                instances: Vec::new(),
            },
        };
        let static_plan = Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
            collision,
            Vec::new(),
        ));
        let owner = Arc::new(
            SimulationNormalArenaPlan::from_parts_with_record_sources_for_test(
                static_plan,
                operations,
                sources,
            ),
        );
        let templates = fixture
            .assembly_ids
            .into_iter()
            .enumerate()
            .map(|(ordinal, object_id)| {
                let inertia_offset = fixture.part_inertia_offsets.get(ordinal).copied().flatten();
                let parts = inertia_offset
                    .map(|inertia_offset| SerialisedAssemblyTemplatePart {
                        volume_object_id: ObjectId(1),
                        root_index: 0,
                        inertia_source: SerialisedAssemblyPartInertiaSource::from_parts_for_test(
                            Arc::clone(&owner),
                            0,
                            inertia_offset,
                        ),
                        inertia: AssemblyPartInertia {
                            inverse_tensor: [1.0, 2.0, 3.0, 4.0],
                            inverse_mass: 0.25,
                            spherical: 1.0,
                            max_velocity: 10.0,
                            max_omega: 11.0,
                            linear_drag: 0.1,
                            angular_drag: 0.2,
                        },
                        inverse_body_transform: [
                            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
                            0.0, 1.0,
                        ],
                        material: AssemblyPartMaterial {
                            static_friction: 1.0,
                            dynamic_friction: 0.5,
                            restitution: 0.25,
                        },
                        has_inverse_body_transform: 0,
                        is_static_body: 0,
                    })
                    .into_iter()
                    .collect();
                SerialisedAssemblyTemplate::from_parts_and_constraint_counts_for_test(
                    Arc::clone(&owner),
                    object_id,
                    0,
                    0,
                    0,
                    0,
                    parts,
                )
            })
            .collect();
        DmoBankProcessedArenaAsset::from_parts_for_test(owner, templates)
    }

    fn source_binding(
        owner: Arc<SimulationNormalArenaPlan>,
        resource_zero_base: u32,
    ) -> Arc<SimulationArenaXenonAddressBinding> {
        Arc::new(
            SimulationArenaXenonAddressBinding::try_new(owner, [resource_zero_base, 0, 0, 0, 0])
                .unwrap(),
        )
    }

    fn source_backend(
        capacity: usize,
        routes: Vec<(
            Arc<SimulationNormalArenaPlan>,
            Arc<SimulationArenaXenonAddressBinding>,
        )>,
    ) -> SourceBackend {
        let image = SimulationXenonAddressImage::try_new(
            0x1000_0080,
            u32::try_from(capacity).unwrap(),
            0,
            0,
        )
        .unwrap();
        let mut backend = SourceBackend::try_new(capacity, 0, 0).unwrap();
        backend.try_bind_source_arenas(image, routes).unwrap();
        backend
    }

    fn register_route(
        models: &mut SimulationDmoModelRegistry,
        asset: &Arc<DmoBankProcessedArenaAsset>,
        operation_index: usize,
    ) {
        let source =
            SimulationDmoModelRegistrationSource::try_prepare(Arc::clone(asset), operation_index)
                .unwrap();
        models.register(source).unwrap();
    }

    fn models_for_catalog(catalog: &DmoBankResidentModelCatalog) -> SimulationDmoModelRegistry {
        let mut models = SimulationDmoModelRegistry::try_new().unwrap();
        for route in &catalog.routes {
            register_route(
                &mut models,
                &route.asset.upgrade().unwrap(),
                route.position.operation_index,
            );
        }
        models
    }

    fn winner_for_key(
        models: &SimulationDmoModelRegistry,
        model_key: u64,
    ) -> SimulationDmoModelCurrentWinner<'_> {
        let plan = locator_plan(&[(0x1234_5678_9abc_def0, &[(1, 2, model_key)])]);
        let occurrence = plan.try_locator_occurrence(0, 0).unwrap();
        let (_, winner) = occurrence
            .try_select_current_model(models.resolver())
            .unwrap();
        winner
    }

    fn one_route_error(mutate: impl FnOnce(&mut AssetFixture)) -> DmoBankResidentModelCatalogError {
        let mut fixture = valid_fixture(0x100, &[(0xaaaa, 0xbbbb)]);
        mutate(&mut fixture);
        let (_storage, collection) = resident_collection(vec![build_asset(fixture)]);
        DmoBankResidentModelCatalog::try_new(&collection).unwrap_err()
    }

    #[test]
    fn catalog_preserves_physical_and_operation_order_and_uses_local_key() {
        let asset_b = build_asset(valid_fixture(0xb0, &[(0xb001, 0xb101), (0xb002, 0xb102)]));
        let asset_a = build_asset(valid_fixture(0xa0, &[(0xa001, 0xa101)]));
        let (_storage, collection) = resident_collection(vec![asset_b, asset_a]);
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();

        assert_eq!(catalog.asset_count(), 2);
        assert_eq!(catalog.model_count(), 3);
        assert!(catalog.is_resident());
        assert_eq!(catalog.routes[0].position.asset_id, CollectionId(0xb0));
        assert_eq!(catalog.routes[0].position.operation_index, 0);
        assert_eq!(catalog.routes[1].position.operation_index, 1);
        assert_eq!(catalog.routes[2].position.asset_id, CollectionId(0xa0));
        let models = models_for_catalog(&catalog);
        let winner = winner_for_key(&models, 0xb102);
        let resolved = catalog.try_resolve_current_winner(&winner).unwrap();
        assert_eq!(resolved.assembly_object_id, ObjectId(17));
        assert_eq!(resolved.asset.resident_asset_id(), CollectionId(0xb0));
    }

    #[test]
    fn record_identity_and_collision_object_gates_are_exact() {
        assert!(matches!(
            one_route_error(|fixture| fixture.routes[0].record_primary ^= 1),
            DmoBankResidentModelCatalogError::PrimaryGuidMismatch { .. }
        ));
        assert!(matches!(
            one_route_error(|fixture| fixture.routes[0].record_local ^= 1),
            DmoBankResidentModelCatalogError::LocalGuidMismatch { .. }
        ));
        assert!(matches!(
            one_route_error(|fixture| fixture.routes[0].source_kind = None),
            DmoBankResidentModelCatalogError::MissingRecordSource(_)
        ));
        assert!(matches!(
            one_route_error(|fixture| {
                fixture.routes[0].source_kind = Some(SimulationRecordSourceKind::TriggerInstance);
            }),
            DmoBankResidentModelCatalogError::WrongRecordSourceKind(_)
        ));
        assert!(matches!(
            one_route_error(|fixture| fixture.routes[0].record_len = 135),
            DmoBankResidentModelCatalogError::ShortModelRecord { actual: 135, .. }
        ));
        assert!(matches!(
            one_route_error(|fixture| fixture.routes[0].collision_object = ObjectId(0)),
            DmoBankResidentModelCatalogError::MissingCollisionModel { .. }
        ));
        assert!(matches!(
            one_route_error(|fixture| {
                fixture.routes[0].collision_object = ObjectId((1 << 22) | 1);
            }),
            DmoBankResidentModelCatalogError::NonLocalCollisionModel { section: 1, .. }
        ));
        assert!(matches!(
            one_route_error(|fixture| fixture.routes[0].collision_object = ObjectId(9)),
            DmoBankResidentModelCatalogError::MissingCollisionModelObject { .. }
        ));
        assert!(matches!(
            one_route_error(|fixture| fixture.dictionary_types[1] = 0xdead_beef),
            DmoBankResidentModelCatalogError::WrongCollisionModelObjectType {
                actual: 0xdead_beef,
                ..
            }
        ));
    }

    #[test]
    fn projected_model_and_assembly_selection_gates_are_exact() {
        assert!(matches!(
            one_route_error(|fixture| fixture.collision_models.clear()),
            DmoBankResidentModelCatalogError::MissingProjectedCollisionModel { .. }
        ));
        assert!(matches!(
            one_route_error(|fixture| {
                fixture
                    .collision_models
                    .push(fixture.collision_models[0].clone());
            }),
            DmoBankResidentModelCatalogError::DuplicateProjectedCollisionModel { .. }
        ));
        assert!(matches!(
            one_route_error(|fixture| fixture.collision_models[0].assembly_object_id = None),
            DmoBankResidentModelCatalogError::MissingAssembly { .. }
        ));
        assert!(matches!(
            one_route_error(|fixture| {
                fixture.collision_models[0].assembly_object_id = Some((1 << 22) | 16);
            }),
            DmoBankResidentModelCatalogError::NonLocalAssembly { section: 1, .. }
        ));
        assert!(matches!(
            one_route_error(|fixture| fixture.assembly_ids.clear()),
            DmoBankResidentModelCatalogError::MissingAssemblyTemplate { .. }
        ));
        assert!(matches!(
            one_route_error(|fixture| fixture.assembly_ids.push(ObjectId(16))),
            DmoBankResidentModelCatalogError::DuplicateAssemblyTemplate {
                first_ordinal: 0,
                second_ordinal: 1,
                ..
            }
        ));
    }

    #[test]
    fn duplicate_keys_report_first_and_second_physical_positions() {
        let mut intra_fixture = valid_fixture(0xa0, &[(1, 7), (2, 8)]);
        intra_fixture.routes[1].record_local = 7;
        intra_fixture.routes[1].record_primary ^= 1;
        intra_fixture.routes[1].collision_object = ObjectId(0);
        let intra = build_asset(intra_fixture);
        let (_storage, collection) = resident_collection(vec![intra]);
        assert_eq!(
            DmoBankResidentModelCatalog::try_new(&collection).unwrap_err(),
            DmoBankResidentModelCatalogError::DuplicateModelId {
                model_id: 7,
                first: DmoBankResidentModelRoutePosition {
                    asset_index: 0,
                    operation_index: 0,
                    asset_id: CollectionId(0xa0),
                },
                second: DmoBankResidentModelRoutePosition {
                    asset_index: 0,
                    operation_index: 1,
                    asset_id: CollectionId(0xa0),
                },
            }
        );

        let first = build_asset(valid_fixture(0xb0, &[(1, 9)]));
        let second = build_asset(valid_fixture(0xa0, &[(2, 9)]));
        let (_storage, collection) = resident_collection(vec![first, second]);
        assert!(matches!(
            DmoBankResidentModelCatalog::try_new(&collection),
            Err(DmoBankResidentModelCatalogError::DuplicateModelId {
                first: DmoBankResidentModelRoutePosition { asset_index: 0, .. },
                second: DmoBankResidentModelRoutePosition { asset_index: 1, .. },
                ..
            })
        ));
    }

    #[test]
    fn capacity_is_exactly_256_and_overflow_reports_next_position() {
        let identities = (0..=DMO_MODEL_ROUTE_CAPACITY)
            .map(|ordinal| {
                let value = u64::try_from(ordinal + 1).unwrap();
                (0x1000 + value, 0x2000 + value)
            })
            .collect::<Vec<_>>();
        let accepted = build_asset(valid_fixture(0xa0, &identities[..DMO_MODEL_ROUTE_CAPACITY]));
        let (_storage, collection) = resident_collection(vec![accepted]);
        assert_eq!(
            DmoBankResidentModelCatalog::try_new(&collection)
                .unwrap()
                .model_count(),
            DMO_MODEL_ROUTE_CAPACITY
        );

        let mut rejected_fixture = valid_fixture(0xa0, &identities);
        rejected_fixture.routes[DMO_MODEL_ROUTE_CAPACITY].record_primary ^= 1;
        rejected_fixture.routes[DMO_MODEL_ROUTE_CAPACITY].collision_object = ObjectId(0);
        let rejected = build_asset(rejected_fixture);
        let (_storage, collection) = resident_collection(vec![rejected]);
        assert_eq!(
            DmoBankResidentModelCatalog::try_new(&collection).unwrap_err(),
            DmoBankResidentModelCatalogError::ModelCapacityExceeded {
                capacity: DMO_MODEL_ROUTE_CAPACITY,
                position: DmoBankResidentModelRoutePosition {
                    asset_index: 0,
                    operation_index: DMO_MODEL_ROUTE_CAPACITY,
                    asset_id: CollectionId(0xa0),
                },
            }
        );
    }

    #[test]
    fn signature_is_stable_but_sensitive_to_physical_order() {
        let (_storage, first_collection) = resident_collection(vec![
            build_asset(valid_fixture(0xb0, &[(1, 11)])),
            build_asset(valid_fixture(0xa0, &[(2, 12)])),
        ]);
        let first = DmoBankResidentModelCatalog::try_new(&first_collection).unwrap();
        let repeated = DmoBankResidentModelCatalog::try_new(&first_collection).unwrap();
        assert_eq!(first.signature(), repeated.signature());

        let (_storage, reversed_collection) = resident_collection(vec![
            build_asset(valid_fixture(0xa0, &[(2, 12)])),
            build_asset(valid_fixture(0xb0, &[(1, 11)])),
        ]);
        let reversed = DmoBankResidentModelCatalog::try_new(&reversed_collection).unwrap();
        assert_ne!(first.signature(), reversed.signature());
    }

    #[test]
    fn failed_later_route_publishes_nothing_and_keeps_resident_owner_untouched() {
        let mut fixture = valid_fixture(0xa0, &[(1, 11), (2, 12)]);
        fixture.routes[1].record_local ^= 1;
        let (mut storage, collection) = resident_collection(vec![build_asset(fixture)]);
        let weak = collection.weak_assets_in_order().unwrap().remove(0);
        assert!(matches!(
            DmoBankResidentModelCatalog::try_new(&collection),
            Err(DmoBankResidentModelCatalogError::LocalGuidMismatch {
                position: DmoBankResidentModelRoutePosition {
                    operation_index: 1,
                    ..
                },
                ..
            })
        ));
        assert!(weak.weak().upgrade().is_some());
        storage.destroy(collection);
        assert!(weak.weak().upgrade().is_none());
    }

    #[test]
    fn weak_catalog_expires_without_aba_or_value_identity_substitution() {
        let (mut storage_a, collection_a) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(1, 7)]))]);
        let catalog_a = DmoBankResidentModelCatalog::try_new(&collection_a).unwrap();
        let models_a = models_for_catalog(&catalog_a);
        let winner_a = winner_for_key(&models_a, 7);
        let source_a = catalog_a.try_resolve_current_winner(&winner_a).unwrap();
        let (_storage_b, collection_b) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(1, 7)]))]);
        let catalog_b = DmoBankResidentModelCatalog::try_new(&collection_b).unwrap();
        let models_b = models_for_catalog(&catalog_b);
        let winner_b = winner_for_key(&models_b, 7);
        let source_b = catalog_b.try_resolve_current_winner(&winner_b).unwrap();
        assert!(!Arc::ptr_eq(&source_a.asset, &source_b.asset));
        drop(source_a);
        storage_a.destroy(collection_a);
        assert!(!catalog_a.is_resident());
        assert!(matches!(
            catalog_a.try_resolve_current_winner(&winner_a),
            Err(DmoBankResidentModelResolveError::ResidentExpired)
        ));
        assert!(catalog_b.is_resident());
        assert!(catalog_b.try_resolve_current_winner(&winner_b).is_ok());
    }

    #[test]
    fn same_collection_id_does_not_collapse_distinct_asset_allocations() {
        let (_storage, collection) = resident_collection(vec![
            build_asset(valid_fixture(0xa0, &[(1, 7)])),
            build_asset(valid_fixture(0xa0, &[(2, 8)])),
        ]);
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();
        let models = models_for_catalog(&catalog);
        let first = catalog
            .try_resolve_current_winner(&winner_for_key(&models, 7))
            .unwrap();
        let second = catalog
            .try_resolve_current_winner(&winner_for_key(&models, 8))
            .unwrap();
        assert!(!Arc::ptr_eq(&first.asset, &second.asset));
        assert_eq!(first.asset.resident_asset_id(), CollectionId(0xa0));
        assert_eq!(second.asset.resident_asset_id(), CollectionId(0xa0));
    }

    #[test]
    fn tombstoned_but_allocated_collection_refuses_residency_and_resolution() {
        let (_storage, collection) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(1, 7)]))]);
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();
        let models = models_for_catalog(&catalog);
        let winner = winner_for_key(&models, 7);
        assert!(catalog.routes[0].asset.upgrade().is_some());
        collection.tombstone_without_releasing_assets_for_test();
        assert!(catalog.collection.is_attached());
        assert!(!catalog.is_resident());
        assert!(matches!(
            catalog.try_resolve_current_winner(&winner),
            Err(DmoBankResidentModelResolveError::ResidentExpired)
        ));
        assert!(catalog.routes[0].asset.upgrade().is_some());
    }

    #[test]
    fn empty_bank_publishes_and_then_expires_without_assets() {
        let (mut storage, collection) = resident_collection(Vec::new());
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();
        assert_eq!(catalog.asset_count(), 0);
        assert_eq!(catalog.model_count(), 0);
        assert!(catalog.is_resident());
        let manager_asset = Arc::new(build_asset(valid_fixture(0xb0, &[(1, 1)])));
        let mut models = SimulationDmoModelRegistry::try_new().unwrap();
        register_route(&mut models, &manager_asset, 0);
        let winner = winner_for_key(&models, 1);
        storage.destroy(collection);
        assert!(!catalog.is_resident());
        assert!(matches!(
            catalog.try_resolve_current_winner(&winner),
            Err(DmoBankResidentModelResolveError::ResidentExpired)
        ));
    }

    #[test]
    fn resolved_part_owner_and_session_keep_exact_asset_alive_after_bank_teardown() {
        let (mut storage, collection) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(1, 7)]))]);
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();
        let models = models_for_catalog(&catalog);
        let winner = winner_for_key(&models, 7);
        let weak_asset = Weak::clone(&catalog.routes[0].asset);
        let image = SimulationXenonAddressImage::try_new(0x1000_0080, 0, 0, 0).unwrap();
        let mut backend = SourceBackend::try_new(0, 0, 0).unwrap();
        backend.try_bind_source_arenas(image, Vec::new()).unwrap();
        let owner = SimulationDmoAssemblyPartOwner::try_new(
            catalog.try_resolve_current_winner(&winner).unwrap(),
            &backend,
        )
        .unwrap();
        let mut session = owner.try_begin_session(Vec::new()).unwrap();
        drop(models);

        storage.destroy(collection);
        assert!(!catalog.is_resident());
        assert!(weak_asset.upgrade().is_some());
        session.attach(&mut backend, 4).unwrap();
        session.detach(&mut backend).unwrap();
        drop(owner);
        assert!(weak_asset.upgrade().is_some());
        drop(session);
        assert!(weak_asset.upgrade().is_none());
    }

    #[test]
    fn locator_bridge_uses_private_model_key_not_guid_distractors() {
        const GUID_DISTRACTOR: u64 = 0x1111_2222_3333_4444;
        const LOCAL_DISTRACTOR: u64 = 0x5555_6666_7777_8888;
        const TARGET_MODEL: u64 = 0x9999_aaaa_bbbb_cccc;
        let locator_plan = locator_plan(&[(
            0xface_cafe_dead_beef,
            &[
                (0x1010, 0x2020, 0x3030),
                (GUID_DISTRACTOR, LOCAL_DISTRACTOR, TARGET_MODEL),
            ],
        )]);
        let occurrence = locator_plan.try_locator_occurrence(0, 1).unwrap();
        let (_storage, collection) = resident_collection(vec![
            build_asset(valid_fixture(0xb0, &[(0xb001, GUID_DISTRACTOR)])),
            build_asset(valid_fixture(0xa0, &[(0xa001, LOCAL_DISTRACTOR)])),
            build_asset(valid_fixture(0xc0, &[(0xc001, TARGET_MODEL)])),
        ]);
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();
        let models = models_for_catalog(&catalog);
        let expected = catalog.routes[2].asset.upgrade().unwrap();
        let backend = source_backend(0, Vec::new());

        let resolved =
            SimulationDmoResolvedLocatorPartOwner::try_new(occurrence, &models, &catalog, &backend)
                .unwrap();
        assert!(Arc::ptr_eq(&resolved.parts.asset, &expected));
        assert_eq!(resolved.parts.assembly_object_id, ObjectId(16));
    }

    #[test]
    fn zero_model_key_and_equal_locator_records_retain_distinct_occurrences() {
        let locator_plan = locator_plan(&[(0x0102_0304_0506_0708, &[(0, 0, 0), (0, 0, 0)])]);
        let (_storage, collection) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(0x1234, 0)]))]);
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();
        let models = models_for_catalog(&catalog);
        let mut backend = source_backend(0, Vec::new());
        let first = SimulationDmoResolvedLocatorPartOwner::try_new(
            locator_plan.try_locator_occurrence(0, 0).unwrap(),
            &models,
            &catalog,
            &backend,
        )
        .unwrap();
        let second = SimulationDmoResolvedLocatorPartOwner::try_new(
            locator_plan.try_locator_occurrence(0, 1).unwrap(),
            &models,
            &catalog,
            &backend,
        )
        .unwrap();
        assert!(!first.locator.is_same_occurrence(&second.locator));

        let mut session = first.try_begin_session(Vec::new()).unwrap();
        assert!(Arc::ptr_eq(&session.owner, &first));
        assert_eq!(session.part_transforms().len(), 0);
        assert!(!session.is_attached());
        session.attach(&mut backend, 4).unwrap();
        assert!(session.is_attached());
        session.detach(&mut backend).unwrap();
        assert!(!session.is_attached());
    }

    #[test]
    fn manager_missing_catalog_missing_stale_and_aba_precede_backend_effects() {
        const MODEL: u64 = 0xfeed_face_cafe_beef;
        let locator_plan = locator_plan(&[(
            0xabcd_ef01_2345_6789,
            &[(1, 2, 0xdead), (3, 4, 0xbead), (5, 6, MODEL)],
        )]);
        let (mut storage_a, collection_a) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(7, MODEL)]))]);
        let catalog_a = DmoBankResidentModelCatalog::try_new(&collection_a).unwrap();
        let registered_a = catalog_a.routes[0].asset.upgrade().unwrap();
        let backend = source_backend(0, Vec::new());
        let mut models = SimulationDmoModelRegistry::try_new().unwrap();

        let Err(manager_missing) = SimulationDmoResolvedLocatorPartOwner::try_new(
            locator_plan.try_locator_occurrence(0, 0).unwrap(),
            &models,
            &catalog_a,
            &backend,
        ) else {
            panic!("absent manager winner unexpectedly resolved");
        };
        assert!(matches!(
            manager_missing,
            SimulationDmoResolvedLocatorPartOwnerError::ManagerMissing
        ));
        assert_eq!(format!("{manager_missing:?}"), "ManagerMissing");
        assert_eq!(
            manager_missing.to_string(),
            "DMO model has no manager winner"
        );
        assert!(Error::source(&manager_missing).is_none());
        assert_eq!(backend.last_add_failure(), None);

        let outside_catalog = Arc::new(build_asset(valid_fixture(0xb0, &[(8, 0xdead)])));
        register_route(&mut models, &outside_catalog, 0);
        assert!(matches!(
            SimulationDmoResolvedLocatorPartOwner::try_new(
                locator_plan.try_locator_occurrence(0, 0).unwrap(),
                &models,
                &catalog_a,
                &backend,
            ),
            Err(SimulationDmoResolvedLocatorPartOwnerError::CatalogMissingModel)
        ));
        register_route(&mut models, &registered_a, 0);

        storage_a.destroy(collection_a);
        let (_storage_b, collection_b) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(7, MODEL)]))]);
        let catalog_b = DmoBankResidentModelCatalog::try_new(&collection_b).unwrap();
        let registered_b = catalog_b.routes[0].asset.upgrade().unwrap();
        assert!(matches!(
            SimulationDmoResolvedLocatorPartOwner::try_new(
                locator_plan.try_locator_occurrence(0, 1).unwrap(),
                &models,
                &catalog_a,
                &backend,
            ),
            Err(SimulationDmoResolvedLocatorPartOwnerError::ManagerMissing)
        ));
        assert!(matches!(
            SimulationDmoResolvedLocatorPartOwner::try_new(
                locator_plan.try_locator_occurrence(0, 2).unwrap(),
                &models,
                &catalog_a,
                &backend,
            ),
            Err(SimulationDmoResolvedLocatorPartOwnerError::ResidentExpired)
        ));
        assert_eq!(backend.last_add_failure(), None);
        assert!(matches!(
            SimulationDmoResolvedLocatorPartOwner::try_new(
                locator_plan.try_locator_occurrence(0, 2).unwrap(),
                &models,
                &catalog_b,
                &backend,
            ),
            Err(SimulationDmoResolvedLocatorPartOwnerError::ExactWinnerMismatch)
        ));
        models.deregister(
            SimulationDmoModelRegistrationSource::try_prepare(registered_a, 0).unwrap(),
        );
        register_route(&mut models, &registered_b, 0);
        assert!(
            SimulationDmoResolvedLocatorPartOwner::try_new(
                locator_plan.try_locator_occurrence(0, 2).unwrap(),
                &models,
                &catalog_b,
                &backend,
            )
            .is_ok()
        );
        assert_eq!(backend.last_add_failure(), None);
    }

    #[test]
    fn current_manager_winner_qualifies_only_its_exact_candidate_catalog() {
        const MODEL: u64 = 0x8182_8384_dead_beef;
        let (_storage_a, collection_a) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(1, MODEL)]))]);
        let (_storage_b, collection_b) =
            resident_collection(vec![build_asset(valid_fixture(0xb0, &[(1, MODEL)]))]);
        let catalog_a = DmoBankResidentModelCatalog::try_new(&collection_a).unwrap();
        let catalog_b = DmoBankResidentModelCatalog::try_new(&collection_b).unwrap();
        let asset_a = catalog_a.routes[0].asset.upgrade().unwrap();
        let asset_b = catalog_b.routes[0].asset.upgrade().unwrap();
        let locator_plan = locator_plan(&[(0x0102_0304_0506_0708, &[(3, 4, MODEL)])]);
        let backend = source_backend(0, Vec::new());
        let mut models = SimulationDmoModelRegistry::try_new().unwrap();

        register_route(&mut models, &asset_a, 0);
        register_route(&mut models, &asset_b, 0);
        models.deregister(
            SimulationDmoModelRegistrationSource::try_prepare(Arc::clone(&asset_b), 0).unwrap(),
        );
        assert!(
            SimulationDmoResolvedLocatorPartOwner::try_new(
                locator_plan.try_locator_occurrence(0, 0).unwrap(),
                &models,
                &catalog_a,
                &backend,
            )
            .is_ok()
        );
        assert!(matches!(
            SimulationDmoResolvedLocatorPartOwner::try_new(
                locator_plan.try_locator_occurrence(0, 0).unwrap(),
                &models,
                &catalog_b,
                &backend,
            ),
            Err(SimulationDmoResolvedLocatorPartOwnerError::ExactWinnerMismatch)
        ));

        models.deregister(
            SimulationDmoModelRegistrationSource::try_prepare(Arc::clone(&asset_a), 0).unwrap(),
        );
        register_route(&mut models, &asset_b, 0);
        register_route(&mut models, &asset_a, 0);
        assert!(
            SimulationDmoResolvedLocatorPartOwner::try_new(
                locator_plan.try_locator_occurrence(0, 0).unwrap(),
                &models,
                &catalog_b,
                &backend,
            )
            .is_ok()
        );
        assert!(matches!(
            SimulationDmoResolvedLocatorPartOwner::try_new(
                locator_plan.try_locator_occurrence(0, 0).unwrap(),
                &models,
                &catalog_a,
                &backend,
            ),
            Err(SimulationDmoResolvedLocatorPartOwnerError::ExactWinnerMismatch)
        ));
        assert_eq!(backend.last_add_failure(), None);
    }

    #[test]
    fn shared_record_handle_cannot_substitute_a_different_processed_asset_wrapper() {
        const MODEL: u64 = 0x1111_2222_3333_4444;
        let (_storage, collection) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(1, MODEL)]))]);
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();
        let catalog_asset = catalog.routes[0].asset.upgrade().unwrap();
        let foreign_wrapper = Arc::new(DmoBankProcessedArenaAsset::from_parts_for_test(
            catalog_asset.plan_owner(),
            Vec::new(),
        ));
        assert!(!Arc::ptr_eq(&catalog_asset, &foreign_wrapper));
        assert!(
            catalog_asset
                .plan()
                .record_handle(0)
                .unwrap()
                .is_same_backing_record(&foreign_wrapper.plan().record_handle(0).unwrap())
        );

        let locator_plan = locator_plan(&[(0x5555_6666_7777_8888, &[(2, 3, MODEL)])]);
        let backend = source_backend(0, Vec::new());
        let mut models = SimulationDmoModelRegistry::try_new().unwrap();
        register_route(&mut models, &foreign_wrapper, 0);
        assert!(matches!(
            SimulationDmoResolvedLocatorPartOwner::try_new(
                locator_plan.try_locator_occurrence(0, 0).unwrap(),
                &models,
                &catalog,
                &backend,
            ),
            Err(SimulationDmoResolvedLocatorPartOwnerError::ExactWinnerMismatch)
        ));
        assert_eq!(backend.last_add_failure(), None);
    }

    #[test]
    fn backend_route_and_transform_refusals_are_effect_free() {
        const MODEL: u64 = 0x1122_3344_5566_7788;
        let mut fixture = valid_fixture(0xa0, &[(0x8877, MODEL)]);
        fixture.part_inertia_offsets[0] = Some(0x10);
        let asset = build_asset(fixture);
        let arena_owner = asset.plan_owner();
        let (_storage, collection) = resident_collection(vec![asset]);
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();
        let models = models_for_catalog(&catalog);
        let locator_plan =
            locator_plan(&[(0x7788_99aa_bbcc_ddee, &[(1, 2, MODEL), (3, 4, MODEL)])]);
        let mut foreign_backend = source_backend(1, Vec::new());
        assert!(matches!(
            SimulationDmoResolvedLocatorPartOwner::try_new(
                locator_plan.try_locator_occurrence(0, 0).unwrap(),
                &models,
                &catalog,
                &foreign_backend,
            ),
            Err(SimulationDmoResolvedLocatorPartOwnerError::PartOwner(
                SimulationDmoAssemblyPartOwnerError::Attachment(
                    SerialisedAssemblyPartAttachmentBindError::Route {
                        ordinal: 0,
                        source: SerialisedAssemblySimulationBindError::ArenaNotRegistered,
                    }
                )
            ))
        ));
        assert_eq!(foreign_backend.last_add_failure(), None);

        let mut backend = source_backend(
            1,
            vec![(
                Arc::clone(&arena_owner),
                source_binding(arena_owner, 0x2000),
            )],
        );
        let resolved = SimulationDmoResolvedLocatorPartOwner::try_new(
            locator_plan.try_locator_occurrence(0, 1).unwrap(),
            &models,
            &catalog,
            &backend,
        )
        .unwrap();
        assert!(matches!(
            resolved.try_begin_session(Vec::new()),
            Err(
                SimulationDmoResolvedLocatorPartSessionCreateError::PartSession(
                    DynamicAssemblySessionCreateError::PartTransformCount {
                        parts: 1,
                        transforms: 0,
                    }
                )
            )
        ));
        assert_eq!(backend.last_add_failure(), None);
        let mut session = resolved.try_begin_session(vec![IDENTITY]).unwrap();
        assert_eq!(
            session.attach(&mut foreign_backend, 4),
            Err(SimulationDmoAssemblyPartAttachError::BackendAuthorityMismatch)
        );
        assert!(!session.is_attached());
        assert_eq!(foreign_backend.last_add_failure(), None);
        session.attach(&mut backend, 4).unwrap();
        session.detach(&mut backend).unwrap();
    }

    #[test]
    fn combined_session_keeps_exact_model_and_locator_sources_alive() {
        const MODEL: u64 = 0x1234_5678_9abc_def0;
        let locator_plan = locator_plan(&[(0x0fed_cba9_8765_4321, &[(0x1111, 0x2222, MODEL)])]);
        let locator_static_owner = locator_plan.normal_plan().static_plan_owner();
        let weak_locator_owner = Arc::downgrade(&locator_static_owner);
        let (mut storage, collection) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(0x3333, MODEL)]))]);
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();
        let mut models = models_for_catalog(&catalog);
        let weak_model_asset = Weak::clone(&catalog.routes[0].asset);
        let mut backend = source_backend(0, Vec::new());
        let resolved = SimulationDmoResolvedLocatorPartOwner::try_new(
            locator_plan.try_locator_occurrence(0, 0).unwrap(),
            &models,
            &catalog,
            &backend,
        )
        .unwrap();
        let mut session = resolved.try_begin_session(Vec::new()).unwrap();

        let registered_asset = catalog.routes[0].asset.upgrade().unwrap();
        models.deregister(
            SimulationDmoModelRegistrationSource::try_prepare(registered_asset, 0).unwrap(),
        );
        drop(models);

        storage.destroy(collection);
        drop(locator_plan);
        drop(locator_static_owner);
        drop(resolved);
        assert!(weak_model_asset.upgrade().is_some());
        assert!(weak_locator_owner.upgrade().is_some());
        session.attach(&mut backend, 4).unwrap();
        session.detach(&mut backend).unwrap();
        drop(session);
        assert!(weak_model_asset.upgrade().is_none());
        assert!(weak_locator_owner.upgrade().is_none());
    }

    fn manager_register_model(
        manager: &mut SimulationDmoRegistrationManager,
        asset: &Arc<DmoBankProcessedArenaAsset>,
        operation_index: usize,
    ) {
        manager
            .register_model(
                SimulationDmoModelRegistrationSource::try_prepare(
                    Arc::clone(asset),
                    operation_index,
                )
                .unwrap(),
            )
            .unwrap();
    }

    fn manager_register_locator(
        manager: &mut SimulationDmoRegistrationManager,
        plan: &SimulationDmoArenaPlan,
        operation_index: usize,
    ) {
        let operation = &plan.operations()[operation_index];
        manager
            .register_locator(
                &plan.source_handle(operation_index).unwrap(),
                operation.asset_id,
            )
            .unwrap();
    }

    #[test]
    fn combined_manager_clear_is_locator_only_idempotent_and_reusable() {
        const CONTAINER: u64 = 0x1111_2222_dead_beef;
        const MODEL: u64 = 0x3333_4444_cafe_babe;
        let locator = locator_plan(&[(CONTAINER, &[(1, 2, MODEL)])]);
        let (_storage, collection) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(7, MODEL)]))]);
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();
        let model_asset = catalog.routes[0].asset.upgrade().unwrap();
        let backend = source_backend(0, Vec::new());
        let mut manager = SimulationDmoRegistrationManager::try_new().unwrap();
        manager_register_model(&mut manager, &model_asset, 0);
        manager_register_locator(&mut manager, &locator, 0);
        assert!(
            manager
                .try_resolve_locator_part_owner(&locator, 0, 0, &catalog, &backend)
                .is_ok()
        );

        manager.clear_locators();
        manager.clear_locators();
        assert!(matches!(
            manager.try_resolve_locator_part_owner(&locator, 0, 0, &catalog, &backend),
            Err(SimulationDmoRegistrationManagerResolveError::Locator(
                SimulationDmoRegisteredLocatorOccurrenceError::NotRegistered
            ))
        ));
        manager_register_locator(&mut manager, &locator, 0);
        assert!(
            manager
                .try_resolve_locator_part_owner(&locator, 0, 0, &catalog, &backend)
                .is_ok()
        );
    }

    #[test]
    fn combined_manager_checks_locator_then_model_then_dead_catalog() {
        const CONTAINER: u64 = 0xaaaa_bbbb_1234_5678;
        const MODEL: u64 = 0xcccc_dddd_8765_4321;
        let locator = locator_plan(&[(CONTAINER, &[(1, 2, MODEL)])]);
        let (mut storage, collection) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(7, MODEL)]))]);
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();
        let model_asset = catalog.routes[0].asset.upgrade().unwrap();
        storage.destroy(collection);
        let backend = source_backend(0, Vec::new());
        let mut manager = SimulationDmoRegistrationManager::try_new().unwrap();

        assert!(matches!(
            manager.try_resolve_locator_part_owner(&locator, 0, 0, &catalog, &backend),
            Err(SimulationDmoRegistrationManagerResolveError::Locator(
                SimulationDmoRegisteredLocatorOccurrenceError::NotRegistered
            ))
        ));
        manager_register_locator(&mut manager, &locator, 0);
        assert!(matches!(
            manager.try_resolve_locator_part_owner(&locator, 0, 0, &catalog, &backend),
            Err(SimulationDmoRegistrationManagerResolveError::Part(
                SimulationDmoResolvedLocatorPartOwnerError::ManagerMissing
            ))
        ));
        manager_register_model(&mut manager, &model_asset, 0);
        assert!(matches!(
            manager.try_resolve_locator_part_owner(&locator, 0, 0, &catalog, &backend),
            Err(SimulationDmoRegistrationManagerResolveError::Part(
                SimulationDmoResolvedLocatorPartOwnerError::ResidentExpired
            ))
        ));
    }

    #[test]
    fn combined_manager_preserves_locator_and_model_duplicate_asymmetry() {
        const CONTAINER: u64 = 0x1000_2000_dead_beef;
        const MODEL: u64 = 0x3000_4000_cafe_babe;
        let locator_a = locator_plan(&[(CONTAINER, &[(1, 2, MODEL)])]);
        let locator_b = locator_plan(&[(CONTAINER, &[(3, 4, MODEL)])]);
        let (_storage_a, collection_a) =
            resident_collection(vec![build_asset(valid_fixture(0xa0, &[(7, MODEL)]))]);
        let (_storage_b, collection_b) =
            resident_collection(vec![build_asset(valid_fixture(0xb0, &[(8, MODEL)]))]);
        let catalog_a = DmoBankResidentModelCatalog::try_new(&collection_a).unwrap();
        let catalog_b = DmoBankResidentModelCatalog::try_new(&collection_b).unwrap();
        let model_a = catalog_a.routes[0].asset.upgrade().unwrap();
        let model_b = catalog_b.routes[0].asset.upgrade().unwrap();
        let backend = source_backend(0, Vec::new());
        let mut manager = SimulationDmoRegistrationManager::try_new().unwrap();

        manager_register_locator(&mut manager, &locator_a, 0);
        manager_register_locator(&mut manager, &locator_b, 0);
        manager_register_model(&mut manager, &model_a, 0);
        manager_register_model(&mut manager, &model_b, 0);
        manager.deregister_locator(&locator_b.source_handle(0).unwrap(), CONTAINER);
        manager.deregister_model(
            SimulationDmoModelRegistrationSource::try_prepare(Arc::clone(&model_b), 0).unwrap(),
        );
        assert!(matches!(
            manager.try_resolve_locator_part_owner(&locator_a, 0, 0, &catalog_a, &backend),
            Err(SimulationDmoRegistrationManagerResolveError::Locator(
                SimulationDmoRegisteredLocatorOccurrenceError::NotRegistered
            ))
        ));

        manager_register_locator(&mut manager, &locator_a, 0);
        assert!(
            manager
                .try_resolve_locator_part_owner(&locator_a, 0, 0, &catalog_a, &backend)
                .is_ok()
        );
        assert!(matches!(
            manager.try_resolve_locator_part_owner(&locator_a, 0, 0, &catalog_b, &backend),
            Err(SimulationDmoRegistrationManagerResolveError::Part(
                SimulationDmoResolvedLocatorPartOwnerError::ExactWinnerMismatch
            ))
        ));
    }

    #[test]
    fn combined_manager_keeps_full_width_locator_and_model_domains() {
        const CONTAINER_A: u64 = 0x1111_0000_dead_beef;
        const CONTAINER_B: u64 = 0x2222_0000_dead_beef;
        const MODEL_A: u64 = 0x3333_0000_cafe_babe;
        const MODEL_B: u64 = 0x4444_0000_cafe_babe;
        let records_a = [(1, 2, MODEL_A)];
        let records_b = [(3, 4, MODEL_B)];
        let locator = locator_plan(&[
            (CONTAINER_A, records_a.as_slice()),
            (CONTAINER_B, records_b.as_slice()),
        ]);
        let (_storage, collection) = resident_collection(vec![build_asset(valid_fixture(
            0xa0,
            &[(7, MODEL_A), (8, MODEL_B)],
        ))]);
        let catalog = DmoBankResidentModelCatalog::try_new(&collection).unwrap();
        let model_asset = catalog.routes[0].asset.upgrade().unwrap();
        let backend = source_backend(0, Vec::new());
        let mut manager = SimulationDmoRegistrationManager::try_new().unwrap();
        manager_register_locator(&mut manager, &locator, 0);
        manager_register_locator(&mut manager, &locator, 1);
        manager_register_model(&mut manager, &model_asset, 0);
        manager_register_model(&mut manager, &model_asset, 1);

        assert!(
            manager
                .try_resolve_locator_part_owner(&locator, 0, 0, &catalog, &backend)
                .is_ok()
        );
        assert!(
            manager
                .try_resolve_locator_part_owner(&locator, 1, 0, &catalog, &backend)
                .is_ok()
        );
        manager.deregister_locator(&locator.source_handle(1).unwrap(), CONTAINER_B);
        assert!(
            manager
                .try_resolve_locator_part_owner(&locator, 0, 0, &catalog, &backend)
                .is_ok()
        );
        assert!(matches!(
            manager.try_resolve_locator_part_owner(&locator, 1, 0, &catalog, &backend),
            Err(SimulationDmoRegistrationManagerResolveError::Locator(
                SimulationDmoRegisteredLocatorOccurrenceError::NotRegistered
            ))
        ));
    }

    #[test]
    fn combined_manager_owns_both_registration_sources_until_drop() {
        const CONTAINER: u64 = 0x5555_6666_7777_8888;
        const MODEL: u64 = 0x9999_aaaa_bbbb_cccc;
        let locator = locator_plan(&[(CONTAINER, &[(1, 2, MODEL)])]);
        let locator_owner = locator.normal_plan().static_plan_owner();
        let weak_locator_owner = Arc::downgrade(&locator_owner);
        let model_asset = Arc::new(build_asset(valid_fixture(0xa0, &[(7, MODEL)])));
        let weak_model_asset = Arc::downgrade(&model_asset);
        let mut manager = SimulationDmoRegistrationManager::try_new().unwrap();
        manager_register_locator(&mut manager, &locator, 0);
        manager_register_model(&mut manager, &model_asset, 0);

        drop(locator_owner);
        drop(locator);
        drop(model_asset);
        assert!(weak_locator_owner.upgrade().is_some());
        assert!(weak_model_asset.upgrade().is_some());
        drop(manager);
        assert!(weak_locator_owner.upgrade().is_none());
        assert!(weak_model_asset.upgrade().is_none());
    }

    #[test]
    fn combined_manager_init_errors_preserve_nested_sources() {
        let model_source =
            SimulationDmoModelRegistryInitError::StorageAllocation { requested: 256 };
        let model = SimulationDmoRegistrationManagerInitError::Models(model_source);
        assert_eq!(
            model.source().unwrap().to_string(),
            model_source.to_string()
        );

        let locator_source =
            SimulationDmoLocatorRegistryInitError::StorageAllocation { requested: 50 };
        let locator = SimulationDmoRegistrationManagerInitError::Locators(locator_source);
        assert_eq!(
            locator.source().unwrap().to_string(),
            locator_source.to_string()
        );
    }
}
