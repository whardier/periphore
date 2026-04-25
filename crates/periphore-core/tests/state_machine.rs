//! Integration tests for periphore-core state machine.
//! All tests live here because [lib] test = false in Cargo.toml.

use periphore_core::{FocusError, FocusState, FocusStateMachine, PeerId};

// ---------------------------------------------------------------------------
// Initial state
// ---------------------------------------------------------------------------

#[test]
fn test_initial_state_is_local_focus() {
    let fsm = FocusStateMachine::new();
    assert_eq!(*fsm.current_state(), FocusState::LocalFocus);
}

#[test]
fn test_default_state_is_local_focus() {
    let fsm = FocusStateMachine::default();
    assert_eq!(*fsm.current_state(), FocusState::LocalFocus);
}

// ---------------------------------------------------------------------------
// transfer_to transitions
// ---------------------------------------------------------------------------

#[test]
fn test_transfer_to_sets_forwarding_state() {
    let mut fsm = FocusStateMachine::new();
    let peer = PeerId::new("abc123");
    fsm.transfer_to(peer.clone()).expect("transfer should succeed from LocalFocus");
    assert_eq!(
        *fsm.current_state(),
        FocusState::ForwardingTo { peer_id: peer }
    );
}

#[test]
fn test_transfer_to_returns_already_forwarding_when_active() {
    let mut fsm = FocusStateMachine::new();
    fsm.transfer_to(PeerId::new("peer-a")).expect("first transfer succeeds");
    let err = fsm.transfer_to(PeerId::new("peer-b")).expect_err("second transfer must fail");
    assert_eq!(err, FocusError::AlreadyForwarding);
}

#[test]
fn test_transfer_to_already_forwarding_does_not_change_target() {
    let mut fsm = FocusStateMachine::new();
    let first_peer = PeerId::new("peer-a");
    fsm.transfer_to(first_peer.clone()).expect("first transfer succeeds");
    let _ = fsm.transfer_to(PeerId::new("peer-b")); // error — state unchanged
    assert_eq!(
        *fsm.current_state(),
        FocusState::ForwardingTo { peer_id: first_peer }
    );
}

// ---------------------------------------------------------------------------
// reclaim transitions
// ---------------------------------------------------------------------------

#[test]
fn test_reclaim_returns_local_focus() {
    let mut fsm = FocusStateMachine::new();
    fsm.transfer_to(PeerId::new("peer-a")).expect("transfer succeeds");
    fsm.reclaim().expect("reclaim should succeed from ForwardingTo");
    assert_eq!(*fsm.current_state(), FocusState::LocalFocus);
}

#[test]
fn test_reclaim_returns_not_forwarding_when_local() {
    let mut fsm = FocusStateMachine::new();
    let err = fsm.reclaim().expect_err("reclaim from LocalFocus must fail");
    assert_eq!(err, FocusError::NotForwarding);
}

// ---------------------------------------------------------------------------
// Full round-trip: transfer → reclaim → transfer again
// ---------------------------------------------------------------------------

#[test]
fn test_round_trip_transfer_reclaim_transfer() {
    let mut fsm = FocusStateMachine::new();

    fsm.transfer_to(PeerId::new("peer-a")).expect("first transfer");
    assert!(matches!(*fsm.current_state(), FocusState::ForwardingTo { .. }));

    fsm.reclaim().expect("reclaim");
    assert_eq!(*fsm.current_state(), FocusState::LocalFocus);

    fsm.transfer_to(PeerId::new("peer-b")).expect("second transfer");
    assert_eq!(
        *fsm.current_state(),
        FocusState::ForwardingTo { peer_id: PeerId::new("peer-b") }
    );
}

// ---------------------------------------------------------------------------
// PeerId helpers
// ---------------------------------------------------------------------------

#[test]
fn test_peer_id_as_str() {
    let peer = PeerId::new("deadbeef");
    assert_eq!(peer.as_str(), "deadbeef");
}

#[test]
fn test_peer_id_display() {
    let peer = PeerId::new("cafebabe");
    assert_eq!(format!("{peer}"), "cafebabe");
}

#[test]
fn test_peer_id_equality() {
    let a = PeerId::new("abc");
    let b = PeerId::new("abc");
    let c = PeerId::new("xyz");
    assert_eq!(a, b);
    assert_ne!(a, c);
}
