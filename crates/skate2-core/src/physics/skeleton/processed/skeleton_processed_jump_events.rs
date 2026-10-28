//! Selected type-0 record dispatch and post-loop jump stores, 82b59de8.
//! This is a child of the SAME processed owner. Skeleton adjustment storage,
//! Actor event production and `HiLOD` scheduling remain actual caller obligations.
use super::SkeletonProcessedContact;
use crate::attribute_runtime::{
    AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
};
use skate2_formats::andale::PackedFastString;
use std::{cmp::Ordering, fmt, sync::Arc};

#[path = "skeleton_processed_animation_wipeout.rs"]
pub(crate) mod animation_wipeout;
#[path = "../movement/skeleton_ground_jump_attributes.rs"]
pub(crate) mod ground_jump_attributes;

pub(crate) const PREPARE: u32 = 0x0020_0000;
pub(crate) const JUMP: u32 = 0x0040_0000;
const ENDPOINTS: u64 = 0xb52e_6ee4_f322_a9f4;
const LOWER: u64 = 0x982f_499e_c977_35a9;
const UPPER: u64 = 0xa1f6_5bb8_c4d1_ba8b;
const RETAIN_ADJUSTMENT: u64 = 0x34bb_37c3_15bc_14b0;

/// Each sample reads the actual captured input's map, pad and g0..g3 at that
/// callback. Stored adjustments belong to Skeleton+3e34/+3e38, not Processed.
pub(crate) trait SkeletonJumpInput {
    type Input;
    type Error;
    fn capture_input(&mut self) -> Result<Option<Self::Input>, Self::Error>;
    fn value(&mut self, input: &Self::Input, id: u32) -> Result<f32, Self::Error>;
    fn store_adjustment_x(&mut self, value: f32) -> Result<(), Self::Error>;
    fn store_adjustment_z(&mut self, value: f32) -> Result<(), Self::Error>;
    fn adjustment_x(&mut self) -> Result<f32, Self::Error>;
    fn adjustment_z(&mut self) -> Result<f32, Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkeletonJumpGlobal {
    Jumping,
    Adjustment,
}
pub(crate) trait SkeletonJumpAttributes {
    type Global;
    type Pointer;
    type Error;
    fn capture(&mut self, kind: SkeletonJumpGlobal) -> Result<Self::Global, Self::Error>;
    fn lookup(&mut self, global: &Self::Global, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn byte(&mut self, pointer: &Self::Pointer) -> Result<u8, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
}

pub(crate) struct SkeletonProcessedJumpEvents {
    spin: Option<f32>,
    flags: u32,
    known: u32,
    minimum: f32,
    height: f32,
    active: bool,
    complete: bool,
    strength: Option<f32>,
    adjustment_x: Option<f32>,
    adjustment_z: Option<f32>,
}
impl SkeletonProcessedJumpEvents {
    pub(crate) const fn uninitialized() -> Self {
        Self {
            spin: None,
            flags: 0,
            known: 0,
            minimum: 0.0,
            height: 0.0,
            active: false,
            complete: false,
            strength: None,
            adjustment_x: None,
            adjustment_z: None,
        }
    }
    pub(super) fn reset_source_fields(&mut self) {
        // Reset39478 stores r30=0 from391ac into processed+8d0.
        // Reset39458 writes f31=+0 to processed+97c.
        self.spin = Some(0.0);
        // SAME complete word: Reset 82b39414/28 writes 0x2000, not a mask.
        // Jump dispatch and ProcessAnimOut must never own separate copies.
        self.flags = 0x2000;
        self.known = u32::MAX;
        // Reset f31=+0 at 3927c; source stores 394bc, 394c4, 394cc.
        self.strength = Some(0.0);
        self.adjustment_x = Some(0.0);
        self.adjustment_z = Some(0.0);
        self.active = false;
        self.complete = false;
    }
    pub(crate) const fn known_flags(&self) -> (u32, u32) {
        (self.flags & (PREPARE | JUMP), self.known & (PREPARE | JUMP))
    }
    pub(super) const fn known_word(&self) -> (u32, u32) {
        (self.flags, self.known)
    }
    /// `ProcessAnimOut` 82e00050..58 inserts only input byte bit zero.
    pub(super) fn process_board_flip_byte(&mut self, value: u8) {
        const MASK: u32 = 0x0010_0000;
        self.flags = (self.flags & !MASK) | (u32::from(value & 1) << 20);
        self.known |= MASK;
    }
    /// Same word insertions at ProcessAnimOut82e00090/000b8.
    pub(super) fn process_animation_flag(&mut self, mask: u32, value: bool) {
        self.flags = (self.flags & !mask) | if value { mask } else { 0 };
        self.known |= mask;
    }
    pub(super) fn process_animation_bits(&mut self, mask: u32, value: u32) {
        self.flags = (self.flags & !mask) | (value & mask);
        self.known |= mask;
    }
    pub(crate) const fn spin(&self) -> Option<f32> {
        self.spin
    }
    pub(crate) const fn strength(&self) -> Option<f32> {
        self.strength
    }
    pub(crate) const fn adjustments(&self) -> (Option<f32>, Option<f32>) {
        (self.adjustment_x, self.adjustment_z)
    }
    pub(crate) const fn complete(&self) -> bool {
        self.complete
    }
}
#[derive(Debug, PartialEq)]
pub(crate) enum SkeletonJumpError<I, A> {
    NoActivePass,
    UnknownJumpFlag,
    UnknownHippyFlag,
    UnknownPayload(usize),
    Input(I),
    Attribute(A),
}
impl<I: fmt::Debug, A: fmt::Debug> fmt::Display for SkeletonJumpError<I, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoActivePass => f.write_str("jump event pass not active"),
            Self::UnknownJumpFlag => f.write_str("processed start-jump bit not established"),
            Self::UnknownHippyFlag => f.write_str("processed hippy-jump bit not established"),
            Self::UnknownPayload(offset) => {
                write!(f, "jump attribute payload byte {offset} unknown")
            }
            Self::Input(e) => write!(f, "jump input: {e:?}"),
            Self::Attribute(e) => write!(f, "jump attribute: {e:?}"),
        }
    }
}
impl SkeletonProcessedContact {
    pub(crate) fn jump_events(&self) -> &SkeletonProcessedJumpEvents {
        &self.jump_events
    }
    /// Source local initialization 59e78/59e80; does not reset native flags or
    /// retained output lanes. Beginning a pass does not claim Actor scheduling.
    pub(crate) fn begin_jump_events(&mut self) {
        let state = &mut self.jump_events;
        state.minimum = 0.0;
        state.height = 0.0;
        state.active = true;
        state.complete = false;
    }
    pub(crate) fn try_apply_jump_event<I: SkeletonJumpInput>(
        &mut self,
        raw: &[u8; 0x40],
        input: &mut I,
    ) -> Result<bool, SkeletonJumpError<I::Error, std::convert::Infallible>> {
        let attribute = crate::actor_construction::anim_out_phys_in::PhysicsAnimationAttribute {
            payload: std::array::from_fn(|index| Some(raw[index])),
            begin: f32::from_be_bytes(raw[0x18..0x1c].try_into().expect("fixed record")),
            end: f32::from_be_bytes(raw[0x1c..0x20].try_into().expect("fixed record")),
            name: PackedFastString::<5> {
                words: std::array::from_fn(|index| {
                    let start = 0x20 + index * 4;
                    u32::from_be_bytes(raw[start..start + 4].try_into().expect("fixed record"))
                }),
            },
            flags: raw[0x34],
            kind: i8::from_ne_bytes([raw[0x35]]),
            sequence: None,
        };
        self.try_apply_typed_jump_event(&attribute, input)
    }
    pub(crate) fn try_apply_typed_jump_event<I: SkeletonJumpInput>(
        &mut self,
        attribute: &crate::actor_construction::anim_out_phys_in::PhysicsAnimationAttribute,
        input: &mut I,
    ) -> Result<bool, SkeletonJumpError<I::Error, std::convert::Infallible>> {
        use SkeletonJumpError as E;
        let state = &mut self.jump_events;
        if !state.active {
            return Err(E::NoActivePass);
        }
        // Signed-byte type test has the same equality domain as this byte test.
        // No status or payload test belongs to the type-zero branch.
        if attribute.kind != 0 {
            return Ok(false);
        }
        let name = attribute.name;
        let matches = |text| name == PackedFastString::<5>::encode(text).expect("source literal");
        let scalar = || -> Result<f32, SkeletonJumpError<I::Error, std::convert::Infallible>> {
            let mut bytes = [0; 4];
            for (index, byte) in bytes.iter_mut().enumerate() {
                *byte = attribute.payload[index].ok_or(E::UnknownPayload(index))?;
            }
            Ok(f32::from_be_bytes(bytes))
        };
        if matches("MinJump") {
            state.minimum = scalar()?;
        } else if matches("TrickHeight") {
            state.height = scalar()?;
        } else if matches("PrepareJump") {
            state.flags |= PREPARE;
            state.known |= PREPARE;
            let result = prepare(input);
            if let Err(error) = result {
                state.active = false;
                return Err(E::Input(error));
            }
        } else if matches("Spin") {
            // 82b5aa20 payload lfs, aa24 fneg, aa28 store processed+97c.
            state.spin = Some(-scalar()?);
        } else if matches("HippyJumping") {
            // Type-zero branch82b5b4c4..b4dc sets bit regardless of payload/status.
            *self
                .physics_state
                .flags_8d0
                .as_mut()
                .ok_or(E::UnknownHippyFlag)? |= 0x8000;
        } else if matches("jump") {
            state.flags |= JUMP;
            state.known |= JUMP;
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    pub(crate) fn try_finish_jump_events<I: SkeletonJumpInput, A: SkeletonJumpAttributes>(
        &mut self,
        input: &mut I,
        attributes: &mut A,
    ) -> Result<(), SkeletonJumpError<I::Error, A::Error>> {
        use SkeletonJumpError as E;
        let state = &mut self.jump_events;
        if !state.active {
            return Err(E::NoActivePass);
        }
        // Host one-attempt guard retains partial source writes on refusal.
        state.active = false;
        if state.known & JUMP == 0 {
            return Err(E::UnknownJumpFlag);
        }
        if state.flags & JUMP != 0 {
            let global = attributes
                .capture(SkeletonJumpGlobal::Jumping)
                .map_err(E::Attribute)?;
            let pointer = attributes
                .lookup(&global, ENDPOINTS)
                .map_err(E::Attribute)?;
            if attributes.byte(&pointer).map_err(E::Attribute)? == 0 {
                state.strength = Some(state.height);
            } else {
                let global = attributes
                    .capture(SkeletonJumpGlobal::Jumping)
                    .map_err(E::Attribute)?;
                let pointer = attributes.lookup(&global, LOWER).map_err(E::Attribute)?;
                let lower = attributes.float(&pointer).map_err(E::Attribute)?;
                if state.height.partial_cmp(&lower) == Some(Ordering::Greater) {
                    let global = attributes
                        .capture(SkeletonJumpGlobal::Jumping)
                        .map_err(E::Attribute)?;
                    let pointer = attributes.lookup(&global, UPPER).map_err(E::Attribute)?;
                    let upper = attributes.float(&pointer).map_err(E::Attribute)?;
                    state.strength = Some(if state.height < upper {
                        state.height
                    } else {
                        1.0
                    });
                } else {
                    state.strength = Some(state.minimum);
                }
            }
        }
        if self.physics_state.flags_8d0.ok_or(E::UnknownHippyFlag)? & 0x8000 != 0 {
            state.strength = Some(state.height);
        }
        let global = attributes
            .capture(SkeletonJumpGlobal::Adjustment)
            .map_err(E::Attribute)?;
        let pointer = attributes
            .lookup(&global, RETAIN_ADJUSTMENT)
            .map_err(E::Attribute)?;
        if attributes.byte(&pointer).map_err(E::Attribute)? != 0 {
            state.adjustment_x = Some(input.adjustment_x().map_err(E::Input)?);
            state.adjustment_z = Some(input.adjustment_z().map_err(E::Input)?);
        } else if let Some(current) = input.capture_input().map_err(E::Input)? {
            let x = input.value(&current, 74).map_err(E::Input)?;
            let x = lower_bound(x);
            let z = input.value(&current, 78).map_err(E::Input)?;
            let z = lower_bound(z);
            state.adjustment_x = Some(upper_bound(x));
            state.adjustment_z = Some(upper_bound(z));
        }
        state.complete = true;
        Ok(())
    }
}
fn prepare<I: SkeletonJumpInput>(input: &mut I) -> Result<(), I::Error> {
    if let Some(current) = input.capture_input()? {
        let x = input.value(&current, 74)?;
        let x = lower_bound(x);
        let z = input.value(&current, 75)?;
        let z = lower_bound(z);
        input.store_adjustment_x(upper_bound(x))?;
        input.store_adjustment_z(upper_bound(z))?;
    }
    Ok(())
}
fn lower_bound(value: f32) -> f32 {
    if -1.0 - value >= 0.0 { -1.0 } else { value }
}
fn upper_bound(value: f32) -> f32 {
    if 1.0 - value >= 0.0 { value } else { 1.0 }
}

/// Caller supplies the actual retained global instances at singleton+124/+d0.
/// This adapter neither binds guessed classes nor manufactures a global owner.
/// Immutable captures do not model native editor reload or singleton replacement.
pub(crate) struct SkeletonJumpVaultAttributes {
    jumping: AttributeRuntimeInstance,
    adjustment: AttributeRuntimeInstance,
    identity: Arc<()>,
}
pub(crate) struct SkeletonJumpGlobalToken {
    identity: Arc<()>,
    instance: AttributeRuntimeInstance,
}
pub(crate) struct SkeletonJumpPointer {
    identity: Arc<()>,
    data: Option<AttributeRuntimeData>,
}
#[derive(Debug)]
pub(crate) enum SkeletonJumpVaultError {
    Foreign,
    EmptyByte,
    Runtime(AttributeRuntimeError),
}
impl fmt::Display for SkeletonJumpVaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Foreign => f.write_str("foreign jump attribute capture"),
            Self::EmptyByte => f.write_str("empty jump byte attribute"),
            Self::Runtime(e) => write!(f, "jump Vault: {e}"),
        }
    }
}
impl SkeletonJumpVaultAttributes {
    pub(crate) fn from_retained_instances(
        jumping: AttributeRuntimeInstance,
        adjustment: AttributeRuntimeInstance,
    ) -> Self {
        Self {
            jumping,
            adjustment,
            identity: Arc::new(()),
        }
    }
}
impl SkeletonJumpAttributes for SkeletonJumpVaultAttributes {
    type Global = SkeletonJumpGlobalToken;
    type Pointer = SkeletonJumpPointer;
    type Error = SkeletonJumpVaultError;
    fn capture(&mut self, kind: SkeletonJumpGlobal) -> Result<Self::Global, Self::Error> {
        Ok(SkeletonJumpGlobalToken {
            identity: Arc::clone(&self.identity),
            instance: match kind {
                SkeletonJumpGlobal::Jumping => self.jumping.clone(),
                SkeletonJumpGlobal::Adjustment => self.adjustment.clone(),
            },
        })
    }
    fn lookup(&mut self, global: &Self::Global, key: u64) -> Result<Self::Pointer, Self::Error> {
        if !Arc::ptr_eq(&self.identity, &global.identity) {
            return Err(SkeletonJumpVaultError::Foreign);
        }
        Ok(SkeletonJumpPointer {
            identity: Arc::clone(&self.identity),
            data: global
                .instance
                .try_get_data(key, 0)
                .map_err(SkeletonJumpVaultError::Runtime)?,
        })
    }
    fn byte(&mut self, pointer: &Self::Pointer) -> Result<u8, Self::Error> {
        if !Arc::ptr_eq(&self.identity, &pointer.identity) {
            return Err(SkeletonJumpVaultError::Foreign);
        }
        pointer.data.as_ref().map_or(Ok(0), |data| {
            data.bytes()
                .first()
                .copied()
                .ok_or(SkeletonJumpVaultError::EmptyByte)
        })
    }
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        if !Arc::ptr_eq(&self.identity, &pointer.identity) {
            return Err(SkeletonJumpVaultError::Foreign);
        }
        let Some(data) = &pointer.data else {
            return Ok(0.0);
        };
        let bytes = data.bytes().try_into().map_err(|_| {
            SkeletonJumpVaultError::Runtime(AttributeRuntimeError::ScalarSize {
                size: data.bytes().len(),
            })
        })?;
        Ok(f32::from_be_bytes(bytes))
    }
}
#[cfg(test)]
#[path = "skeleton_processed_jump_events_tests.rs"]
mod tests;
