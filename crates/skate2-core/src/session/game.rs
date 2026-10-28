//! Build-scoped offline game-info and initial stream-focus behavior.
//!
//! The supplied debug reference build performs these operations in separate
//! phases. `Sk8::BE::CommonBackEnd::CommonBackEnd` at `0x825623c8` installs the
//! base-world and provisional-location strings. Later,
//! `Sk8::GameModule::OneOffLoad` at `0x8273add8` registers a fixed training
//! location and conditionally selects it. Still later, `OneOff` calls
//! `Sk8::GameModule::InitStreamerLocation` at `0x8272e888`, which samples the
//! then-current game mode and new-profile flag. The types below keep those
//! sampling points separate.

use std::{collections::HashMap, convert::Infallible, error::Error, fmt, sync::Arc};

/// Base world selected by `CommonBackEnd::CommonBackEnd` at `0x82562434..0x82562444`.
pub const DEBUG_REFERENCE_WORLD_FILE: &str = "BAM";

/// Provisional location selected at `0x82562454`.
pub const DEBUG_REFERENCE_PROVISIONAL_LOCATION: &str = "Z_TrainingStart";

/// Runtime-only location registered by `GameModule::OneOffLoad` at `0x8273b260`.
pub const DEBUG_REFERENCE_FIXED_TRAINING_LOCATION: &str = "Z_TrainingStart_Fixed";

/// Stream directory passed by `GameModule::StartWorldStreamLoad` at `0x8272d148`.
pub const WORLD_STREAM_PATH: &str = "data\\content\\world\\stream";

/// Compiled default installed for `kEnableTextureStream` at `0x82f4e1e0`.
///
/// The load machine still samples the live cVar when simulation XML completes;
/// this value is not a runtime override.
pub const DEBUG_REFERENCE_TEXTURE_STREAM_DEFAULT: bool = true;

/// Raw retry value passed to `EA::Thread::ThreadSleep` at `0x8273b560..0x8273b568`.
pub const ONE_OFF_WORLD_STREAM_RETRY_RAW: u32 = 1;

/// Raw retry value passed by both main-DMO load waits.
///
/// `GameModule::SecondaryLoad01` stores this value at `0x8272d574`, and the
/// later `GameModule::GameLoad` barrier stores the same value at `0x8273c08c`.
/// The two waits deliberately remain distinct from the later BAM world-stream
/// loop even though all three happen to yield raw value one.
pub const MAIN_DMO_BANK_RETRY_RAW: u32 = 1;

/// Raw retry value passed by `GameModule::SecondaryLoad02`.
///
/// The asset-activation wait stores this value at `0x8272d654` before calling
/// `EA::Thread::ThreadSleep`. It is a distinct recovered call site from the
/// main-DMO waits even though both pass raw value one.
pub const SECONDARY_LOAD_ASSET_RETRY_RAW: u32 = 1;

/// Nested BIG synchronously loaded by `SubtitleManager::LoadBigSubtitleFile`.
pub const SECONDARY_LOAD_SUBTITLE_BIG_PATH: &str = "data\\fe\\subtitles\\subtitles.big";

/// Camera state graph loaded by `GameModule::GameLoad` after `SecondaryLoad`.
pub const DEBUG_REFERENCE_DEFAULT_CAMERA_STATE_GRAPH_PATH: &str =
    "script/camera/Default_cameragraph.xml";

const REFERENCE_C_STRING_PAYLOAD_LIMIT: usize = 255;
const REFERENCE_WORLD_STREAM_INLINE_NODE_COUNT: usize = 16;

/// Exact words of the `Matrix44Affine` passed at `0x8273b260`.
///
/// The basis vectors come from `gIVector`, `gJVector`, and `gKVector` at
/// `0x8227e8f0..0x8227e91f`. The translation constants are loaded from
/// `0x820dc79c`, `0x820dc798`, and `0x820dc794`, respectively. `OneOff` writes
/// positive zero to the fourth translation lane; it does not copy the XML
/// transform's homogeneous `1.0` lane.
pub const DEBUG_REFERENCE_FIXED_TRAINING_AFFINE_WORDS: [u32; 16] = [
    0x3f80_0000,
    0,
    0,
    0,
    0,
    0x3f80_0000,
    0,
    0,
    0,
    0,
    0x3f80_0000,
    0,
    0xc525_6ddb,
    0x4301_b88c,
    0xc2cb_1ef8,
    0,
];

/// Build-scoped values installed by the supplied debug reference's
/// `CommonBackEnd::CommonBackEnd` constructor.
///
/// This deliberately has no [`Default`] implementation: these literals are
/// proven for the supplied debug executable, not for every Skate 2 build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommonBackEndSeed {
    pub world_file: &'static str,
    pub provisional_location: &'static str,
    /// Actual sData.mCameraSet copied at8256245c..6c.
    pub camera_set: &'static str,
}

impl CommonBackEndSeed {
    /// Returns the exact constructor seed for the supplied debug reference.
    #[must_use]
    pub const fn debug_reference() -> Self {
        Self {
            world_file: DEBUG_REFERENCE_WORLD_FILE,
            provisional_location: DEBUG_REFERENCE_PROVISIONAL_LOCATION,
            camera_set: "Default",
        }
    }
}

/// A runtime four-by-four affine matrix retained as exact floating-point
/// words.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AffineMatrixWords {
    words: [u32; 16],
}

impl AffineMatrixWords {
    #[must_use]
    pub const fn from_words(words: [u32; 16]) -> Self {
        Self { words }
    }

    #[must_use]
    pub const fn words(self) -> [u32; 16] {
        self.words
    }
}

/// One non-streamed location registration performed during `OneOff` load.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NonStreamedLocationRegistration {
    pub name: &'static str,
    pub transform: AffineMatrixWords,
}

/// Build-scoped `OneOff` location work for the supplied debug reference.
///
/// `select_location == None` means preserve the location that existed when
/// `OneOff` reached this phase. It does not mean clear the location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OfflineOneOffPlan {
    pub register_location: NonStreamedLocationRegistration,
    pub select_location: Option<&'static str>,
}

impl OfflineOneOffPlan {
    /// Reconstructs `GameModule::OneOffLoad` at `0x8273b1d4..0x8273b280`.
    ///
    /// The fixed location is registered for both branches. The `kbDebugFE`
    /// test at `0x8273b264..0x8273b270` skips `SetLocationName` only when the
    /// cVar is true.
    #[must_use]
    pub const fn debug_reference(debug_fe: bool) -> Self {
        Self {
            register_location: NonStreamedLocationRegistration {
                name: DEBUG_REFERENCE_FIXED_TRAINING_LOCATION,
                transform: AffineMatrixWords::from_words(
                    DEBUG_REFERENCE_FIXED_TRAINING_AFFINE_WORDS,
                ),
            },
            select_location: if debug_fe {
                None
            } else {
                Some(DEBUG_REFERENCE_FIXED_TRAINING_LOCATION)
            },
        }
    }
}

/// Raw `Sk8::BE::CommonBackEnd::GameMode` value.
///
/// Only values needed by the recovered offline focus predicate are named.
/// Unknown values remain round-trippable instead of being rejected or mapped
/// to a guessed mode.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct GameMode(i32);

impl GameMode {
    /// `GM_NoProfileFreeSkate`, installed at `0x825623f0`.
    pub const NO_PROFILE_FREE_SKATE: Self = Self(0);

    /// `GM_SinglePlayer`, compared at `0x8272e9b8`.
    pub const SINGLE_PLAYER: Self = Self(1);

    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> i32 {
        self.0
    }

    #[must_use]
    pub const fn is_single_player(self) -> bool {
        self.0 == Self::SINGLE_PLAYER.0
    }
}

/// Raw `ProfileBackEnd::tProfileData::mNewProfile` value.
///
/// The PDB declares this field as a four-byte `int`, and
/// `GameModule::InitStreamerLocation` loads it from the owning profile backend
/// at `+0x68` at `0x8272e9c8..0x8272e9d4`. Every nonzero value takes the
/// new-profile branch; this type prevents an earlier ABI-level normalization
/// to `bool` from hiding that behavior.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct NewProfileFlag(i32);

impl NewProfileFlag {
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> i32 {
        self.0
    }

    #[must_use]
    pub const fn is_set(self) -> bool {
        self.0 != 0
    }
}

/// Four exact words backing the reference's 16-byte `Vector3` position.
///
/// Keeping the fourth lane and all IEEE-754 payload bits avoids silently
/// normalizing signed zero, infinities, or NaNs during the focus decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocationPositionWords {
    words: [u32; 4],
}

impl LocationPositionWords {
    /// Positive-zero value constructed at `0x8272e8b4..0x8272e8c8`.
    pub const POSITIVE_ZERO: Self = Self { words: [0; 4] };

    #[must_use]
    pub const fn from_words(words: [u32; 4]) -> Self {
        Self { words }
    }

    #[must_use]
    pub const fn words(self) -> [u32; 4] {
        self.words
    }
}

/// Selects the first offline streamer position from a co-temporal state sample.
///
/// `GameModule::InitStreamerLocation` compares game mode at
/// `0x8272e9b4..0x8272e9bc`, then reads `ProfileBackEnd::mNewProfile` at offset
/// `+0x68` at `0x8272e9c8..0x8272e9d4`. Only the conjunction
/// `GM_SinglePlayer && mNewProfile != 0` bypasses the copy from
/// `GameInfo::locationPosition` at `0x8272e9d8..0x8272e9e8`; the local focus
/// remains the positive-zero value initialized earlier.
///
/// Callers should supply `game_mode` and `new_profile` when this transition is
/// executed, rather than precomputing the predicate during the earlier `OneOff`
/// location-selection phase.
#[must_use]
pub const fn initial_stream_focus(
    game_mode: GameMode,
    new_profile: NewProfileFlag,
    location_position: LocationPositionWords,
) -> LocationPositionWords {
    if game_mode.is_single_player() && new_profile.is_set() {
        LocationPositionWords::POSITIVE_ZERO
    } else {
        location_position
    }
}

/// Raw game-stream handle returned by `cGameAssetManager::LoadGameStreamFile`.
///
/// The reference counter starts at one but increments as an ordinary `u32`, so
/// zero must remain representable after wraparound.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct WorldStreamHandle(u32);

impl WorldStreamHandle {
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Which validated input failed before entering the native stream owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldStreamStringField {
    WorldFile,
    BigFile,
    StreamPath,
}

/// Validation failure for the reference's fixed-size C-string inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorldStreamSpecError {
    InteriorNul {
        field: WorldStreamStringField,
    },
    TooLong {
        field: WorldStreamStringField,
        byte_len: usize,
        maximum: usize,
    },
}

impl fmt::Display for WorldStreamSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InteriorNul { field } => write!(formatter, "{field:?} contains an interior NUL"),
            Self::TooLong {
                field,
                byte_len,
                maximum,
            } => write!(
                formatter,
                "{field:?} is {byte_len} bytes; the validated maximum is {maximum}"
            ),
        }
    }
}

impl Error for WorldStreamSpecError {}

/// Owned path and basename passed to `LoadGameStreamFile` at `0x8272d1ac`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldStreamSpec {
    stream_path: String,
    basename: String,
}

impl WorldStreamSpec {
    /// Reconstructs `GameModule::StartWorldStreamLoad` at `0x8272d0c0` for
    /// inputs that fit its fixed-size C buffers.
    ///
    /// The last dot is removed before the last backslash is searched. Only a
    /// backslash is a path separator. Invalid inputs return an explicit native
    /// error instead of reproducing the reference's buffer overflow or null
    /// pointer behavior.
    ///
    /// # Errors
    ///
    /// Returns an error when either input contains an interior NUL or when a
    /// resulting reference C string would exceed its 255-byte payload.
    pub fn derive(world_file: &str, big_file: Option<&str>) -> Result<Self, WorldStreamSpecError> {
        validate_reference_string(WorldStreamStringField::WorldFile, world_file)?;
        if let Some(big_file) = big_file {
            validate_no_nul(WorldStreamStringField::BigFile, big_file)?;
        }

        let without_extension = world_file
            .rfind('.')
            .map_or(world_file, |dot| &world_file[..dot]);
        let basename = without_extension
            .rfind('\\')
            .map_or(without_extension, |slash| &without_extension[slash + 1..]);
        let stream_path = big_file.map_or_else(
            || WORLD_STREAM_PATH.to_owned(),
            |prefix| format!("{prefix}|{WORLD_STREAM_PATH}"),
        );
        validate_reference_string(WorldStreamStringField::StreamPath, &stream_path)?;

        Ok(Self {
            stream_path,
            basename: basename.to_owned(),
        })
    }

    #[must_use]
    pub fn stream_path(&self) -> &str {
        &self.stream_path
    }

    #[must_use]
    pub fn basename(&self) -> &str {
        &self.basename
    }
}

fn validate_no_nul(field: WorldStreamStringField, value: &str) -> Result<(), WorldStreamSpecError> {
    if value.as_bytes().contains(&0) {
        Err(WorldStreamSpecError::InteriorNul { field })
    } else {
        Ok(())
    }
}

fn validate_reference_string(
    field: WorldStreamStringField,
    value: &str,
) -> Result<(), WorldStreamSpecError> {
    validate_no_nul(field, value)?;
    if value.len() > REFERENCE_C_STRING_PAYLOAD_LIMIT {
        Err(WorldStreamSpecError::TooLong {
            field,
            byte_len: value.len(),
            maximum: REFERENCE_C_STRING_PAYLOAD_LIMIT,
        })
    } else {
        Ok(())
    }
}

/// Exact `AssetPaths::tStreamType` values consumed by `LoadStreamFile`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum WorldStreamType {
    Presentation = 0,
    Simulation = 1,
    Texture = 2,
}

impl WorldStreamType {
    const fn slot(self) -> usize {
        self as usize
    }
}

/// Raw `cStreamFile::mState` sample returned by the inner stream-file pump.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct StreamFileState(u32);

impl StreamFileState {
    /// `eStreamFileState::kStreamFileState_LoadDone` in the reference PDB.
    pub const LOADED: Self = Self(4);

    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    const fn is_loaded(self) -> bool {
        self.0 == Self::LOADED.0
    }
}

/// Raw `cStreamFile::mStreamXmlState` sample.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct StreamXmlState(u32);

impl StreamXmlState {
    /// `eStreamXmlState::kStreamXmlState_LoadDone` in the reference PDB.
    pub const LOAD_DONE: Self = Self(2);

    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    const fn is_load_done(self) -> bool {
        self.0 == Self::LOAD_DONE.0
    }
}

