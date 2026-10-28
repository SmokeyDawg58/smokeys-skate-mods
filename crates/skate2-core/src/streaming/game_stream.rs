//! Evidence-backed inner world-stream load orchestration.
//!
//! This module models `cStreamFile` and its owned `cGameStream` without
//! reproducing the console's file-loader or asset-manager internals. The
//! collaborator boundary keeps asynchronous reads explicit and consumes the
//! existing checked XSM/XST/XSS parsers.

use skate2_formats::stream::{AssetToc, CollectionMap, CollectionSpace, StreamTableError};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

const TABLE_VERSION: u32 = 3;
const NATIVE_PATH_CAPACITY: usize = 256;
const READ_ALIGNMENT: u32 = 0x10;

/// The three native world-stream variants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum StreamKind {
    Presentation = 0,
    Simulation = 1,
    Texture = 2,
}

impl StreamKind {
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Presentation => "Pres",
            Self::Simulation => "Sim",
            Self::Texture => "Tex",
        }
    }
}

/// Raw `eGameStreamState` stored at `cGameStream + 0x24` (object size `0x260`).
///
/// PDB members name 2, 3, 4, 6, 7, and 8. Raw 1 and 5 are observed transition
/// values in `cGameStream::Load`/`Update`, but are deliberately not presented
/// as PDB enum names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct GameStreamState(u32);

impl GameStreamState {
    pub const BEGIN_MAP_READ: Self = Self(1);
    pub const LOADING_MAP: Self = Self(2);
    pub const LOADING_TOC: Self = Self(3);
    pub const LOADED: Self = Self(4);
    pub const BEGIN_UNLOAD: Self = Self(5);
    pub const UNLOADING: Self = Self(6);
    pub const UNLOADED: Self = Self(7);
    pub const LOAD_FAILED: Self = Self(8);

    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Raw `eStreamFileState` at `cStreamFile + 0x28` (object size `0x290`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct StreamFileState(u32);

impl StreamFileState {
    pub const INIT: Self = Self(0);
    pub const LOAD_STREAM_WAIT: Self = Self(1);
    pub const LOAD_SPACE_FILE_WAIT: Self = Self(2);
    pub const LOAD_STREAM_XML_WAIT: Self = Self(3);
    pub const LOAD_DONE: Self = Self(4);
    pub const UNLOAD_WAIT: Self = Self(5);
    pub const UNLOAD_DONE: Self = Self(6);

    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Raw `eStreamXmlState` at `cStreamFile + 0x2c`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct StreamXmlState(u32);

impl StreamXmlState {
    pub const INIT: Self = Self(0);
    pub const LOAD_STREAM_XML_WAIT: Self = Self(1);
    pub const LOAD_DONE: Self = Self(2);

    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// A reference async read and the flags passed to the file-loader boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AsyncReadRequest {
    pub stage: ReadStage,
    pub path: String,
    pub flags: u32,
    pub alignment: u32,
}

/// File identity retained with a live async handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadStage {
    CollectionMap,
    AssetToc,
    CollectionSpace,
    StreamXml,
}

/// Result of one non-blocking file-loader poll.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AsyncReadPoll {
    Pending,
    Complete(Vec<u8>),
    /// Native terminal read status 1. A backend must keep returning `Failed`
    /// for this live handle until the state machine releases it.
    Failed,
}

/// Exact derived paths retained by `cGameStream`/`cStreamFile`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamPaths {
    pub collection_directory: String,
    pub manifest_stem: String,
    pub collection_space: String,
    pub stream_xml: String,
}

impl StreamPaths {
    /// Reconstructs `AssetPaths::BuildStreamPath` (`0x82aba3f0`) and the
    /// later XSS/XML filename construction for valid native inputs.
    ///
    /// # Errors
    ///
    /// Returns [`StreamPathError`] for an interior NUL or a derived path that
    /// cannot fit its native 256-byte C buffer.
    pub fn derive(
        base_path: &str,
        basename: &str,
        kind: StreamKind,
    ) -> Result<Self, StreamPathError> {
        reject_nul("base_path", base_path)?;
        reject_nul("basename", basename)?;

        let typed_name = format!("{basename}_{}", kind.suffix());
        let collection_directory = format!("{base_path}\\{basename}\\");
        let manifest_stem = format!("{collection_directory}{typed_name}");
        let collection_space = format!("{manifest_stem}.xss");
        let stream_xml = format!("data\\stream\\{typed_name}.xml");

        for (field, value) in [
            ("collection_directory", &collection_directory),
            ("manifest_stem", &manifest_stem),
            ("collection_space", &collection_space),
            ("stream_xml", &stream_xml),
        ] {
            if value.len() >= NATIVE_PATH_CAPACITY {
                return Err(StreamPathError::TooLong {
                    field,
                    length: value.len(),
                    capacity: NATIVE_PATH_CAPACITY,
                });
            }
        }

        Ok(Self {
            collection_directory,
            manifest_stem,
            collection_space,
            stream_xml,
        })
    }

    #[must_use]
    pub fn collection_map(&self) -> String {
        format!("{}.xsm", self.manifest_stem)
    }

    #[must_use]
    pub fn asset_toc(&self) -> String {
        format!("{}.xst", self.manifest_stem)
    }
}

#[derive(Debug)]
struct RegistrationFileIdentity {
    _private: u8,
}

/// Stable identity of one live reconstructed `cStreamFile`.
///
/// The reference uses the address of its `cStreamFile`. This safe native
/// boundary instead compares and hashes the identity of a private owned
/// allocation. It deliberately exposes no guessed numeric guest value.
#[derive(Clone)]
pub struct RegistrationFileId(Arc<RegistrationFileIdentity>);

impl RegistrationFileId {
    fn new() -> Self {
        Self(Arc::new(RegistrationFileIdentity { _private: 0 }))
    }
}

impl fmt::Debug for RegistrationFileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RegistrationFileId(..)")
    }
}

impl PartialEq for RegistrationFileId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for RegistrationFileId {}

impl Hash for RegistrationFileId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

/// Borrowed registration identity and exact paths retained by one stream file.
#[derive(Clone, Copy, Debug)]
pub struct StreamRegistrationContext<'a> {
    id: &'a RegistrationFileId,
    kind: StreamKind,
    paths: &'a StreamPaths,
}

impl<'a> StreamRegistrationContext<'a> {
    fn new(id: &'a RegistrationFileId, kind: StreamKind, paths: &'a StreamPaths) -> Self {
        Self { id, kind, paths }
    }

    #[must_use]
    pub const fn id(self) -> &'a RegistrationFileId {
        self.id
    }

    #[must_use]
    pub const fn kind(self) -> StreamKind {
        self.kind
    }

    #[must_use]
    pub const fn paths(self) -> &'a StreamPaths {
        self.paths
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StreamPathError {
    InteriorNul {
        field: &'static str,
    },
    TooLong {
        field: &'static str,
        length: usize,
        capacity: usize,
    },
}

fn reject_nul(field: &'static str, value: &str) -> Result<(), StreamPathError> {
    if value.as_bytes().contains(&0) {
        Err(StreamPathError::InteriorNul { field })
    } else {
        Ok(())
    }
}

/// Side effects below the checked state machine.
///
/// `begin_read` must only publish a fully initialized owned handle. That is a
/// deliberate native-safety boundary, not an attempt to reproduce the
/// reference loader's publication-before-initialization race.
pub trait InnerStreamBackend {
    type Handle;
    type Error;

    /// # Errors
    ///
    /// Returns the backend error if an owned async operation cannot be begun.
    fn begin_read(&mut self, request: AsyncReadRequest) -> Result<Self::Handle, Self::Error>;
    /// # Errors
    ///
    /// Returns the backend error if the live async operation cannot be polled.
    fn poll_read(&mut self, handle: &mut Self::Handle) -> Result<AsyncReadPoll, Self::Error>;

    /// # Errors
    ///
    /// Returns the backend error if collection registrations cannot be applied.
    fn collection_map_loaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        map: &CollectionMap,
    ) -> Result<(), Self::Error>;
    /// # Errors
    ///
    /// Returns the backend error if asset registrations cannot be applied.
    fn asset_toc_loaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        toc: &AssetToc,
    ) -> Result<(), Self::Error>;
    /// # Errors
    ///
    /// Returns the backend error if spatial data cannot be installed.
    fn collection_space_loaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        space: &CollectionSpace,
    ) -> Result<(), Self::Error>;
    /// # Errors
    ///
    /// Returns the backend error if the opaque XML data cannot be installed.
    fn stream_xml_loaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        bytes: &[u8],
    ) -> Result<(), Self::Error>;

    /// # Errors
    ///
    /// Returns the backend error if collection registrations cannot be removed.
    fn collection_map_unloaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        map: &CollectionMap,
    ) -> Result<(), Self::Error>;
    /// # Errors
    ///
    /// Returns the backend error if asset registrations cannot be removed.
    fn asset_toc_unloaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        toc: &AssetToc,
    ) -> Result<(), Self::Error>;
    /// # Errors
    ///
    /// Returns the backend error if spatial/XML ownership cannot be released.
    fn stream_file_unloaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        space: Option<&CollectionSpace>,
        had_stream_xml: bool,
    ) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub enum InnerStreamError<E> {
    Path(StreamPathError),
    Backend(E),
    Table {
        stage: ReadStage,
        source: StreamTableError,
    },
    UnexpectedVersion {
        stage: ReadStage,
        expected: u32,
        actual: u32,
    },
    InvalidGameStreamState(u32),
    InvalidStreamFileState(u32),
    InvalidStreamXmlState(u32),
    MissingRead(ReadStage),
    ReadStageMismatch {
        expected: ReadStage,
        actual: ReadStage,
    },
    ReadAlreadyActive(ReadStage),
    StreamXmlAlreadyStarted(u32),
    StreamXmlRequiresLoadedFile(u32),
    TextureStreamHasNoOuterXml,
}

impl<E> From<StreamPathError> for InnerStreamError<E> {
    fn from(value: StreamPathError) -> Self {
        Self::Path(value)
    }
}

