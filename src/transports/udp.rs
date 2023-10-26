use crate::c::*;
use crate::transport::{Client, Connection, Server, SetReadTimeout};
use crate::transports::sockets::{DgramListener, DgramSocket};
use libc::*;
use std::io::{Read, Write};
use std::net::SocketAddrV4;

use super::sockets::ConnectionFactory;

type Result<T> = crate::transport::Result<T>;

pub struct UdpServer {
    address: SocketAddrV4,
}

impl UdpServer {
    pub fn new(address: SocketAddrV4) -> Self {
        Self { address }
    }
}

#[derive(Clone)]
pub struct UdpConnection {
    socket: DgramSocket,
    destination: SocketAddrV4,
}

impl UdpConnection {
    pub fn new(socket: DgramSocket, destination: SocketAddrV4) -> Self {
        Self {
            socket,
            destination,
        }
    }
}

impl Read for UdpConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // it reads from any source address
        let (read, _) = self.socket.recvfrom(buf)?;
        Ok(read)
    }
}

impl Write for UdpConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.socket.sendto(buf, &self.destination)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.socket.flush()
    }
}

impl SetReadTimeout for UdpConnection {
    fn set_read_timeout(&mut self, milliseconds: Option<u64>) -> std::io::Result<()> {
        self.socket.set_timeout(milliseconds)
    }
}

impl Connection for UdpConnection {
    fn header_size() -> usize {
        0
    }
}

impl Drop for UdpConnection {
    fn drop(&mut self) {
        _ = self.socket.sendto(&[], &self.destination);
    }
}

pub struct UdpConnectionFactory;

impl ConnectionFactory<UdpConnection> for UdpConnectionFactory {
    fn new_connection(&self, socket: DgramSocket, destination: SocketAddrV4) -> UdpConnection {
        UdpConnection {
            socket,
            destination,
        }
    }
}

type UdpListener = DgramListener<UdpConnection, UdpConnectionFactory>;

impl Server<UdpListener, UdpConnection> for UdpServer {
    fn listen(&self) -> Result<UdpListener> {
        unsafe {
            let fd = Fd::new(handle_os_result(socket(AF_INET, SOCK_DGRAM, 0))?);

            handle_os_result(setsockopt(
                fd.value(),
                SOL_SOCKET,
                SO_REUSEADDR,
                &1 as *const i32 as *const c_void,
                std::mem::size_of::<i32>() as u32,
            ))?;

            let (address, address_length) = self.address.into_c();
            handle_os_result(bind(fd.value(), address.as_ptr(), address_length))?;

            Ok(DgramListener::new(
                DgramSocket::new(fd),
                UdpConnectionFactory,
            ))
        }
    }
}

pub struct UdpClient {
    address: SocketAddrV4,
}

impl UdpClient {
    pub fn new(address: SocketAddrV4) -> Self {
        Self { address }
    }
}

impl Client<UdpConnection> for UdpClient {
    fn connect(&self) -> Result<UdpConnection> {
        unsafe {
            let fd = Fd::new(handle_os_result(socket(AF_INET, SOCK_DGRAM, 0))?);
            let socket = DgramSocket::new(fd);
            socket.connect(&self.address)?;
            Ok(UdpConnection::new(socket, self.address.clone()))
        }
    }
}
