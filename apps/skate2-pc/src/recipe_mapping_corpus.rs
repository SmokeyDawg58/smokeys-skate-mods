//! Mounted `RecipeMan::InitRecipeMappings` collaborator.
//!
//! The supplied build's compiled `kUseForcedRecipes` default is false. This
//! corpus backend returns that stable value on every native re-sample; it does
//! not claim a live cVar owner or concurrent mutation timing.

use super::{AttributeCollectionMap, resolve_attribute_value};
use skate2_core::living_world_recipe_manager::{
    FORCED_PEDESTRIAN_MODEL_ATTRIBUTE_ID, FORCED_VEHICLE_MODEL_ATTRIBUTE_ID,
    LIVING_WORLD_CONFIG_CLASS_ID, LIVING_WORLD_FORCED_CONFIG_COLLECTION_ID,
    LIVING_WORLD_MODELS_CLASS_ID, LOAD_MODEL_ATTRIBUTE_ID, RecipeLoadingQueues, RecipeMappingEntry,
    RecipeMappingProvider, RecipeMappingTable, build_recipe_loading_queues, build_recipe_mappings,
};
use skate2_formats::{
    attrib::{
        AttributeDefinition, AttributeExport, AttributeExportKind, AttributePointerLocation,
        AttributePointerValue, AttributeVaultMetadata,
    },
    mount::MountSet,
};
use std::{collections::BTreeMap, error::Error, fmt};

const SHIPPED_REQUIRED: [u64; 15] = [
    0xb7cd_d46e_e0b8_e9b0,
    0x29ea_db80_7f46_edf6,
    0x7db7_14d7_e927_5d53,
    0x8151_8f0e_dafe_70a6,
    0xef73_f403_30bd_5d77,
    0x56f3_cd83_30b9_78ee,
    0x5bbf_cd8b_c95c_b2e7,
    0x58b6_0bae_7322_56bb,
    0xcb2c_143b_9929_a70a,
    0x041a_0b27_8eae_b09c,
    0x1e58_8e6d_426d_1a2f,
    0x5ef2_eddd_c75f_25de,
    0x030a_b922_4e08_cbd1,
    0x8960_aeb0_8f12_71cd,
    0xf977_b957_a0e8_68cb,
];

const SHIPPED_SECONDARY: [u64; 39] = [
    0xbba7_197f_af01_7c2f,
    0x47ad_17fd_06f5_3aba,
    0xa2b2_41f6_ac35_b367,
    0xc81c_3aac_7f34_f22d,
    0x3dcd_5154_e552_6fa7,
    0x4ce4_5ca0_fc8f_0308,
    0x9bc1_f7cb_5a63_b806,
    0x036e_aaba_3222_ec24,
    0xc801_dd94_c359_8622,
    0x34a2_a830_f2ab_aee2,
    0x5ac9_5e9c_35f6_5bed,
    0xe602_f3b8_5b00_f793,
    0x3ba9_21fc_ef1b_45fd,
    0x445f_8033_83f2_6837,
    0x6daf_1766_3d47_fd14,
    0xc8ea_be24_881e_8004,
    0x538d_c4b8_8de9_4552,
    0x785d_7012_1bfa_192d,
    0xf455_ac5a_7af6_9c18,
    0x1594_5e1b_6db9_452d,
    0x3457_c13f_e6d1_28f1,
    0x9552_f722_9512_9362,
    0x05f6_62f7_109b_dede,
    0x798c_b224_ed7f_bfee,
    0xfe1f_4594_b6f9_18e2,
    0xe4cf_d73f_9ee2_fe3e,
    0x2743_8ef4_e22d_3d39,
    0x6032_5114_3925_78e4,
    0x5dbe_f74c_53c5_5e6e,
    0xde2d_9c8a_d95e_dcb0,
    0x195a_abb6_6c9e_4935,
    0x19ee_bdd7_dc36_58a3,
    0xb21b_a6b1_85b0_1ec9,
    0x79b9_da91_8371_86b1,
    0x8d7a_a208_b7ef_f707,
    0x325e_baa6_9b25_8848,
    0x9a4d_5e87_ade3_af28,
    0xedeb_a27c_4163_bfae,
    0x52be_4f59_7e67_7629,
];

