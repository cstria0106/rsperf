use crate::c::*;
use crate::transport::{Client, Connection, Server, SetReadTimeout};
use crate::transports::sockets::{DgramListener, DgramSocket};
use etherparse::{Ipv4Header, SerializedSize};
use libc::*;
use std::io::{Read, Write};
use std::net::SocketAddrV4;

use super::sockets::ConnectionFactory;

type Result<T> = crate::transport::Result<T>;

const PROTOCOL: i32 = 200;

pub struct RawServer {
    interface: String,
}

impl RawServer {
    pub fn new(interface: String) -> Self {
        Self { interface }
    }
}

#[derive(Clone)]
pub struct RawConnection {
    socket: DgramSocket,
    destination: SocketAddrV4,
}

impl RawConnection {
    pub fn new(socket: DgramSocket, destination: SocketAddrV4) -> Self {
        Self {
            socket,
            destination,
        }
    }
}

impl Read for RawConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // it reads from any source address
        let (read, _) = self.socket.recvfrom(buf)?;
        Ok(read)
    }
}

impl Write for RawConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.socket.sendto(buf, &self.destination)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.socket.flush()
    }
}

impl SetReadTimeout for RawConnection {
    fn set_read_timeout(&mut self, milliseconds: Option<u64>) -> std::io::Result<()> {
        self.socket.set_timeout(milliseconds)
    }
}

impl Connection for RawConnection {
    fn header_size() -> usize {
        Ipv4Header::SERIALIZED_SIZE
    }
}

impl Drop for RawConnection {
    fn drop(&mut self) {
        _ = self.socket.sendto(&[], &self.destination);
    }
}

pub struct RawConnectionFactory;

impl ConnectionFactory<RawConnection> for RawConnectionFactory {
    fn new_connection(&self, socket: DgramSocket, destination: SocketAddrV4) -> RawConnection {
        RawConnection {
            socket,
            destination,
        }
    }
}

type RawListener = DgramListener<RawConnection, RawConnectionFactory>;

impl Server<RawListener, RawConnection> for RawServer {
    fn listen(&self) -> Result<RawListener> {
        unsafe {
            let fd = Fd::new(handle_os_result(socket(AF_INET, SOCK_RAW, PROTOCOL))?);
            // Bind to device
            handle_os_result(setsockopt(
                fd.value(),
                SOL_SOCKET,
                SO_BINDTODEVICE,
                self.interface.as_ptr() as *const c_void,
                self.interface.len() as socklen_t,
            ))?;
            Ok(DgramListener::new(
                DgramSocket::new(fd),
                RawConnectionFactory,
            ))
        }
    }
}

pub struct RawClient {
    interface: String,
    destination: SocketAddrV4,
}

impl RawClient {
    pub fn new(interface: String, destination: SocketAddrV4) -> Self {
        Self {
            interface,
            destination,
        }
    }
}

impl Client<RawConnection> for RawClient {
    fn connect(&self) -> Result<RawConnection> {
        unsafe {
            let fd = Fd::new(handle_os_result(socket(AF_INET, SOCK_RAW, PROTOCOL))?);
            handle_os_result(setsockopt(
                fd.value(),
                SOL_SOCKET,
                SO_BINDTODEVICE,
                self.interface.as_ptr() as *const c_void,
                self.interface.len() as socklen_t,
            ))?;
            let socket = DgramSocket::new(fd);
            socket.connect(&self.destination)?;
            Ok(RawConnection::new(socket, self.destination.clone()))
        }
    }
}
