//! HTTPS downloads from GitHub (rustls, with the Mozilla roots built in): a release's small files
//! with a short timeout, and its zip in a file, hashed as it arrives.

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::time::Duration;

/// How long the small files may take, and a zip's connection, its first byte and all of it.
const SMALL_TIMEOUT: Duration = Duration::from_secs(20);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const BODY_TIMEOUT: Duration = Duration::from_secs(30 * 60);

pub(crate) struct Client {
    small: ureq::Agent,
    large: ureq::Agent,
}

impl Client {
    pub(crate) fn new(user_agent: &str) -> Self {
        let small = ureq::Agent::config_builder().user_agent(user_agent).timeout_global(Some(SMALL_TIMEOUT)).build().into();
        let large = ureq::Agent::config_builder()
            .user_agent(user_agent)
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_recv_response(Some(RESPONSE_TIMEOUT))
            .timeout_recv_body(Some(BODY_TIMEOUT))
            .build()
            .into();
        Self { small, large }
    }

    /// A small file, up to `limit` bytes.
    pub(crate) fn small(&self, url: &str, limit: u64) -> Result<Vec<u8>, String> {
        let mut response = self.small.get(url).call().map_err(|error| format!("{url}: {error}"))?;
        response.body_mut().with_config().limit(limit).read_to_vec().map_err(|error| format!("{url}: {error}"))
    }

    /// The file at `url` into `path`, which must come to exactly `size` bytes, telling `progress`
    /// the bytes so far; its SHA-256.
    pub(crate) fn download(&self, url: &str, path: &Path, size: u64, progress: &mut dyn FnMut(u64)) -> Result<[u8; 32], String> {
        let mut response = self.large.get(url).call().map_err(|error| format!("{url}: {error}"))?;
        let mut body = response.body_mut().with_config().limit(size.saturating_add(1)).reader();
        let mut file = BufWriter::new(File::create(path).map_err(|error| format!("{}: {error}", path.display()))?);
        let mut hash = Sha256::new();
        let mut buffer = vec![0_u8; 64 * 1024];
        let mut done = 0_u64;
        loop {
            let read = body.read(&mut buffer).map_err(|error| format!("{url}: {error}"))?;
            if read == 0 {
                break;
            }
            let chunk = &buffer[..read];
            done += chunk.len() as u64;
            if done > size {
                return Err(format!("{url} is larger than its release says"));
            }
            hash.update(chunk);
            file.write_all(chunk).map_err(|error| format!("{}: {error}", path.display()))?;
            progress(done);
        }
        file.into_inner().map_err(|error| format!("{}: {error}", path.display()))?.sync_all().map_err(|error| format!("{}: {error}", path.display()))?;
        if done != size {
            return Err(format!("{url} ended after {done} of {size} bytes"));
        }
        let mut sha256 = [0_u8; 32];
        sha256.copy_from_slice(&hash.finalize());
        Ok(sha256)
    }
}
