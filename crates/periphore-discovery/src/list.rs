//! In-memory discovered peer list with cap enforcement and TTL garbage collection.
//!
//! D-07: Hybrid expiry -- mDNS goodbye removes immediately, TTL GC sweeps stale entries.
//! D-08: TTL = 5 minutes since last_seen.
//! D-09: Cap = 64 peers; evict oldest last_seen on overflow.

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use periphore_protocol::DiscoveredPeerInfo;

/// Maximum number of discovered peers tracked simultaneously (D-09).
const MAX_PEERS: usize = 64;

/// Time-to-live for entries not refreshed by mDNS re-announcement (D-08).
const TTL: Duration = Duration::from_secs(300);

/// Discovery source for a peer entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoverySource {
    /// Discovered via mDNS broadcast on the local network.
    Mdns,
    /// Discovered via SSH tunnel port probe on localhost.
    SshProbe,
}

impl DiscoverySource {
    /// String representation for IPC serialization.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mdns => "mdns",
            Self::SshProbe => "ssh_probe",
        }
    }
}

/// Internal entry for a discovered peer.
#[derive(Debug, Clone)]
pub(crate) struct DiscoveredPeerEntry {
    pub(crate) hostname: String,
    pub(crate) port: u16,
    pub(crate) last_seen: Instant,
    pub(crate) source: DiscoverySource,
    /// mDNS fullname for matching ServiceRemoved events.
    /// None for SSH probe entries.
    pub(crate) mdns_fullname: Option<String>,
}

/// In-memory list of discovered peers.
///
/// Thread-safe access via `Arc<std::sync::Mutex<DiscoveredPeerList>>`.
/// Lock acquisition uses `unwrap_or_else(|e| e.into_inner())` for poison recovery
/// (matches periphore-net ConnectionManager pattern).
#[derive(Debug)]
pub struct DiscoveredPeerList {
    /// Keyed by "hostname:port" composite key.
    entries: HashMap<String, DiscoveredPeerEntry>,
}

impl DiscoveredPeerList {
    /// Create an empty discovered peer list.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Insert or update a discovered peer entry.
    ///
    /// If the peer already exists (by "hostname:port" key), refreshes `last_seen`.
    /// If at capacity (64), evicts the entry with the oldest `last_seen` timestamp
    /// and logs a warning (D-09).
    pub fn upsert(
        &mut self,
        hostname: String,
        port: u16,
        source: DiscoverySource,
        mdns_fullname: Option<String>,
    ) {
        let key = format!("{hostname}:{port}");
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_seen = Instant::now();
            // Update fullname if provided (mDNS re-announcement may include it)
            if mdns_fullname.is_some() {
                entry.mdns_fullname = mdns_fullname;
            }
            return;
        }
        // Cap enforcement (D-09): evict oldest if at capacity
        if self.entries.len() >= MAX_PEERS {
            if let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.last_seen)
                .map(|(k, _)| k.clone())
            {
                tracing::warn!(
                    evicted = %oldest_key,
                    "discovered peer list full ({MAX_PEERS}) -- evicting oldest"
                );
                self.entries.remove(&oldest_key);
            }
        }
        self.entries.insert(
            key,
            DiscoveredPeerEntry {
                hostname,
                port,
                last_seen: Instant::now(),
                source,
                mdns_fullname,
            },
        );
    }

    /// Remove a peer by hostname:port key (mDNS goodbye, D-07).
    pub fn remove(&mut self, hostname: &str, port: u16) {
        let key = format!("{hostname}:{port}");
        self.entries.remove(&key);
    }

    /// Remove a peer by mDNS fullname (ServiceRemoved event).
    ///
    /// Searches for the entry whose `mdns_fullname` matches and removes it.
    /// Returns true if an entry was removed.
    pub fn remove_by_fullname(&mut self, fullname: &str) -> bool {
        let key = self
            .entries
            .iter()
            .find(|(_, e)| e.mdns_fullname.as_deref() == Some(fullname))
            .map(|(k, _)| k.clone());
        if let Some(key) = key {
            self.entries.remove(&key);
            true
        } else {
            false
        }
    }

    /// Remove all entries whose `last_seen` exceeds the TTL (5 minutes, D-08).
    ///
    /// Returns the number of entries removed.
    pub fn gc(&mut self) -> usize {
        let now = Instant::now();
        let before = self.entries.len();
        self.entries
            .retain(|_, e| now.duration_since(e.last_seen) < TTL);
        before - self.entries.len()
    }

    /// Return a snapshot of all discovered peers as IPC-serializable structs.
    ///
    /// Converts internal `Instant` timestamps to `SystemTime` epoch seconds
    /// for cross-process serialization (Pitfall 6 mitigation).
    pub fn snapshot(&self) -> Vec<DiscoveredPeerInfo> {
        let now_instant = Instant::now();
        let now_system = SystemTime::now();
        self.entries
            .values()
            .map(|e| {
                // Convert Instant to approximate SystemTime:
                // system_time = now_system - (now_instant - e.last_seen)
                let elapsed = now_instant.duration_since(e.last_seen);
                let last_seen_system = now_system
                    .checked_sub(elapsed)
                    .unwrap_or(UNIX_EPOCH);
                let last_seen_epoch = last_seen_system
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                DiscoveredPeerInfo {
                    hostname: e.hostname.clone(),
                    port: e.port,
                    last_seen_epoch,
                    source: e.source.as_str().to_owned(),
                }
            })
            .collect()
    }

    /// Number of entries currently in the list.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for DiscoveredPeerList {
    fn default() -> Self {
        Self::new()
    }
}