struct PendingRead<H> {
    stage: ReadStage,
    handle: H,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameStreamUnloadError {
    AlreadyRequested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamFileUnloadError {
    UnsupportedState(u32),
    ReadActive(ReadStage),
    Nested(GameStreamUnloadError),
}

/// Typed reconstruction of the load/unload portion of `cGameStream`.
pub struct GameStream<H, E> {
    registration_id: RegistrationFileId,
    kind: StreamKind,
    paths: StreamPaths,
    state: GameStreamState,
    unload_requested: bool,
    read: Option<PendingRead<H>>,
    collection_map: Option<CollectionMap>,
    asset_toc: Option<AssetToc>,
    reference_read_failure: Option<ReadStage>,
    terminal_error: Option<Arc<InnerStreamError<E>>>,
}

/// Owns an XSM/XST-only manifest registration without inventing an XSS file.
///
/// This is a host composition around the recovered [`GameStream`] state
/// machine, not a claim that the reference has a second `cStreamFile` type.
/// It is used for manifest banks, such as the main DMO bank, whose native hull
/// path consumes the nested collection and asset tables without the ordinary
/// world stream's collection-space and XML stages.
///
/// The owner deliberately performs no implicit cleanup from [`Drop`]. On the
/// clean path, callers request and poll unload so the XSM registrations, XST
/// registrations, and final registration-file identity are released in
/// observable order. A terminal load/unload fault retains whichever catalog
/// prefix or suffix was already applied; no rollback or retry is invented.
pub struct ManifestOnlyStream<H, E> {
    registration_id: RegistrationFileId,
    kind: StreamKind,
    paths: StreamPaths,
    game_stream: GameStream<H, E>,
    file_released: bool,
    terminal_error: Option<Arc<InnerStreamError<E>>>,
}

impl<H, E> ManifestOnlyStream<H, E> {
    /// Starts the recovered nested manifest load in raw state 1.
    ///
    /// # Errors
    ///
    /// Returns [`StreamPathError`] when the derived native C paths do not fit.
    pub fn load(
        base_path: &str,
        basename: &str,
        kind: StreamKind,
    ) -> Result<Self, StreamPathError> {
        let paths = StreamPaths::derive(base_path, basename, kind)?;
        let registration_id = RegistrationFileId::new();
        let game_stream = GameStream::new(kind, paths.clone(), registration_id.clone());
        Ok(Self {
            registration_id,
            kind,
            paths,
            game_stream,
            file_released: false,
            terminal_error: None,
        })
    }

    #[must_use]
    pub const fn state(&self) -> GameStreamState {
        self.game_stream.state()
    }

    #[must_use]
    pub fn paths(&self) -> &StreamPaths {
        &self.paths
    }

    #[must_use]
    pub fn registration_id(&self) -> &RegistrationFileId {
        &self.registration_id
    }

    #[must_use]
    pub fn registration_context(&self) -> StreamRegistrationContext<'_> {
        StreamRegistrationContext::new(&self.registration_id, self.kind, &self.paths)
    }

    #[must_use]
    pub fn collection_map(&self) -> Option<&CollectionMap> {
        self.game_stream.collection_map()
    }

    #[must_use]
    pub fn asset_toc(&self) -> Option<&AssetToc> {
        self.game_stream.asset_toc()
    }

    /// Returns true only after nested unload and final file-owner removal.
    #[must_use]
    pub const fn is_released(&self) -> bool {
        self.file_released
    }

    /// Returns the first terminal safe-boundary error, if polling failed.
    #[must_use]
    pub fn terminal_error(&self) -> Option<&Arc<InnerStreamError<E>>> {
        self.terminal_error.as_ref()
    }

    /// Reports a native XSM/XST read-failure branch without rewriting it.
    #[must_use]
    pub const fn reference_read_failure(&self) -> Option<ReadStage> {
        self.game_stream.reference_read_failure()
    }

    /// Requests the recovered nested XSM/XST unload path.
    ///
    /// # Errors
    ///
    /// Returns [`GameStreamUnloadError::AlreadyRequested`] for a duplicate
    /// request.
    pub fn request_unload(&mut self) -> Result<(), GameStreamUnloadError> {
        self.game_stream.request_unload()
    }

    /// Performs one nested update and, once it reaches raw state 7, releases
    /// the registration-file owner with no XSS or XML payload.
    ///
    /// # Errors
    ///
    /// Returns and permanently replays the first nested or backend error.
    pub fn poll<B>(&mut self, backend: &mut B) -> Result<(), Arc<InnerStreamError<E>>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        if let Some(error) = self.terminal_error.as_ref() {
            return Err(Arc::clone(error));
        }

        let result = self.poll_once(backend);
        if let Err(error) = &result {
            self.terminal_error = Some(Arc::clone(error));
        }
        result
    }

    fn poll_once<B>(&mut self, backend: &mut B) -> Result<(), Arc<InnerStreamError<E>>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        self.game_stream.poll(backend)?;
        if self.game_stream.state() == GameStreamState::UNLOADED && !self.file_released {
            backend
                .stream_file_unloaded(self.registration_context(), None, false)
                .map_err(|source| Arc::new(InnerStreamError::Backend(source)))?;
            self.file_released = true;
        }
        Ok(())
    }
}

impl<H, E> GameStream<H, E> {
    fn new(kind: StreamKind, paths: StreamPaths, registration_id: RegistrationFileId) -> Self {
        Self {
            registration_id,
            kind,
            paths,
            state: GameStreamState::BEGIN_MAP_READ,
            unload_requested: false,
            read: None,
            collection_map: None,
            asset_toc: None,
            reference_read_failure: None,
            terminal_error: None,
        }
    }

    #[must_use]
    pub const fn state(&self) -> GameStreamState {
        self.state
    }

    #[must_use]
    pub const fn unload_requested(&self) -> bool {
        self.unload_requested
    }

    #[must_use]
    pub fn collection_map(&self) -> Option<&CollectionMap> {
        self.collection_map.as_ref()
    }

    #[must_use]
    pub fn asset_toc(&self) -> Option<&AssetToc> {
        self.asset_toc.as_ref()
    }

    /// Returns the first terminal safe-boundary error, if polling failed.
    #[must_use]
    pub fn terminal_error(&self) -> Option<&Arc<InnerStreamError<E>>> {
        self.terminal_error.as_ref()
    }

    /// Reports a native read-failure branch without rewriting its raw state.
    #[must_use]
    pub const fn reference_read_failure(&self) -> Option<ReadStage> {
        self.reference_read_failure
    }

    /// Mirrors `cGameStream::Unload` at `0x82d540f0`.
    ///
    /// # Errors
    ///
    /// Returns [`GameStreamUnloadError::AlreadyRequested`] for a duplicate
    /// request, matching the reference assertion as a safe typed error.
    pub fn request_unload(&mut self) -> Result<(), GameStreamUnloadError> {
        if self.unload_requested {
            return Err(GameStreamUnloadError::AlreadyRequested);
        }
        self.unload_requested = true;
        Ok(())
    }

    /// Performs one `cGameStream::Update` (`0x82d5db60`) dispatch.
    ///
    /// # Errors
    ///
    /// Returns [`InnerStreamError`] for invalid state/ownership, malformed
    /// tables or versions, and collaborator failures.
    pub fn poll<B>(&mut self, backend: &mut B) -> Result<(), Arc<InnerStreamError<E>>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        if let Some(error) = self.terminal_error.as_ref() {
            return Err(Arc::clone(error));
        }
        let result = self.poll_once(backend).map_err(Arc::new);
        if let Err(error) = &result {
            self.terminal_error = Some(Arc::clone(error));
        }
        result
    }

    fn poll_once<B>(&mut self, backend: &mut B) -> Result<(), InnerStreamError<E>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        match self.state.raw() {
            0 | 7 => Ok(()),
            1 => self.begin_map_or_unload(backend),
            2 => self.poll_map(backend),
            3 => self.poll_toc(backend),
            4 | 8 => {
                if self.unload_requested {
                    self.state = GameStreamState::BEGIN_UNLOAD;
                }
                Ok(())
            }
            5 => self.begin_unload(backend),
            6 => {
                self.state = GameStreamState::UNLOADED;
                Ok(())
            }
            raw => Err(InnerStreamError::InvalidGameStreamState(raw)),
        }
    }

    fn begin_map_or_unload<B>(&mut self, backend: &mut B) -> Result<(), InnerStreamError<E>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        if self.unload_requested {
            self.state = GameStreamState::BEGIN_UNLOAD;
            return Ok(());
        }
        let request = AsyncReadRequest {
            stage: ReadStage::CollectionMap,
            path: self.paths.collection_map(),
            flags: 1,
            alignment: READ_ALIGNMENT,
        };
        self.begin_read(backend, request)?;
        self.state = GameStreamState::LOADING_MAP;
        Ok(())
    }

    fn poll_map<B>(&mut self, backend: &mut B) -> Result<(), InnerStreamError<E>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        match self.poll_read(backend, ReadStage::CollectionMap)? {
            AsyncReadPoll::Pending => Ok(()),
            AsyncReadPoll::Failed => {
                self.read = None;
                self.reference_read_failure = Some(ReadStage::CollectionMap);
                self.state = GameStreamState::LOAD_FAILED;
                Ok(())
            }
            AsyncReadPoll::Complete(bytes) => {
                self.read = None;
                if self.unload_requested {
                    self.state = GameStreamState::BEGIN_UNLOAD;
                    return Ok(());
                }
                let map =
                    CollectionMap::parse(&bytes).map_err(|source| InnerStreamError::Table {
                        stage: ReadStage::CollectionMap,
                        source,
                    })?;
                require_version(ReadStage::CollectionMap, map.version)?;
                let context =
                    StreamRegistrationContext::new(&self.registration_id, self.kind, &self.paths);
                backend
                    .collection_map_loaded(context, &map)
                    .map_err(InnerStreamError::Backend)?;
                self.collection_map = Some(map);

                let request = AsyncReadRequest {
                    stage: ReadStage::AssetToc,
                    path: self.paths.asset_toc(),
                    flags: 1,
                    alignment: READ_ALIGNMENT,
                };
                self.begin_read(backend, request)?;
                self.state = GameStreamState::LOADING_TOC;
                Ok(())
            }
        }
    }

    fn poll_toc<B>(&mut self, backend: &mut B) -> Result<(), InnerStreamError<E>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        match self.poll_read(backend, ReadStage::AssetToc)? {
            AsyncReadPoll::Pending => Ok(()),
            AsyncReadPoll::Failed => {
                self.reference_read_failure = Some(ReadStage::AssetToc);
                Ok(())
            }
            AsyncReadPoll::Complete(bytes) => {
                self.read = None;
                if self.unload_requested {
                    self.state = GameStreamState::BEGIN_UNLOAD;
                    return Ok(());
                }
                let toc = AssetToc::parse(&bytes).map_err(|source| InnerStreamError::Table {
                    stage: ReadStage::AssetToc,
                    source,
                })?;
                require_version(ReadStage::AssetToc, toc.version)?;
                let context =
                    StreamRegistrationContext::new(&self.registration_id, self.kind, &self.paths);
                backend
                    .asset_toc_loaded(context, &toc)
                    .map_err(InnerStreamError::Backend)?;
                self.asset_toc = Some(toc);
                self.state = GameStreamState::LOADED;
                Ok(())
            }
        }
    }

    fn begin_unload<B>(&mut self, backend: &mut B) -> Result<(), InnerStreamError<E>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        let context = StreamRegistrationContext::new(&self.registration_id, self.kind, &self.paths);
        if let Some(map) = self.collection_map.as_ref() {
            backend
                .collection_map_unloaded(context, map)
                .map_err(InnerStreamError::Backend)?;
        }
        if let Some(toc) = self.asset_toc.as_ref() {
            backend
                .asset_toc_unloaded(context, toc)
                .map_err(InnerStreamError::Backend)?;
        }
        self.collection_map = None;
        self.asset_toc = None;
        self.read = None;
        self.state = GameStreamState::UNLOADING;
        Ok(())
    }

    fn begin_read<B>(
        &mut self,
        backend: &mut B,
        request: AsyncReadRequest,
    ) -> Result<(), InnerStreamError<E>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        if let Some(read) = self.read.as_ref() {
            return Err(InnerStreamError::ReadAlreadyActive(read.stage));
        }
        let stage = request.stage;
        let handle = backend
            .begin_read(request)
            .map_err(InnerStreamError::Backend)?;
        self.read = Some(PendingRead { stage, handle });
        Ok(())
    }

    fn poll_read<B>(
        &mut self,
        backend: &mut B,
        expected: ReadStage,
    ) -> Result<AsyncReadPoll, InnerStreamError<E>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        let read = self
            .read
            .as_mut()
            .ok_or(InnerStreamError::MissingRead(expected))?;
        if read.stage != expected {
            return Err(InnerStreamError::ReadStageMismatch {
                expected,
                actual: read.stage,
            });
        }
        backend
            .poll_read(&mut read.handle)
            .map_err(InnerStreamError::Backend)
    }
}