/// Native boundary below the proven outer game-stream machine.
///
/// Implementations own manifest and XSF work. The outer machine relies only on
/// the exact state samples and calls observed at `0x824e1f48`; it does not
/// infer how those formats are decoded.
pub trait WorldStreamBackend {
    type StreamFile;
    type Error;

    /// # Errors
    ///
    /// Returns the backend error if the owned inner file cannot be created.
    fn begin_stream_file(
        &mut self,
        stream_path: &str,
        basename: &str,
        stream_type: WorldStreamType,
    ) -> Result<Self::StreamFile, Self::Error>;

    /// Performs the inner stream-file update and returns its resulting raw
    /// `mState`. Implementations also own the reference inner routine's
    /// registration side effects. Only exact value four advances the outer
    /// machine.
    ///
    /// # Errors
    ///
    /// Returns the backend error if the inner file cannot be advanced or its
    /// post-load side effects cannot be published.
    fn poll_stream_file(
        &mut self,
        stream_file: &mut Self::StreamFile,
    ) -> Result<StreamFileState, Self::Error>;

    /// # Errors
    ///
    /// Returns the backend error if the XML read cannot be started.
    fn begin_stream_xml(
        &mut self,
        stream_file: &mut Self::StreamFile,
        basename: &str,
    ) -> Result<(), Self::Error>;

    fn stream_xml_state(&mut self, stream_file: &mut Self::StreamFile) -> StreamXmlState;

    /// # Errors
    ///
    /// Returns the backend error if the active XML read cannot be advanced.
    fn update_stream_xml(&mut self, stream_file: &mut Self::StreamFile) -> Result<(), Self::Error>;

    /// Samples the live cVar only when simulation XML has completed.
    /// # Errors
    /// Returns the live setting failure without beginning texture loading.
    fn texture_stream_enabled(&mut self) -> Result<bool, Self::Error>;

    fn finalize_world_bounds(&mut self);
}

/// Result of one call to `UpdateGameStreamFileLoaded`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldStreamPoll {
    Pending,
    CompletedThisPoll,
}

/// Fallible collaborator operation attempted by the outer stream controller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldStreamOperation {
    TextureStreamGate,
    BeginStreamFile(WorldStreamType),
    PollStreamFile(WorldStreamType),
    BeginStreamXml(WorldStreamType),
    UpdateStreamXml(WorldStreamType),
}

/// Error returned by the safe outer owner.
///
/// The reference load path has no recoverable error channel. The reconstruction
/// reports collaborator failures explicitly and latches the record after its
/// first failure so partially applied work is never retried. Every later poll
/// returns an `Arc` clone of that original cause.
#[derive(Debug, Eq, PartialEq)]
pub enum WorldStreamControllerError<E> {
    Invariant(WorldStreamInvariantError),
    Backend {
        handle: WorldStreamHandle,
        operation: WorldStreamOperation,
        source: E,
    },
}

impl<E: fmt::Display> fmt::Display for WorldStreamControllerError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invariant(error) => error.fmt(formatter),
            Self::Backend {
                handle,
                operation,
                source,
            } => write!(
                formatter,
                "world-stream handle {} failed during {operation:?}: {source}",
                handle.raw()
            ),
        }
    }
}

impl<E> Error for WorldStreamControllerError<E> where E: Error + 'static {}

/// Raw outer state stored in `cGameAssetManager::tGameStream::mGameStreamState`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct WorldStreamOuterState(u32);

impl WorldStreamOuterState {
    pub const DORMANT: Self = Self(0);
    pub const BEGIN_PRESENTATION: Self = Self(1);
    pub const POLL_PRESENTATION_FILE: Self = Self(2);
    pub const POLL_PRESENTATION_XML: Self = Self(3);
    pub const POLL_SIMULATION_FILE: Self = Self(4);
    pub const POLL_SIMULATION_XML: Self = Self(5);
    pub const POLL_TEXTURE_FILE: Self = Self(6);
    pub const FINALIZE_WORLD_BOUNDS: Self = Self(7);

    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Invariant failure at the native ownership boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldStreamInvariantError {
    MissingRecord {
        handle: WorldStreamHandle,
    },
    InvalidOuterState {
        handle: WorldStreamHandle,
        raw_state: u32,
    },
    MissingStreamFile {
        handle: WorldStreamHandle,
        stream_type: WorldStreamType,
    },
    HandleCollision {
        handle: WorldStreamHandle,
    },
}

impl fmt::Display for WorldStreamInvariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRecord { handle } => {
                write!(formatter, "missing world-stream handle {}", handle.raw())
            }
            Self::InvalidOuterState { handle, raw_state } => write!(
                formatter,
                "world-stream handle {} has invalid outer state {raw_state}",
                handle.raw()
            ),
            Self::MissingStreamFile {
                handle,
                stream_type,
            } => write!(
                formatter,
                "world-stream handle {} is missing its {stream_type:?} file",
                handle.raw()
            ),
            Self::HandleCollision { handle } => {
                write!(formatter, "world-stream handle {} collided", handle.raw())
            }
        }
    }
}

impl Error for WorldStreamInvariantError {}

struct WorldStreamRecord<F, E> {
    spec: WorldStreamSpec,
    state: WorldStreamOuterState,
    files: [Option<F>; 3],
    terminal_error: Option<Arc<WorldStreamControllerError<E>>>,
}

/// Texture-file ownership returned with one completed world-stream load.
///
/// Presentation and Simulation files are mandatory on the exact completed
/// path. Texture is the sole optional file because the live state-five cVar
/// gate can advance directly to finalization without creating it.
#[derive(Debug, Eq, PartialEq)]
pub enum CompletedTextureStream<F> {
    Disabled,
    Loaded(F),
}

/// Exclusive ownership of every file from one completed outer load.
///
/// Exact native completion is the state-seven case of
/// `cGameAssetManager::UpdateGameStreamFileLoaded` at `0x824e1f48`: it
/// calculates world bounds, writes raw state zero, and returns the one-poll
/// completion pulse. Moving the dormant record out for diagnostic-driven
/// explicit unload is a high-confidence safe Rust ownership extension, not a
/// reconstructed native scheduling operation.
#[derive(Debug, Eq, PartialEq)]
pub struct CompletedWorldStreamLoad<F> {
    handle: WorldStreamHandle,
    spec: WorldStreamSpec,
    presentation: F,
    simulation: F,
    texture: CompletedTextureStream<F>,
}

impl<F> CompletedWorldStreamLoad<F> {
    #[must_use]
    pub const fn handle(&self) -> WorldStreamHandle {
        self.handle
    }

    #[must_use]
    pub const fn spec(&self) -> &WorldStreamSpec {
        &self.spec
    }

    #[must_use]
    pub fn presentation_mut(&mut self) -> &mut F {
        &mut self.presentation
    }

    #[must_use]
    pub fn simulation_mut(&mut self) -> &mut F {
        &mut self.simulation
    }

    #[must_use]
    pub fn texture_mut(&mut self) -> Option<&mut F> {
        match &mut self.texture {
            CompletedTextureStream::Disabled => None,
            CompletedTextureStream::Loaded(file) => Some(file),
        }
    }

    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        WorldStreamHandle,
        WorldStreamSpec,
        F,
        F,
        CompletedTextureStream<F>,
    ) {
        (
            self.handle,
            self.spec,
            self.presentation,
            self.simulation,
            self.texture,
        )
    }
}

/// Rejection from atomically taking a completed world-stream record.
#[derive(Debug, Eq, PartialEq)]
pub enum TakeCompletedWorldStreamError<E> {
    Invariant(WorldStreamInvariantError),
    NotCompleted {
        handle: WorldStreamHandle,
        state: WorldStreamOuterState,
    },
    Terminal(Arc<WorldStreamControllerError<E>>),
}

impl<E: fmt::Display> fmt::Display for TakeCompletedWorldStreamError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invariant(error) => error.fmt(formatter),
            Self::NotCompleted { handle, state } => write!(
                formatter,
                "world-stream handle {} is not complete in outer state {}",
                handle.raw(),
                state.raw()
            ),
            Self::Terminal(error) => error.fmt(formatter),
        }
    }
}

impl<E> Error for TakeCompletedWorldStreamError<E> where E: Error + 'static {}

/// Safe native owner for the reference's game-stream records.
///
/// `LoadGameStreamFile` at `0x824dba00` publishes its pointer before filling
/// the strings and initial state. This owner deliberately initializes the full
/// record before insertion, preserving the supported single-owner behavior
/// without reproducing that data race.
/// The reference fixed hash map owns 16 inline nodes and enables heap overflow;
/// the native `HashMap` likewise does not impose a 16-record limit.
pub struct WorldStreamLoads<F, E = Infallible> {
    next_handle: u32,
    records: HashMap<WorldStreamHandle, WorldStreamRecord<F, E>>,
}

impl<F, E> Default for WorldStreamLoads<F, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F, E> WorldStreamLoads<F, E> {
    /// Matches the handle seed written by `cGameAssetManager` at `0x824dfce4`.
    #[must_use]
    pub fn debug_reference() -> Self {
        Self::new()
    }

    /// Constructs the native empty stream record owner with handle seed one.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_handle: 1,
            records: HashMap::with_capacity(REFERENCE_WORLD_STREAM_INLINE_NODE_COUNT),
        }
    }

    /// Creates a fully initialized record in exact outer state one.
    ///
    /// # Errors
    ///
    /// Returns an invariant error when the next wrapping handle collides with
    /// a retained record. This is a deliberate safe deviation from the native
    /// duplicate path, which leaks the new record and returns the old handle.
    pub fn start(
        &mut self,
        spec: WorldStreamSpec,
    ) -> Result<WorldStreamHandle, WorldStreamInvariantError> {
        let handle = WorldStreamHandle::from_raw(self.next_handle);
        if self.records.contains_key(&handle) {
            return Err(WorldStreamInvariantError::HandleCollision { handle });
        }
        self.next_handle = self.next_handle.wrapping_add(1);
        self.records.insert(
            handle,
            WorldStreamRecord {
                spec,
                state: WorldStreamOuterState::BEGIN_PRESENTATION,
                files: std::array::from_fn(|_| None),
                terminal_error: None,
            },
        );
        Ok(handle)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns the record's current raw-preserving outer state.
    ///
    /// # Errors
    ///
    /// Returns an invariant error when `handle` is not owned by this manager.
    pub fn state(
        &self,
        handle: WorldStreamHandle,
    ) -> Result<WorldStreamOuterState, WorldStreamInvariantError> {
        self.records
            .get(&handle)
            .map(|record| record.state)
            .ok_or(WorldStreamInvariantError::MissingRecord { handle })
    }

    /// Atomically moves every file out of an exactly completed dormant record.
    ///
    /// The exact state-seven case at `0x824e1f48` performs final world-bounds
    /// calculation before writing raw state zero and reporting completion.
    /// Therefore state seven is still in progress; only raw zero is eligible.
    /// This method performs no backend call and imposes no unload order.
    ///
    /// # Errors
    ///
    /// Returns the original shared terminal cause for a faulted record, a
    /// typed state/invariant error for an in-progress or malformed record, or
    /// `MissingRecord` after a successful prior take. Every rejection leaves
    /// the record unchanged.
    pub fn take_completed(
        &mut self,
        handle: WorldStreamHandle,
    ) -> Result<CompletedWorldStreamLoad<F>, TakeCompletedWorldStreamError<E>> {
        let record = self
            .records
            .get(&handle)
            .ok_or(TakeCompletedWorldStreamError::Invariant(
                WorldStreamInvariantError::MissingRecord { handle },
            ))?;

        if let Some(error) = record.terminal_error.as_ref() {
            return Err(TakeCompletedWorldStreamError::Terminal(Arc::clone(error)));
        }

        match record.state.raw() {
            0 => {}
            1..=7 => {
                return Err(TakeCompletedWorldStreamError::NotCompleted {
                    handle,
                    state: record.state,
                });
            }
            raw_state => {
                return Err(TakeCompletedWorldStreamError::Invariant(
                    WorldStreamInvariantError::InvalidOuterState { handle, raw_state },
                ));
            }
        }

        for stream_type in [WorldStreamType::Presentation, WorldStreamType::Simulation] {
            if record.files[stream_type.slot()].is_none() {
                return Err(TakeCompletedWorldStreamError::Invariant(
                    WorldStreamInvariantError::MissingStreamFile {
                        handle,
                        stream_type,
                    },
                ));
            }
        }

        let Some(WorldStreamRecord {
            spec,
            files: [Some(presentation), Some(simulation), texture],
            ..
        }) = self.records.remove(&handle)
        else {
            // `&mut self` excludes an intervening mutation after the complete
            // preflight above. Keep the public boundary total if that internal
            // guarantee is ever broken by a future refactor.
            return Err(TakeCompletedWorldStreamError::Invariant(
                WorldStreamInvariantError::MissingRecord { handle },
            ));
        };
        let texture = texture.map_or(
            CompletedTextureStream::Disabled,
            CompletedTextureStream::Loaded,
        );

        Ok(CompletedWorldStreamLoad {
            handle,
            spec,
            presentation,
            simulation,
            texture,
        })
    }

    /// Advances exactly one case of `UpdateGameStreamFileLoaded` at
    /// `0x824e1f48`.
    ///
    /// # Errors
    ///
    /// Returns a controller error for a missing handle, invalid raw outer
    /// state, absent required stream-file slot, or fallible backend operation.
    /// A record that faults is latched; later polls return the same shared
    /// cause without invoking the backend again.
    pub fn poll<B>(
        &mut self,
        handle: WorldStreamHandle,
        backend: &mut B,
    ) -> Result<WorldStreamPoll, Arc<WorldStreamControllerError<E>>>
    where
        B: WorldStreamBackend<StreamFile = F, Error = E>,
    {
        let record = self.records.get_mut(&handle).ok_or_else(|| {
            Arc::new(WorldStreamControllerError::Invariant(
                WorldStreamInvariantError::MissingRecord { handle },
            ))
        })?;

        if let Some(error) = record.terminal_error.as_ref() {
            return Err(Arc::clone(error));
        }

        let result = poll_world_stream_record(record, handle, backend).map_err(Arc::new);
        if let Err(error) = &result {
            record.terminal_error = Some(Arc::clone(error));
        }
        result
    }
}

