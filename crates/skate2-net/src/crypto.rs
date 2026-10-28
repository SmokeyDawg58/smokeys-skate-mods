//! The connection's encryption, in place of the original's `DirtySock` `ProtoTunnel`. A client and
//! the server agree on fresh keys for every connection (X25519), and only the same build of the
//! game can take part: the build's secret, made anew each time the game is compiled, keys the
//! handshake's proofs and the key derivation, and never travels. Every datagram after the
//! handshake is sealed with ChaCha20-Poly1305: a packet capture shows no message, and nothing can
//! be forged into a connection or replayed on it.

use chacha20poly1305::aead::{AeadInPlace, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, Tag};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::sync::OnceLock;
use x25519_dalek::{PublicKey, StaticSecret};

/// Bytes of a handshake key on the wire.
pub const KEY_BYTES: usize = 32;
/// Bytes of a handshake proof.
pub const PROOF_BYTES: usize = 16;
/// Bytes sealing adds to a datagram: its counter and its tag.
pub const SEAL_BYTES: usize = 4 + TAG_BYTES;

const TAG_BYTES: usize = 16;
/// Opened counters remembered behind the highest one, against replays.
const REPLAY_WINDOW: u32 = 128;

/// A build of the game: the secret its build script made when it was compiled. The server and
/// the client of one executable are always the same build; a different build cannot complete
/// the handshake.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Build([u8; 32]);

impl std::fmt::Debug for Build {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Build({})", self.id())
    }
}

static INSTALLED: OnceLock<Build> = OnceLock::new();

impl Build {
    /// The build of code that installs none (tests).
    pub const DEVELOPMENT: Self = Self([0; 32]);

    #[must_use]
    pub const fn new(secret: [u8; 32]) -> Self {
        Self(secret)
    }

    /// A build script's secret, as 64 hexadecimal digits.
    #[must_use]
    pub fn from_hex(hex: &str) -> Option<Self> {
        let digits = hex.trim().as_bytes();
        if digits.len() != 64 {
            return None;
        }
        let mut secret = [0; 32];
        for (byte, pair) in secret.iter_mut().zip(digits.chunks(2)) {
            *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
        }
        Some(Self(secret))
    }

    /// Makes this the process's build; the first one installed stays.
    pub fn install(self) {
        let _ = INSTALLED.set(self);
    }

    /// The installed build, else [`Build::DEVELOPMENT`].
    #[must_use]
    pub fn current() -> Self {
        INSTALLED.get().copied().unwrap_or(Self::DEVELOPMENT)
    }

    /// A short name of the build for logs, which says nothing of its secret.
    #[must_use]
    pub fn id(self) -> String {
        let digest = Sha256::new().chain_update(b"skate2 build id").chain_update(self.0).finalize();
        digest[..4].iter().fold(String::with_capacity(8), |mut id, byte| {
            let _ = write!(id, "{byte:02x}");
            id
        })
    }

    fn mac(self, label: &[u8], parts: &[&[u8]]) -> Hmac<Sha256> {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.0).expect("HMAC takes a key of any length");
        mac.update(label);
        for part in parts {
            mac.update(part);
        }
        mac
    }

    fn proof(self, label: &[u8], parts: &[&[u8]]) -> [u8; PROOF_BYTES] {
        let full = self.mac(label, parts).finalize().into_bytes();
        let mut proof = [0; PROOF_BYTES];
        proof.copy_from_slice(&full[..PROOF_BYTES]);
        proof
    }

    fn verify(self, label: &[u8], parts: &[&[u8]], proof: &[u8]) -> bool {
        proof.len() == PROOF_BYTES && self.mac(label, parts).verify_truncated_left(proof).is_ok()
    }

    /// The connect datagram's proof that its sender is this build.
    #[must_use]
    pub fn connect_proof(self, nonce: u64, client: &[u8; KEY_BYTES]) -> [u8; PROOF_BYTES] {
        self.proof(b"skate2 connect", &[&nonce.to_be_bytes(), client])
    }

    #[must_use]
    pub fn verify_connect(self, nonce: u64, client: &[u8; KEY_BYTES], proof: &[u8]) -> bool {
        self.verify(b"skate2 connect", &[&nonce.to_be_bytes(), client], proof)
    }

    /// The accept datagram's proof that the server is this build and answers this connect.
    #[must_use]
    pub fn accept_proof(self, nonce: u64, connection: u32, client: &[u8; KEY_BYTES], server: &[u8; KEY_BYTES]) -> [u8; PROOF_BYTES] {
        self.proof(b"skate2 accept", &[&nonce.to_be_bytes(), &connection.to_be_bytes(), client, server])
    }

    #[must_use]
    pub fn verify_accept(self, nonce: u64, connection: u32, client: &[u8; KEY_BYTES], server: &[u8; KEY_BYTES], proof: &[u8]) -> bool {
        self.verify(b"skate2 accept", &[&nonce.to_be_bytes(), &connection.to_be_bytes(), client, server], proof)
    }
}

/// One side's key pair for a connection's exchange, fresh from the system's random source.
pub struct Handshake {
    secret: StaticSecret,
    public: [u8; KEY_BYTES],
}

