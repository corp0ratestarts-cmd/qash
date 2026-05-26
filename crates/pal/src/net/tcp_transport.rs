//! TCP-backed CommitmentTransport for production Domain B deployment.
//!
//! Uses fixed-size `CommitmentFrame` framing (no length prefix required —
//! the frame size is a protocol constant). Frames are written atomically
//! using `write_all` + `flush`; reads consume exactly `COMMITMENT_FRAME_BYTES`
//! per call, returning `None` on would-block rather than blocking indefinitely.
//!
//! This is Domain B infrastructure. `CommitmentFrame` carries only hashed
//! commitments (state root, receipt root, efb root, evidence root) — no raw
//! Domain A state crosses this boundary.

use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

use crate::commitment_transport::{
    CommitmentFrame, CommitmentFrameError, CommitmentTransport, COMMITMENT_FRAME_BYTES,
};

#[derive(Debug)]
pub enum TcpTransportError {
    Frame(CommitmentFrameError),
    Io(io::Error),
}

impl From<CommitmentFrameError> for TcpTransportError {
    fn from(e: CommitmentFrameError) -> Self {
        TcpTransportError::Frame(e)
    }
}

impl From<io::Error> for TcpTransportError {
    fn from(e: io::Error) -> Self {
        TcpTransportError::Io(e)
    }
}

/// TCP-backed `CommitmentTransport`.
///
/// Wraps a single `TcpStream`. For production use, create one
/// `TcpCommitmentTransport` per peer connection. The stream is set to
/// non-blocking for `recv_commitment`; `send_commitment` always blocks
/// until the write is flushed.
pub struct TcpCommitmentTransport {
    stream: TcpStream,
}

impl TcpCommitmentTransport {
    /// Connect to a remote commitment peer.
    pub fn connect(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        // Disable Nagle — commitment frames are fixed-size and should be
        // sent immediately; buffering adds latency with no benefit here.
        stream.set_nodelay(true)?;
        stream.set_nonblocking(false)?;
        Ok(Self { stream })
    }

    /// Wrap an already-connected stream (e.g., accepted from a `TcpListener`).
    pub fn from_stream(stream: TcpStream) -> io::Result<Self> {
        stream.set_nodelay(true)?;
        stream.set_nonblocking(false)?;
        Ok(Self { stream })
    }
}

impl CommitmentTransport for TcpCommitmentTransport {
    type Error = TcpTransportError;

    fn send_commitment(&mut self, frame: CommitmentFrame) -> Result<(), TcpTransportError> {
        let encoded = frame.encode();
        self.stream.write_all(&encoded)?;
        self.stream.flush()?;
        Ok(())
    }

    fn recv_commitment(&mut self) -> Result<Option<CommitmentFrame>, TcpTransportError> {
        let mut buf = [0u8; COMMITMENT_FRAME_BYTES];

        // Set non-blocking for the peek/receive attempt so we can return
        // `None` when no frame is available without blocking the caller.
        self.stream.set_nonblocking(true)?;
        let result = self.stream.read_exact(&mut buf);
        self.stream.set_nonblocking(false)?;

        match result {
            Ok(()) => Ok(Some(CommitmentFrame::decode(&buf)?)),
            Err(ref e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut =>
            {
                Ok(None)
            }
            Err(e) => Err(TcpTransportError::Io(e)),
        }
    }
}

/// Accept commitment connections on a TCP listener, yielding one
/// `TcpCommitmentTransport` per accepted connection.
pub struct TcpCommitmentListener {
    listener: TcpListener,
}

impl TcpCommitmentListener {
    pub fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        Ok(Self { listener })
    }

    pub fn local_addr(&self) -> io::Result<std::net::SocketAddr> {
        self.listener.local_addr()
    }

    pub fn accept(&self) -> io::Result<TcpCommitmentTransport> {
        let (stream, _peer) = self.listener.accept()?;
        TcpCommitmentTransport::from_stream(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcp_transport_round_trips_single_frame() {
        let listener = TcpCommitmentListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let frame = CommitmentFrame {
            epoch: 42,
            state_root: [0xAA; 32],
            receipt_root: [0xBB; 32],
            efb_root: [0xCC; 32],
            evidence_root: [0xDD; 32],
        };

        let handle = std::thread::spawn(move || {
            let mut sender = TcpCommitmentTransport::connect(addr).unwrap();
            sender.send_commitment(frame).unwrap();
        });

        let mut receiver = listener.accept().unwrap();
        // Block until the frame arrives.
        receiver.stream.set_nonblocking(false).unwrap();
        let mut buf = [0u8; COMMITMENT_FRAME_BYTES];
        receiver.stream.read_exact(&mut buf).unwrap();
        let received = CommitmentFrame::decode(&buf).unwrap();

        handle.join().unwrap();
        assert_eq!(received, frame);
    }

    #[test]
    fn tcp_transport_multi_frame_sequence() {
        let listener = TcpCommitmentListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let frames: Vec<CommitmentFrame> = (0..5)
            .map(|i| CommitmentFrame {
                epoch: i,
                state_root: [i as u8; 32],
                receipt_root: [i as u8 + 1; 32],
                efb_root: [i as u8 + 2; 32],
                evidence_root: [i as u8 + 3; 32],
            })
            .collect();

        let frames_clone = frames.clone();
        let handle = std::thread::spawn(move || {
            let mut sender = TcpCommitmentTransport::connect(addr).unwrap();
            for frame in frames_clone {
                sender.send_commitment(frame).unwrap();
            }
        });

        let mut receiver = listener.accept().unwrap();
        receiver.stream.set_nonblocking(false).unwrap();
        let mut received = Vec::new();
        for _ in 0..5 {
            let mut buf = [0u8; COMMITMENT_FRAME_BYTES];
            receiver.stream.read_exact(&mut buf).unwrap();
            received.push(CommitmentFrame::decode(&buf).unwrap());
        }

        handle.join().unwrap();
        assert_eq!(received, frames);
    }
}