// Keeping the eight exact raw cases together makes the one-case native dispatch
// auditable against `0x824e1f48`.
#[allow(clippy::too_many_lines)]
fn poll_world_stream_record<F, E, B>(
    record: &mut WorldStreamRecord<F, E>,
    handle: WorldStreamHandle,
    backend: &mut B,
) -> Result<WorldStreamPoll, WorldStreamControllerError<E>>
where
    B: WorldStreamBackend<StreamFile = F, Error = E>,
{
    match record.state.raw() {
        0 => Ok(WorldStreamPoll::Pending),
        1 => {
            let stream_type = WorldStreamType::Presentation;
            let file = backend
                .begin_stream_file(
                    record.spec.stream_path(),
                    record.spec.basename(),
                    stream_type,
                )
                .map_err(|source| WorldStreamControllerError::Backend {
                    handle,
                    operation: WorldStreamOperation::BeginStreamFile(stream_type),
                    source,
                })?;
            record.files[stream_type.slot()] = Some(file);
            record.state = WorldStreamOuterState::POLL_PRESENTATION_FILE;
            Ok(WorldStreamPoll::Pending)
        }
        2 => {
            let stream_type = WorldStreamType::Presentation;
            let basename = record.spec.basename.clone();
            let file = stream_file(record, handle, stream_type)
                .map_err(WorldStreamControllerError::Invariant)?;
            let state = backend.poll_stream_file(file).map_err(|source| {
                WorldStreamControllerError::Backend {
                    handle,
                    operation: WorldStreamOperation::PollStreamFile(stream_type),
                    source,
                }
            })?;
            if state.is_loaded() {
                backend
                    .begin_stream_xml(file, &basename)
                    .map_err(|source| WorldStreamControllerError::Backend {
                        handle,
                        operation: WorldStreamOperation::BeginStreamXml(stream_type),
                        source,
                    })?;
                record.state = WorldStreamOuterState::POLL_PRESENTATION_XML;
            }
            Ok(WorldStreamPoll::Pending)
        }
        3 => {
            let stream_type = WorldStreamType::Presentation;
            let file = stream_file(record, handle, stream_type)
                .map_err(WorldStreamControllerError::Invariant)?;
            if backend.stream_xml_state(file).is_load_done() {
                let simulation_type = WorldStreamType::Simulation;
                let simulation = backend
                    .begin_stream_file(
                        record.spec.stream_path(),
                        record.spec.basename(),
                        simulation_type,
                    )
                    .map_err(|source| WorldStreamControllerError::Backend {
                        handle,
                        operation: WorldStreamOperation::BeginStreamFile(simulation_type),
                        source,
                    })?;
                record.files[simulation_type.slot()] = Some(simulation);
                record.state = WorldStreamOuterState::POLL_SIMULATION_FILE;
            } else {
                backend.update_stream_xml(file).map_err(|source| {
                    WorldStreamControllerError::Backend {
                        handle,
                        operation: WorldStreamOperation::UpdateStreamXml(stream_type),
                        source,
                    }
                })?;
            }
            Ok(WorldStreamPoll::Pending)
        }
        4 => {
            let stream_type = WorldStreamType::Simulation;
            let basename = record.spec.basename.clone();
            let file = stream_file(record, handle, stream_type)
                .map_err(WorldStreamControllerError::Invariant)?;
            let state = backend.poll_stream_file(file).map_err(|source| {
                WorldStreamControllerError::Backend {
                    handle,
                    operation: WorldStreamOperation::PollStreamFile(stream_type),
                    source,
                }
            })?;
            if state.is_loaded() {
                backend
                    .begin_stream_xml(file, &basename)
                    .map_err(|source| WorldStreamControllerError::Backend {
                        handle,
                        operation: WorldStreamOperation::BeginStreamXml(stream_type),
                        source,
                    })?;
                record.state = WorldStreamOuterState::POLL_SIMULATION_XML;
            }
            Ok(WorldStreamPoll::Pending)
        }
        5 => {
            let stream_type = WorldStreamType::Simulation;
            let file = stream_file(record, handle, stream_type)
                .map_err(WorldStreamControllerError::Invariant)?;
            if backend.stream_xml_state(file).is_load_done() {
                if backend.texture_stream_enabled().map_err(|source| {
                    WorldStreamControllerError::Backend {
                        handle,
                        operation: WorldStreamOperation::TextureStreamGate,
                        source,
                    }
                })? {
                    let texture_type = WorldStreamType::Texture;
                    let texture = backend
                        .begin_stream_file(
                            record.spec.stream_path(),
                            record.spec.basename(),
                            texture_type,
                        )
                        .map_err(|source| WorldStreamControllerError::Backend {
                            handle,
                            operation: WorldStreamOperation::BeginStreamFile(texture_type),
                            source,
                        })?;
                    record.files[texture_type.slot()] = Some(texture);
                    record.state = WorldStreamOuterState::POLL_TEXTURE_FILE;
                } else {
                    record.state = WorldStreamOuterState::FINALIZE_WORLD_BOUNDS;
                }
            } else {
                backend.update_stream_xml(file).map_err(|source| {
                    WorldStreamControllerError::Backend {
                        handle,
                        operation: WorldStreamOperation::UpdateStreamXml(stream_type),
                        source,
                    }
                })?;
            }
            Ok(WorldStreamPoll::Pending)
        }
        6 => {
            let stream_type = WorldStreamType::Texture;
            let file = stream_file(record, handle, stream_type)
                .map_err(WorldStreamControllerError::Invariant)?;
            let state = backend.poll_stream_file(file).map_err(|source| {
                WorldStreamControllerError::Backend {
                    handle,
                    operation: WorldStreamOperation::PollStreamFile(stream_type),
                    source,
                }
            })?;
            if state.is_loaded() {
                record.state = WorldStreamOuterState::FINALIZE_WORLD_BOUNDS;
            }
            Ok(WorldStreamPoll::Pending)
        }
        7 => {
            backend.finalize_world_bounds();
            record.state = WorldStreamOuterState::DORMANT;
            Ok(WorldStreamPoll::CompletedThisPoll)
        }
        raw_state => Err(WorldStreamControllerError::Invariant(
            WorldStreamInvariantError::InvalidOuterState { handle, raw_state },
        )),
    }
}

fn stream_file<F, E>(
    record: &mut WorldStreamRecord<F, E>,
    handle: WorldStreamHandle,
    stream_type: WorldStreamType,
) -> Result<&mut F, WorldStreamInvariantError> {
    record.files[stream_type.slot()]
        .as_mut()
        .ok_or(WorldStreamInvariantError::MissingStreamFile {
            handle,
            stream_type,
        })
}

/// Narrow runtime port used by the two recovered main-DMO load waits.
///
/// The native methods are `void`/`bool`. A native PC collaborator may expose
/// a checked error channel, but the driver preserves the reference's call
/// order and performs no retry or rollback after such an error.
pub trait MainDmoBankLoadRuntime {
    type Error;

    /// Calls `cDMOBankManager::LoadMainDMOBank` (`0x82e0e628`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure while issuing the request.
    fn load_main_dmo_bank(&mut self) -> Result<(), Self::Error>;

    /// Calls `cDMOBankManager::IsMainDMOBankLoaded` (`0x82e0e698`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure while sampling loaded state.
    fn is_main_dmo_bank_loaded(&mut self) -> Result<bool, Self::Error>;

    /// Calls `cDMOBankManager::Update` (`0x82e0e720`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure while advancing the manager.
    fn update_main_dmo_bank(&mut self) -> Result<(), Self::Error>;
}

/// Runs the main-DMO wait shared by `SecondaryLoad01` and `GameLoad`.
///
/// Both callers check loaded before entering the loop. Every false result is
/// followed by exactly one manager update, one raw-value-one sleep, and only
/// then another loaded check. Consequently the sleep still occurs when that
/// update made the bank loaded. This differs from the later world-stream loop,
/// whose completion poll bypasses its retry sleep.
///
/// # Errors
///
/// Returns the first checked runtime error without inventing a native retry or
/// cleanup path.
pub fn wait_for_main_dmo_bank<R, Yield>(
    runtime: &mut R,
    mut yield_after_update: Yield,
) -> Result<(), R::Error>
where
    R: MainDmoBankLoadRuntime,
    Yield: FnMut(u32),
{
    while !runtime.is_main_dmo_bank_loaded()? {
        runtime.update_main_dmo_bank()?;
        yield_after_update(MAIN_DMO_BANK_RETRY_RAW);
    }
    Ok(())
}

/// Runs the main-DMO portion of `GameModule::SecondaryLoad01`.
///
/// The load request at `0x8272d550` is issued exactly once before the shared
/// immediate-check/update/sleep/recheck wait at `0x8272d554..0x8272d590`.
/// Subtitle mounting and diagnostic logging surrounding this slice remain
/// outside this narrow driver.
///
/// # Errors
///
/// Returns the first checked runtime error without invoking the wait after a
/// rejected load request.
pub fn run_secondary_load_main_dmo_bank<R, Yield>(
    runtime: &mut R,
    yield_after_update: Yield,
) -> Result<(), R::Error>
where
    R: MainDmoBankLoadRuntime,
    Yield: FnMut(u32),
{
    runtime.load_main_dmo_bank()?;
    wait_for_main_dmo_bank(runtime, yield_after_update)
}

/// Narrow runtime port for `GameModule::SecondaryLoad02` (`0x8272d5a8`).
///
/// The reference passes the resource allocator to
/// `cGlobalFileLocationManager::LoadGlobalLocations`, pumps that same manager's
/// inherited `cAssetActivationManager` interface, and finally invokes
/// `Sk8::Audio::AudioSystem::SecondaryLoad` through virtual slot `+0x10` of the
/// singleton's primary `IAudioInterface` vtable.
pub trait SecondaryLoadGlobalLocationsRuntime {
    type Error;

    /// Calls `cGlobalFileLocationManager::LoadGlobalLocations` (`0x82a8f3b0`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure while issuing the load.
    fn load_global_locations(&mut self) -> Result<(), Self::Error>;

    /// Calls `cAssetActivationManager::IsPendingRequests` (`0x82d54870`).
    ///
    /// The low byte is retained instead of normalizing it to Rust `bool`: the
    /// PowerPC caller continues its loop only when that byte is exactly one.
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure while sampling pending state.
    fn pending_asset_activation_requests_raw(&mut self) -> Result<u8, Self::Error>;

    /// Calls `cAssetActivationManager::Update` (`0x82d5d8e0`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure while advancing pending work.
    fn update_asset_activation_requests(&mut self) -> Result<(), Self::Error>;

    /// Calls `Sk8::Audio::AudioSystem::SecondaryLoad` (`0x824fe178`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure from the audio collaborator.
    fn secondary_load_audio(&mut self) -> Result<(), Self::Error>;
}

/// Runs the recovered body of `GameModule::SecondaryLoad02`.
///
/// The global-location request occurs once, followed by an immediate pending
/// check. Each raw-one result causes exactly one activation update and one
/// raw-value-one sleep before the next check. A zero or any noncanonical byte
/// other than one exits the loop and invokes `AudioSystem`'s secondary load.
/// This exact-one predicate mirrors `clrlwi`/`cmplwi 1` at
/// `0x8272d63c..0x8272d670`; it is intentionally not a generic nonzero test.
///
/// # Errors
///
/// Returns the first checked runtime error without inventing a native retry or
/// cleanup path.
pub fn run_secondary_load_global_locations<R, Sleep>(
    runtime: &mut R,
    mut sleep_after_update: Sleep,
) -> Result<(), R::Error>
where
    R: SecondaryLoadGlobalLocationsRuntime,
    Sleep: FnMut(u32),
{
    runtime.load_global_locations()?;
    while runtime.pending_asset_activation_requests_raw()? == 1 {
        runtime.update_asset_activation_requests()?;
        sleep_after_update(SECONDARY_LOAD_ASSET_RETRY_RAW);
    }
    runtime.secondary_load_audio()
}

/// Complete gameplay collaborators called by the direct `SecondaryLoad` group.
///
/// The two inherited ports retain their independently recovered request/wait
/// contracts. This outer seam adds only the private subtitle-archive load that
/// precedes `SecondaryLoad01` and the recipe initialization performed by
/// `SecondaryLoad03`.
pub trait GameSecondaryLoadRuntime:
    MainDmoBankLoadRuntime
    + SecondaryLoadGlobalLocationsRuntime<Error = <Self as MainDmoBankLoadRuntime>::Error>
{
    /// Calls `SubtitleManager::LoadBigSubtitleFile` (`0x826aae08`).
    ///
    /// `path` is [`SECONDARY_LOAD_SUBTITLE_BIG_PATH`]. The recovered manager
    /// synchronously loads, parses, and retains the archive in its private
    /// ordered list; this is not a global VFS mount.
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure from the mount collaborator.
    fn load_big_subtitle_file(
        &mut self,
        path: &'static str,
    ) -> Result<(), <Self as MainDmoBankLoadRuntime>::Error>;

    /// Calls `LivingWorld::RecipeMan::Init` (`0x827289f8`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure from the recipe collaborator.
    fn initialize_living_world_recipes(
        &mut self,
    ) -> Result<(), <Self as MainDmoBankLoadRuntime>::Error>;
}

/// Runs the substantive calls of the direct four-part `SecondaryLoad` group.
///
/// `GameModule::GameLoad` executes these operations in order after storing its
/// `+0x5c` byte: private subtitle BIG load; `SecondaryLoad01` main-DMO request and wait;
/// `SecondaryLoad02` global-location request, activation wait, and audio
/// secondary load; then `SecondaryLoad03` recipe initialization. The compiled
/// fourth helper emits only diagnostic strings and therefore adds no gameplay
/// collaborator here.
///
/// Both recovered waits use the same sleep callback but retain their distinct
/// raw-one constants and predicates. Checked errors stop at the exact failed
/// boundary without rolling back preceding native-void effects.
///
/// # Errors
///
/// Returns the first checked failure from the subtitle load, DMO, global-location,
/// activation, audio, or recipe collaborator.
pub fn run_game_secondary_load_group<R, Sleep>(
    runtime: &mut R,
    mut sleep_after_update: Sleep,
) -> Result<(), <R as MainDmoBankLoadRuntime>::Error>
where
    R: GameSecondaryLoadRuntime,
    Sleep: FnMut(u32),
{
    runtime.load_big_subtitle_file(SECONDARY_LOAD_SUBTITLE_BIG_PATH)?;
    run_secondary_load_main_dmo_bank(runtime, &mut sleep_after_update)?;
    run_secondary_load_global_locations(runtime, &mut sleep_after_update)?;
    runtime.initialize_living_world_recipes()
}

/// Unconditional setup between the optional `SecondaryLoad` group and the later
/// main-DMO readiness barrier in `GameModule::GameLoad`.
pub trait GameLoadPreDmoBarrierRuntime: GameSecondaryLoadRuntime {
    /// Requests the state graph named
    /// `script/camera/Default_cameragraph.xml` with the raw false flag.
    ///
    /// The cache may satisfy that logical request from a sibling compiled
    /// `.stategraph`; this boundary does not claim a direct physical XML read.
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure from the state-graph cache.
    fn load_default_camera_state_graph(
        &mut self,
        path: &'static str,
        flag: bool,
    ) -> Result<(), <Self as MainDmoBankLoadRuntime>::Error>;

