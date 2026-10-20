//! Length-prefixed frame codec for the local daemon transport.
//!
//! Every frame is a 4-byte big-endian length header followed by that
//! many payload bytes. The length is validated before any allocation so
//! an oversized or truncated frame never causes unbounded buffering.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::IpcError;

/// Number of bytes in the length header.
pub const LENGTH_PREFIX_BYTES: usize = 4;

/// Default maximum accepted frame payload size (1 MiB).
pub const DEFAULT_MAX_FRAME_BYTES: usize = 1024 * 1024;

/// Encodes a payload into a length-prefixed frame.
pub fn encode_frame(payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(LENGTH_PREFIX_BYTES + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

/// Decodes a length header and validates it against `max_frame_bytes`.
pub fn decode_length(
    header: &[u8; LENGTH_PREFIX_BYTES],
    max_frame_bytes: usize,
) -> Result<usize, IpcError> {
    let length = u32::from_be_bytes(*header) as usize;
    if length > max_frame_bytes {
        return Err(IpcError::FrameTooLarge {
            limit: max_frame_bytes,
        });
    }
    Ok(length)
}

/// Writes a payload as a single length-prefixed frame and flushes it.
pub async fn write_frame<W>(writer: &mut W, payload: &[u8]) -> Result<(), IpcError>
where
    W: AsyncWrite + Unpin,
{
    let frame = encode_frame(payload);
    writer
        .write_all(&frame)
        .await
        .map_err(IpcError::TransportUnavailable)?;
    writer.flush().await.map_err(IpcError::TransportUnavailable)
}

/// Reads exactly one length-prefixed frame.
///
/// Returns `Ok(None)` when the peer closed the connection cleanly on a
/// frame boundary.
pub async fn read_frame<R>(
    reader: &mut R,
    max_frame_bytes: usize,
) -> Result<Option<Vec<u8>>, IpcError>
where
    R: AsyncRead + Unpin,
{
    let mut header = [0_u8; LENGTH_PREFIX_BYTES];
    let mut filled = 0;
    while filled < LENGTH_PREFIX_BYTES {
        let read = reader
            .read(&mut header[filled..])
            .await
            .map_err(IpcError::TransportUnavailable)?;
        if read == 0 {
            if filled == 0 {
                return Ok(None);
            }
            return Err(IpcError::MalformedFrame {
                reason: "truncated length header",
            });
        }
        filled += read;
    }

    let length = decode_length(&header, max_frame_bytes)?;
    if length == 0 {
        return Err(IpcError::MalformedFrame {
            reason: "empty frame payload",
        });
    }

    let mut payload = vec![0_u8; length];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(IpcError::TransportUnavailable)?;
    Ok(Some(payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::IpcErrorCode;

    #[test]
    fn encodes_length_prefixed_payload() {
        let frame = encode_frame(b"hello");
        assert_eq!(frame.len(), LENGTH_PREFIX_BYTES + 5);
        assert_eq!(&frame[..LENGTH_PREFIX_BYTES], &5_u32.to_be_bytes());
        assert_eq!(&frame[LENGTH_PREFIX_BYTES..], b"hello");
    }

    #[test]
    fn rejects_oversized_length_before_allocating() {
        let header = (DEFAULT_MAX_FRAME_BYTES as u32 + 1).to_be_bytes();
        let error = decode_length(&header, DEFAULT_MAX_FRAME_BYTES).unwrap_err();
        assert_eq!(error.code(), IpcErrorCode::FrameTooLarge);
    }

    #[tokio::test]
    async fn round_trips_a_frame() {
        let mut buffer = Vec::new();
        write_frame(&mut buffer, b"payload")
            .await
            .expect("write succeeds");
        let mut cursor = std::io::Cursor::new(buffer);
        let frame = read_frame(&mut cursor, DEFAULT_MAX_FRAME_BYTES)
            .await
            .expect("read succeeds")
            .expect("frame present");
        assert_eq!(frame, b"payload");
    }

    #[tokio::test]
    async fn reports_clean_close_at_frame_boundary() {
        let mut cursor = std::io::Cursor::new(Vec::new());
        assert!(read_frame(&mut cursor, DEFAULT_MAX_FRAME_BYTES)
            .await
            .expect("read succeeds")
            .is_none());
    }

    #[tokio::test]
    async fn rejects_truncated_length_header() {
        let mut cursor = std::io::Cursor::new(vec![0_u8, 1]);
        let error = read_frame(&mut cursor, DEFAULT_MAX_FRAME_BYTES)
            .await
            .unwrap_err();
        assert_eq!(error.code(), IpcErrorCode::MalformedFrame);
    }

    #[tokio::test]
    async fn rejects_truncated_payload() {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&8_u32.to_be_bytes());
        buffer.extend_from_slice(b"short");
        let mut cursor = std::io::Cursor::new(buffer);
        assert!(read_frame(&mut cursor, DEFAULT_MAX_FRAME_BYTES)
            .await
            .is_err());
    }
}
