//! periphore-net codec: LengthDelimitedCodec + postcard framing for PeerMessage.
//!
//! D-18: 4-byte big-endian length header (LengthDelimitedCodec default) + postcard.
//! T-6-01: max_frame_length(64 * 1024) prevents OOM from malicious length header.
//!
//! CALLER RESPONSIBILITY: TCP_NODELAY must be set BEFORE calling split_framed().
//! See CLAUDE.md — TCP_NODELAY is a hard requirement set immediately after connect/accept.

use bytes::{Bytes, BytesMut};
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use periphore_protocol::PeerMessage;

use crate::error::NetError;

/// Maximum frame length in bytes. Protects against OOM from malicious peers (T-6-01).
/// No PeerMessage should ever approach this limit in normal operation.
pub const MAX_FRAME_LENGTH: usize = 64 * 1024;

/// Split a `TcpStream` into typed framed halves for concurrent send and receive.
///
/// Returns `(FramedRead, FramedWrite)` — each can be moved into a separate task.
/// Uses 4-byte big-endian length header (LengthDelimitedCodec default = D-18).
///
/// # Precondition
/// The caller MUST call `stream.set_nodelay(true)` BEFORE calling this function.
/// Setting TCP_NODELAY after the first write is too late — Nagle's algorithm may
/// already have delayed the initial handshake bytes (CLAUDE.md hard requirement).
pub fn split_framed(
    stream: TcpStream,
) -> (
    FramedRead<tokio::net::tcp::OwnedReadHalf, LengthDelimitedCodec>,
    FramedWrite<tokio::net::tcp::OwnedWriteHalf, LengthDelimitedCodec>,
) {
    let (read_half, write_half) = stream.into_split();
    // Two separate codec instances — LengthDelimitedCodec may not implement Clone.
    let read_codec = LengthDelimitedCodec::builder()
        .max_frame_length(MAX_FRAME_LENGTH)
        .new_codec();
    let write_codec = LengthDelimitedCodec::builder()
        .max_frame_length(MAX_FRAME_LENGTH)
        .new_codec();
    let framed_read = FramedRead::new(read_half, read_codec);
    let framed_write = FramedWrite::new(write_half, write_codec);
    (framed_read, framed_write)
}

/// Encode a `PeerMessage` to `Bytes` for sending via `FramedWrite`.
pub fn encode_message(msg: &PeerMessage) -> Result<Bytes, NetError> {
    postcard::to_allocvec(msg)
        .map(Bytes::from)
        .map_err(|e| NetError::Encode(e.to_string()))
}

/// Decode a `PeerMessage` from a raw frame received via `FramedRead`.
pub fn decode_message(frame: BytesMut) -> Result<PeerMessage, NetError> {
    postcard::from_bytes(&frame).map_err(|e| NetError::Decode(e.to_string()))
}
