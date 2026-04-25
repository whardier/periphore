//! periphore-core: pure-logic focus/transfer state machine.
//!
//! Zero platform deps, no async, fully unit-testable.
//! Phase 6 wires this into `periphored` when real peers exist.

use thiserror::Error;

// ---------------------------------------------------------------------------
// PeerId — unique peer identity (fingerprint hex string)
// ---------------------------------------------------------------------------

/// Unique peer identifier: the fingerprint hex string of the peer's Ed25519 public key.
///
/// This is a newtype wrapping `String`. Phase 6 aligns it with
/// `periphore-protocol`'s peer identity types when the TCP connection exists.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerId(pub String);

impl PeerId {
    /// Create a new `PeerId` from a fingerprint hex string.
    pub fn new(fingerprint_hex: impl Into<String>) -> Self {
        Self(fingerprint_hex.into())
    }

    /// Return the inner fingerprint hex string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// ---------------------------------------------------------------------------
// FocusState — current input focus routing state
// ---------------------------------------------------------------------------

/// The current input focus routing state for this node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusState {
    /// This node has local input focus — input events are consumed locally.
    LocalFocus,
    /// This node is forwarding input to a remote peer.
    ForwardingTo {
        /// The peer receiving forwarded input events.
        peer_id: PeerId,
    },
}

// ---------------------------------------------------------------------------
// FocusError — invalid state transition errors
// ---------------------------------------------------------------------------

/// Errors from invalid `FocusStateMachine` transitions.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FocusError {
    /// `transfer_to()` called while already forwarding to a peer.
    #[error("already forwarding to peer — reclaim focus first")]
    AlreadyForwarding,
    /// `reclaim()` called while not forwarding to any peer.
    #[error("not currently forwarding to any peer")]
    NotForwarding,
}

// ---------------------------------------------------------------------------
// FocusStateMachine — owns current state, exposes pure transition methods
// ---------------------------------------------------------------------------

/// Pure-logic focus/transfer state machine.
///
/// Owns the current `FocusState` and enforces valid transitions.
/// All methods are synchronous and have no I/O side-effects.
#[derive(Debug)]
pub struct FocusStateMachine {
    state: FocusState,
}

impl FocusStateMachine {
    /// Create a new state machine starting in `LocalFocus`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: FocusState::LocalFocus,
        }
    }

    /// Return the current focus state.
    #[must_use]
    pub fn current_state(&self) -> &FocusState {
        &self.state
    }

    /// Transfer input focus to a remote peer.
    ///
    /// Transitions `LocalFocus → ForwardingTo { peer_id }`.
    ///
    /// # Errors
    /// Returns [`FocusError::AlreadyForwarding`] if already forwarding to a peer.
    /// Reclaim focus first with [`Self::reclaim()`].
    pub fn transfer_to(&mut self, peer_id: PeerId) -> Result<(), FocusError> {
        match &self.state {
            FocusState::ForwardingTo { .. } => Err(FocusError::AlreadyForwarding),
            FocusState::LocalFocus => {
                self.state = FocusState::ForwardingTo { peer_id };
                Ok(())
            }
        }
    }

    /// Reclaim input focus to local.
    ///
    /// Transitions `ForwardingTo { .. } → LocalFocus`.
    ///
    /// # Errors
    /// Returns [`FocusError::NotForwarding`] if already in `LocalFocus`.
    pub fn reclaim(&mut self) -> Result<(), FocusError> {
        match &self.state {
            FocusState::LocalFocus => Err(FocusError::NotForwarding),
            FocusState::ForwardingTo { .. } => {
                self.state = FocusState::LocalFocus;
                Ok(())
            }
        }
    }
}

impl Default for FocusStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
