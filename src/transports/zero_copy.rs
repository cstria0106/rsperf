use std::io::{Read, Write};
use crate::transport::{Client, Connection, Listener, Server, SetReadTimeout};
use libc::*;

struct ZeroCopyConnection {}

impl Read for ZeroCopyConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
    }
}

impl Write for ZeroCopyConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Clone for ZeroCopyConnection {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl SetReadTimeout for ZeroCopyConnection {
    fn set_read_timeout(&mut self, milliseconds: Option<u64>) -> std::io::Result<()> {
        todo!()
    }
}

impl Connection for ZeroCopyConnection {
    fn header_size() -> usize {
        0
    }
}

struct ZeroCopyListener {}

impl Listener<ZeroCopyConnection> for ZeroCopyListener {
    fn accept(&self) -> crate::transport::Result<ZeroCopyConnection> {
        todo!()
    }
}

struct ZeroCopyServer {}

impl Server<ZeroCopyListener, ZeroCopyConnection> for ZeroCopyServer {
    fn listen(&self) -> crate::transport::Result<ZeroCopyListener> {
        todo!()
    }
}

struct ZeroCopyClient {}

impl Client<ZeroCopyConnection> for ZeroCopyClient {
    fn connect(&self) -> crate::transport::Result<ZeroCopyConnection> {
        todo!()
    }
}
