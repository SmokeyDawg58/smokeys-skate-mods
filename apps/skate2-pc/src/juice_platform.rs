//! PC socket platform mapping for the retained core Juice boot owner: Windows starts Winsock
//! first, other systems' sockets need no start-up.
//! No socket, endpoint identity, or successful handshake is invented.
use skate2_core::{JuiceBootPlatform, JuiceConnection, JuiceConsoleName, JuiceEndpoint};
use std::{
    io,
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    sync::Arc,
    time::Duration,
};

/// Host must explicitly map its debug identity; disabled startup never needs it.
pub(crate) struct JuiceDebugIdentity {
    pub(crate) address: [u8; 4],
    pub(crate) console_name_status: u32,
    pub(crate) console_name: Option<Vec<u8>>,
}
/// One successful [`start_sockets`], held while a connection may use the sockets.
struct SocketLease;
#[cfg(windows)]
impl Drop for SocketLease {
    fn drop(&mut self) {
        // One cleanup for this successful startup. Other std/library startups
        // have their own reference counts. Native UnInitialize ignores status.
        let _ = rustix::net::wsa_cleanup();
    }
}
/// Actual WSAStartup(2.2), safely wrapped by the pinned rustix dependency; false when it failed.
#[cfg(windows)]
fn start_sockets() -> bool {
    rustix::net::wsa_startup().is_ok()
}
/// Other systems' sockets need no start-up.
#[cfg(not(windows))]
const fn start_sockets() -> bool {
    true
}
pub(crate) struct PcJuiceConnection {
    // Rust drops fields in declaration order: socket closes before our socket lease.
    stream: TcpStream,
    _sockets: Arc<SocketLease>,
}
impl PcJuiceConnection {
    pub(crate) fn stream(&self) -> &TcpStream {
        &self.stream
    }
}
pub(crate) struct PcJuicePlatform {
    attempted: bool,
    sockets: Option<Arc<SocketLease>>,
    identity: Option<JuiceDebugIdentity>,
    last_connection_error: Option<io::Error>,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PcJuiceError {
    AlreadyInitialized,
    InitializationRequired,
    MissingDebugIdentity,
    UnsupportedAddress(Vec<u8>),
}
impl std::fmt::Display for PcJuiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInitialized => {
                f.write_str("Juice network initialization already attempted")
            }
            Self::InitializationRequired => {
                f.write_str("Juice network was not initialized successfully")
            }
            Self::MissingDebugIdentity => {
                f.write_str("enabled Juice requires an explicit host debug identity mapping")
            }
            Self::UnsupportedAddress(bytes) => write!(
                f,
                "Juice endpoint is outside admitted dotted IPv4 syntax: {bytes:?}"
            ),
        }
    }
}
impl std::error::Error for PcJuiceError {}
impl PcJuicePlatform {
    pub(crate) const fn new(identity: Option<JuiceDebugIdentity>) -> Self {
        Self {
            attempted: false,
            sockets: None,
            identity,
            last_connection_error: None,
        }
    }
    pub(crate) fn last_connection_error(&self) -> Option<&io::Error> {
        self.last_connection_error.as_ref()
    }
}
impl JuiceBootPlatform for PcJuicePlatform {
    type Error = PcJuiceError;
    type Connection = PcJuiceConnection;
    fn initialize_network(&mut self) -> Result<bool, Self::Error> {
        if self.attempted {
            return Err(PcJuiceError::AlreadyInitialized);
        }
        self.attempted = true;
        // Xbox XNet startup has no PC counterpart; this is an explicit mapping.
        if !start_sockets() {
            return Ok(false);
        }
        self.sockets = Some(Arc::new(SocketLease));
        Ok(true)
    }
    fn debug_address(&mut self) -> Result<[u8; 4], Self::Error> {
        self.identity
            .as_ref()
            .map(|identity| identity.address)
            .ok_or(PcJuiceError::MissingDebugIdentity)
    }
    fn console_name(&mut self) -> Result<JuiceConsoleName, Self::Error> {
        let identity = self
            .identity
            .as_ref()
            .ok_or(PcJuiceError::MissingDebugIdentity)?;
        Ok(JuiceConsoleName {
            status: identity.console_name_status,
            bytes: identity.console_name.clone(),
        })
    }
    fn connect(
        &mut self,
        endpoint: &JuiceEndpoint,
    ) -> Result<JuiceConnection<Self::Connection>, Self::Error> {
        let sockets = self
            .sockets
            .as_ref()
            .ok_or(PcJuiceError::InitializationRequired)?;
        let address = std::str::from_utf8(&endpoint.address)
            .ok()
            .and_then(|text| text.parse::<Ipv4Addr>().ok())
            .ok_or_else(|| PcJuiceError::UnsupportedAddress(endpoint.address.clone()))?;
        self.last_connection_error = None;
        // Native inet_addr returns INADDR_NONE for this bit pattern.
        if address == Ipv4Addr::BROADCAST {
            return Ok(JuiceConnection::Failed);
        }
        match TcpStream::connect(SocketAddrV4::new(address, endpoint.port)) {
            Ok(stream) => Ok(JuiceConnection::Connected(PcJuiceConnection {
                stream,
                _sockets: Arc::clone(sockets),
            })),
            Err(error) => {
                self.last_connection_error = Some(error);
                Ok(JuiceConnection::Failed)
            }
        }
    }
    fn sleep_milliseconds(&mut self, milliseconds: u32) -> Result<(), Self::Error> {
        std::thread::sleep(Duration::from_millis(u64::from(milliseconds)));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_socket_initialization_retains_cleanup_lease_without_connecting() {
        let mut platform = PcJuicePlatform::new(None);
        assert!(platform.initialize_network().unwrap());
        assert_eq!(
            platform.initialize_network(),
            Err(PcJuiceError::AlreadyInitialized)
        );
        assert_eq!(
            platform.debug_address(),
            Err(PcJuiceError::MissingDebugIdentity)
        );
        assert!(platform.last_connection_error().is_none());
        let lease = Arc::clone(platform.sockets.as_ref().unwrap());
        let weak = Arc::downgrade(&lease);
        drop(platform);
        assert!(
            weak.upgrade().is_some(),
            "retained socket/session lease keeps the sockets alive"
        );
        drop(lease);
        assert!(weak.upgrade().is_none(), "final lease runs cleanup");
    }
    #[test]
    fn explicit_debug_identity_preserves_failed_name_status_without_fabricated_bytes() {
        let mut platform = PcJuicePlatform::new(Some(JuiceDebugIdentity {
            address: [10, 20, 1, 2],
            console_name_status: 5,
            console_name: None,
        }));
        assert_eq!(platform.debug_address().unwrap(), [10, 20, 1, 2]);
        let name = platform.console_name().unwrap();
        assert_eq!(name.status, 5);
        assert!(name.bytes.is_none());
    }
}