const SHIPPED_MAPPING_ENTRIES: [(u64, u64, bool); 54] = [
    (0xa8ae_0c85_0ebb_bbfd, 0x030a_b922_4e08_cbd1, true),
    (0xa8ae_0c85_0ebb_bbfd, 0x8960_aeb0_8f12_71cd, true),
    (0xa8ae_0c85_0ebb_bbfd, 0xf977_b957_a0e8_68cb, true),
    (0x1562_d5e0_4bc3_9614, 0x041a_0b27_8eae_b09c, true),
    (0x1562_d5e0_4bc3_9614, 0x1e58_8e6d_426d_1a2f, true),
    (0x1562_d5e0_4bc3_9614, 0x5ef2_eddd_c75f_25de, true),
    (0x916c_84f7_3149_9248, 0x29ea_db80_7f46_edf6, true),
    (0x916c_84f7_3149_9248, 0x7db7_14d7_e927_5d53, true),
    (0x916c_84f7_3149_9248, 0x8151_8f0e_dafe_70a6, true),
    (0x7fb4_08b2_7e0c_2521, 0x56f3_cd83_30b9_78ee, true),
    (0x7fb4_08b2_7e0c_2521, 0x5bbf_cd8b_c95c_b2e7, true),
    (0xc281_eaea_ad42_219b, 0x58b6_0bae_7322_56bb, true),
    (0xc281_eaea_ad42_219b, 0xcb2c_143b_9929_a70a, true),
    (0xbb5c_8edb_f201_ac9d, 0xb7cd_d46e_e0b8_e9b0, true),
    (0xbd79_86b3_d673_b134, 0xef73_f403_30bd_5d77, true),
    (0xdca9_faa9_2bda_1c8a, 0x05f6_62f7_109b_dede, false),
    (0xdca9_faa9_2bda_1c8a, 0x798c_b224_ed7f_bfee, false),
    (0xdca9_faa9_2bda_1c8a, 0xfe1f_4594_b6f9_18e2, false),
    (0x62e6_9f57_ed03_12fc, 0x1594_5e1b_6db9_452d, false),
    (0x62e6_9f57_ed03_12fc, 0x3457_c13f_e6d1_28f1, false),
    (0x62e6_9f57_ed03_12fc, 0x9552_f722_9512_9362, false),
    (0x153d_8034_460d_6051, 0x325e_baa6_9b25_8848, false),
    (0x153d_8034_460d_6051, 0x9a4d_5e87_ade3_af28, false),
    (0x153d_8034_460d_6051, 0xedeb_a27c_4163_bfae, false),
    (0xaceb_008f_9d80_5483, 0x34a2_a830_f2ab_aee2, false),
    (0xaceb_008f_9d80_5483, 0x5ac9_5e9c_35f6_5bed, false),
    (0xaceb_008f_9d80_5483, 0xe602_f3b8_5b00_f793, false),
    (0x13c6_2aae_f214_349d, 0x3ba9_21fc_ef1b_45fd, false),
    (0x13c6_2aae_f214_349d, 0x445f_8033_83f2_6837, false),
    (0x13c6_2aae_f214_349d, 0x6daf_1766_3d47_fd14, false),
    (0xcf10_791d_c5c3_3c13, 0x47ad_17fd_06f5_3aba, false),
    (0xcf10_791d_c5c3_3c13, 0xa2b2_41f6_ac35_b367, false),
    (0xcf10_791d_c5c3_3c13, 0xc81c_3aac_7f34_f22d, false),
    (0x1bf4_c3c1_72a6_f81a, 0x538d_c4b8_8de9_4552, false),
    (0x1bf4_c3c1_72a6_f81a, 0x785d_7012_1bfa_192d, false),
    (0x1bf4_c3c1_72a6_f81a, 0xf455_ac5a_7af6_9c18, false),
    (0x8d1a_ab96_6e2b_650f, 0x5dbe_f74c_53c5_5e6e, false),
    (0x8d1a_ab96_6e2b_650f, 0xde2d_9c8a_d95e_dcb0, false),
    (0x0abe_4139_de1a_e16a, 0x036e_aaba_3222_ec24, false),
    (0x0abe_4139_de1a_e16a, 0xc801_dd94_c359_8622, false),
    (0x5d3b_409b_4750_a362, 0x195a_abb6_6c9e_4935, false),
    (0x5d3b_409b_4750_a362, 0x19ee_bdd7_dc36_58a3, false),
    (0x5d3b_409b_4750_a362, 0xb21b_a6b1_85b0_1ec9, false),
    (0x01a6_c596_8e02_163d, 0x2743_8ef4_e22d_3d39, false),
    (0x01a6_c596_8e02_163d, 0x6032_5114_3925_78e4, false),
    (0x42e5_f4d3_cb27_83e2, 0x3dcd_5154_e552_6fa7, false),
    (0x42e5_f4d3_cb27_83e2, 0x4ce4_5ca0_fc8f_0308, false),
    (0x42e5_f4d3_cb27_83e2, 0x9bc1_f7cb_5a63_b806, false),
    (0x23e9_5e38_df87_3a6a, 0x52be_4f59_7e67_7629, false),
    (0x0e7e_bc63_a64b_786a, 0x79b9_da91_8371_86b1, false),
    (0x0e7e_bc63_a64b_786a, 0x8d7a_a208_b7ef_f707, false),
    (0x8e3c_ee2e_8cee_3440, 0xbba7_197f_af01_7c2f, false),
    (0x5249_a4c7_13df_82ff, 0xc8ea_be24_881e_8004, false),
    (0x2e0e_f977_4e79_1492, 0xe4cf_d73f_9ee2_fe3e, false),
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecipeMappingCorpusSummary {
    pub groups: usize,
    pub entries: usize,
    pub mapping_entries: Vec<(u64, u64, bool)>,
    pub required: Vec<u64>,
    pub initial: Vec<u64>,
    pub secondary_before_truncation: Vec<u64>,
    pub secondary: Vec<u64>,
}

impl RecipeMappingCorpusSummary {
    #[must_use]
    pub fn matches_shipped_default(&self) -> bool {
        self.groups == 24
            && self.entries == 54
            && self.mapping_entries == SHIPPED_MAPPING_ENTRIES
            && self.required == SHIPPED_REQUIRED
            && self.initial.is_empty()
            && self.secondary_before_truncation == SHIPPED_SECONDARY
            && self.secondary == SHIPPED_SECONDARY[..10]
    }
}

#[derive(Debug)]
struct MountedRecipeProviderError(String);

impl fmt::Display for MountedRecipeProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for MountedRecipeProviderError {}

struct MountedRecipeProvider<'a> {
    buffers: [&'a [u8]; 2],
    pointers: &'a BTreeMap<AttributePointerLocation, AttributePointerValue>,
    classes: &'a BTreeMap<u64, &'a AttributeExport>,
    model_class: &'a AttributeExport,
    model_load: AttributeDefinition,
    collections: &'a AttributeCollectionMap<'a>,
}

impl MountedRecipeProvider<'_> {
    fn collection(
        &self,
        class_id: u64,
        collection_id: u64,
    ) -> Result<
        &(
            &AttributeExport,
            skate2_formats::attrib::AttributeCollectionLoadData,
        ),
        MountedRecipeProviderError,
    > {
        self.collections
            .get(&(class_id, collection_id))
            .ok_or_else(|| {
                MountedRecipeProviderError(format!(
                    "missing RecipeMan collection {class_id:016x}:{collection_id:016x}"
                ))
            })
    }

    fn value(
        &self,
        class: &AttributeExport,
        collection_id: u64,
        definition: &AttributeDefinition,
    ) -> Result<Option<Vec<u8>>, MountedRecipeProviderError> {
        let (collection_export, collection) = self.collection(class.object_id, collection_id)?;
        resolve_attribute_value(
            &self.buffers,
            self.pointers,
            self.collections,
            class,
            collection_export,
            collection,
            definition,
        )
        .map_err(|source| MountedRecipeProviderError(source.to_string()))
    }

    fn is_descendant_of(
        &self,
        class_id: u64,
        collection_id: u64,
        ancestor_id: u64,
    ) -> Result<bool, MountedRecipeProviderError> {
        self.collection(class_id, collection_id)?;
        self.collection(class_id, ancestor_id)?;
        let mut current = collection_id;
        let mut visited = Vec::new();
        loop {
            let parent = self.collection(class_id, current)?.1.parent_id;
            if parent == ancestor_id {
                return Ok(true);
            }
            if parent == 0 {
                return Ok(false);
            }
            if visited.contains(&parent) {
                return Err(MountedRecipeProviderError(format!(
                    "cyclic RecipeMan parent chain at {parent:016x}"
                )));
            }
            visited.push(parent);
            current = parent;
        }
    }
}

