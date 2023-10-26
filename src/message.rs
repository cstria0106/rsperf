use serde::{Deserialize, Serialize};
use snafu::{prelude::*, Backtrace};
use std::io::{BufReader, Read, Write};

use crate::test::TestPlan;
use crate::transport::{SetReadTimeout, TransportMode};

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("system error: {}", source), context(false))]
    IO {
        source: std::io::Error,
        backtrace: Backtrace,
    },
    #[snafu(display("serde error: {}", source), context(false))]
    SerDe {
        source: bincode::Error,
        backtrace: Backtrace,
    },
    #[snafu(display("read timeout"))]
    ReadTimeout { backtrace: Backtrace },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Syn {
    pub mode: TransportMode,
    pub options: TestPlan,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SynAck {
    pub test_id: usize,
    pub test_plan: TestPlan,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Syn(Syn),
    SynAck(SynAck),
}

const MESSAGE_SIGNATURE: &[u8] = b"@PERF@";

pub struct MessageReader<R: Read, T: SetReadTimeout> {
    timeout: T,
    reader: BufReader<R>,
    buffer: Vec<u8>,
}

impl<R: Read + SetReadTimeout + Clone> MessageReader<R, R> {
    pub fn new(read: R) -> Self {
        Self {
            timeout: read.clone(),
            reader: BufReader::new(read.clone()),
            buffer: vec![0; 1050],
        }
    }

    pub fn read(&mut self) -> Result<Message> {
        loop {
            // Read until signature come
            'outer: loop {
                let mut signature_buffer = [0u8];
                for &byte in MESSAGE_SIGNATURE {
                    self.reader.read_exact(&mut signature_buffer)?;
                    if signature_buffer[0] != byte {
                        continue 'outer;
                    }
                }
                break;
            }

            // Read message size
            let mut size_buffer = [0u8; 4];
            self.reader.read_exact(&mut size_buffer)?;
            let size = u32::from_be_bytes(size_buffer) as usize;

            // Read message
            self.buffer.resize(size, 0);
            self.reader.read_exact(&mut self.buffer)?;
            if let Ok(message) = bincode::deserialize(&self.buffer) {
                break Ok(message);
            }
        }
    }

    /// Read message until the function `f` returns `Some<T>`
    pub fn read_until<T>(&mut self, f: fn(message: Message) -> Option<T>) -> Result<T> {
        let value = loop {
            let message = self.read()?;
            if let Some(value) = f(message) {
                break value;
            }
        };
        Ok(value)
    }

    pub fn read_until_timeout<T>(
        &mut self,
        f: fn(message: Message) -> Option<T>,
        milliseconds: u64,
    ) -> Result<T> {
        self.timeout.set_read_timeout(Some(milliseconds))?;
        let value = loop {
            let message = self.read().map_err(|e| match e {
                // Convert IO TimedOut error to ReadTimeout error
                Error::IO { source, backtrace } => match source.kind() {
                    std::io::ErrorKind::WouldBlock => ReadTimeoutSnafu.build(),
                    _ => Error::IO { source, backtrace },
                },
                _ => e,
            })?;

            if let Some(value) = f(message) {
                break value;
            }
        };
        self.timeout.set_read_timeout(None)?;
        Ok(value)
    }
}

pub struct MessageWriter<W: Write> {
    writer: W,
    buffer: Vec<u8>,
}

impl<W: Write> MessageWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            buffer: Vec::new(),
        }
    }

    pub fn write(&mut self, message: Message) -> Result<()> {
        self.buffer.clear();

        // Write signature
        self.buffer.extend_from_slice(MESSAGE_SIGNATURE);

        // Write size
        let message_size = bincode::serialized_size(&message)? as usize;
        self.buffer
            .extend_from_slice(&(message_size as u32).to_be_bytes());

        // Write message
        let len = self.buffer.len();
        self.buffer.resize(self.buffer.len() + message_size, 0);
        bincode::serialize_into(&mut self.buffer[len..], &message)?;
        self.writer.write(&self.buffer)?;
        Ok(())
    }
}