/// Typed reconstruction of `cStreamFile::Load`/`Update`/`Unload`.
pub struct StreamFile<H, E> {
    registration_id: RegistrationFileId,
    kind: StreamKind,
    paths: StreamPaths,
    state: StreamFileState,
    xml_state: StreamXmlState,
    game_stream: GameStream<H, E>,
    read: Option<PendingRead<H>>,
    collection_space: Option<CollectionSpace>,
    stream_xml_loaded: bool,
    reference_read_failure: Option<ReadStage>,
    terminal_error: Option<Arc<InnerStreamError<E>>>,
}

impl<H, E> StreamFile<H, E> {
    /// Starts the nested stream in raw state 1, matching `cStreamFile::Load`
    /// (`0x824d7748`) after its synchronous setup.
    ///
    /// # Errors
    ///
    /// Returns [`StreamPathError`] when a valid native C path cannot be built.
    pub fn load(
        base_path: &str,
        basename: &str,
        kind: StreamKind,
    ) -> Result<Self, StreamPathError> {
        let paths = StreamPaths::derive(base_path, basename, kind)?;
        let registration_id = RegistrationFileId::new();
        let game_stream = GameStream::new(kind, paths.clone(), registration_id.clone());
        Ok(Self {
            registration_id,
            kind,
            paths,
            state: StreamFileState::LOAD_STREAM_WAIT,
            xml_state: StreamXmlState::INIT,
            game_stream,
            read: None,
            collection_space: None,
            stream_xml_loaded: false,
            reference_read_failure: None,
            terminal_error: None,
        })
    }

    #[must_use]
    pub const fn state(&self) -> StreamFileState {
        self.state
    }

    #[must_use]
    pub const fn xml_state(&self) -> StreamXmlState {
        self.xml_state
    }

    #[must_use]
    pub const fn game_stream_state(&self) -> GameStreamState {
        self.game_stream.state()
    }

    #[must_use]
    pub fn paths(&self) -> &StreamPaths {
        &self.paths
    }

    #[must_use]
    pub fn registration_id(&self) -> &RegistrationFileId {
        &self.registration_id
    }

    #[must_use]
    pub fn registration_context(&self) -> StreamRegistrationContext<'_> {
        StreamRegistrationContext::new(&self.registration_id, self.kind, &self.paths)
    }

    #[must_use]
    pub fn collection_space(&self) -> Option<&CollectionSpace> {
        self.collection_space.as_ref()
    }

    /// Returns the first terminal safe-boundary error, if polling failed.
    #[must_use]
    pub fn terminal_error(&self) -> Option<&Arc<InnerStreamError<E>>> {
        self.terminal_error.as_ref()
    }

    /// Reports XSM/XST/XSS native failure stalls while preserving their exact
    /// raw states. XML failure is an ordinary native completion and is omitted.
    #[must_use]
    pub fn reference_read_failure(&self) -> Option<ReadStage> {
        self.reference_read_failure
            .or_else(|| self.game_stream.reference_read_failure())
    }

    /// Performs one `cStreamFile::Update` dispatch (`0x824e0ba8`), including
    /// the independent XML-state dispatch at the end of the function.
    ///
    /// # Errors
    ///
    /// Returns [`InnerStreamError`] for invalid state/ownership, malformed
    /// tables or versions, and collaborator failures.
    pub fn poll<B>(&mut self, backend: &mut B) -> Result<(), Arc<InnerStreamError<E>>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        if let Some(error) = self.terminal_error.as_ref() {
            return Err(Arc::clone(error));
        }

        let result = self.poll_once(backend);
        if let Err(error) = &result {
            self.terminal_error = Some(Arc::clone(error));
        }
        result
    }

    fn poll_once<B>(&mut self, backend: &mut B) -> Result<(), Arc<InnerStreamError<E>>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        match self.state.raw() {
            0 | 4 | 6 => {}
            1 => {
                self.game_stream.poll(backend)?;
                if self.game_stream.state() == GameStreamState::LOADED {
                    let request = AsyncReadRequest {
                        stage: ReadStage::CollectionSpace,
                        path: self.paths.collection_space.clone(),
                        flags: 0,
                        alignment: READ_ALIGNMENT,
                    };
                    self.begin_read(backend, request).map_err(Arc::new)?;
                    self.state = StreamFileState::LOAD_SPACE_FILE_WAIT;
                }
            }
            2 => self.poll_collection_space(backend).map_err(Arc::new)?,
            5 => {
                self.game_stream.poll(backend)?;
                if self.game_stream.state() == GameStreamState::UNLOADED {
                    backend
                        .stream_file_unloaded(
                            self.registration_context(),
                            self.collection_space.as_ref(),
                            self.stream_xml_loaded,
                        )
                        .map_err(|error| Arc::new(InnerStreamError::Backend(error)))?;
                    self.read = None;
                    self.collection_space = None;
                    self.stream_xml_loaded = false;
                    self.state = StreamFileState::UNLOAD_DONE;
                }
            }
            raw => {
                return Err(Arc::new(InnerStreamError::InvalidStreamFileState(raw)));
            }
        }

        self.poll_xml(backend).map_err(Arc::new)
    }

    /// Starts the separate stream XML read (`cStreamFile::LoadStreamXml`,
    /// `0x824d7c60`). The outer reference invokes it only for Pres and Sim.
    ///
    /// # Errors
    ///
    /// Returns [`InnerStreamError`] for Texture, duplicate/conflicting reads,
    /// or failure to begin the owned async operation.
    pub fn begin_stream_xml<B>(&mut self, backend: &mut B) -> Result<(), Arc<InnerStreamError<E>>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        if let Some(error) = self.terminal_error.as_ref() {
            return Err(Arc::clone(error));
        }

        let result = self.begin_stream_xml_once(backend).map_err(Arc::new);
        if let Err(error) = &result {
            self.terminal_error = Some(Arc::clone(error));
        }
        result
    }

    fn begin_stream_xml_once<B>(&mut self, backend: &mut B) -> Result<(), InnerStreamError<E>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        if self.kind == StreamKind::Texture {
            return Err(InnerStreamError::TextureStreamHasNoOuterXml);
        }
        if self.state != StreamFileState::LOAD_DONE {
            return Err(InnerStreamError::StreamXmlRequiresLoadedFile(
                self.state.raw(),
            ));
        }
        if self.xml_state != StreamXmlState::INIT {
            return Err(InnerStreamError::StreamXmlAlreadyStarted(
                self.xml_state.raw(),
            ));
        }
        let request = AsyncReadRequest {
            stage: ReadStage::StreamXml,
            path: self.paths.stream_xml.clone(),
            flags: 0,
            alignment: READ_ALIGNMENT,
        };
        self.xml_state = StreamXmlState::LOAD_STREAM_XML_WAIT;
        if let Err(error) = self.begin_read(backend, request) {
            // The reference publishes raw 1 before calling ASyncLoad. Keep that
            // call-time ordering, but roll back if the safe collaborator cannot
            // return a fully initialized owned handle.
            self.xml_state = StreamXmlState::INIT;
            return Err(error);
        }
        Ok(())
    }

    /// Requests the proven load-wait or loaded unload paths.
    ///
    /// # Errors
    ///
    /// Returns [`StreamFileUnloadError`] for a state with unproven cancellation
    /// ownership, an outstanding file-level read, or a duplicate nested request.
    pub fn request_unload(&mut self) -> Result<(), StreamFileUnloadError> {
        if !matches!(self.state.raw(), 1 | 4) {
            return Err(StreamFileUnloadError::UnsupportedState(self.state.raw()));
        }
        if let Some(read) = self.read.as_ref() {
            return Err(StreamFileUnloadError::ReadActive(read.stage));
        }
        self.game_stream
            .request_unload()
            .map_err(StreamFileUnloadError::Nested)?;
        self.state = StreamFileState::UNLOAD_WAIT;
        Ok(())
    }

    fn poll_collection_space<B>(&mut self, backend: &mut B) -> Result<(), InnerStreamError<E>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        match self.poll_read(backend, ReadStage::CollectionSpace)? {
            AsyncReadPoll::Pending => Ok(()),
            AsyncReadPoll::Failed => {
                self.reference_read_failure = Some(ReadStage::CollectionSpace);
                Ok(())
            }
            AsyncReadPoll::Complete(bytes) => {
                self.read = None;
                let space =
                    CollectionSpace::parse(&bytes).map_err(|source| InnerStreamError::Table {
                        stage: ReadStage::CollectionSpace,
                        source,
                    })?;
                backend
                    .collection_space_loaded(self.registration_context(), &space)
                    .map_err(InnerStreamError::Backend)?;
                self.collection_space = Some(space);
                self.state = StreamFileState::LOAD_DONE;
                Ok(())
            }
        }
    }

    fn poll_xml<B>(&mut self, backend: &mut B) -> Result<(), InnerStreamError<E>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        match self.xml_state.raw() {
            0 | 2 => Ok(()),
            1 => match self.poll_read(backend, ReadStage::StreamXml)? {
                AsyncReadPoll::Pending => Ok(()),
                AsyncReadPoll::Failed => {
                    self.read = None;
                    self.xml_state = StreamXmlState::LOAD_DONE;
                    Ok(())
                }
                AsyncReadPoll::Complete(bytes) => {
                    self.read = None;
                    backend
                        .stream_xml_loaded(self.registration_context(), &bytes)
                        .map_err(InnerStreamError::Backend)?;
                    self.stream_xml_loaded = true;
                    self.xml_state = StreamXmlState::LOAD_DONE;
                    Ok(())
                }
            },
            raw => Err(InnerStreamError::InvalidStreamXmlState(raw)),
        }
    }

    fn begin_read<B>(
        &mut self,
        backend: &mut B,
        request: AsyncReadRequest,
    ) -> Result<(), InnerStreamError<E>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        if let Some(read) = self.read.as_ref() {
            return Err(InnerStreamError::ReadAlreadyActive(read.stage));
        }
        let stage = request.stage;
        let handle = backend
            .begin_read(request)
            .map_err(InnerStreamError::Backend)?;
        self.read = Some(PendingRead { stage, handle });
        Ok(())
    }

    fn poll_read<B>(
        &mut self,
        backend: &mut B,
        expected: ReadStage,
    ) -> Result<AsyncReadPoll, InnerStreamError<E>>
    where
        B: InnerStreamBackend<Handle = H, Error = E>,
    {
        let read = self
            .read
            .as_mut()
            .ok_or(InnerStreamError::MissingRead(expected))?;
        if read.stage != expected {
            return Err(InnerStreamError::ReadStageMismatch {
                expected,
                actual: read.stage,
            });
        }
        backend
            .poll_read(&mut read.handle)
            .map_err(InnerStreamError::Backend)
    }
}