impl RecipeMappingProvider for MountedRecipeProvider<'_> {
    type Error = MountedRecipeProviderError;

    fn use_forced_recipes(&mut self) -> bool {
        false
    }

    fn forced_model_collection(&mut self, attribute_id: u64) -> Result<Option<u64>, Self::Error> {
        if !matches!(
            attribute_id,
            FORCED_PEDESTRIAN_MODEL_ATTRIBUTE_ID | FORCED_VEHICLE_MODEL_ATTRIBUTE_ID
        ) {
            return Err(MountedRecipeProviderError(format!(
                "unknown forced RecipeMan attribute {attribute_id:016x}"
            )));
        }
        let config_class = self
            .classes
            .get(&LIVING_WORLD_CONFIG_CLASS_ID)
            .copied()
            .ok_or_else(|| MountedRecipeProviderError("missing livingworld config class".into()))?;
        let definition = config_class
            .class_definitions(&self.buffers, self.pointers)
            .map_err(|source| MountedRecipeProviderError(source.to_string()))?
            .into_iter()
            .find(|definition| definition.key == attribute_id)
            .ok_or_else(|| {
                MountedRecipeProviderError(format!(
                    "missing forced RecipeMan definition {attribute_id:016x}"
                ))
            })?;
        let Some(value) = self.value(
            config_class,
            LIVING_WORLD_FORCED_CONFIG_COLLECTION_ID,
            &definition,
        )?
        else {
            return Ok(None);
        };
        let collection = value.get(8..16).ok_or_else(|| {
            MountedRecipeProviderError("truncated forced RecipeMan RefSpec".to_owned())
        })?;
        Ok(Some(u64::from_be_bytes(
            collection
                .try_into()
                .expect("checked eight-byte RefSpec collection key"),
        )))
    }

    fn leaf_descendants_sorted(
        &mut self,
        class_id: u64,
        seed_id: u64,
    ) -> Result<Vec<u64>, Self::Error> {
        if class_id != LIVING_WORLD_MODELS_CLASS_ID {
            return Err(MountedRecipeProviderError(format!(
                "unexpected RecipeMan model class {class_id:016x}"
            )));
        }
        self.collection(class_id, seed_id)?;
        let all = self
            .collections
            .keys()
            .filter_map(|&(candidate_class, candidate)| {
                (candidate_class == class_id).then_some(candidate)
            })
            .collect::<Vec<_>>();
        let mut parents = self
            .collections
            .iter()
            .filter_map(|(&(candidate_class, _), (_, collection))| {
                (candidate_class == class_id).then_some(collection.parent_id)
            })
            .collect::<Vec<_>>();
        parents.sort_unstable();
        let mut leaves = Vec::new();
        for candidate in all {
            if parents.binary_search(&candidate).is_err()
                && self.is_descendant_of(class_id, candidate, seed_id)?
            {
                leaves.push(candidate);
            }
        }
        Ok(leaves)
    }

    fn load_model_nonzero(
        &mut self,
        class_id: u64,
        collection_id: u64,
    ) -> Result<bool, Self::Error> {
        if class_id != LIVING_WORLD_MODELS_CLASS_ID {
            return Err(MountedRecipeProviderError(format!(
                "unexpected RecipeMan model class {class_id:016x}"
            )));
        }
        let Some(value) = self.value(self.model_class, collection_id, &self.model_load)? else {
            return Ok(false);
        };
        Ok(value.first().is_some_and(|byte| *byte != 0))
    }

    fn collection_parent(&mut self, class_id: u64, collection_id: u64) -> Result<u64, Self::Error> {
        Ok(self.collection(class_id, collection_id)?.1.parent_id)
    }
}