impl std::fmt::Debug for Handshake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handshake").finish_non_exhaustive()
    }
}

impl Handshake {
    /// `None` when the system's random source fails.
    #[must_use]
    pub fn new() -> Option<Self> {
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).ok()?;
        let secret = StaticSecret::from(bytes);
        let public = PublicKey::from(&secret).to_bytes();
        Some(Self { secret, public })
    }

    #[must_use]
    pub fn public(&self) -> [u8; KEY_BYTES] {
        self.public
    }

    /// The shared secret with the other side's key; `None` for a key that leaves nothing
    /// secret (a small-order point).
    fn agree(&self, other: &[u8; KEY_BYTES]) -> Option<[u8; 32]> {
        let shared = self.secret.diffie_hellman(&PublicKey::from(*other));
        shared.was_contributory().then(|| *shared.as_bytes())
    }
}

/// An established connection's keys, one each way, its send counter and its replay window.
pub struct Session {
    seal: ChaCha20Poly1305,
    open: ChaCha20Poly1305,
    sent: u32,
    highest: Option<u32>,
    /// Bit `n`: the counter `highest - n` was opened.
    window: u128,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session").field("sent", &self.sent).field("highest", &self.highest).finish_non_exhaustive()
    }
}

impl Session {
    /// The client's session once the server's accept arrived; `None` for a key that leaves
    /// nothing secret.
    #[must_use]
    pub fn client(build: Build, handshake: &Handshake, server: &[u8; KEY_BYTES], nonce: u64, connection: u32) -> Option<Self> {
        let (to_server, to_client) = derive(build, &handshake.agree(server)?, nonce, connection, &handshake.public, server);
        Some(Self::new(&to_server, &to_client))
    }

    /// The server's session for a client's connect.
    #[must_use]
    pub fn server(build: Build, handshake: &Handshake, client: &[u8; KEY_BYTES], nonce: u64, connection: u32) -> Option<Self> {
        let (to_server, to_client) = derive(build, &handshake.agree(client)?, nonce, connection, client, &handshake.public);
        Some(Self::new(&to_client, &to_server))
    }

    fn new(seal: &[u8; 32], open: &[u8; 32]) -> Self {
        Self {
            seal: ChaCha20Poly1305::new(Key::from_slice(seal)),
            open: ChaCha20Poly1305::new(Key::from_slice(open)),
            sent: 0,
            highest: None,
            window: 0,
        }
    }

    /// Two ends of one connection on fresh keys, for tests of what runs over it.
    #[cfg(test)]
    pub(crate) fn pair() -> (Self, Self) {
        let mut keys = [0; 64];
        getrandom::fill(&mut keys).expect("random source");
        let (one, two) = keys.split_at(32);
        let (one, two): (&[u8; 32], &[u8; 32]) = (one.try_into().expect("32"), two.try_into().expect("32"));
        (Self::new(one, two), Self::new(two, one))
    }

    /// The datagram sealed: its first `clear` bytes stay readable and are authenticated, its
    /// counter follows them, then the rest encrypted, then the tag. `None` once the counter is
    /// spent, after which the connection has to end.
    pub fn seal(&mut self, datagram: &[u8], clear: usize) -> Option<Vec<u8>> {
        let counter = self.sent;
        self.sent = counter.checked_add(1)?;
        let clear = clear.min(datagram.len());
        let mut sealed = Vec::with_capacity(datagram.len() + SEAL_BYTES);
        sealed.extend_from_slice(&datagram[..clear]);
        sealed.extend_from_slice(&counter.to_be_bytes());
        let authenticated = sealed.len();
        sealed.extend_from_slice(&datagram[clear..]);
        let (header, body) = sealed.split_at_mut(authenticated);
        let tag = self.seal.encrypt_in_place_detached(&nonce(counter), header, body).ok()?;
        sealed.extend_from_slice(&tag);
        Some(sealed)
    }

    /// A sealed datagram's plaintext after its counter; `None` for one that is not this
    /// connection's, was altered, or was opened before.
    pub fn open(&mut self, datagram: &[u8], clear: usize) -> Option<Vec<u8>> {
        if datagram.len() < clear + SEAL_BYTES {
            return None;
        }
        let counter = u32::from_be_bytes(datagram[clear..clear + 4].try_into().ok()?);
        if !self.fresh(counter) {
            return None;
        }
        let (header, rest) = datagram.split_at(clear + 4);
        let (body, tag) = rest.split_at(rest.len() - TAG_BYTES);
        let mut plain = body.to_vec();
        self.open.decrypt_in_place_detached(&nonce(counter), header, &mut plain, Tag::from_slice(tag)).ok()?;
        self.mark(counter);
        Some(plain)
    }

    fn fresh(&self, counter: u32) -> bool {
        match self.highest {
            Some(highest) if counter <= highest => {
                let age = highest - counter;
                age < REPLAY_WINDOW && self.window & (1_u128 << age) == 0
            }
            _ => true,
        }
    }

