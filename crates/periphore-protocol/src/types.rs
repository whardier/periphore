use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub id:     u32,
    pub width:  u32,
    pub height: u32,
    pub x:      i32,
    pub y:      i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeMapping {
    pub from_monitor: u32,
    pub from_edge:    Edge,
    pub to_peer:      String, // peer fingerprint (hex string)
    pub to_monitor:   u32,
    pub to_edge:      Edge,
}

/// Unified input event type shared between the wire protocol and IPC layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputEvent {
    Mouse(MouseEventData),
    Key(KeyEventData),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MouseEventData {
    pub dx: i32,
    pub dy: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyEventData {
    pub scancode:  u32,
    pub pressed:   bool,
    pub modifiers: u8,
}