/// Replays the shipped default Vault through the reconstructed mapping and
/// queue builders.
///
/// # Errors
///
/// Returns a checked mount, Vault, pointer, schema, hierarchy, mapping, or
/// queue failure.
pub fn load(mounts: &MountSet) -> Result<RecipeMappingCorpusSummary, Box<dyn Error>> {
    let vlt = mounts.read_path("data\\db\\skater.vlt")?;
    let bin = mounts.read_path("data\\db\\skater.bin")?;
    let vault = AttributeVaultMetadata::parse(&vlt)?;
    let pointers = vault.resolve_pointers(&[vlt.len(), bin.len()])?;
    let buffers = [vlt.as_slice(), bin.as_slice()];
    let classes = vault
        .exports
        .iter()
        .filter(|export| export.kind() == AttributeExportKind::Class)
        .map(|export| (export.object_id, export))
        .collect::<BTreeMap<_, _>>();
    let collections = vault
        .exports
        .iter()
        .filter(|export| export.kind() == AttributeExportKind::Collection)
        .map(|export| {
            let collection = export.collection_load_data(&vlt)?;
            Ok((
                (collection.class_id, collection.collection_id),
                (export, collection),
            ))
        })
        .collect::<Result<AttributeCollectionMap<'_>, skate2_formats::attrib::AttributeChunkError>>(
        )?;
    let model_class = classes
        .get(&LIVING_WORLD_MODELS_CLASS_ID)
        .copied()
        .ok_or("missing livingworld_models class")?;
    let model_definitions = model_class.class_definitions(&buffers, &pointers)?;
    let definition = |definitions: &[AttributeDefinition], key: u64, name: &'static str| {
        definitions
            .iter()
            .find(|definition| definition.key == key)
            .cloned()
            .ok_or(name)
    };
    let mut provider = MountedRecipeProvider {
        buffers,
        pointers: &pointers,
        classes: &classes,
        model_class,
        model_load: definition(
            &model_definitions,
            LOAD_MODEL_ATTRIBUTE_ID,
            "missing livingworld_models Load_Model definition",
        )?,
        collections: &collections,
    };
    let mut mappings = RecipeMappingTable::new();
    build_recipe_mappings(&mut mappings, &mut provider)?;
    let untruncated = build_recipe_loading_queues(&mappings, 15, 64)?;
    let limited = build_recipe_loading_queues(&mappings, 15, 10)?;
    Ok(summary(&mappings, &untruncated, &limited))
}

fn summary(
    mappings: &RecipeMappingTable,
    untruncated: &RecipeLoadingQueues,
    limited: &RecipeLoadingQueues,
) -> RecipeMappingCorpusSummary {
    RecipeMappingCorpusSummary {
        groups: mappings.group_count(),
        entries: mappings.entry_count(),
        mapping_entries: mappings
            .insertion_order_entries()
            .map(|(group_id, RecipeMappingEntry { model_id, required })| {
                (group_id, model_id, required)
            })
            .collect(),
        required: limited.required().to_vec(),
        initial: limited.initial().to_vec(),
        secondary_before_truncation: untruncated.secondary().to_vec(),
        secondary: limited.secondary().to_vec(),
    }
}
