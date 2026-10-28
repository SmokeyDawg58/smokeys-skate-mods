//! `CameraAttribManager.Init825822c8` retained Vault bindings and subscriptions.
//! `ReadAttribs825835a8` remains the separately scheduled parameter producer.
use crate::{
    attribute_notifications::{AttributeNotificationError, AttributeNotifications},
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
};
use std::sync::{Arc, atomic::AtomicBool};

const CAMERA: u64 = 0x55d8_01ed_e7e3_38b6;
const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
const TRACKER: u64 = 0x6491_aac7_665a_9703;
const LAG: u64 = 0xda09_886b_3631_5041;
/// Class, collection, generated fallback-layout size (zero means plain Instance).
pub(crate) const BINDINGS: [(u64, u64, usize); 20] = [
    (CAMERA, 0x844b_635f_ff81_b46a, 0x2b0), // Editor
    (CAMERA, 0x3173_4b9d_0805_149b, 0x2b0), // Dynamics
    (0xee7b_30a6_40fd_613b, DEFAULT, 0xe8), // DropPredictor
    (0x5431_5ef6_b7f5_1c87, DEFAULT, 0x34), // GrindDetector
    (CAMERA, 0xef79_fff9_62a2_cda7, 0x2b0), // RigConfig
    (CAMERA, 0xe17c_ee78_ad9d_e326, 0x2b0), // SubjectTracker
    (0x02ca_f2d3_1ee9_6767, DEFAULT, 0xac), // Shake
    (CAMERA, 0xd2fa_0ba9_ec44_66da, 0x2b0), // RigDrives
    (CAMERA, 0x42bc_873e_b2c5_3253, 0x2b0), // Rotation
    (LAG, 0x0611_0c5b_d81c_df44, 0x48),     // PushLag
    (LAG, 0x6ce7_d771_f9b4_5ab0, 0x48),     // PumpLag
    (TRACKER, 0xa9b5_7e47_9fe1_918d, 0x18), // Distance
    (TRACKER, 0xa2cd_db16_9ac9_5646, 0x18), // Elevation
    (TRACKER, 0xa9ab_94b7_e6b6_2c22, 0x18), // Heading
    (TRACKER, 0x5f2b_d817_e868_a0fe, 0x18), // Pan
    (TRACKER, 0xd0c2_e4c8_c3c1_d6f4, 0x18), // Tilt
    (0x901a_bc79_a7db_1e0a, DEFAULT, 0x1c), // Compass
    (0x4cf6_4378_945f_a4dd, DEFAULT, 0),    // CameraPositioner plain Instance
    (0x1ffd_c8e3_aca0_7c1f, DEFAULT, 0x78), // Rendering
    (CAMERA, 0x42c5_ff84_7bc2_c700, 0x2b0), // FreeCam
];

pub(crate) struct CameraAttributes {
    pub(crate) bindings: Vec<AttributeRuntimeInstance>,
    pub(crate) changed: Arc<AtomicBool>,
    /// Actual subobject remains unwritten until scheduled `ReadAttribs`. Consumers
    /// retain this owner's identity without manufacturing parameter defaults.
    pub(crate) parameters: CameraParameterPublication,
}
pub(crate) enum CameraParameterPublication {
    Unwritten,
    Fields(Box<crate::camera_parameters::CameraParameters>),
}
impl CameraParameterPublication {
    pub(crate) fn fields_mut(&mut self) -> &mut crate::camera_parameters::CameraParameters {
        if matches!(self, Self::Unwritten) {
            *self = Self::Fields(Box::default());
        }
        match self {
            Self::Fields(fields) => fields,
            Self::Unwritten => unreachable!("published above"),
        }
    }
}
impl CameraAttributes {
    pub(crate) fn begin_init() -> Self {
        Self {
            bindings: Vec::new(),
            changed: Arc::new(AtomicBool::new(true)),
            parameters: CameraParameterPublication::Unwritten,
        }
    }
    /// Store each real retained instance, then register keys in source order.
    /// `RigDrives` deliberately has no notification; six equal Default keys remain.
    pub(crate) fn initialize(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        monitor: &AttributeNotifications,
    ) -> Result<(), AttributeNotificationError> {
        self.bindings
            .try_reserve_exact(BINDINGS.len())
            .map_err(|_| AttributeNotificationError::Allocation)?;
        for (class, collection, _) in BINDINGS {
            self.bindings.push(vault.instance(class, collection));
        }
        for (index, (_, collection, _)) in BINDINGS.iter().enumerate() {
            if index != 7 {
                monitor.register_camera(*collection, &self.changed)?;
            }
        }
        Ok(())
    }
}