    /// Calls `LivingWorld::Manager::PreLoadStateGraphs` (`0x82724280`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure from the `LivingWorld` manager.
    fn preload_living_world_state_graphs(
        &mut self,
    ) -> Result<(), <Self as MainDmoBankLoadRuntime>::Error>;
}

/// Runs `GameLoad` from its `+0x5c` branch through the later DMO barrier.
///
/// Raw zero is changed to one before the complete `SecondaryLoad` group. Any
/// nonzero byte is preserved and skips that group. Both paths then load the
/// default camera state graph, preload `LivingWorld` state graphs, and finally
/// execute the independent no-request main-DMO wait. The two unconditional
/// setup calls therefore cannot be folded into the zero-only group.
///
/// # Errors
///
/// Returns the first checked collaborator failure. The native stores and
/// preceding void effects are not rolled back.
pub fn run_game_load_through_main_dmo_barrier<R, Sleep>(
    secondary_load_byte: &mut u8,
    runtime: &mut R,
    mut sleep_after_update: Sleep,
) -> Result<(), <R as MainDmoBankLoadRuntime>::Error>
where
    R: GameLoadPreDmoBarrierRuntime,
    Sleep: FnMut(u32),
{
    if *secondary_load_byte == 0 {
        *secondary_load_byte = 1;
        run_game_secondary_load_group(runtime, &mut sleep_after_update)?;
    }
    runtime
        .load_default_camera_state_graph(DEBUG_REFERENCE_DEFAULT_CAMERA_STATE_GRAPH_PATH, false)?;
    runtime.preload_living_world_state_graphs()?;
    wait_for_main_dmo_bank(runtime, sleep_after_update)
}

/// Collaborators selected by `GameModule::backgroundSecondaryLoad`.
///
/// The native worker receives its owning `GameModule`, reads the raw word at
/// `+0x84`, advances that word, and invokes exactly one helper per entry.
pub trait BackgroundSecondaryLoadRuntime {
    type Error;

    /// Invokes `GameModule::SecondaryLoad01` (`0x8272d480`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure from the first helper.
    fn secondary_load01(&mut self, stored_state: u32) -> Result<(), Self::Error>;

    /// Invokes `GameModule::SecondaryLoad02` (`0x8272d5a8`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure from the second helper.
    fn secondary_load02(&mut self, stored_state: u32) -> Result<(), Self::Error>;

    /// Invokes `GameModule::SecondaryLoad03` (`0x8272d6b8`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure from the third helper.
    fn secondary_load03(&mut self, stored_state: u32) -> Result<(), Self::Error>;
}

/// Failure from one invocation of [`run_background_secondary_load`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackgroundSecondaryLoadError<E> {
    /// The `GameModule +0x84` word was not one of the four worker-owned states.
    InvalidState { raw_state: u32 },
    /// A checked native-host collaborator failed after its state was advanced.
    Runtime(E),
}

impl<E: fmt::Display> fmt::Display for BackgroundSecondaryLoadError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState { raw_state } => {
                write!(
                    formatter,
                    "invalid background secondary-load state {raw_state}"
                )
            }
            Self::Runtime(source) => {
                write!(formatter, "background secondary-load failed: {source}")
            }
        }
    }
}

impl<E: Error + 'static> Error for BackgroundSecondaryLoadError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidState { .. } => None,
            Self::Runtime(source) => Some(source),
        }
    }
}

/// Runs one exact dispatch of `GameModule::backgroundSecondaryLoad`.
///
/// The recovered sparse transitions are `5 -> 6` before `SecondaryLoad01`,
/// `8 -> 9` before `SecondaryLoad02`, `11 -> 12` before `SecondaryLoad03`, and
/// `14 -> 15` before `SecondaryLoad04`. The fourth helper only emits diagnostic
/// start/end strings in this build, so the typed reconstruction retains its
/// state transition without inventing a gameplay-side collaborator. Any other
/// raw value reaches the native assertion at `0x8272d81c`.
///
/// Each collaborator receives the newly stored word so adapters and tests can
/// observe the native store-before-call ordering without aliasing owner state.
///
/// A checked host error does not roll the state back because every native store
/// precedes its corresponding call.
///
/// # Errors
///
/// Returns [`BackgroundSecondaryLoadError::InvalidState`] for a word outside
/// the four recovered cases, or wraps the first checked collaborator failure.
pub fn run_background_secondary_load<R>(
    raw_state: &mut u32,
    runtime: &mut R,
) -> Result<(), BackgroundSecondaryLoadError<R::Error>>
where
    R: BackgroundSecondaryLoadRuntime,
{
    match *raw_state {
        5 => {
            *raw_state = 6;
            runtime
                .secondary_load01(*raw_state)
                .map_err(BackgroundSecondaryLoadError::Runtime)
        }
        8 => {
            *raw_state = 9;
            runtime
                .secondary_load02(*raw_state)
                .map_err(BackgroundSecondaryLoadError::Runtime)
        }
        11 => {
            *raw_state = 12;
            runtime
                .secondary_load03(*raw_state)
                .map_err(BackgroundSecondaryLoadError::Runtime)
        }
        14 => {
            *raw_state = 15;
            Ok(())
        }
        raw_state => Err(BackgroundSecondaryLoadError::InvalidState { raw_state }),
    }
}

/// Preserves the two main-DMO barriers in `GameModule::GameLoad`.
///
/// When the byte at `GameModule +0x5c` is zero, the reference stores one before
/// invoking `SecondaryLoad01` and the other secondary-load helpers. Their wider
/// effects remain an explicit caller-owned operation here. After that optional
/// group and the intervening reference work, `GameLoad` always performs the
/// common no-load DMO wait. An already nonzero byte therefore skips the group
/// but does not skip the later readiness barrier.
///
/// The callback receives the stored byte so tests and application adapters can
/// observe that the write preceded the group. Checked callback/runtime errors
/// do not roll the byte back; the native calls expose no recoverable failure
/// branch from which such a rollback could be inferred.
///
/// # Errors
///
/// Returns the first checked secondary-group or DMO-runtime failure.
pub fn run_game_load_main_dmo_barriers<R, Secondary, Sleep>(
    secondary_load_byte: &mut u8,
    runtime: &mut R,
    run_secondary_load_group: Secondary,
    mut sleep_after_update: Sleep,
) -> Result<(), R::Error>
where
    R: MainDmoBankLoadRuntime,
    Secondary: FnOnce(&mut R, &mut Sleep, u8) -> Result<(), R::Error>,
    Sleep: FnMut(u32),
{
    if *secondary_load_byte == 0 {
        *secondary_load_byte = 1;
        run_secondary_load_group(runtime, &mut sleep_after_update, *secondary_load_byte)?;
    }
    wait_for_main_dmo_bank(runtime, sleep_after_update)
}

