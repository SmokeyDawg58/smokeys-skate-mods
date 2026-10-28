//! Selected Juice constructor/Start and installed remap over one retained owner.
//! `82a776f0`, `826e36b0`, `82a76e90`, `82a74c70`; successful handshake is pending.
use super::JuiceTrickInput;
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    input_pad::{device::PadRemap, device_manager::InputDeviceManager},
};
use std::sync::{Arc, Mutex};
#[path = "juice_input_ini.rs"]
mod ini;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JuiceHostOption {
    OriginalConfiguration,
    DisableBeforeStart,
}
#[derive(Clone, Debug, Default)]
struct JuiceConfig {
    enabled: bool,
    port: i32,
    server: Option<Vec<u8>>,
    htc: Option<Vec<u8>>,
    madrid: Option<Vec<u8>>,
    vmc: Option<Vec<u8>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JuiceEndpoint {
    pub address: Vec<u8>,
    pub port: u16,
}
pub struct JuiceConsoleName {
    pub status: u32,
    pub bytes: Option<Vec<u8>>,
}
/// Platform result, not a guessed offline answer. A successful socket is retained.
pub enum JuiceConnection<C> {
    Failed,
    Connected(C),
}
pub trait JuiceBootPlatform {
    type Error;
    type Connection;
    fn initialize_network(&mut self) -> Result<bool, Self::Error>;
    fn debug_address(&mut self) -> Result<[u8; 4], Self::Error>;
    fn console_name(&mut self) -> Result<JuiceConsoleName, Self::Error>;
    fn connect(
        &mut self,
        endpoint: &JuiceEndpoint,
    ) -> Result<JuiceConnection<Self::Connection>, Self::Error>;
    fn sleep_milliseconds(&mut self, milliseconds: u32) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JuiceBootPhase {
    Static,
    Config,
    NetworkInitialization,
    Constructed,
    Starting,
    Connecting,
    FailureSleep,
    Disabled,
    ConnectionFailed,
    HandshakePending,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum JuiceIniError {
    FieldCapacity(usize),
    UnterminatedKey,
    UnterminatedAddress,
}
impl std::fmt::Display for JuiceIniError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FieldCapacity(capacity) => write!(
                f,
                "Juice INI field exceeds admitted buffer capacity {capacity}"
            ),
            Self::UnterminatedKey => {
                f.write_str("Juice INI section or key has no known native terminator")
            }
            Self::UnterminatedAddress => {
                f.write_str("Juice INI server address has no known copied terminator")
            }
        }
    }
}
#[derive(Debug)]
pub(crate) enum JuiceBootError<E> {
    Poisoned,
    Phase(JuiceBootPhase),
    Ini(JuiceIniError),
    Platform(E),
    Cvar(InputCvarError),
    Missing(&'static str),
    EndpointCapacity,
    HandshakePending,
    ForeignRemap,
    ReadyRemapPending,
}
impl<E: std::fmt::Display> std::fmt::Display for JuiceBootError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("Juice boot owner poisoned"),
            Self::Phase(p) => write!(f, "Juice phase {p:?} does not admit this operation"),
            Self::Ini(e) => write!(f, "Juice INI: {e}"),
            Self::Platform(e) => write!(f, "Juice platform: {e}"),
            Self::Cvar(e) => write!(f, "Juice startup cvar: {e}"),
            Self::Missing(s) => write!(f, "Juice {s} is unknown"),
            Self::EndpointCapacity => f.write_str("Juice endpoint exceeds native destination"),
            Self::HandshakePending => {
                f.write_str("Juice socket connected; actual protocol handshake is pending")
            }
            Self::ForeignRemap => f.write_str("Juice remap belongs to another boot occurrence"),
            Self::ReadyRemapPending => f.write_str("ready Juice gamepad plugin execution pending"),
        }
    }
}
struct BootState<C> {
    phase: JuiceBootPhase,
    config: JuiceConfig,
    source_enabled: Option<bool>,
    option: JuiceHostOption,
    network_initialized: Option<bool>,
    wait_for_ip: Option<bool>,
    ready: bool,
    endpoint: Option<JuiceEndpoint>,
    connection: Option<C>,
}
#[derive(Clone)]
pub(crate) struct JuiceRemapHandle(Arc<()>);
pub(crate) struct JuiceInputBoot<C> {
    state: Arc<Mutex<BootState<C>>>,
    occurrence: Arc<()>,
    juice: JuiceTrickInput,
}
impl<C> Clone for JuiceInputBoot<C> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            occurrence: Arc::clone(&self.occurrence),
            juice: self.juice.clone(),
        }
    }
}
impl<C> JuiceInputBoot<C> {
    pub(crate) fn new_static_initialized(juice: JuiceTrickInput) -> Self {
        Self {
            state: Arc::new(Mutex::new(BootState {
                phase: JuiceBootPhase::Static,
                config: JuiceConfig::default(),
                source_enabled: None,
                option: JuiceHostOption::OriginalConfiguration,
                network_initialized: None,
                wait_for_ip: None,
                ready: false,
                endpoint: None,
                connection: None,
            })),
            occurrence: Arc::new(()),
            juice,
        }
    }
    pub(crate) fn trick_input(&self) -> JuiceTrickInput {
        self.juice.clone()
    }
    /// None is an actual absent file result supplied by the host, not a mount error.
    pub(crate) fn try_construct<P: JuiceBootPlatform<Connection = C>>(
        &self,
        bytes: Option<&[u8]>,
        option: JuiceHostOption,
        platform: &mut P,
    ) -> Result<(), JuiceBootError<P::Error>> {
        let mut state = self.state.lock().map_err(|_| JuiceBootError::Poisoned)?;
        if state.phase != JuiceBootPhase::Static {
            return Err(JuiceBootError::Phase(state.phase));
        }
        state.phase = JuiceBootPhase::Config;
        if let Some(bytes) = bytes {
            ini::apply(&mut state.config, bytes).map_err(JuiceBootError::Ini)?;
        }
        state.source_enabled = Some(state.config.enabled);
        state.option = option;
        if option == JuiceHostOption::DisableBeforeStart {
            state.config.enabled = false;
        }
        state.phase = JuiceBootPhase::NetworkInitialization;
        drop(state);
        let initialized = platform
            .initialize_network()
            .map_err(JuiceBootError::Platform)?;
        let mut state = self.state.lock().map_err(|_| JuiceBootError::Poisoned)?;
        state.network_initialized = Some(initialized);
        state.phase = JuiceBootPhase::Constructed;
        Ok(())
    }
    pub(crate) fn try_start<P: JuiceBootPlatform<Connection = C>>(
        &self,
        cvars: &InputCvarReferences,
        platform: &mut P,
    ) -> Result<(), JuiceBootError<P::Error>> {
        let mut state = self.state.lock().map_err(|_| JuiceBootError::Poisoned)?;
        if state.phase != JuiceBootPhase::Constructed {
            return Err(JuiceBootError::Phase(state.phase));
        }
        state.wait_for_ip = Some(
            cvars
                .try_juice_connection_retry()
                .map_err(JuiceBootError::Cvar)?,
        );
        state.phase = JuiceBootPhase::Starting;
        if !state.config.enabled {
            state.phase = JuiceBootPhase::Disabled;
            return Ok(());
        }
        let config = state.config.clone();
        let network_initialized = state.network_initialized;
        drop(state);
        let endpoint = select_endpoint(&config, platform)?;
        let mut state = self.state.lock().map_err(|_| JuiceBootError::Poisoned)?;
        state.endpoint = Some(endpoint.clone());
        state.phase = JuiceBootPhase::Connecting;
        drop(state);
        let result = if network_initialized == Some(true) {
            platform
                .connect(&endpoint)
                .map_err(JuiceBootError::Platform)?
        } else {
            JuiceConnection::Failed
        };
        match result {
            JuiceConnection::Connected(socket) => {
                let mut state = self.state.lock().map_err(|_| JuiceBootError::Poisoned)?;
                state.connection = Some(socket);
                state.phase = JuiceBootPhase::HandshakePending;
                Err(JuiceBootError::HandshakePending)
            }
            JuiceConnection::Failed => {
                self.state
                    .lock()
                    .map_err(|_| JuiceBootError::Poisoned)?
                    .phase = JuiceBootPhase::FailureSleep;
                platform
                    .sleep_milliseconds(2000)
                    .map_err(JuiceBootError::Platform)?;
                self.state
                    .lock()
                    .map_err(|_| JuiceBootError::Poisoned)?
                    .phase = JuiceBootPhase::ConnectionFailed;
                Ok(())
            }
        }
    }
    /// Boot installs the actual instance after Start returns (`82740e54`).
    pub(crate) fn try_install(
        &self,
        devices: &mut InputDeviceManager<JuiceRemapHandle>,
    ) -> Result<(), JuiceBootError<std::convert::Infallible>> {
        let handle = self.try_remap_handle()?;
        if devices.devices_mut().is_none() {
            return Err(JuiceBootError::Missing("initialized device manager"));
        }
        devices.set_external_remap(Some(&handle));
        Ok(())
    }
    pub(crate) fn try_remap_handle(
        &self,
    ) -> Result<JuiceRemapHandle, JuiceBootError<std::convert::Infallible>> {
        let state = self.state.lock().map_err(|_| JuiceBootError::Poisoned)?;
        if !matches!(
            state.phase,
            JuiceBootPhase::Disabled | JuiceBootPhase::ConnectionFailed
        ) {
            return Err(JuiceBootError::Phase(state.phase));
        }
        Ok(JuiceRemapHandle(Arc::clone(&self.occurrence)))
    }
    pub(crate) fn source_enabled(&self) -> Result<bool, JuiceBootError<std::convert::Infallible>> {
        self.state
            .lock()
            .map_err(|_| JuiceBootError::Poisoned)?
            .source_enabled
            .ok_or(JuiceBootError::Missing("completed original enable setting"))
    }
}
impl<C> PadRemap<JuiceRemapHandle> for JuiceInputBoot<C> {
    type Error = JuiceBootError<std::convert::Infallible>;
    fn update(
        &mut self,
        handle: &JuiceRemapHandle,
        _player: u32,
        _values: &mut [f32; 24],
    ) -> Result<(), Self::Error> {
        if !Arc::ptr_eq(&self.occurrence, &handle.0) {
            return Err(JuiceBootError::ForeignRemap);
        }
        let state = self.state.lock().map_err(|_| JuiceBootError::Poisoned)?;
        if state.ready {
            return Err(JuiceBootError::ReadyRemapPending);
        }
        Ok(())
    }
}
fn select_endpoint<P: JuiceBootPlatform>(
    config: &JuiceConfig,
    platform: &mut P,
) -> Result<JuiceEndpoint, JuiceBootError<P::Error>> {
    let mut address = config
        .server
        .clone()
        .ok_or(JuiceBootError::Missing("default server IP"))?;
    if address.len() >= 16 {
        return Err(JuiceBootError::EndpointCapacity);
    }
    let [a, b, c, d] = platform.debug_address().map_err(JuiceBootError::Platform)?;
    let debug = format!("{a}.{b}.{c}.{d}");
    if debug.len() > 7 {
        if debug.starts_with("10.107.") {
            address = config
                .htc
                .clone()
                .ok_or(JuiceBootError::Missing("HTC server IP"))?;
        }
        if debug.starts_with("10.20.") {
            address = config
                .madrid
                .clone()
                .ok_or(JuiceBootError::Missing("Madrid server IP"))?;
        }
    }
    let name = platform.console_name().map_err(JuiceBootError::Platform)?;
    if name.status == 47_841_280 {
        let bytes = name
            .bytes
            .ok_or(JuiceBootError::Missing("successful console name"))?;
        let end = bytes.iter().position(|&v| v == 0).unwrap_or(bytes.len());
        let name = &bytes[..end];
        if name.len() >= 3 && name[..3].eq_ignore_ascii_case(b"vmc") {
            address = config
                .vmc
                .clone()
                .ok_or(JuiceBootError::Missing("VMC server IP"))?;
        }
        if name.len() >= 11 && name[..4].eq_ignore_ascii_case(b"jcon") {
            let suffix = &name[4..];
            let end = suffix
                .iter()
                .position(|&v| v == b'-' || v == b'_')
                .ok_or(JuiceBootError::Missing("bounded JCON delimiter"))?;
            address = suffix[..end].to_vec();
        }
    }
    if address.len() >= 16 {
        return Err(JuiceBootError::EndpointCapacity);
    }
    let low = config.port.to_be_bytes();
    Ok(JuiceEndpoint {
        address,
        port: u16::from_be_bytes([low[2], low[3]]),
    })
}
#[cfg(test)]
#[path = "juice_input_boot_tests.rs"]
mod tests;
