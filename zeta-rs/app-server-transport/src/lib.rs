//! Bounded JSON-lines transport building blocks for the app server.

use std::io::{self, BufRead, Write};

pub const DEFAULT_MAX_MESSAGE_BYTES: usize = 1_048_576;

/// A JSON Lines transport whose read and write operations enforce the negotiated message limit.
pub struct JsonlTransport<R, W> {
    reader: R,
    writer: W,
    max_message_bytes: usize,
}

impl<R: BufRead, W: Write> JsonlTransport<R, W> {
    pub fn new(reader: R, writer: W, max_message_bytes: usize) -> Self {
        Self {
            reader,
            writer,
            max_message_bytes,
        }
    }

    pub fn read_message(&mut self) -> io::Result<Option<String>> {
        let mut bytes = Vec::new();
        let read = self.reader.read_until(b'\n', &mut bytes)?;
        if read == 0 {
            return Ok(None);
        }
        if bytes.len() > self.max_message_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "JSON-RPC message exceeds limit",
            ));
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
        String::from_utf8(bytes).map(Some).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "JSON-RPC message is not UTF-8")
        })
    }

    pub fn write_message(&mut self, message: &str) -> io::Result<()> {
        if message.len() > self.max_message_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "JSON-RPC message exceeds limit",
            ));
        }
        self.writer.write_all(message.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()
    }
}

pub struct StdioTransport;
impl StdioTransport {
    pub fn listen_uri() -> &'static str {
        "stdio://"
    }
}