/// Runs the reference `OneOff` worker's blocking world-stream segment.
///
/// The callbacks make the observable ordering explicit: the handle is stored
/// before the immediate first poll, raw retry value one is yielded only after
/// a pending poll, and streamer-location sampling/initialization occurs only
/// after the one-poll completion pulse.
/// This helper ends after that callback. The separate raw-value-one sleep that
/// follows `InitStreamerLocation` in later `OneOffLoad` work is outside this
/// world-stream segment.
///
/// # Errors
///
/// Returns any invariant or backend error raised while creating or polling the
/// owned world-stream record. The location callback is not invoked on error.
pub fn run_offline_one_off_world_stream<F, B, Store, Yield, Init>(
    loads: &mut WorldStreamLoads<F, B::Error>,
    backend: &mut B,
    spec: WorldStreamSpec,
    mut store_handle: Store,
    mut yield_pending: Yield,
    mut init_streamer_location: Init,
) -> Result<WorldStreamHandle, Arc<WorldStreamControllerError<B::Error>>>
where
    B: WorldStreamBackend<StreamFile = F>,
    Store: FnMut(WorldStreamHandle),
    Yield: FnMut(u32),
    Init: FnMut(),
{
    let handle = loads
        .start(spec)
        .map_err(|error| Arc::new(WorldStreamControllerError::Invariant(error)))?;
    store_handle(handle);
    while let WorldStreamPoll::Pending = loads.poll(handle, backend)? {
        yield_pending(ONE_OFF_WORLD_STREAM_RETRY_RAW);
    }
    init_streamer_location();
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    const UNUSUAL_POSITION: LocationPositionWords =
        LocationPositionWords::from_words([0x8000_0000, 0x7fc1_2345, 0xff80_0000, 0x7fa0_00a5]);

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum DmoWaitEvent {
        SecondaryGroup(u8),
        Load,
        IsLoaded(bool),
        Update,
        Yield(u32),
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum SecondaryLoad02Event {
        LoadGlobalLocations,
        IsPending(u8),
        UpdateAssetActivation,
        Sleep(u32),
        AudioSecondaryLoad,
    }

    struct MockSecondaryLoad02Runtime {
        events: Rc<RefCell<Vec<SecondaryLoad02Event>>>,
        pending_results: Vec<u8>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum BackgroundSecondaryLoadEvent {
        SecondaryLoad01(u32),
        SecondaryLoad02(u32),
        SecondaryLoad03(u32),
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum SecondaryLoadGroupEvent {
        LoadBigSubtitleFile(&'static str),
        LoadMainDmo,
        IsMainDmoLoaded(bool),
        UpdateMainDmo,
        LoadGlobalLocations,
        IsPending(u8),
        UpdateAssetActivation,
        Sleep(u32),
        AudioSecondaryLoad,
        InitializeRecipes,
        LoadDefaultCameraStateGraph { path: &'static str, flag: bool },
        PreloadLivingWorldStateGraphs,
    }

    struct MockGameSecondaryLoadRuntime {
        events: Rc<RefCell<Vec<SecondaryLoadGroupEvent>>>,
        dmo_loaded_results: Vec<bool>,
        pending_results: Vec<u8>,
    }

    impl MainDmoBankLoadRuntime for MockGameSecondaryLoadRuntime {
        type Error = Infallible;

        fn load_main_dmo_bank(&mut self) -> Result<(), Self::Error> {
            self.events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::LoadMainDmo);
            Ok(())
        }

        fn is_main_dmo_bank_loaded(&mut self) -> Result<bool, Self::Error> {
            let loaded = self.dmo_loaded_results.remove(0);
            self.events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::IsMainDmoLoaded(loaded));
            Ok(loaded)
        }

        fn update_main_dmo_bank(&mut self) -> Result<(), Self::Error> {
            self.events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::UpdateMainDmo);
            Ok(())
        }
    }

    impl SecondaryLoadGlobalLocationsRuntime for MockGameSecondaryLoadRuntime {
        type Error = Infallible;

        fn load_global_locations(&mut self) -> Result<(), Self::Error> {
            self.events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::LoadGlobalLocations);
            Ok(())
        }

        fn pending_asset_activation_requests_raw(&mut self) -> Result<u8, Self::Error> {
            let pending = self.pending_results.remove(0);
            self.events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::IsPending(pending));
            Ok(pending)
        }

        fn update_asset_activation_requests(&mut self) -> Result<(), Self::Error> {
            self.events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::UpdateAssetActivation);
            Ok(())
        }

        fn secondary_load_audio(&mut self) -> Result<(), Self::Error> {
            self.events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::AudioSecondaryLoad);
            Ok(())
        }
    }

    impl GameSecondaryLoadRuntime for MockGameSecondaryLoadRuntime {
        fn load_big_subtitle_file(
            &mut self,
            path: &'static str,
        ) -> Result<(), <Self as MainDmoBankLoadRuntime>::Error> {
            self.events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::LoadBigSubtitleFile(path));
            Ok(())
        }

        fn initialize_living_world_recipes(
            &mut self,
        ) -> Result<(), <Self as MainDmoBankLoadRuntime>::Error> {
            self.events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::InitializeRecipes);
            Ok(())
        }
    }

    impl GameLoadPreDmoBarrierRuntime for MockGameSecondaryLoadRuntime {
        fn load_default_camera_state_graph(
            &mut self,
            path: &'static str,
            flag: bool,
        ) -> Result<(), <Self as MainDmoBankLoadRuntime>::Error> {
            self.events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::LoadDefaultCameraStateGraph { path, flag });
            Ok(())
        }

        fn preload_living_world_state_graphs(
            &mut self,
        ) -> Result<(), <Self as MainDmoBankLoadRuntime>::Error> {
            self.events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::PreloadLivingWorldStateGraphs);
            Ok(())
        }
    }

    struct MockBackgroundSecondaryLoadRuntime {
        events: Vec<BackgroundSecondaryLoadEvent>,
    }

    impl BackgroundSecondaryLoadRuntime for MockBackgroundSecondaryLoadRuntime {
        type Error = Infallible;

        fn secondary_load01(&mut self, stored_state: u32) -> Result<(), Self::Error> {
            self.events
                .push(BackgroundSecondaryLoadEvent::SecondaryLoad01(stored_state));
            Ok(())
        }

        fn secondary_load02(&mut self, stored_state: u32) -> Result<(), Self::Error> {
            self.events
                .push(BackgroundSecondaryLoadEvent::SecondaryLoad02(stored_state));
            Ok(())
        }

        fn secondary_load03(&mut self, stored_state: u32) -> Result<(), Self::Error> {
            self.events
                .push(BackgroundSecondaryLoadEvent::SecondaryLoad03(stored_state));
            Ok(())
        }
    }

    impl SecondaryLoadGlobalLocationsRuntime for MockSecondaryLoad02Runtime {
        type Error = Infallible;

        fn load_global_locations(&mut self) -> Result<(), Self::Error> {
            self.events
                .borrow_mut()
                .push(SecondaryLoad02Event::LoadGlobalLocations);
            Ok(())
        }

        fn pending_asset_activation_requests_raw(&mut self) -> Result<u8, Self::Error> {
            let pending = self.pending_results.remove(0);
            self.events
                .borrow_mut()
                .push(SecondaryLoad02Event::IsPending(pending));
            Ok(pending)
        }

        fn update_asset_activation_requests(&mut self) -> Result<(), Self::Error> {
            self.events
                .borrow_mut()
                .push(SecondaryLoad02Event::UpdateAssetActivation);
            Ok(())
        }

        fn secondary_load_audio(&mut self) -> Result<(), Self::Error> {
            self.events
                .borrow_mut()
                .push(SecondaryLoad02Event::AudioSecondaryLoad);
            Ok(())
        }
    }

    fn secondary_load02_runtime(
        pending_results: Vec<u8>,
    ) -> (
        MockSecondaryLoad02Runtime,
        Rc<RefCell<Vec<SecondaryLoad02Event>>>,
    ) {
        let events = Rc::new(RefCell::new(Vec::new()));
        (
            MockSecondaryLoad02Runtime {
                events: Rc::clone(&events),
                pending_results,
            },
            events,
        )
    }

    struct MockMainDmoBankRuntime {
        events: Rc<RefCell<Vec<DmoWaitEvent>>>,
        updates_remaining: usize,
        load_requested: bool,
    }

    impl MainDmoBankLoadRuntime for MockMainDmoBankRuntime {
        type Error = Infallible;

        fn load_main_dmo_bank(&mut self) -> Result<(), Self::Error> {
            self.events.borrow_mut().push(DmoWaitEvent::Load);
            self.load_requested = true;
            Ok(())
        }

        fn is_main_dmo_bank_loaded(&mut self) -> Result<bool, Self::Error> {
            let loaded = self.load_requested && self.updates_remaining == 0;
            self.events
                .borrow_mut()
                .push(DmoWaitEvent::IsLoaded(loaded));
            Ok(loaded)
        }

        fn update_main_dmo_bank(&mut self) -> Result<(), Self::Error> {
            self.events.borrow_mut().push(DmoWaitEvent::Update);
            self.updates_remaining = self.updates_remaining.saturating_sub(1);
            Ok(())
        }
    }

    fn dmo_wait_runtime(
        updates_remaining: usize,
    ) -> (MockMainDmoBankRuntime, Rc<RefCell<Vec<DmoWaitEvent>>>) {
        let events = Rc::new(RefCell::new(Vec::new()));
        (
            MockMainDmoBankRuntime {
                events: Rc::clone(&events),
                updates_remaining,
                load_requested: false,
            },
            events,
        )
    }

    #[test]
    fn debug_reference_common_backend_seed_is_exact() {
        let seed = CommonBackEndSeed::debug_reference();

        assert_eq!(seed.world_file, "BAM");
        assert_eq!(seed.provisional_location, "Z_TrainingStart");
    }

    #[test]
    fn world_stream_handle_round_trips_all_raw_values() {
        for raw in [0, 1, u32::MAX] {
            assert_eq!(WorldStreamHandle::from_raw(raw).raw(), raw);
        }
    }

    #[test]
    fn raw_world_stream_states_round_trip_unusual_values() {
        for raw in [0, 2, 7, 8, 0x8000_0000, u32::MAX] {
            assert_eq!(WorldStreamOuterState::from_raw(raw).raw(), raw);
            assert_eq!(StreamFileState::from_raw(raw).raw(), raw);
            assert_eq!(StreamXmlState::from_raw(raw).raw(), raw);
        }
    }

    #[test]
    fn secondary_load_checks_immediately_after_its_single_dmo_request() {
        let (mut runtime, events) = dmo_wait_runtime(0);

        run_secondary_load_main_dmo_bank(&mut runtime, |raw| {
            events.borrow_mut().push(DmoWaitEvent::Yield(raw));
        })
        .unwrap();

        assert_eq!(
            *events.borrow(),
            [DmoWaitEvent::Load, DmoWaitEvent::IsLoaded(true)]
        );
    }

    #[test]
    fn secondary_load_sleeps_after_every_pending_dmo_update_before_recheck() {
        let (mut runtime, events) = dmo_wait_runtime(2);

        run_secondary_load_main_dmo_bank(&mut runtime, |raw| {
            events.borrow_mut().push(DmoWaitEvent::Yield(raw));
        })
        .unwrap();

        assert_eq!(
            *events.borrow(),
            [
                DmoWaitEvent::Load,
                DmoWaitEvent::IsLoaded(false),
                DmoWaitEvent::Update,
                DmoWaitEvent::Yield(1),
                DmoWaitEvent::IsLoaded(false),
                DmoWaitEvent::Update,
                // The update above makes the bank loaded, but native
                // SecondaryLoad01 still sleeps before observing that fact.
                DmoWaitEvent::Yield(1),
                DmoWaitEvent::IsLoaded(true),
            ]
        );
    }

    #[test]
    fn secondary_load02_checks_immediately_then_calls_audio_when_not_pending() {
        let (mut runtime, events) = secondary_load02_runtime(vec![0]);

        run_secondary_load_global_locations(&mut runtime, |raw| {
            events.borrow_mut().push(SecondaryLoad02Event::Sleep(raw));
        })
        .unwrap();

        assert_eq!(
            *events.borrow(),
            [
                SecondaryLoad02Event::LoadGlobalLocations,
                SecondaryLoad02Event::IsPending(0),
                SecondaryLoad02Event::AudioSecondaryLoad,
            ]
        );
    }

    #[test]
    fn secondary_load02_sleeps_after_each_raw_one_update_before_recheck() {
        let (mut runtime, events) = secondary_load02_runtime(vec![1, 1, 0]);

        run_secondary_load_global_locations(&mut runtime, |raw| {
            events.borrow_mut().push(SecondaryLoad02Event::Sleep(raw));
        })
        .unwrap();

        assert_eq!(
            *events.borrow(),
            [
                SecondaryLoad02Event::LoadGlobalLocations,
                SecondaryLoad02Event::IsPending(1),
                SecondaryLoad02Event::UpdateAssetActivation,
                SecondaryLoad02Event::Sleep(1),
                SecondaryLoad02Event::IsPending(1),
                SecondaryLoad02Event::UpdateAssetActivation,
                SecondaryLoad02Event::Sleep(1),
                SecondaryLoad02Event::IsPending(0),
                SecondaryLoad02Event::AudioSecondaryLoad,
            ]
        );
    }

    #[test]
    fn secondary_load02_preserves_the_native_exact_one_pending_predicate() {
        let (mut runtime, events) = secondary_load02_runtime(vec![2]);

        run_secondary_load_global_locations(&mut runtime, |raw| {
            events.borrow_mut().push(SecondaryLoad02Event::Sleep(raw));
        })
        .unwrap();

        assert_eq!(
            *events.borrow(),
            [
                SecondaryLoad02Event::LoadGlobalLocations,
                SecondaryLoad02Event::IsPending(2),
                SecondaryLoad02Event::AudioSecondaryLoad,
            ]
        );
    }

    #[test]
    fn direct_secondary_load_group_preserves_all_substantive_helper_order() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = MockGameSecondaryLoadRuntime {
            events: Rc::clone(&events),
            dmo_loaded_results: vec![false, true],
            pending_results: vec![1, 0],
        };

        run_game_secondary_load_group(&mut runtime, |raw| {
            events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::Sleep(raw));
        })
        .unwrap();

        assert_eq!(
            *events.borrow(),
            [
                SecondaryLoadGroupEvent::LoadBigSubtitleFile(SECONDARY_LOAD_SUBTITLE_BIG_PATH,),
                SecondaryLoadGroupEvent::LoadMainDmo,
                SecondaryLoadGroupEvent::IsMainDmoLoaded(false),
                SecondaryLoadGroupEvent::UpdateMainDmo,
                SecondaryLoadGroupEvent::Sleep(1),
                SecondaryLoadGroupEvent::IsMainDmoLoaded(true),
                SecondaryLoadGroupEvent::LoadGlobalLocations,
                SecondaryLoadGroupEvent::IsPending(1),
                SecondaryLoadGroupEvent::UpdateAssetActivation,
                SecondaryLoadGroupEvent::Sleep(1),
                SecondaryLoadGroupEvent::IsPending(0),
                SecondaryLoadGroupEvent::AudioSecondaryLoad,
                SecondaryLoadGroupEvent::InitializeRecipes,
            ]
        );
    }

    #[test]
    fn game_load_runs_unconditional_state_graph_setup_before_second_dmo_barrier() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = MockGameSecondaryLoadRuntime {
            events: Rc::clone(&events),
            dmo_loaded_results: vec![true, true],
            pending_results: vec![0],
        };
        let mut secondary_load_byte = 0;

        run_game_load_through_main_dmo_barrier(&mut secondary_load_byte, &mut runtime, |raw| {
            events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::Sleep(raw));
        })
        .unwrap();

        assert_eq!(secondary_load_byte, 1);
        assert_eq!(
            *events.borrow(),
            [
                SecondaryLoadGroupEvent::LoadBigSubtitleFile(SECONDARY_LOAD_SUBTITLE_BIG_PATH,),
                SecondaryLoadGroupEvent::LoadMainDmo,
                SecondaryLoadGroupEvent::IsMainDmoLoaded(true),
                SecondaryLoadGroupEvent::LoadGlobalLocations,
                SecondaryLoadGroupEvent::IsPending(0),
                SecondaryLoadGroupEvent::AudioSecondaryLoad,
                SecondaryLoadGroupEvent::InitializeRecipes,
                SecondaryLoadGroupEvent::LoadDefaultCameraStateGraph {
                    path: DEBUG_REFERENCE_DEFAULT_CAMERA_STATE_GRAPH_PATH,
                    flag: false,
                },
                SecondaryLoadGroupEvent::PreloadLivingWorldStateGraphs,
                SecondaryLoadGroupEvent::IsMainDmoLoaded(true),
            ]
        );
    }

    #[test]
    fn nonzero_game_load_still_runs_state_graph_setup_before_pending_dmo_wait() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = MockGameSecondaryLoadRuntime {
            events: Rc::clone(&events),
            dmo_loaded_results: vec![false, true],
            pending_results: Vec::new(),
        };
        let mut secondary_load_byte = 7;

        run_game_load_through_main_dmo_barrier(&mut secondary_load_byte, &mut runtime, |raw| {
            events
                .borrow_mut()
                .push(SecondaryLoadGroupEvent::Sleep(raw));
        })
        .unwrap();

        assert_eq!(secondary_load_byte, 7);
        assert_eq!(
            *events.borrow(),
            [
                SecondaryLoadGroupEvent::LoadDefaultCameraStateGraph {
                    path: DEBUG_REFERENCE_DEFAULT_CAMERA_STATE_GRAPH_PATH,
                    flag: false,
                },
                SecondaryLoadGroupEvent::PreloadLivingWorldStateGraphs,
                SecondaryLoadGroupEvent::IsMainDmoLoaded(false),
                SecondaryLoadGroupEvent::UpdateMainDmo,
                SecondaryLoadGroupEvent::Sleep(1),
                SecondaryLoadGroupEvent::IsMainDmoLoaded(true),
            ]
        );
    }

    #[test]
    fn background_secondary_load_advances_each_sparse_state_before_its_call() {
        let cases = [
            (5, 6, Some(BackgroundSecondaryLoadEvent::SecondaryLoad01(6))),
            (8, 9, Some(BackgroundSecondaryLoadEvent::SecondaryLoad02(9))),
            (
                11,
                12,
                Some(BackgroundSecondaryLoadEvent::SecondaryLoad03(12)),
            ),
            (14, 15, None),
        ];

        for (initial, expected, event) in cases {
            let mut runtime = MockBackgroundSecondaryLoadRuntime { events: Vec::new() };
            let mut raw_state = initial;

            run_background_secondary_load(&mut raw_state, &mut runtime).unwrap();

            assert_eq!(raw_state, expected);
            assert_eq!(runtime.events, event.into_iter().collect::<Vec<_>>());
        }
    }

    #[test]
    fn background_secondary_load_rejects_non_worker_state_without_mutation() {
        let mut runtime = MockBackgroundSecondaryLoadRuntime { events: Vec::new() };
        let mut raw_state = 7;

        assert_eq!(
            run_background_secondary_load(&mut raw_state, &mut runtime),
            Err(BackgroundSecondaryLoadError::InvalidState { raw_state: 7 })
        );
        assert_eq!(raw_state, 7);
        assert!(runtime.events.is_empty());
    }

    #[test]
    fn later_game_load_dmo_barrier_rechecks_without_reissuing_load() {
        let (mut runtime, events) = dmo_wait_runtime(1);
        let secondary_events = Rc::clone(&events);
        let sleep_events = Rc::clone(&events);
        let mut secondary_load_byte = 0;
        run_game_load_main_dmo_barriers(
            &mut secondary_load_byte,
            &mut runtime,
            |runtime, sleep, stored_byte| {
                secondary_events
                    .borrow_mut()
                    .push(DmoWaitEvent::SecondaryGroup(stored_byte));
                run_secondary_load_main_dmo_bank(runtime, sleep)
            },
            |raw| sleep_events.borrow_mut().push(DmoWaitEvent::Yield(raw)),
        )
        .unwrap();

        assert_eq!(secondary_load_byte, 1);
        assert_eq!(
            *events.borrow(),
            [
                DmoWaitEvent::SecondaryGroup(1),
                DmoWaitEvent::Load,
                DmoWaitEvent::IsLoaded(false),
                DmoWaitEvent::Update,
                DmoWaitEvent::Yield(1),
                DmoWaitEvent::IsLoaded(true),
                DmoWaitEvent::IsLoaded(true),
            ]
        );
    }

    #[test]
    fn already_secondary_loaded_path_skips_group_but_keeps_later_dmo_wait() {
        let (mut runtime, events) = dmo_wait_runtime(1);
        runtime.load_requested = true;
        let sleep_events = Rc::clone(&events);
        let mut secondary_load_byte = 7;

        run_game_load_main_dmo_barriers(
            &mut secondary_load_byte,
            &mut runtime,
            |_, _, _| panic!("nonzero secondary-load byte invoked the group"),
            |raw| sleep_events.borrow_mut().push(DmoWaitEvent::Yield(raw)),
        )
        .unwrap();

        assert_eq!(secondary_load_byte, 7);
        assert_eq!(
            *events.borrow(),
            [
                DmoWaitEvent::IsLoaded(false),
                DmoWaitEvent::Update,
                DmoWaitEvent::Yield(1),
                DmoWaitEvent::IsLoaded(true),
            ]
        );
    }

    #[test]
    fn fixed_training_registration_preserves_every_affine_word() {
        let registration = OfflineOneOffPlan::debug_reference(false).register_location;
        let expected = [
            0x3f80_0000,
            0,
            0,
            0,
            0,
            0x3f80_0000,
            0,
            0,
            0,
            0,
            0x3f80_0000,
            0,
            0xc525_6ddb,
            0x4301_b88c,
            0xc2cb_1ef8,
            0,
        ];

        assert_eq!(registration.name, "Z_TrainingStart_Fixed");
        assert_eq!(registration.transform.words(), expected);
        assert_eq!(registration.transform.words()[15], 0x0000_0000);
        assert!(f32::from_bits(registration.transform.words()[15]).is_sign_positive());
    }

    #[test]
    fn one_off_always_registers_fixed_but_debug_fe_preserves_selection() {
        let normal = OfflineOneOffPlan::debug_reference(false);
        let debug = OfflineOneOffPlan::debug_reference(true);

        assert_eq!(normal.register_location, debug.register_location);
        assert_eq!(normal.select_location, Some("Z_TrainingStart_Fixed"));
        assert_eq!(debug.select_location, None);
    }

    #[test]
    fn game_mode_round_trips_unknown_values_and_names_only_single_player() {
        assert_eq!(GameMode::NO_PROFILE_FREE_SKATE.raw(), 0);
        assert!(!GameMode::NO_PROFILE_FREE_SKATE.is_single_player());
        assert_eq!(GameMode::SINGLE_PLAYER.raw(), 1);
        assert!(GameMode::SINGLE_PLAYER.is_single_player());

        for raw in [i32::MIN, -1, 2, 17, i32::MAX] {
            let mode = GameMode::from_raw(raw);
            assert_eq!(mode.raw(), raw);
            assert!(!mode.is_single_player());
        }
    }

    #[test]
    fn only_single_player_new_profile_gets_positive_zero_focus() {
        for raw in [1, -1, i32::MIN, i32::MAX] {
            let flag = NewProfileFlag::from_raw(raw);
            assert_eq!(flag.raw(), raw);
            assert!(flag.is_set());
            assert_eq!(
                initial_stream_focus(GameMode::SINGLE_PLAYER, flag, UNUSUAL_POSITION),
                LocationPositionWords::POSITIVE_ZERO
            );
        }

        let clear = NewProfileFlag::from_raw(0);
        assert_eq!(clear.raw(), 0);
        assert!(!clear.is_set());
        assert_eq!(
            initial_stream_focus(GameMode::SINGLE_PLAYER, clear, UNUSUAL_POSITION),
            UNUSUAL_POSITION
        );
        assert_eq!(
            initial_stream_focus(
                GameMode::NO_PROFILE_FREE_SKATE,
                NewProfileFlag::from_raw(-1),
                UNUSUAL_POSITION,
            ),
            UNUSUAL_POSITION
        );
        assert_eq!(
            initial_stream_focus(
                GameMode::from_raw(37),
                NewProfileFlag::from_raw(1),
                UNUSUAL_POSITION,
            ),
            UNUSUAL_POSITION
        );
        assert_eq!(
            LocationPositionWords::POSITIVE_ZERO.words(),
            [0x0000_0000; 4]
        );
    }

    #[test]
    fn location_position_copy_preserves_unusual_ieee_words() {
        let copied = initial_stream_focus(
            GameMode::from_raw(-37),
            NewProfileFlag::from_raw(0),
            UNUSUAL_POSITION,
        );

        assert_eq!(
            copied.words(),
            [0x8000_0000, 0x7fc1_2345, 0xff80_0000, 0x7fa0_00a5]
        );
    }

    #[test]
    fn debug_reference_boot_slice_does_not_inject_robs_park() {
        let seed = CommonBackEndSeed::debug_reference();
        let normal = OfflineOneOffPlan::debug_reference(false);
        let debug = OfflineOneOffPlan::debug_reference(true);

        assert_ne!(seed.world_file, "A_RobsPark");
        assert_ne!(seed.provisional_location, "A_RobsPark");
        assert_ne!(normal.register_location.name, "A_RobsPark");
        assert_ne!(normal.select_location, Some("A_RobsPark"));
        assert_ne!(debug.select_location, Some("A_RobsPark"));
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum StreamEvent {
        BeginFile(WorldStreamType, String, String),
        PollFile(WorldStreamType, u32),
        BeginXml(WorldStreamType, String),
        ReadXml(WorldStreamType, u32),
        UpdateXml(WorldStreamType),
        TextureGate(bool),
        FinalizeWorldBounds,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum WorkerEvent {
        Store(u32),
        Stream(StreamEvent),
        Yield(u32),
        Init([u32; 4]),
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct MockStreamFile {
        stream_type: WorldStreamType,
    }

    struct MockStreamBackend {
        events: Rc<RefCell<Vec<StreamEvent>>>,
        worker_events: Option<Rc<RefCell<Vec<WorkerEvent>>>>,
        file_pending_polls: [usize; 3],
        file_terminal_states: [StreamFileState; 3],
        xml_pending_updates: [usize; 2],
        xml_pending_states: [StreamXmlState; 2],
        texture_enabled: bool,
        fail_operation: Option<WorldStreamOperation>,
    }

    impl MockStreamBackend {
        fn ready(texture_enabled: bool) -> Self {
            Self {
                events: Rc::new(RefCell::new(Vec::new())),
                worker_events: None,
                file_pending_polls: [0; 3],
                file_terminal_states: [StreamFileState::LOADED; 3],
                xml_pending_updates: [0; 2],
                xml_pending_states: [StreamXmlState::from_raw(1); 2],
                texture_enabled,
                fail_operation: None,
            }
        }

        fn record(&self, event: StreamEvent) {
            if let Some(worker_events) = &self.worker_events {
                worker_events
                    .borrow_mut()
                    .push(WorkerEvent::Stream(event.clone()));
            }
            self.events.borrow_mut().push(event);
        }
    }

    impl WorldStreamBackend for MockStreamBackend {
        type StreamFile = MockStreamFile;
        type Error = WorldStreamOperation;

        fn begin_stream_file(
            &mut self,
            stream_path: &str,
            basename: &str,
            stream_type: WorldStreamType,
        ) -> Result<Self::StreamFile, Self::Error> {
            let operation = WorldStreamOperation::BeginStreamFile(stream_type);
            if self.fail_operation == Some(operation) {
                return Err(operation);
            }
            self.record(StreamEvent::BeginFile(
                stream_type,
                stream_path.to_owned(),
                basename.to_owned(),
            ));
            Ok(MockStreamFile { stream_type })
        }

        fn poll_stream_file(
            &mut self,
            stream_file: &mut Self::StreamFile,
        ) -> Result<StreamFileState, Self::Error> {
            let operation = WorldStreamOperation::PollStreamFile(stream_file.stream_type);
            if self.fail_operation == Some(operation) {
                return Err(operation);
            }
            let slot = stream_file.stream_type.slot();
            let state = if self.file_pending_polls[slot] == 0 {
                self.file_terminal_states[slot]
            } else {
                self.file_pending_polls[slot] -= 1;
                StreamFileState::from_raw(2)
            };
            self.record(StreamEvent::PollFile(stream_file.stream_type, state.raw()));
            Ok(state)
        }

        fn begin_stream_xml(
            &mut self,
            stream_file: &mut Self::StreamFile,
            basename: &str,
        ) -> Result<(), Self::Error> {
            let operation = WorldStreamOperation::BeginStreamXml(stream_file.stream_type);
            if self.fail_operation == Some(operation) {
                return Err(operation);
            }
            self.record(StreamEvent::BeginXml(
                stream_file.stream_type,
                basename.to_owned(),
            ));
            Ok(())
        }

        fn stream_xml_state(&mut self, stream_file: &mut Self::StreamFile) -> StreamXmlState {
            let slot = stream_file.stream_type.slot();
            let state = if self.xml_pending_updates[slot] == 0 {
                StreamXmlState::LOAD_DONE
            } else {
                self.xml_pending_states[slot]
            };
            self.record(StreamEvent::ReadXml(stream_file.stream_type, state.raw()));
            state
        }

        fn update_stream_xml(
            &mut self,
            stream_file: &mut Self::StreamFile,
        ) -> Result<(), Self::Error> {
            let operation = WorldStreamOperation::UpdateStreamXml(stream_file.stream_type);
            if self.fail_operation == Some(operation) {
                return Err(operation);
            }
            let slot = stream_file.stream_type.slot();
            self.xml_pending_updates[slot] -= 1;
            self.record(StreamEvent::UpdateXml(stream_file.stream_type));
            Ok(())
        }

        fn texture_stream_enabled(&mut self) -> Result<bool, Self::Error> {
            self.record(StreamEvent::TextureGate(self.texture_enabled));
            if self.fail_operation == Some(WorldStreamOperation::TextureStreamGate) {
                return Err(WorldStreamOperation::TextureStreamGate);
            }
            Ok(self.texture_enabled)
        }

        fn finalize_world_bounds(&mut self) {
            self.record(StreamEvent::FinalizeWorldBounds);
        }
    }

    fn bam_spec() -> WorldStreamSpec {
        WorldStreamSpec::derive("BAM", None).unwrap()
    }

    #[test]
    fn live_texture_failure_is_latched_before_any_texture_or_bounds_work() {
        let mut loads = WorldStreamLoads::new();
        let handle = loads.start(bam_spec()).unwrap();
        let mut backend = MockStreamBackend::ready(true);
        for _ in 0..4 {
            assert_eq!(
                loads.poll(handle, &mut backend).unwrap(),
                WorldStreamPoll::Pending
            );
        }
        assert_eq!(loads.state(handle).unwrap().raw(), 5);
        backend.fail_operation = Some(WorldStreamOperation::TextureStreamGate);
        let first = loads.poll(handle, &mut backend).unwrap_err();
        assert!(matches!(
            first.as_ref(),
            WorldStreamControllerError::Backend {
                operation: WorldStreamOperation::TextureStreamGate,
                ..
            }
        ));
        assert_eq!(loads.state(handle).unwrap().raw(), 5);
        let events = backend.events.borrow().clone();
        assert_eq!(events.last(), Some(&StreamEvent::TextureGate(true)));
        backend.fail_operation = None;
        let again = loads.poll(handle, &mut backend).unwrap_err();
        assert!(Arc::ptr_eq(&first, &again));
        assert_eq!(*backend.events.borrow(), events);
    }

    #[test]
    fn allocator_starts_at_one_and_advances_to_two() {
        let mut loads: WorldStreamLoads<MockStreamFile> = WorldStreamLoads::debug_reference();

        let first = loads.start(bam_spec()).unwrap();
        let second = loads.start(bam_spec()).unwrap();

        assert_eq!(first.raw(), 1);
        assert_eq!(second.raw(), 2);
    }

    #[test]
    fn allocator_wraps_from_u32_max_to_zero() {
        let mut loads: WorldStreamLoads<MockStreamFile> = WorldStreamLoads::debug_reference();
        loads.next_handle = u32::MAX;

        let maximum = loads.start(bam_spec()).unwrap();
        let wrapped = loads.start(bam_spec()).unwrap();

        assert_eq!(maximum.raw(), u32::MAX);
        assert_eq!(wrapped.raw(), 0);
    }

    #[test]
    fn allocator_rejects_collision_with_retained_handle() {
        let mut loads: WorldStreamLoads<MockStreamFile> = WorldStreamLoads::debug_reference();
        let retained = loads.start(bam_spec()).unwrap();
        loads.next_handle = retained.raw();

        assert_eq!(
            loads.start(bam_spec()),
            Err(WorldStreamInvariantError::HandleCollision { handle: retained })
        );
        assert_eq!(loads.len(), 1);
        assert_eq!(loads.next_handle, retained.raw());
        assert_eq!(
            loads.state(retained).unwrap(),
            WorldStreamOuterState::BEGIN_PRESENTATION
        );
    }

    #[test]
    fn fixed_hash_map_overflow_accepts_records_beyond_sixteen_inline_nodes() {
        let mut loads: WorldStreamLoads<MockStreamFile> = WorldStreamLoads::debug_reference();

        for expected_raw in 1..=17 {
            assert_eq!(loads.start(bam_spec()).unwrap().raw(), expected_raw);
        }

        assert_eq!(loads.len(), REFERENCE_WORLD_STREAM_INLINE_NODE_COUNT + 1);
        assert_eq!(loads.next_handle, 18);
    }

    #[test]
    fn every_poll_state_that_requires_a_stream_file_rejects_a_missing_slot() {
        let cases = [
            (
                WorldStreamOuterState::POLL_PRESENTATION_FILE,
                WorldStreamType::Presentation,
            ),
            (
                WorldStreamOuterState::POLL_PRESENTATION_XML,
                WorldStreamType::Presentation,
            ),
            (
                WorldStreamOuterState::POLL_SIMULATION_FILE,
                WorldStreamType::Simulation,
            ),
            (
                WorldStreamOuterState::POLL_SIMULATION_XML,
                WorldStreamType::Simulation,
            ),
            (
                WorldStreamOuterState::POLL_TEXTURE_FILE,
                WorldStreamType::Texture,
            ),
        ];

        for (state, stream_type) in cases {
            let mut loads: WorldStreamLoads<MockStreamFile, WorldStreamOperation> =
                WorldStreamLoads::debug_reference();
            let handle = loads.start(bam_spec()).unwrap();
            loads.records.get_mut(&handle).unwrap().state = state;
            let mut backend = MockStreamBackend::ready(true);

            let first = loads.poll(handle, &mut backend).unwrap_err();
            assert_eq!(
                *first,
                WorldStreamControllerError::Invariant(
                    WorldStreamInvariantError::MissingStreamFile {
                        handle,
                        stream_type
                    }
                )
            );
            assert!(backend.events.borrow().is_empty());
            let second = loads.poll(handle, &mut backend).unwrap_err();
            assert!(Arc::ptr_eq(&first, &second));
        }
    }

    #[test]
    fn every_fallible_backend_operation_latches_its_first_fault() {
        let cases = [
            (
                WorldStreamOuterState::BEGIN_PRESENTATION,
                WorldStreamOperation::BeginStreamFile(WorldStreamType::Presentation),
                None,
            ),
            (
                WorldStreamOuterState::POLL_PRESENTATION_FILE,
                WorldStreamOperation::PollStreamFile(WorldStreamType::Presentation),
                Some(WorldStreamType::Presentation),
            ),
            (
                WorldStreamOuterState::POLL_PRESENTATION_FILE,
                WorldStreamOperation::BeginStreamXml(WorldStreamType::Presentation),
                Some(WorldStreamType::Presentation),
            ),
            (
                WorldStreamOuterState::POLL_PRESENTATION_XML,
                WorldStreamOperation::UpdateStreamXml(WorldStreamType::Presentation),
                Some(WorldStreamType::Presentation),
            ),
            (
                WorldStreamOuterState::POLL_PRESENTATION_XML,
                WorldStreamOperation::BeginStreamFile(WorldStreamType::Simulation),
                Some(WorldStreamType::Presentation),
            ),
            (
                WorldStreamOuterState::POLL_SIMULATION_FILE,
                WorldStreamOperation::PollStreamFile(WorldStreamType::Simulation),
                Some(WorldStreamType::Simulation),
            ),
            (
                WorldStreamOuterState::POLL_SIMULATION_FILE,
                WorldStreamOperation::BeginStreamXml(WorldStreamType::Simulation),
                Some(WorldStreamType::Simulation),
            ),
            (
                WorldStreamOuterState::POLL_SIMULATION_XML,
                WorldStreamOperation::UpdateStreamXml(WorldStreamType::Simulation),
                Some(WorldStreamType::Simulation),
            ),
            (
                WorldStreamOuterState::POLL_SIMULATION_XML,
                WorldStreamOperation::BeginStreamFile(WorldStreamType::Texture),
                Some(WorldStreamType::Simulation),
            ),
            (
                WorldStreamOuterState::POLL_TEXTURE_FILE,
                WorldStreamOperation::PollStreamFile(WorldStreamType::Texture),
                Some(WorldStreamType::Texture),
            ),
        ];

        for (state, operation, required_file) in cases {
            let mut loads = WorldStreamLoads::debug_reference();
            let handle = loads.start(bam_spec()).unwrap();
            let record = loads.records.get_mut(&handle).unwrap();
            record.state = state;
            if let Some(stream_type) = required_file {
                record.files[stream_type.slot()] = Some(MockStreamFile { stream_type });
            }
            let mut backend = MockStreamBackend::ready(true);
            backend.fail_operation = Some(operation);
            if let WorldStreamOperation::UpdateStreamXml(stream_type) = operation {
                backend.xml_pending_updates[stream_type.slot()] = 1;
            }

            let first = loads.poll(handle, &mut backend).unwrap_err();
            assert_eq!(
                *first,
                WorldStreamControllerError::Backend {
                    handle,
                    operation,
                    source: operation,
                }
            );
            let event_count = backend.events.borrow().len();
            backend.fail_operation = None;
            let second = loads.poll(handle, &mut backend).unwrap_err();
            assert!(Arc::ptr_eq(&first, &second));
            assert_eq!(backend.events.borrow().len(), event_count);
            assert_eq!(loads.state(handle).unwrap(), state);
        }
    }

    fn poll_until_complete(
        loads: &mut WorldStreamLoads<MockStreamFile, WorldStreamOperation>,
        backend: &mut MockStreamBackend,
        handle: WorldStreamHandle,
    ) -> Vec<WorldStreamPoll> {
        let mut outcomes = Vec::new();
        loop {
            let outcome = loads.poll(handle, backend).unwrap();
            outcomes.push(outcome);
            if outcome == WorldStreamPoll::CompletedThisPoll {
                return outcomes;
            }
        }
    }

    #[test]
    fn completed_take_moves_enabled_files_without_backend_calls_or_trace_changes() {
        let mut loads = WorldStreamLoads::debug_reference();
        let spec = bam_spec();
        let handle = loads.start(spec.clone()).unwrap();
        let mut backend = MockStreamBackend::ready(true);

        let outcomes = poll_until_complete(&mut loads, &mut backend, handle);
        let events_before_take = backend.events.borrow().clone();
        let mut completed = loads.take_completed(handle).unwrap();

        assert_eq!(outcomes.len(), 7);
        assert_eq!(outcomes.last(), Some(&WorldStreamPoll::CompletedThisPoll));
        assert_eq!(*backend.events.borrow(), events_before_take);
        assert_eq!(completed.handle(), handle);
        assert_eq!(completed.spec(), &spec);
        assert_eq!(
            completed.presentation_mut().stream_type,
            WorldStreamType::Presentation
        );
        assert_eq!(
            completed.simulation_mut().stream_type,
            WorldStreamType::Simulation
        );
        assert_eq!(
            completed.texture_mut().map(|file| file.stream_type),
            Some(WorldStreamType::Texture)
        );
        assert!(loads.is_empty());

        assert_eq!(
            loads.take_completed(handle),
            Err(TakeCompletedWorldStreamError::Invariant(
                WorldStreamInvariantError::MissingRecord { handle }
            ))
        );
        assert_eq!(
            loads.state(handle),
            Err(WorldStreamInvariantError::MissingRecord { handle })
        );
        let events_before_missing_poll = backend.events.borrow().clone();
        assert_eq!(
            *loads.poll(handle, &mut backend).unwrap_err(),
            WorldStreamControllerError::Invariant(WorldStreamInvariantError::MissingRecord {
                handle
            })
        );
        assert_eq!(*backend.events.borrow(), events_before_missing_poll);

        let next = loads.start(bam_spec()).unwrap();
        assert_eq!(next.raw(), handle.raw() + 1);

        let (taken_handle, taken_spec, presentation, simulation, texture) = completed.into_parts();
        assert_eq!(taken_handle, handle);
        assert_eq!(taken_spec, spec);
        assert_eq!(presentation.stream_type, WorldStreamType::Presentation);
        assert_eq!(simulation.stream_type, WorldStreamType::Simulation);
        assert_eq!(
            texture,
            CompletedTextureStream::Loaded(MockStreamFile {
                stream_type: WorldStreamType::Texture
            })
        );
    }

    #[test]
    fn completed_take_preserves_disabled_texture_after_an_extra_dormant_poll() {
        let mut loads = WorldStreamLoads::debug_reference();
        let handle = loads.start(bam_spec()).unwrap();
        let mut backend = MockStreamBackend::ready(false);
        assert_eq!(
            poll_until_complete(&mut loads, &mut backend, handle).last(),
            Some(&WorldStreamPoll::CompletedThisPoll)
        );
        let events_after_completion = backend.events.borrow().clone();

        assert_eq!(
            loads.poll(handle, &mut backend).unwrap(),
            WorldStreamPoll::Pending
        );
        assert_eq!(*backend.events.borrow(), events_after_completion);
        let mut completed = loads.take_completed(handle).unwrap();

        assert!(completed.texture_mut().is_none());
        assert!(matches!(
            completed.into_parts().4,
            CompletedTextureStream::Disabled
        ));
        assert_eq!(*backend.events.borrow(), events_after_completion);
    }

    #[test]
    fn completed_take_rejects_every_in_progress_state_without_mutation() {
        for raw_state in 1..=7 {
            let mut loads: WorldStreamLoads<MockStreamFile, WorldStreamOperation> =
                WorldStreamLoads::debug_reference();
            let handle = loads.start(bam_spec()).unwrap();
            let record = loads.records.get_mut(&handle).unwrap();
            record.state = WorldStreamOuterState::from_raw(raw_state);
            record.files[WorldStreamType::Presentation.slot()] = Some(MockStreamFile {
                stream_type: WorldStreamType::Presentation,
            });
            record.files[WorldStreamType::Simulation.slot()] = Some(MockStreamFile {
                stream_type: WorldStreamType::Simulation,
            });

            assert_eq!(
                loads.take_completed(handle),
                Err(TakeCompletedWorldStreamError::NotCompleted {
                    handle,
                    state: WorldStreamOuterState::from_raw(raw_state),
                })
            );
            assert_eq!(loads.len(), 1);
            assert_eq!(
                loads.state(handle).unwrap(),
                WorldStreamOuterState::from_raw(raw_state)
            );
            let retained = loads.records.get(&handle).unwrap();
            assert!(retained.files[WorldStreamType::Presentation.slot()].is_some());
            assert!(retained.files[WorldStreamType::Simulation.slot()].is_some());
        }
    }

    #[test]
    fn state_seven_must_finalize_before_completed_take() {
        let mut loads: WorldStreamLoads<MockStreamFile, WorldStreamOperation> =
            WorldStreamLoads::debug_reference();
        let handle = loads.start(bam_spec()).unwrap();
        let record = loads.records.get_mut(&handle).unwrap();
        record.state = WorldStreamOuterState::FINALIZE_WORLD_BOUNDS;
        record.files[WorldStreamType::Presentation.slot()] = Some(MockStreamFile {
            stream_type: WorldStreamType::Presentation,
        });
        record.files[WorldStreamType::Simulation.slot()] = Some(MockStreamFile {
            stream_type: WorldStreamType::Simulation,
        });
        let mut backend = MockStreamBackend::ready(false);

        assert_eq!(
            loads.take_completed(handle),
            Err(TakeCompletedWorldStreamError::NotCompleted {
                handle,
                state: WorldStreamOuterState::FINALIZE_WORLD_BOUNDS,
            })
        );
        assert!(backend.events.borrow().is_empty());
        assert_eq!(
            loads.poll(handle, &mut backend).unwrap(),
            WorldStreamPoll::CompletedThisPoll
        );
        assert_eq!(
            backend.events.borrow().as_slice(),
            &[StreamEvent::FinalizeWorldBounds]
        );
        assert!(loads.take_completed(handle).is_ok());
    }

    #[test]
    fn completed_take_replays_terminal_arc_and_retains_the_record() {
        let mut loads = WorldStreamLoads::debug_reference();
        let handle = loads.start(bam_spec()).unwrap();
        let mut backend = MockStreamBackend::ready(false);
        backend.fail_operation = Some(WorldStreamOperation::BeginStreamFile(
            WorldStreamType::Presentation,
        ));
        let terminal = loads.poll(handle, &mut backend).unwrap_err();
        let event_count = backend.events.borrow().len();

        let taken_error = loads.take_completed(handle).unwrap_err();
        let TakeCompletedWorldStreamError::Terminal(taken_terminal) = taken_error else {
            panic!("expected the retained terminal error");
        };
        assert!(Arc::ptr_eq(&terminal, &taken_terminal));
        assert_eq!(loads.len(), 1);
        assert_eq!(
            loads.state(handle).unwrap(),
            WorldStreamOuterState::BEGIN_PRESENTATION
        );
        assert_eq!(backend.events.borrow().len(), event_count);
        let replay = loads.poll(handle, &mut backend).unwrap_err();
        assert!(Arc::ptr_eq(&terminal, &replay));
    }

    #[test]
    fn completed_take_validates_required_files_and_invalid_states_atomically() {
        for missing_type in [WorldStreamType::Presentation, WorldStreamType::Simulation] {
            let mut loads: WorldStreamLoads<MockStreamFile, WorldStreamOperation> =
                WorldStreamLoads::debug_reference();
            let handle = loads.start(bam_spec()).unwrap();
            let record = loads.records.get_mut(&handle).unwrap();
            record.state = WorldStreamOuterState::DORMANT;
            for stream_type in [WorldStreamType::Presentation, WorldStreamType::Simulation] {
                if stream_type != missing_type {
                    record.files[stream_type.slot()] = Some(MockStreamFile { stream_type });
                }
            }

            assert_eq!(
                loads.take_completed(handle),
                Err(TakeCompletedWorldStreamError::Invariant(
                    WorldStreamInvariantError::MissingStreamFile {
                        handle,
                        stream_type: missing_type,
                    }
                ))
            );
            assert_eq!(loads.len(), 1);
            assert_eq!(loads.state(handle).unwrap(), WorldStreamOuterState::DORMANT);
        }

        let mut loads: WorldStreamLoads<MockStreamFile, WorldStreamOperation> =
            WorldStreamLoads::debug_reference();
        let handle = loads.start(bam_spec()).unwrap();
        loads.records.get_mut(&handle).unwrap().state = WorldStreamOuterState::from_raw(99);
        assert_eq!(
            loads.take_completed(handle),
            Err(TakeCompletedWorldStreamError::Invariant(
                WorldStreamInvariantError::InvalidOuterState {
                    handle,
                    raw_state: 99,
                }
            ))
        );
        assert_eq!(loads.len(), 1);

        let missing = WorldStreamHandle::from_raw(77);
        assert_eq!(
            loads.take_completed(missing),
            Err(TakeCompletedWorldStreamError::Invariant(
                WorldStreamInvariantError::MissingRecord { handle: missing }
            ))
        );
    }

    #[test]
    fn derives_reference_path_and_preserves_dot_before_backslash_order() {
        let bam = WorldStreamSpec::derive("BAM", None).unwrap();
        assert_eq!(bam.stream_path(), "data\\content\\world\\stream");
        assert_eq!(bam.basename(), "BAM");

        let prefixed = WorldStreamSpec::derive("dir\\BAM.more.xst", Some("worldbam.big")).unwrap();
        assert_eq!(
            prefixed.stream_path(),
            "worldbam.big|data\\content\\world\\stream"
        );
        assert_eq!(prefixed.basename(), "BAM.more");

        assert_eq!(
            WorldStreamSpec::derive("dir.name\\BAM", None)
                .unwrap()
                .basename(),
            "dir"
        );
        assert_eq!(
            WorldStreamSpec::derive("foo/bar.xst", None)
                .unwrap()
                .basename(),
            "foo/bar"
        );
        assert_eq!(
            WorldStreamSpec::derive("dir\\", None).unwrap().basename(),
            ""
        );
        assert_eq!(
            WorldStreamSpec::derive("BAM.", Some(""))
                .unwrap()
                .stream_path(),
            "|data\\content\\world\\stream"
        );
    }

    #[test]
    fn rejects_inputs_that_would_not_fit_reference_c_strings() {
        assert_eq!(
            WorldStreamSpec::derive("BAM\0extra", None),
            Err(WorldStreamSpecError::InteriorNul {
                field: WorldStreamStringField::WorldFile,
            })
        );
        assert_eq!(
            WorldStreamSpec::derive("BAM", Some("bad\0prefix")),
            Err(WorldStreamSpecError::InteriorNul {
                field: WorldStreamStringField::BigFile,
            })
        );
        let maximum_length = "x".repeat(255);
        assert_eq!(
            WorldStreamSpec::derive(&maximum_length, None)
                .unwrap()
                .basename(),
            maximum_length
        );
        let too_long = "x".repeat(256);
        assert_eq!(
            WorldStreamSpec::derive(&too_long, None),
            Err(WorldStreamSpecError::TooLong {
                field: WorldStreamStringField::WorldFile,
                byte_len: 256,
                maximum: 255,
            })
        );
        let prefix = "p".repeat(230);
        assert!(matches!(
            WorldStreamSpec::derive("BAM", Some(&prefix)),
            Err(WorldStreamSpecError::TooLong {
                field: WorldStreamStringField::StreamPath,
                ..
            })
        ));
    }

    #[test]
    fn texture_enabled_ready_path_is_exactly_seven_one_case_polls() {
        let mut loads = WorldStreamLoads::debug_reference();
        let handle = loads.start(bam_spec()).unwrap();
        let mut backend = MockStreamBackend::ready(true);

        let outcomes = poll_until_complete(&mut loads, &mut backend, handle);

        assert_eq!(handle.raw(), 1);
        assert_eq!(outcomes.len(), 7);
        assert_eq!(&outcomes[..6], &[WorldStreamPoll::Pending; 6]);
        assert_eq!(outcomes[6], WorldStreamPoll::CompletedThisPoll);
        assert_eq!(
            *backend.events.borrow(),
            vec![
                StreamEvent::BeginFile(
                    WorldStreamType::Presentation,
                    WORLD_STREAM_PATH.to_owned(),
                    "BAM".to_owned(),
                ),
                StreamEvent::PollFile(WorldStreamType::Presentation, 4),
                StreamEvent::BeginXml(WorldStreamType::Presentation, "BAM".to_owned()),
                StreamEvent::ReadXml(WorldStreamType::Presentation, 2),
                StreamEvent::BeginFile(
                    WorldStreamType::Simulation,
                    WORLD_STREAM_PATH.to_owned(),
                    "BAM".to_owned(),
                ),
                StreamEvent::PollFile(WorldStreamType::Simulation, 4),
                StreamEvent::BeginXml(WorldStreamType::Simulation, "BAM".to_owned()),
                StreamEvent::ReadXml(WorldStreamType::Simulation, 2),
                StreamEvent::TextureGate(true),
                StreamEvent::BeginFile(
                    WorldStreamType::Texture,
                    WORLD_STREAM_PATH.to_owned(),
                    "BAM".to_owned(),
                ),
                StreamEvent::PollFile(WorldStreamType::Texture, 4),
                StreamEvent::FinalizeWorldBounds,
            ]
        );
        assert_eq!(loads.state(handle).unwrap(), WorldStreamOuterState::DORMANT);
        assert_eq!(
            loads.poll(handle, &mut backend).unwrap(),
            WorldStreamPoll::Pending
        );
        assert_eq!(
            backend.events.borrow().last(),
            Some(&StreamEvent::FinalizeWorldBounds)
        );
    }

    #[test]
    fn texture_disabled_path_still_needs_a_separate_final_poll() {
        let mut loads = WorldStreamLoads::debug_reference();
        let handle = loads.start(bam_spec()).unwrap();
        let mut backend = MockStreamBackend::ready(false);

        let outcomes = poll_until_complete(&mut loads, &mut backend, handle);

        assert_eq!(outcomes.len(), 6);
        assert_eq!(&outcomes[..5], &[WorldStreamPoll::Pending; 5]);
        assert_eq!(outcomes[5], WorldStreamPoll::CompletedThisPoll);
        assert_eq!(
            backend.events.borrow().as_slice(),
            &[
                StreamEvent::BeginFile(
                    WorldStreamType::Presentation,
                    WORLD_STREAM_PATH.to_owned(),
                    "BAM".to_owned(),
                ),
                StreamEvent::PollFile(WorldStreamType::Presentation, 4),
                StreamEvent::BeginXml(WorldStreamType::Presentation, "BAM".to_owned()),
                StreamEvent::ReadXml(WorldStreamType::Presentation, 2),
                StreamEvent::BeginFile(
                    WorldStreamType::Simulation,
                    WORLD_STREAM_PATH.to_owned(),
                    "BAM".to_owned(),
                ),
                StreamEvent::PollFile(WorldStreamType::Simulation, 4),
                StreamEvent::BeginXml(WorldStreamType::Simulation, "BAM".to_owned()),
                StreamEvent::ReadXml(WorldStreamType::Simulation, 2),
                StreamEvent::TextureGate(false),
                StreamEvent::FinalizeWorldBounds,
            ]
        );
    }

    #[test]
    fn file_and_xml_delays_repeat_only_the_current_case() {
        let mut loads = WorldStreamLoads::debug_reference();
        let handle = loads.start(bam_spec()).unwrap();
        let mut backend = MockStreamBackend::ready(true);
        backend.file_pending_polls = [1; 3];
        backend.xml_pending_updates = [1; 2];

        let outcomes = poll_until_complete(&mut loads, &mut backend, handle);

        assert_eq!(outcomes.len(), 12);
        assert_eq!(
            *backend.events.borrow(),
            vec![
                StreamEvent::BeginFile(
                    WorldStreamType::Presentation,
                    WORLD_STREAM_PATH.to_owned(),
                    "BAM".to_owned(),
                ),
                StreamEvent::PollFile(WorldStreamType::Presentation, 2),
                StreamEvent::PollFile(WorldStreamType::Presentation, 4),
                StreamEvent::BeginXml(WorldStreamType::Presentation, "BAM".to_owned()),
                StreamEvent::ReadXml(WorldStreamType::Presentation, 1),
                StreamEvent::UpdateXml(WorldStreamType::Presentation),
                StreamEvent::ReadXml(WorldStreamType::Presentation, 2),
                StreamEvent::BeginFile(
                    WorldStreamType::Simulation,
                    WORLD_STREAM_PATH.to_owned(),
                    "BAM".to_owned(),
                ),
                StreamEvent::PollFile(WorldStreamType::Simulation, 2),
                StreamEvent::PollFile(WorldStreamType::Simulation, 4),
                StreamEvent::BeginXml(WorldStreamType::Simulation, "BAM".to_owned()),
                StreamEvent::ReadXml(WorldStreamType::Simulation, 1),
                StreamEvent::UpdateXml(WorldStreamType::Simulation),
                StreamEvent::ReadXml(WorldStreamType::Simulation, 2),
                StreamEvent::TextureGate(true),
                StreamEvent::BeginFile(
                    WorldStreamType::Texture,
                    WORLD_STREAM_PATH.to_owned(),
                    "BAM".to_owned(),
                ),
                StreamEvent::PollFile(WorldStreamType::Texture, 2),
                StreamEvent::PollFile(WorldStreamType::Texture, 4),
                StreamEvent::FinalizeWorldBounds,
            ]
        );
    }

    #[test]
    fn texture_gate_is_live_only_at_the_simulation_xml_transition() {
        let mut loads = WorldStreamLoads::debug_reference();
        let handle = loads.start(bam_spec()).unwrap();
        let mut backend = MockStreamBackend::ready(true);
        for _ in 0..4 {
            assert_eq!(
                loads.poll(handle, &mut backend).unwrap(),
                WorldStreamPoll::Pending
            );
        }
        assert_eq!(
            loads.state(handle).unwrap(),
            WorldStreamOuterState::POLL_SIMULATION_XML
        );
        backend.texture_enabled = false;
        assert_eq!(
            loads.poll(handle, &mut backend).unwrap(),
            WorldStreamPoll::Pending
        );
        backend.texture_enabled = true;
        assert_eq!(
            loads.poll(handle, &mut backend).unwrap(),
            WorldStreamPoll::CompletedThisPoll
        );
        assert!(!backend.events.borrow().iter().any(|event| matches!(
            event,
            StreamEvent::BeginFile(WorldStreamType::Texture, _, _)
        )));

        let mut loads = WorldStreamLoads::debug_reference();
        let handle = loads.start(bam_spec()).unwrap();
        let mut backend = MockStreamBackend::ready(true);
        for _ in 0..5 {
            assert_eq!(
                loads.poll(handle, &mut backend).unwrap(),
                WorldStreamPoll::Pending
            );
        }
        assert_eq!(
            loads.state(handle).unwrap(),
            WorldStreamOuterState::POLL_TEXTURE_FILE
        );
        backend.texture_enabled = false;
        assert_eq!(
            poll_until_complete(&mut loads, &mut backend, handle).len(),
            2
        );
        assert_eq!(
            backend
                .events
                .borrow()
                .iter()
                .filter(|event| matches!(event, StreamEvent::TextureGate(_)))
                .count(),
            1
        );
    }

    #[test]
    fn raw_eight_and_arbitrary_non_loaded_file_states_remain_pending() {
        let mut loads = WorldStreamLoads::debug_reference();
        let handle = loads.start(bam_spec()).unwrap();
        let mut backend = MockStreamBackend::ready(true);
        backend.file_terminal_states[WorldStreamType::Presentation.slot()] =
            StreamFileState::from_raw(8);

        assert_eq!(
            loads.poll(handle, &mut backend).unwrap(),
            WorldStreamPoll::Pending
        );
        for _ in 0..2 {
            assert_eq!(
                loads.poll(handle, &mut backend).unwrap(),
                WorldStreamPoll::Pending
            );
            assert_eq!(
                loads.state(handle).unwrap(),
                WorldStreamOuterState::POLL_PRESENTATION_FILE
            );
        }
        assert_eq!(
            backend.events.borrow().as_slice(),
            &[
                StreamEvent::BeginFile(
                    WorldStreamType::Presentation,
                    WORLD_STREAM_PATH.to_owned(),
                    "BAM".to_owned(),
                ),
                StreamEvent::PollFile(WorldStreamType::Presentation, 8),
                StreamEvent::PollFile(WorldStreamType::Presentation, 8),
            ]
        );

        for (outer_state, stream_type) in [
            (
                WorldStreamOuterState::POLL_PRESENTATION_FILE,
                WorldStreamType::Presentation,
            ),
            (
                WorldStreamOuterState::POLL_SIMULATION_FILE,
                WorldStreamType::Simulation,
            ),
        ] {
            let mut loads = WorldStreamLoads::debug_reference();
            let handle = loads.start(bam_spec()).unwrap();
            let record = loads.records.get_mut(&handle).unwrap();
            record.state = outer_state;
            record.files[stream_type.slot()] = Some(MockStreamFile { stream_type });
            let mut backend = MockStreamBackend::ready(true);
            backend.file_terminal_states[stream_type.slot()] = StreamFileState::from_raw(37);

            assert_eq!(
                loads.poll(handle, &mut backend).unwrap(),
                WorldStreamPoll::Pending
            );
            assert_eq!(loads.state(handle).unwrap(), outer_state);
            assert_eq!(
                backend.events.borrow().as_slice(),
                &[StreamEvent::PollFile(stream_type, 37)]
            );
        }
    }

    #[test]
    fn arbitrary_non_done_xml_states_update_and_remain_in_the_same_stage() {
        for (outer_state, stream_type) in [
            (
                WorldStreamOuterState::POLL_PRESENTATION_XML,
                WorldStreamType::Presentation,
            ),
            (
                WorldStreamOuterState::POLL_SIMULATION_XML,
                WorldStreamType::Simulation,
            ),
        ] {
            let mut loads = WorldStreamLoads::debug_reference();
            let handle = loads.start(bam_spec()).unwrap();
            let record = loads.records.get_mut(&handle).unwrap();
            record.state = outer_state;
            record.files[stream_type.slot()] = Some(MockStreamFile { stream_type });
            let mut backend = MockStreamBackend::ready(true);
            backend.xml_pending_updates[stream_type.slot()] = 1;
            backend.xml_pending_states[stream_type.slot()] = StreamXmlState::from_raw(99);

            assert_eq!(
                loads.poll(handle, &mut backend).unwrap(),
                WorldStreamPoll::Pending
            );
            assert_eq!(loads.state(handle).unwrap(), outer_state);
            assert_eq!(
                backend.events.borrow().as_slice(),
                &[
                    StreamEvent::ReadXml(stream_type, 99),
                    StreamEvent::UpdateXml(stream_type),
                ]
            );
        }
    }

    #[test]
    fn invalid_outer_state_and_missing_records_are_invariant_errors() {
        let missing = WorldStreamHandle::from_raw(77);
        let mut loads = WorldStreamLoads::debug_reference();
        let mut backend = MockStreamBackend::ready(true);
        assert_eq!(
            *loads.poll(missing, &mut backend).unwrap_err(),
            WorldStreamControllerError::Invariant(WorldStreamInvariantError::MissingRecord {
                handle: missing
            })
        );

        let handle = loads.start(bam_spec()).unwrap();
        loads.records.get_mut(&handle).unwrap().state = WorldStreamOuterState::from_raw(99);
        let first = loads.poll(handle, &mut backend).unwrap_err();
        assert_eq!(
            *first,
            WorldStreamControllerError::Invariant(WorldStreamInvariantError::InvalidOuterState {
                handle,
                raw_state: 99,
            })
        );
        let second = loads.poll(handle, &mut backend).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn one_off_world_stream_segment_stores_polls_yields_and_samples_focus_late() {
        let mut loads = WorldStreamLoads::debug_reference();
        let mut backend = MockStreamBackend::ready(false);
        let order = Rc::new(RefCell::new(Vec::new()));
        backend.worker_events = Some(Rc::clone(&order));
        let sampled_position = Rc::new(RefCell::new(UNUSUAL_POSITION));

        let store_order = Rc::clone(&order);
        let yield_order = Rc::clone(&order);
        let init_order = Rc::clone(&order);
        let yielded_position = Rc::clone(&sampled_position);
        let initialized_position = Rc::clone(&sampled_position);
        let handle = run_offline_one_off_world_stream(
            &mut loads,
            &mut backend,
            bam_spec(),
            |handle| {
                store_order
                    .borrow_mut()
                    .push(WorkerEvent::Store(handle.raw()));
            },
            |raw| {
                yield_order.borrow_mut().push(WorkerEvent::Yield(raw));
                *yielded_position.borrow_mut() = LocationPositionWords::POSITIVE_ZERO;
            },
            || {
                init_order
                    .borrow_mut()
                    .push(WorkerEvent::Init(initialized_position.borrow().words()));
            },
        )
        .unwrap();

        assert_eq!(handle.raw(), 1);
        let order = order.borrow();
        assert_eq!(
            order.as_slice(),
            &[
                WorkerEvent::Store(1),
                WorkerEvent::Stream(StreamEvent::BeginFile(
                    WorldStreamType::Presentation,
                    WORLD_STREAM_PATH.to_owned(),
                    "BAM".to_owned(),
                )),
                WorkerEvent::Yield(1),
                WorkerEvent::Stream(StreamEvent::PollFile(WorldStreamType::Presentation, 4,)),
                WorkerEvent::Stream(StreamEvent::BeginXml(
                    WorldStreamType::Presentation,
                    "BAM".to_owned(),
                )),
                WorkerEvent::Yield(1),
                WorkerEvent::Stream(StreamEvent::ReadXml(WorldStreamType::Presentation, 2,)),
                WorkerEvent::Stream(StreamEvent::BeginFile(
                    WorldStreamType::Simulation,
                    WORLD_STREAM_PATH.to_owned(),
                    "BAM".to_owned(),
                )),
                WorkerEvent::Yield(1),
                WorkerEvent::Stream(StreamEvent::PollFile(WorldStreamType::Simulation, 4)),
                WorkerEvent::Stream(StreamEvent::BeginXml(
                    WorldStreamType::Simulation,
                    "BAM".to_owned(),
                )),
                WorkerEvent::Yield(1),
                WorkerEvent::Stream(StreamEvent::ReadXml(WorldStreamType::Simulation, 2)),
                WorkerEvent::Stream(StreamEvent::TextureGate(false)),
                WorkerEvent::Yield(1),
                WorkerEvent::Stream(StreamEvent::FinalizeWorldBounds),
                WorkerEvent::Init([0, 0, 0, 0]),
            ]
        );
        assert_eq!(
            backend.events.borrow().last(),
            Some(&StreamEvent::FinalizeWorldBounds)
        );
    }
}