    fn mark(&mut self, counter: u32) {
        match self.highest {
            Some(highest) if counter <= highest => self.window |= 1_u128 << (highest - counter),
            highest => {
                let shift = highest.map_or(REPLAY_WINDOW, |highest| counter - highest);
                self.window = if shift >= REPLAY_WINDOW { 0 } else { self.window << shift };
                self.window |= 1;
                self.highest = Some(counter);
            }
        }
    }
}

/// The two directions' keys (client to server, server to client) of one exchange, salted with
/// the build's secret, so another build derives others.
fn derive(build: Build, shared: &[u8; 32], nonce: u64, connection: u32, client: &[u8; KEY_BYTES], server: &[u8; KEY_BYTES]) -> ([u8; 32], [u8; 32]) {
    let hkdf = Hkdf::<Sha256>::new(Some(&build.0), shared);
    let mut info = Vec::with_capacity(19 + 8 + 4 + 2 * KEY_BYTES);
    info.extend_from_slice(b"skate2 session keys");
    info.extend_from_slice(&nonce.to_be_bytes());
    info.extend_from_slice(&connection.to_be_bytes());
    info.extend_from_slice(client);
    info.extend_from_slice(server);
    let mut keys = [0; 64];
    hkdf.expand(&info, &mut keys).expect("64 bytes is within HKDF-SHA256's output");
    let mut to_server = [0; 32];
    let mut to_client = [0; 32];
    to_server.copy_from_slice(&keys[..32]);
    to_client.copy_from_slice(&keys[32..]);
    (to_server, to_client)
}

fn nonce(counter: u32) -> Nonce {
    let mut nonce = [0; 12];
    nonce[..4].copy_from_slice(&counter.to_be_bytes());
    *Nonce::from_slice(&nonce)
}

#[cfg(test)]
mod tests {
    use super::{Build, Handshake, Session};

    fn connected(client_build: Build, server_build: Build) -> Option<(Session, Session)> {
        let client = Handshake::new()?;
        let server = Handshake::new()?;
        let nonce = 77;
        let connection = 5;
        let to_server = Session::client(client_build, &client, &server.public(), nonce, connection)?;
        let to_client = Session::server(server_build, &server, &client.public(), nonce, connection)?;
        Some((to_server, to_client))
    }

    #[test]
    fn one_build_seals_and_opens_both_ways() {
        let build = Build::new([9; 32]);
        let (mut client, mut server) = connected(build, build).expect("handshake");
        let sealed = client.seal(b"HDR!hello server", 4).expect("seal");
        assert_eq!(&sealed[..4], b"HDR!");
        assert_eq!(server.open(&sealed, 4).as_deref(), Some(&b"hello server"[..]));
        let reply = server.seal(b"HDR!hello client", 4).expect("seal");
        assert_eq!(client.open(&reply, 4).as_deref(), Some(&b"hello client"[..]));
    }

    #[test]
    fn another_build_neither_proves_itself_nor_opens() {
        let ours = Build::new([9; 32]);
        let theirs = Build::new([8; 32]);
        let handshake = Handshake::new().expect("random source");
        let proof = theirs.connect_proof(3, &handshake.public());
        assert!(!ours.verify_connect(3, &handshake.public(), &proof));
        assert!(theirs.verify_connect(3, &handshake.public(), &proof));
        let (mut client, mut server) = connected(theirs, ours).expect("handshake");
        let sealed = client.seal(b"HDR!hello", 4).expect("seal");
        assert_eq!(server.open(&sealed, 4), None);
    }

    #[test]
    fn a_sealed_datagram_shows_nothing_and_opens_once() {
        let (mut client, mut server) = Session::pair();
        let message = b"HDR!SkaterA signs in with build 1234";
        let sealed = client.seal(message, 4).expect("seal");
        assert!(!sealed.windows(7).any(|window| window == b"SkaterA"));
        assert!(server.open(&sealed, 4).is_some());
        assert_eq!(server.open(&sealed, 4), None, "a replay is refused");
        let mut altered = client.seal(message, 4).expect("seal");
        let last = altered.len() - 1;
        altered[last] ^= 1;
        assert_eq!(server.open(&altered, 4), None, "an altered datagram is refused");
        let mut header = client.seal(message, 4).expect("seal");
        header[0] ^= 1;
        assert_eq!(server.open(&header, 4), None, "the readable header is authenticated");
    }

    #[test]
    fn late_datagrams_open_within_the_window() {
        let (mut client, mut server) = Session::pair();
        let sealed: Vec<Vec<u8>> = (0..200).map(|_| client.seal(b"HDR!x", 4).expect("seal")).collect();
        assert!(server.open(&sealed[150], 4).is_some());
        assert!(server.open(&sealed[100], 4).is_some(), "50 behind is in the window");
        assert!(server.open(&sealed[10], 4).is_none(), "140 behind is out of it");
        assert!(server.open(&sealed[199], 4).is_some());
        assert!(server.open(&sealed[150], 4).is_none(), "already opened");
    }

    #[test]
    fn a_build_secret_reads_back_from_hex() {
        let hex = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
        let build = Build::from_hex(hex).expect("hex");
        assert_eq!(build, Build::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]));
        assert_eq!(build.id().len(), 8);
        assert_eq!(Build::from_hex("12"), None);
    }
}