fn require_version<E>(stage: ReadStage, actual: u32) -> Result<(), InnerStreamError<E>> {
    if actual == TABLE_VERSION {
        Ok(())
    } else {
        Err(InnerStreamError::UnexpectedVersion {
            stage,
            expected: TABLE_VERSION,
            actual,
        })
    }
}

impl fmt::Display for StreamPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InteriorNul { field } => write!(formatter, "{field} contains an interior NUL"),
            Self::TooLong {
                field,
                length,
                capacity,
            } => write!(
                formatter,
                "{field} is {length} bytes, exceeding native capacity {capacity} including NUL"
            ),
        }
    }
}

impl std::error::Error for StreamPathError {}

impl<E: fmt::Debug> fmt::Display for InnerStreamError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<E> std::error::Error for InnerStreamError<E> where E: std::error::Error + 'static {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mounted_stream_backend::StreamRegistrationSink, world_stream_catalog::WorldStreamCatalog,
    };
    use std::collections::{HashSet, VecDeque};

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Event {
        Begin(AsyncReadRequest),
        Poll(ReadStage),
        MapLoaded(usize),
        TocLoaded(usize),
        SpaceLoaded(usize),
        XmlLoaded(StreamKind, Vec<u8>),
        MapUnloaded(usize),
        TocUnloaded(usize),
        StreamFileUnloaded(bool, bool),
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ContextSnapshot {
        id: RegistrationFileId,
        kind: StreamKind,
        paths: StreamPaths,
    }

    #[derive(Debug, Eq, PartialEq)]
    struct TestError(&'static str);

    impl fmt::Display for TestError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl std::error::Error for TestError {}

    struct ReadPlan {
        stage: ReadStage,
        polls: VecDeque<AsyncReadPoll>,
    }

    struct TestHandle {
        stage: ReadStage,
        polls: VecDeque<AsyncReadPoll>,
        failed: bool,
    }

    struct TestBackend {
        plans: VecDeque<ReadPlan>,
        events: Vec<Event>,
        contexts: Vec<ContextSnapshot>,
        fail_next_begin: bool,
        fail_next_poll: bool,
        fail_next_callback: bool,
        fail_callback_at: Option<usize>,
        callback_count: usize,
    }

    impl TestBackend {
        fn new(plans: impl IntoIterator<Item = (ReadStage, Vec<AsyncReadPoll>)>) -> Self {
            Self {
                plans: plans
                    .into_iter()
                    .map(|(stage, polls)| ReadPlan {
                        stage,
                        polls: polls.into(),
                    })
                    .collect(),
                events: Vec::new(),
                contexts: Vec::new(),
                fail_next_begin: false,
                fail_next_poll: false,
                fail_next_callback: false,
                fail_callback_at: None,
                callback_count: 0,
            }
        }

        fn callback(&mut self, event: Event) -> Result<(), TestError> {
            let callback_index = self.callback_count;
            self.callback_count += 1;
            if self.fail_next_callback || self.fail_callback_at == Some(callback_index) {
                self.fail_next_callback = false;
                return Err(TestError("callback"));
            }
            self.events.push(event);
            Ok(())
        }

        fn record_context(&mut self, context: StreamRegistrationContext<'_>) {
            self.contexts.push(ContextSnapshot {
                id: context.id().clone(),
                kind: context.kind(),
                paths: context.paths().clone(),
            });
        }
    }

    impl InnerStreamBackend for TestBackend {
        type Handle = TestHandle;
        type Error = TestError;

        fn begin_read(&mut self, request: AsyncReadRequest) -> Result<Self::Handle, Self::Error> {
            if self.fail_next_begin {
                self.fail_next_begin = false;
                return Err(TestError("begin"));
            }
            let plan = self.plans.pop_front().expect("unexpected read request");
            assert_eq!(plan.stage, request.stage);
            self.events.push(Event::Begin(request));
            Ok(TestHandle {
                stage: plan.stage,
                polls: plan.polls,
                failed: false,
            })
        }

        fn poll_read(&mut self, handle: &mut Self::Handle) -> Result<AsyncReadPoll, Self::Error> {
            if self.fail_next_poll {
                self.fail_next_poll = false;
                return Err(TestError("poll"));
            }
            self.events.push(Event::Poll(handle.stage));
            if handle.failed {
                return Ok(AsyncReadPoll::Failed);
            }
            let result = handle.polls.pop_front().unwrap_or(AsyncReadPoll::Pending);
            if result == AsyncReadPoll::Failed {
                handle.failed = true;
            }
            Ok(result)
        }

        fn collection_map_loaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            map: &CollectionMap,
        ) -> Result<(), Self::Error> {
            self.record_context(context);
            self.callback(Event::MapLoaded(map.collections.len()))
        }

        fn asset_toc_loaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            toc: &AssetToc,
        ) -> Result<(), Self::Error> {
            self.record_context(context);
            self.callback(Event::TocLoaded(toc.assets.len()))
        }

        fn collection_space_loaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            space: &CollectionSpace,
        ) -> Result<(), Self::Error> {
            self.record_context(context);
            self.callback(Event::SpaceLoaded(space.collections.len()))
        }

        fn stream_xml_loaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            bytes: &[u8],
        ) -> Result<(), Self::Error> {
            self.record_context(context);
            self.callback(Event::XmlLoaded(context.kind(), bytes.to_vec()))
        }

        fn collection_map_unloaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            map: &CollectionMap,
        ) -> Result<(), Self::Error> {
            self.record_context(context);
            self.callback(Event::MapUnloaded(map.collections.len()))
        }

        fn asset_toc_unloaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            toc: &AssetToc,
        ) -> Result<(), Self::Error> {
            self.record_context(context);
            self.callback(Event::TocUnloaded(toc.assets.len()))
        }

        fn stream_file_unloaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            space: Option<&CollectionSpace>,
            had_stream_xml: bool,
        ) -> Result<(), Self::Error> {
            self.record_context(context);
            self.callback(Event::StreamFileUnloaded(space.is_some(), had_stream_xml))
        }
    }

    struct CatalogBackend {
        io: TestBackend,
        catalog: WorldStreamCatalog,
    }

    impl CatalogBackend {
        fn new(plans: impl IntoIterator<Item = (ReadStage, Vec<AsyncReadPoll>)>) -> Self {
            Self {
                io: TestBackend::new(plans),
                catalog: WorldStreamCatalog::new(),
            }
        }

        fn catalog(
            result: Result<(), crate::world_stream_catalog::CatalogError>,
        ) -> Result<(), TestError> {
            result.map_err(|_| TestError("catalog"))
        }
    }

    impl InnerStreamBackend for CatalogBackend {
        type Handle = TestHandle;
        type Error = TestError;

        fn begin_read(&mut self, request: AsyncReadRequest) -> Result<Self::Handle, Self::Error> {
            self.io.begin_read(request)
        }

        fn poll_read(&mut self, handle: &mut Self::Handle) -> Result<AsyncReadPoll, Self::Error> {
            self.io.poll_read(handle)
        }

        fn collection_map_loaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            map: &CollectionMap,
        ) -> Result<(), Self::Error> {
            self.io.collection_map_loaded(context, map)?;
            Self::catalog(StreamRegistrationSink::collection_map_loaded(
                &mut self.catalog,
                context,
                map,
            ))
        }

        fn asset_toc_loaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            toc: &AssetToc,
        ) -> Result<(), Self::Error> {
            self.io.asset_toc_loaded(context, toc)?;
            Self::catalog(StreamRegistrationSink::asset_toc_loaded(
                &mut self.catalog,
                context,
                toc,
            ))
        }

        fn collection_space_loaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            space: &CollectionSpace,
        ) -> Result<(), Self::Error> {
            self.io.collection_space_loaded(context, space)?;
            Self::catalog(StreamRegistrationSink::collection_space_loaded(
                &mut self.catalog,
                context,
                space,
            ))
        }

        fn stream_xml_loaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            bytes: &[u8],
        ) -> Result<(), Self::Error> {
            self.io.stream_xml_loaded(context, bytes)?;
            Self::catalog(StreamRegistrationSink::stream_xml_loaded(
                &mut self.catalog,
                context,
                bytes,
            ))
        }

        fn collection_map_unloaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            map: &CollectionMap,
        ) -> Result<(), Self::Error> {
            self.io.collection_map_unloaded(context, map)?;
            Self::catalog(StreamRegistrationSink::collection_map_unloaded(
                &mut self.catalog,
                context,
                map,
            ))
        }

        fn asset_toc_unloaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            toc: &AssetToc,
        ) -> Result<(), Self::Error> {
            self.io.asset_toc_unloaded(context, toc)?;
            Self::catalog(StreamRegistrationSink::asset_toc_unloaded(
                &mut self.catalog,
                context,
                toc,
            ))
        }

        fn stream_file_unloaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            space: Option<&CollectionSpace>,
            had_stream_xml: bool,
        ) -> Result<(), Self::Error> {
            self.io
                .stream_file_unloaded(context, space, had_stream_xml)?;
            Self::catalog(StreamRegistrationSink::stream_file_unloaded(
                &mut self.catalog,
                context,
                space,
                had_stream_xml,
            ))
        }
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn collection_map_bytes(count: usize) -> Vec<u8> {
        let mut bytes = vec![0; 32 + count * 104];
        bytes[..4].copy_from_slice(b"CMAP");
        put_u32(&mut bytes, 4, TABLE_VERSION);
        put_u64(&mut bytes, 8, 0x1020_3040_5060_7080);
        put_u32(&mut bytes, 16, u32::try_from(count).unwrap());
        put_u32(&mut bytes, 20, 2);
        for index in 0..count {
            let start = 32 + index * 104;
            put_u64(&mut bytes, start, index as u64 + 1);
            put_u64(&mut bytes, start + 8, u64::MAX);
            let name = format!("collection_{index}");
            bytes[start + 28..start + 28 + name.len()].copy_from_slice(name.as_bytes());
        }
        bytes
    }

    fn asset_toc_bytes(count: usize) -> Vec<u8> {
        let mut bytes = vec![0; 24 + count * 64];
        bytes[..4].copy_from_slice(b"ATOC");
        put_u32(&mut bytes, 4, TABLE_VERSION);
        put_u64(&mut bytes, 8, 0x1020_3040_5060_7080);
        put_u32(&mut bytes, 16, u32::try_from(count).unwrap());
        for index in 0..count {
            let start = 24 + index * 64;
            put_u64(&mut bytes, start, 0x1000 + index as u64);
        }
        bytes
    }

    fn collection_space_bytes(count: usize) -> Vec<u8> {
        let mut bytes = vec![0; 24 + count * 80];
        bytes[..4].copy_from_slice(b"CSPA");
        put_u32(&mut bytes, 4, TABLE_VERSION);
        put_u64(&mut bytes, 8, 0x1020_3040_5060_7080);
        put_u32(&mut bytes, 16, u32::try_from(count).unwrap());
        for index in 0..count {
            let start = 24 + index * 80;
            put_u64(&mut bytes, start, 0x2000 + index as u64);
        }
        bytes
    }

    fn paths(kind: StreamKind) -> StreamPaths {
        StreamPaths::derive("data/content/world/stream", "BAM", kind).unwrap()
    }

    fn game_stream(kind: StreamKind) -> GameStream<TestHandle, TestError> {
        GameStream::new(kind, paths(kind), RegistrationFileId::new())
    }

    #[test]
    fn raw_state_wrappers_preserve_unusual_values() {
        assert_eq!(GameStreamState::from_raw(0xdead_beef).raw(), 0xdead_beef);
        assert_eq!(StreamFileState::from_raw(0x8000_0003).raw(), 0x8000_0003);
        assert_eq!(StreamXmlState::from_raw(u32::MAX).raw(), u32::MAX);
        assert_ne!(
            GameStreamState::LOAD_FAILED.raw(),
            StreamFileState::LOAD_DONE.raw()
        );
    }

    #[test]
    fn paths_preserve_input_slashes_suffixes_and_duplicate_separator() {
        let derived = paths(StreamKind::Presentation);
        assert_eq!(
            derived.collection_directory,
            "data/content/world/stream\\BAM\\"
        );
        assert_eq!(
            derived.manifest_stem,
            "data/content/world/stream\\BAM\\BAM_Pres"
        );
        assert_eq!(
            derived.collection_map(),
            format!("{}.xsm", derived.manifest_stem)
        );
        assert_eq!(
            derived.asset_toc(),
            format!("{}.xst", derived.manifest_stem)
        );
        assert_eq!(
            derived.collection_space,
            format!("{}.xss", derived.manifest_stem)
        );
        assert_eq!(derived.stream_xml, "data\\stream\\BAM_Pres.xml");

        let duplicate = StreamPaths::derive("base\\", "name", StreamKind::Texture).unwrap();
        assert_eq!(duplicate.collection_directory, "base\\\\name\\");
    }

    #[test]
    fn registration_identity_clones_compare_and_hash_by_owned_allocation() {
        let first =
            StreamFile::<TestHandle, TestError>::load("root", "BAM", StreamKind::Presentation)
                .unwrap();
        let second =
            StreamFile::<TestHandle, TestError>::load("root", "BAM", StreamKind::Presentation)
                .unwrap();
        let clone = first.registration_id().clone();

        assert_eq!(clone, *first.registration_id());
        assert_ne!(clone, *second.registration_id());
        let mut ids = HashSet::new();
        assert!(ids.insert(clone.clone()));
        assert!(!ids.insert(first.registration_id().clone()));
        assert!(ids.insert(second.registration_id().clone()));
        assert!(ids.contains(&clone));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn registration_context_retains_mixed_and_duplicate_separator_paths() {
        let file = StreamFile::<TestHandle, TestError>::load(
            "base/stream\\",
            "BAM/sub",
            StreamKind::Texture,
        )
        .unwrap();
        let context = file.registration_context();

        assert_eq!(context.id(), file.registration_id());
        assert_eq!(context.kind(), StreamKind::Texture);
        assert_eq!(
            context.paths().collection_directory,
            "base/stream\\\\BAM/sub\\"
        );
        assert_eq!(
            context.paths().manifest_stem,
            "base/stream\\\\BAM/sub\\BAM/sub_Tex"
        );
    }

    #[test]
    fn every_callback_for_one_stream_file_receives_the_same_context() {
        let mut backend = TestBackend::new([
            (
                ReadStage::CollectionMap,
                vec![AsyncReadPoll::Complete(collection_map_bytes(1))],
            ),
            (
                ReadStage::AssetToc,
                vec![AsyncReadPoll::Complete(asset_toc_bytes(1))],
            ),
            (
                ReadStage::CollectionSpace,
                vec![AsyncReadPoll::Complete(collection_space_bytes(1))],
            ),
            (
                ReadStage::StreamXml,
                vec![AsyncReadPoll::Complete(b"<stream/>".to_vec())],
            ),
        ]);
        let mut file = StreamFile::load(
            "data/content\\world/stream\\",
            "BAM",
            StreamKind::Presentation,
        )
        .unwrap();
        let expected_id = file.registration_id().clone();
        let expected_paths = file.paths().clone();

        for _ in 0..4 {
            file.poll(&mut backend).unwrap();
        }
        assert_eq!(file.state(), StreamFileState::LOAD_DONE);
        file.begin_stream_xml(&mut backend).unwrap();
        file.poll(&mut backend).unwrap();
        file.request_unload().unwrap();
        for _ in 0..3 {
            file.poll(&mut backend).unwrap();
        }
        assert_eq!(file.state(), StreamFileState::UNLOAD_DONE);

        assert_eq!(backend.contexts.len(), 7);
        assert!(backend.contexts.iter().all(|context| {
            context.id == expected_id
                && context.kind == StreamKind::Presentation
                && context.paths == expected_paths
        }));
        assert_eq!(
            backend
                .contexts
                .iter()
                .map(|context| context.id.clone())
                .collect::<HashSet<_>>()
                .len(),
            1
        );
    }

    #[test]
    fn paths_reject_nul_and_native_buffer_overflow() {
        assert_eq!(
            StreamPaths::derive("bad\0base", "BAM", StreamKind::Simulation),
            Err(StreamPathError::InteriorNul { field: "base_path" })
        );
        assert!(matches!(
            StreamPaths::derive(&"a".repeat(250), "BAM", StreamKind::Simulation),
            Err(StreamPathError::TooLong { .. })
        ));

        let accepted =
            StreamPaths::derive(&"a".repeat(242), "B", StreamKind::Presentation).unwrap();
        assert_eq!(accepted.collection_space.len(), 255);
        assert_eq!(accepted.collection_map().len(), 255);
        assert_eq!(accepted.asset_toc().len(), 255);
        assert!(matches!(
            StreamPaths::derive(&"a".repeat(243), "B", StreamKind::Presentation),
            Err(StreamPathError::TooLong {
                length: 256,
                capacity: 256,
                ..
            })
        ));
    }

    #[test]
    fn game_stream_chains_xsm_to_xst_in_the_completion_poll_without_a_16_cap() {
        let map = collection_map_bytes(17);
        let toc = asset_toc_bytes(2);
        let mut backend = TestBackend::new([
            (
                ReadStage::CollectionMap,
                vec![AsyncReadPoll::Pending, AsyncReadPoll::Complete(map)],
            ),
            (
                ReadStage::AssetToc,
                vec![AsyncReadPoll::Pending, AsyncReadPoll::Complete(toc)],
            ),
        ]);
        let mut stream = game_stream(StreamKind::Simulation);

        stream.poll(&mut backend).unwrap();
        assert_eq!(stream.state(), GameStreamState::LOADING_MAP);
        stream.poll(&mut backend).unwrap();
        assert_eq!(stream.state(), GameStreamState::LOADING_MAP);
        stream.poll(&mut backend).unwrap();
        assert_eq!(stream.state(), GameStreamState::LOADING_TOC);
        assert_eq!(stream.collection_map().unwrap().collections.len(), 17);
        stream.poll(&mut backend).unwrap();
        assert_eq!(stream.state(), GameStreamState::LOADING_TOC);
        stream.poll(&mut backend).unwrap();
        assert_eq!(stream.state(), GameStreamState::LOADED);

        assert!(backend.events.windows(2).any(|events| {
            matches!(events, [Event::MapLoaded(17), Event::Begin(request)]
                if request.stage == ReadStage::AssetToc)
        }));
        for request in backend.events.iter().filter_map(|event| match event {
            Event::Begin(request) => Some(request),
            _ => None,
        }) {
            assert_eq!(request.flags, 1);
            assert_eq!(request.alignment, 0x10);
        }
    }

    #[test]
    fn manifest_only_stream_stops_after_xst_and_releases_its_file_owner() {
        let mut backend = TestBackend::new([
            (
                ReadStage::CollectionMap,
                vec![AsyncReadPoll::Complete(collection_map_bytes(1))],
            ),
            (
                ReadStage::AssetToc,
                vec![AsyncReadPoll::Complete(asset_toc_bytes(2))],
            ),
        ]);
        let mut stream =
            ManifestOnlyStream::load("data\\content\\world\\dmo", "DMO", StreamKind::Presentation)
                .unwrap();
        let expected_id = stream.registration_id().clone();
        assert_eq!(
            stream.paths().manifest_stem,
            "data\\content\\world\\dmo\\DMO\\DMO_Pres"
        );

        for _ in 0..3 {
            stream.poll(&mut backend).unwrap();
        }
        assert_eq!(stream.state(), GameStreamState::LOADED);
        assert_eq!(stream.collection_map().unwrap().collections.len(), 1);
        assert_eq!(stream.asset_toc().unwrap().assets.len(), 2);
        assert!(!stream.is_released());
        assert!(backend.events.iter().all(|event| {
            !matches!(
                event,
                Event::Begin(AsyncReadRequest {
                    stage: ReadStage::CollectionSpace | ReadStage::StreamXml,
                    ..
                })
            )
        }));
        let requests = backend
            .events
            .iter()
            .filter_map(|event| match event {
                Event::Begin(request) => Some(request),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].path, stream.paths().collection_map());
        assert_eq!(requests[1].path, stream.paths().asset_toc());
        assert!(
            requests
                .iter()
                .all(|request| request.flags == 1 && request.alignment == 0x10)
        );

        stream.request_unload().unwrap();
        for _ in 0..3 {
            stream.poll(&mut backend).unwrap();
        }
        assert_eq!(stream.state(), GameStreamState::UNLOADED);
        assert!(stream.is_released());
        assert_eq!(
            &backend.events[backend.events.len() - 3..],
            [
                Event::MapUnloaded(1),
                Event::TocUnloaded(2),
                Event::StreamFileUnloaded(false, false),
            ]
        );
        assert!(backend.contexts.iter().all(|context| {
            context.id == expected_id
                && context.kind == StreamKind::Presentation
                && context.paths == *stream.paths()
        }));
    }

    #[test]
    fn manifest_only_stream_uses_unique_identity_and_latches_final_release_failure() {
        let first = ManifestOnlyStream::<TestHandle, TestError>::load(
            "root",
            "bank",
            StreamKind::Presentation,
        )
        .unwrap();
        let second = ManifestOnlyStream::<TestHandle, TestError>::load(
            "root",
            "bank",
            StreamKind::Presentation,
        )
        .unwrap();
        assert_ne!(first.registration_id(), second.registration_id());

        let mut backend = TestBackend::new([
            (
                ReadStage::CollectionMap,
                vec![AsyncReadPoll::Complete(collection_map_bytes(1))],
            ),
            (
                ReadStage::AssetToc,
                vec![AsyncReadPoll::Complete(asset_toc_bytes(1))],
            ),
        ]);
        backend.fail_callback_at = Some(4);
        let mut stream =
            ManifestOnlyStream::load("root", "bank", StreamKind::Presentation).unwrap();
        for _ in 0..3 {
            stream.poll(&mut backend).unwrap();
        }
        stream.request_unload().unwrap();
        stream.poll(&mut backend).unwrap();
        stream.poll(&mut backend).unwrap();
        let first = stream.poll(&mut backend).unwrap_err();
        let event_count = backend.events.len();
        let second = stream.poll(&mut backend).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(Arc::ptr_eq(stream.terminal_error().unwrap(), &first));
        assert!(matches!(
            first.as_ref(),
            InnerStreamError::Backend(TestError("callback"))
        ));
        assert!(!stream.is_released());
        assert_eq!(backend.events.len(), event_count);
    }

    #[test]
    fn manifest_only_stream_preserves_nested_error_identity_and_drop_is_inert() {
        let mut malformed = TestBackend::new([(
            ReadStage::CollectionMap,
            vec![AsyncReadPoll::Complete(b"bad".to_vec())],
        )]);
        let mut stream =
            ManifestOnlyStream::load("root", "bank", StreamKind::Presentation).unwrap();
        stream.poll(&mut malformed).unwrap();
        let first = stream.poll(&mut malformed).unwrap_err();
        let event_count = malformed.events.len();
        let second = stream.poll(&mut malformed).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(Arc::ptr_eq(stream.terminal_error().unwrap(), &first));
        assert_eq!(malformed.events.len(), event_count);

        let mut loaded = TestBackend::new([
            (
                ReadStage::CollectionMap,
                vec![AsyncReadPoll::Complete(collection_map_bytes(1))],
            ),
            (
                ReadStage::AssetToc,
                vec![AsyncReadPoll::Complete(asset_toc_bytes(1))],
            ),
        ]);
        let mut stream =
            ManifestOnlyStream::load("root", "bank", StreamKind::Presentation).unwrap();
        for _ in 0..3 {
            stream.poll(&mut loaded).unwrap();
        }
        let event_count = loaded.events.len();
        drop(stream);
        assert_eq!(loaded.events.len(), event_count);
        assert!(!loaded.events.iter().any(|event| matches!(
            event,
            Event::MapUnloaded(_) | Event::TocUnloaded(_) | Event::StreamFileUnloaded(..)
        )));
    }

    #[test]
    fn manifest_only_stream_catalog_lifecycle_and_faulted_prefixes_are_explicit() {
        let plans = [
            (
                ReadStage::CollectionMap,
                vec![AsyncReadPoll::Complete(collection_map_bytes(1))],
            ),
            (
                ReadStage::AssetToc,
                vec![AsyncReadPoll::Complete(asset_toc_bytes(2))],
            ),
        ];
        let mut backend = CatalogBackend::new(plans);
        let mut stream =
            ManifestOnlyStream::load("root", "bank", StreamKind::Presentation).unwrap();
        let owner = stream.registration_id().clone();
        for _ in 0..3 {
            stream.poll(&mut backend).unwrap();
        }
        let loaded = backend.catalog.counts().unwrap();
        assert_eq!(loaded.files, 1);
        assert_eq!(loaded.collections, 1);
        assert_eq!(loaded.asset_entries, 2);
        assert_eq!(loaded.asset_references, 2);
        assert!(backend.catalog.file(&owner).unwrap().is_some());

        stream.request_unload().unwrap();
        for _ in 0..3 {
            stream.poll(&mut backend).unwrap();
        }
        let released = backend.catalog.counts().unwrap();
        assert_eq!(released.files, 0);
        assert_eq!(released.collections, 0);
        assert_eq!(released.asset_entries, 0);
        assert_eq!(released.asset_references, 0);
        assert!(backend.catalog.file(&owner).unwrap().is_none());
        let event_count = backend.io.events.len();
        stream.poll(&mut backend).unwrap();
        assert_eq!(backend.io.events.len(), event_count);

        let mut parse_fault = CatalogBackend::new([
            (
                ReadStage::CollectionMap,
                vec![AsyncReadPoll::Complete(collection_map_bytes(1))],
            ),
            (
                ReadStage::AssetToc,
                vec![AsyncReadPoll::Complete(b"bad".to_vec())],
            ),
        ]);
        let mut stream =
            ManifestOnlyStream::load("root", "fault", StreamKind::Presentation).unwrap();
        stream.poll(&mut parse_fault).unwrap();
        stream.poll(&mut parse_fault).unwrap();
        stream.poll(&mut parse_fault).unwrap_err();
        let retained_prefix = parse_fault.catalog.counts().unwrap();
        assert_eq!(retained_prefix.files, 1);
        assert_eq!(retained_prefix.collections, 1);
        assert_eq!(retained_prefix.asset_entries, 0);
        stream.request_unload().unwrap();
        stream.poll(&mut parse_fault).unwrap_err();
        assert_eq!(parse_fault.catalog.counts().unwrap().files, 1);

        let plans = [
            (
                ReadStage::CollectionMap,
                vec![AsyncReadPoll::Complete(collection_map_bytes(1))],
            ),
            (
                ReadStage::AssetToc,
                vec![AsyncReadPoll::Complete(asset_toc_bytes(1))],
            ),
        ];
        let mut unload_fault = CatalogBackend::new(plans);
        unload_fault.io.fail_callback_at = Some(3);
        let mut stream =
            ManifestOnlyStream::load("root", "fault", StreamKind::Presentation).unwrap();
        for _ in 0..3 {
            stream.poll(&mut unload_fault).unwrap();
        }
        stream.request_unload().unwrap();
        stream.poll(&mut unload_fault).unwrap();
        stream.poll(&mut unload_fault).unwrap_err();
        let retained_suffix = unload_fault.catalog.counts().unwrap();
        assert_eq!(retained_suffix.files, 1);
        assert_eq!(retained_suffix.collections, 0);
        assert_eq!(retained_suffix.asset_entries, 1);
        assert_eq!(retained_suffix.asset_references, 1);
    }

    #[test]
    fn map_read_failure_becomes_raw_8_but_toc_failure_stalls_raw_3() {
        let mut map_failure =
            TestBackend::new([(ReadStage::CollectionMap, vec![AsyncReadPoll::Failed])]);
        let mut failed = game_stream(StreamKind::Texture);
        failed.poll(&mut map_failure).unwrap();
        failed.poll(&mut map_failure).unwrap();
        assert_eq!(failed.state(), GameStreamState::LOAD_FAILED);
        assert_eq!(
            failed.reference_read_failure(),
            Some(ReadStage::CollectionMap)
        );
        failed.poll(&mut map_failure).unwrap();
        assert_eq!(failed.state(), GameStreamState::LOAD_FAILED);

        let mut toc_failure = TestBackend::new([
            (
                ReadStage::CollectionMap,
                vec![AsyncReadPoll::Complete(collection_map_bytes(1))],
            ),
            (
                ReadStage::AssetToc,
                vec![AsyncReadPoll::Failed, AsyncReadPoll::Failed],
            ),
        ]);
        let mut stalled = game_stream(StreamKind::Texture);
        stalled.poll(&mut toc_failure).unwrap();
        stalled.poll(&mut toc_failure).unwrap();
        stalled.poll(&mut toc_failure).unwrap();
        stalled.poll(&mut toc_failure).unwrap();
        assert_eq!(stalled.state(), GameStreamState::LOADING_TOC);
        assert_eq!(stalled.reference_read_failure(), Some(ReadStage::AssetToc));
        assert_eq!(
            toc_failure
                .events
                .iter()
                .filter(|event| **event == Event::Poll(ReadStage::AssetToc))
                .count(),
            2
        );
    }

    #[test]
    fn nested_raw_8_remains_owned_by_game_stream_and_file_stays_raw_1() {
        let mut backend =
            TestBackend::new([(ReadStage::CollectionMap, vec![AsyncReadPoll::Failed])]);
        let mut file = StreamFile::load("root", "BAM", StreamKind::Texture).unwrap();

        file.poll(&mut backend).unwrap();
        file.poll(&mut backend).unwrap();
        assert_eq!(file.game_stream_state(), GameStreamState::LOAD_FAILED);
        assert_eq!(file.state(), StreamFileState::LOAD_STREAM_WAIT);
        file.poll(&mut backend).unwrap();
        assert_eq!(file.game_stream_state().raw(), 8);
        assert_eq!(file.state().raw(), 1);
    }

    #[test]
    fn stream_file_chains_nested_loaded_to_xss_and_xss_failure_stalls() {
        let mut backend = TestBackend::new([
            (
                ReadStage::CollectionMap,
                vec![AsyncReadPoll::Complete(collection_map_bytes(1))],
            ),
            (
                ReadStage::AssetToc,
                vec![AsyncReadPoll::Complete(asset_toc_bytes(1))],
            ),
            (ReadStage::CollectionSpace, vec![AsyncReadPoll::Failed]),
        ]);
        let mut file = StreamFile::load(
            "data\\content\\world\\stream",
            "BAM",
            StreamKind::Presentation,
        )
        .unwrap();

        file.poll(&mut backend).unwrap();
        file.poll(&mut backend).unwrap();
        file.poll(&mut backend).unwrap();
        assert_eq!(file.game_stream_state(), GameStreamState::LOADED);
        assert_eq!(file.state(), StreamFileState::LOAD_SPACE_FILE_WAIT);
        assert!(matches!(backend.events.last(), Some(Event::Begin(request))
            if request.stage == ReadStage::CollectionSpace
                && request.flags == 0
                && request.alignment == 0x10));

        file.poll(&mut backend).unwrap();
        file.poll(&mut backend).unwrap();
        file.poll(&mut backend).unwrap();
        assert_eq!(file.state(), StreamFileState::LOAD_SPACE_FILE_WAIT);
        assert_eq!(
            file.reference_read_failure(),
            Some(ReadStage::CollectionSpace)
        );
        assert_eq!(
            backend
                .events
                .iter()
                .filter(|event| **event == Event::Poll(ReadStage::CollectionSpace))
                .count(),
            3
        );
    }

    #[test]
    fn stream_file_completes_xss_on_its_own_poll() {
        let mut space = collection_space_bytes(17);
        put_u32(&mut space, 4, 0xfeed_beef);
        let mut backend = TestBackend::new([
            (
                ReadStage::CollectionMap,
                vec![AsyncReadPoll::Complete(collection_map_bytes(1))],
            ),
            (
                ReadStage::AssetToc,
                vec![AsyncReadPoll::Complete(asset_toc_bytes(1))],
            ),
            (
                ReadStage::CollectionSpace,
                vec![AsyncReadPoll::Pending, AsyncReadPoll::Complete(space)],
            ),
        ]);
        let mut file = StreamFile::load("root", "BAM", StreamKind::Simulation).unwrap();
        for _ in 0..4 {
            file.poll(&mut backend).unwrap();
        }
        assert_eq!(file.state(), StreamFileState::LOAD_SPACE_FILE_WAIT);
        file.poll(&mut backend).unwrap();
        assert_eq!(file.state(), StreamFileState::LOAD_DONE);
        assert_eq!(file.collection_space().unwrap().collections.len(), 17);
        assert_eq!(file.collection_space().unwrap().version, 0xfeed_beef);
        assert_eq!(backend.events.last(), Some(&Event::SpaceLoaded(17)));
    }

    #[test]
    fn pres_and_sim_xml_are_async_but_texture_is_rejected() {
        for kind in [StreamKind::Presentation, StreamKind::Simulation] {
            let bytes = vec![u8::try_from(kind as u32).unwrap(), 0xaa];
            let mut backend = TestBackend::new([(
                ReadStage::StreamXml,
                vec![
                    AsyncReadPoll::Pending,
                    AsyncReadPoll::Complete(bytes.clone()),
                ],
            )]);
            let mut file = StreamFile::load("root", "BAM", kind).unwrap();
            file.state = StreamFileState::LOAD_DONE;
            file.game_stream.state = GameStreamState::LOADED;
            file.begin_stream_xml(&mut backend).unwrap();
            assert_eq!(file.xml_state(), StreamXmlState::LOAD_STREAM_XML_WAIT);
            assert!(matches!(backend.events.first(), Some(Event::Begin(request))
                if request.stage == ReadStage::StreamXml
                    && request.path == format!("data\\stream\\BAM_{}.xml", kind.suffix())
                    && request.flags == 0
                    && request.alignment == 0x10));
            assert!(
                !backend
                    .events
                    .iter()
                    .any(|event| matches!(event, Event::XmlLoaded(..)))
            );
            file.poll(&mut backend).unwrap();
            assert_eq!(file.xml_state(), StreamXmlState::LOAD_STREAM_XML_WAIT);
            file.poll(&mut backend).unwrap();
            assert_eq!(file.xml_state(), StreamXmlState::LOAD_DONE);
            assert_eq!(backend.events.last(), Some(&Event::XmlLoaded(kind, bytes)));
        }

        let mut backend = TestBackend::new([]);
        let mut texture = StreamFile::load("root", "BAM", StreamKind::Texture).unwrap();
        let error = texture.begin_stream_xml(&mut backend).unwrap_err();
        assert!(matches!(
            error.as_ref(),
            InnerStreamError::TextureStreamHasNoOuterXml
        ));
    }

    #[test]
    fn xml_failure_is_done_while_xss_failure_is_not() {
        let mut backend = TestBackend::new([(ReadStage::StreamXml, vec![AsyncReadPoll::Failed])]);
        let mut file = StreamFile::load("root", "BAM", StreamKind::Presentation).unwrap();
        file.state = StreamFileState::LOAD_DONE;
        file.game_stream.state = GameStreamState::LOADED;
        file.begin_stream_xml(&mut backend).unwrap();
        file.poll(&mut backend).unwrap();
        assert_eq!(file.xml_state(), StreamXmlState::LOAD_DONE);
        assert_eq!(file.reference_read_failure(), None);
        assert!(
            !backend
                .events
                .iter()
                .any(|event| matches!(event, Event::XmlLoaded(..)))
        );
    }

    #[test]
    fn malformed_tables_and_wrong_versions_are_explicit_errors() {
        let cases = [
            (ReadStage::CollectionMap, b"bad".to_vec()),
            (ReadStage::AssetToc, b"bad".to_vec()),
        ];
        for (stage, bytes) in cases {
            let mut backend = TestBackend::new([
                (
                    ReadStage::CollectionMap,
                    vec![AsyncReadPoll::Complete(
                        if stage == ReadStage::CollectionMap {
                            bytes.clone()
                        } else {
                            collection_map_bytes(1)
                        },
                    )],
                ),
                (ReadStage::AssetToc, vec![AsyncReadPoll::Complete(bytes)]),
            ]);
            let mut stream = game_stream(StreamKind::Simulation);
            stream.poll(&mut backend).unwrap();
            let result = stream.poll(&mut backend);
            let first = if stage == ReadStage::AssetToc {
                assert!(result.is_ok());
                stream.poll(&mut backend).unwrap_err()
            } else {
                result.unwrap_err()
            };
            let second = stream.poll(&mut backend).unwrap_err();
            assert!(Arc::ptr_eq(&first, &second));
            assert!(matches!(
                first.as_ref(),
                InnerStreamError::Table {
                    stage: actual_stage,
                    ..
                } if *actual_stage == stage
            ));
        }

        let mut wrong = collection_map_bytes(1);
        put_u32(&mut wrong, 4, 4);
        let mut backend = TestBackend::new([(
            ReadStage::CollectionMap,
            vec![AsyncReadPoll::Complete(wrong)],
        )]);
        let mut stream = game_stream(StreamKind::Simulation);
        stream.poll(&mut backend).unwrap();
        let error = stream.poll(&mut backend).unwrap_err();
        let repeated = stream.poll(&mut backend).unwrap_err();
        assert!(Arc::ptr_eq(&error, &repeated));
        assert!(matches!(
            error.as_ref(),
            InnerStreamError::UnexpectedVersion {
                stage: ReadStage::CollectionMap,
                expected: 3,
                actual: 4
            }
        ));

        let mut backend = TestBackend::new([(
            ReadStage::CollectionSpace,
            vec![AsyncReadPoll::Complete(b"bad".to_vec())],
        )]);
        let mut file = StreamFile::load("root", "BAM", StreamKind::Texture).unwrap();
        file.state = StreamFileState::LOAD_SPACE_FILE_WAIT;
        file.read = Some(PendingRead {
            stage: ReadStage::CollectionSpace,
            handle: backend
                .begin_read(AsyncReadRequest {
                    stage: ReadStage::CollectionSpace,
                    path: "bad.xss".to_owned(),
                    flags: 0,
                    alignment: 0x10,
                })
                .unwrap(),
        });
        let error = file.poll(&mut backend).unwrap_err();
        let repeated = file.poll(&mut backend).unwrap_err();
        assert!(Arc::ptr_eq(&error, &repeated));
        assert!(matches!(
            error.as_ref(),
            InnerStreamError::Table {
                stage: ReadStage::CollectionSpace,
                ..
            }
        ));
    }

    #[test]
    fn invalid_raw_states_and_missing_reads_are_invariants_not_pending() {
        let mut backend = TestBackend::new([]);
        let mut stream = game_stream(StreamKind::Texture);
        stream.state = GameStreamState::from_raw(0xf00d);
        let error = stream.poll(&mut backend).unwrap_err();
        assert!(matches!(
            error.as_ref(),
            InnerStreamError::InvalidGameStreamState(0xf00d)
        ));

        let mut invalid_file = StreamFile::load("root", "BAM", StreamKind::Texture).unwrap();
        invalid_file.state = StreamFileState::from_raw(3);
        let error = invalid_file.poll(&mut backend).unwrap_err();
        assert!(matches!(
            error.as_ref(),
            InnerStreamError::InvalidStreamFileState(3)
        ));

        let mut invalid_xml = StreamFile::load("root", "BAM", StreamKind::Texture).unwrap();
        invalid_xml.state = StreamFileState::LOAD_DONE;
        invalid_xml.xml_state = StreamXmlState::from_raw(9);
        let error = invalid_xml.poll(&mut backend).unwrap_err();
        assert!(matches!(
            error.as_ref(),
            InnerStreamError::InvalidStreamXmlState(9)
        ));

        let mut missing_read = StreamFile::load("root", "BAM", StreamKind::Texture).unwrap();
        missing_read.state = StreamFileState::LOAD_SPACE_FILE_WAIT;
        let error = missing_read.poll(&mut backend).unwrap_err();
        assert!(matches!(
            error.as_ref(),
            InnerStreamError::MissingRead(ReadStage::CollectionSpace)
        ));
    }

    #[test]
    fn unload_uses_three_nested_polls_and_finishes_file_when_raw_7_is_visible() {
        let map = CollectionMap::parse(&collection_map_bytes(17)).unwrap();
        let toc = AssetToc::parse(&asset_toc_bytes(2)).unwrap();
        let space = CollectionSpace::parse(&collection_space_bytes(1)).unwrap();
        let mut backend = TestBackend::new([]);
        let mut file = StreamFile::load("root", "BAM", StreamKind::Texture).unwrap();
        file.state = StreamFileState::LOAD_DONE;
        file.game_stream.state = GameStreamState::LOADED;
        file.game_stream.collection_map = Some(map);
        file.game_stream.asset_toc = Some(toc);
        file.collection_space = Some(space);

        file.request_unload().unwrap();
        assert_eq!(file.state(), StreamFileState::UNLOAD_WAIT);
        assert!(file.game_stream.unload_requested());
        file.poll(&mut backend).unwrap();
        assert_eq!(file.game_stream_state(), GameStreamState::BEGIN_UNLOAD);
        file.poll(&mut backend).unwrap();
        assert_eq!(file.game_stream_state(), GameStreamState::UNLOADING);
        file.poll(&mut backend).unwrap();
        assert_eq!(file.game_stream_state(), GameStreamState::UNLOADED);
        assert_eq!(file.state(), StreamFileState::UNLOAD_DONE);
        assert_eq!(
            backend.events,
            vec![
                Event::MapUnloaded(17),
                Event::TocUnloaded(2),
                Event::StreamFileUnloaded(true, false),
            ]
        );
    }

    #[test]
    fn unload_during_map_completion_discards_bytes_without_parsing() {
        let mut backend = TestBackend::new([(
            ReadStage::CollectionMap,
            vec![AsyncReadPoll::Complete(b"not a map".to_vec())],
        )]);
        let mut stream = game_stream(StreamKind::Simulation);
        stream.poll(&mut backend).unwrap();
        stream.request_unload().unwrap();
        stream.poll(&mut backend).unwrap();
        assert_eq!(stream.state(), GameStreamState::BEGIN_UNLOAD);
    }

    #[test]
    fn poll_errors_are_terminal_and_preserve_the_first_cause() {
        let mut malformed = TestBackend::new([(
            ReadStage::CollectionMap,
            vec![AsyncReadPoll::Complete(b"bad".to_vec())],
        )]);
        let mut stream = game_stream(StreamKind::Simulation);
        stream.poll(&mut malformed).unwrap();
        let first = stream.poll(&mut malformed).unwrap_err();
        let event_count = malformed.events.len();
        let second = stream.poll(&mut malformed).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(Arc::ptr_eq(
            stream.terminal_error().expect("stored terminal error"),
            &first
        ));
        assert_eq!(malformed.events.len(), event_count);
        assert!(matches!(
            first.as_ref(),
            InnerStreamError::Table {
                stage: ReadStage::CollectionMap,
                ..
            }
        ));

        let mut next_begin = TestBackend::new([(
            ReadStage::CollectionMap,
            vec![AsyncReadPoll::Complete(collection_map_bytes(1))],
        )]);
        let mut stream = game_stream(StreamKind::Presentation);
        stream.poll(&mut next_begin).unwrap();
        next_begin.fail_next_begin = true;
        let first = stream.poll(&mut next_begin).unwrap_err();
        let second = stream.poll(&mut next_begin).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(matches!(
            first.as_ref(),
            InnerStreamError::Backend(TestError("begin"))
        ));
        assert_eq!(
            next_begin
                .events
                .iter()
                .filter(|event| matches!(event, Event::MapLoaded(1)))
                .count(),
            1
        );
    }

    #[test]
    fn file_and_xml_callback_errors_are_terminal_without_repeating_side_effects() {
        let mut space_backend = TestBackend::new([(
            ReadStage::CollectionSpace,
            vec![AsyncReadPoll::Complete(collection_space_bytes(1))],
        )]);
        let mut file = StreamFile::load("root", "BAM", StreamKind::Simulation).unwrap();
        file.state = StreamFileState::LOAD_SPACE_FILE_WAIT;
        file.game_stream.state = GameStreamState::LOADED;
        file.read = Some(PendingRead {
            stage: ReadStage::CollectionSpace,
            handle: space_backend
                .begin_read(AsyncReadRequest {
                    stage: ReadStage::CollectionSpace,
                    path: "space.xss".to_owned(),
                    flags: 0,
                    alignment: 0x10,
                })
                .unwrap(),
        });
        space_backend.fail_next_callback = true;
        let first = file.poll(&mut space_backend).unwrap_err();
        let event_count = space_backend.events.len();
        let second = file.poll(&mut space_backend).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(space_backend.events.len(), event_count);

        let mut xml_backend = TestBackend::new([(
            ReadStage::StreamXml,
            vec![AsyncReadPoll::Complete(b"<stream/>".to_vec())],
        )]);
        let mut file = StreamFile::load("root", "BAM", StreamKind::Presentation).unwrap();
        file.state = StreamFileState::LOAD_DONE;
        file.game_stream.state = GameStreamState::LOADED;
        file.begin_stream_xml(&mut xml_backend).unwrap();
        xml_backend.fail_next_callback = true;
        let first = file.poll(&mut xml_backend).unwrap_err();
        let second = file.poll(&mut xml_backend).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(matches!(
            first.as_ref(),
            InnerStreamError::Backend(TestError("callback"))
        ));
    }

    #[test]
    fn partial_unload_failure_is_not_retried() {
        let map = CollectionMap::parse(&collection_map_bytes(1)).unwrap();
        let toc = AssetToc::parse(&asset_toc_bytes(1)).unwrap();
        let mut backend = TestBackend::new([]);
        backend.fail_callback_at = Some(1);
        let mut stream = game_stream(StreamKind::Texture);
        stream.state = GameStreamState::BEGIN_UNLOAD;
        stream.collection_map = Some(map);
        stream.asset_toc = Some(toc);

        let first = stream.poll(&mut backend).unwrap_err();
        let second = stream.poll(&mut backend).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(
            backend
                .events
                .iter()
                .filter(|event| matches!(event, Event::MapUnloaded(1)))
                .count(),
            1
        );
        assert!(
            !backend
                .events
                .iter()
                .any(|event| matches!(event, Event::TocUnloaded(_)))
        );
    }

    #[test]
    fn xml_start_requires_the_proven_loaded_file_order() {
        let mut backend = TestBackend::new([]);
        let mut file = StreamFile::load("root", "BAM", StreamKind::Presentation).unwrap();
        let first = file.begin_stream_xml(&mut backend).unwrap_err();
        let second = file.begin_stream_xml(&mut backend).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(matches!(
            first.as_ref(),
            InnerStreamError::StreamXmlRequiresLoadedFile(1)
        ));
        assert_eq!(file.xml_state(), StreamXmlState::INIT);
        assert!(backend.events.is_empty());
    }

    #[test]
    fn backend_begin_poll_and_callback_failures_are_propagated() {
        let mut begin = TestBackend::new([]);
        begin.fail_next_begin = true;
        let mut stream = game_stream(StreamKind::Texture);
        let error = stream.poll(&mut begin).unwrap_err();
        assert!(matches!(
            error.as_ref(),
            InnerStreamError::Backend(TestError("begin"))
        ));

        let mut poll = TestBackend::new([(ReadStage::CollectionMap, vec![AsyncReadPoll::Pending])]);
        let mut stream = game_stream(StreamKind::Texture);
        stream.poll(&mut poll).unwrap();
        poll.fail_next_poll = true;
        let error = stream.poll(&mut poll).unwrap_err();
        assert!(matches!(
            error.as_ref(),
            InnerStreamError::Backend(TestError("poll"))
        ));

        let mut callback = TestBackend::new([(
            ReadStage::CollectionMap,
            vec![AsyncReadPoll::Complete(collection_map_bytes(1))],
        )]);
        let mut stream = game_stream(StreamKind::Texture);
        stream.poll(&mut callback).unwrap();
        callback.fail_next_callback = true;
        let error = stream.poll(&mut callback).unwrap_err();
        assert!(matches!(
            error.as_ref(),
            InnerStreamError::Backend(TestError("callback"))
        ));
    }
}
