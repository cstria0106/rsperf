use crate::c::*;
use crate::transport::{Client, Connection, Listener, Server, SetReadTimeout};
use etherparse::{Ipv4Header, SerializedSize};
use libc::*;
use std::io::{Read, Write};
use std::net::SocketAddrV4;
use std::sync::Arc;

type Result<T> = crate::transport::Result<T>;

const PROTOCOL: i32 = 200;

/// Socket struct that implements recvfrom sendto
#[derive(Clone)]
pub struct RawSocket {
    fd: Arc<Fd>,
}

impl RawSocket {
    fn new(fd: Fd) -> Self {
        Self { fd: Arc::new(fd) }
    }

    fn recvfrom(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddrV4)> {
        loop {
            unsafe {
                let mut addr: sockaddr_in = std::mem::zeroed();
                let mut addrlen: socklen_t = std::mem::size_of_val(&addr) as socklen_t;
                let read = handle_os_result(recvfrom(
                    self.fd.value(),
                    buf.as_mut_ptr() as *mut c_void,
                    buf.len(),
                    MSG_NOSIGNAL,
                    &mut addr as *mut sockaddr_in as *mut sockaddr,
                    &mut addrlen as *mut socklen_t,
                ))?;

                break Ok((read as usize, SocketAddrV4::from_c(&addr)));
            }
        }
    }

    fn sendto(&self, buf: &[u8], destination: &SocketAddrV4) -> std::io::Result<usize> {
        unsafe {
            let (destination, len) = destination.into_c();
            Ok(handle_os_result(sendto(
                self.fd.value(),
                buf.as_ptr() as *const c_void,
                buf.len(),
                MSG_NOSIGNAL,
                destination.as_ptr(),
                len,
            ))? as usize)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct RawConnection {
    socket: RawSocket,
    destination: SocketAddrV4,
}

impl RawConnection {
    pub fn new(socket: RawSocket, destination: SocketAddrV4) -> Self {
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
        self.socket.fd.set_timeout(milliseconds)
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

pub struct RawListener {
    socket: RawSocket,
}

impl RawListener {
    pub fn new(socket: RawSocket) -> Self {
        Self { socket }
    }
}

impl Listener<RawConnection> for RawListener {
    fn accept(&self) -> Result<RawConnection> {
        loop {
            let mut buffer = [0; 1050];
            let (read, address) = self.socket.recvfrom(&mut buffer)?;

            let (_, payload) = match Ipv4Header::from_slice(&buffer[..read]) {
                Ok(result) => result,
                Err(_) => continue,
            };

            if payload.len() == 0 {
                break Ok(RawConnection::new(self.socket.clone(), address));
            }
        }
    }
}

pub struct RawServer {
    interface: String,
}

impl RawServer {
    pub fn new(interface: String) -> Self {
        Self { interface }
    }
}

impl Server<RawListener, RawConnection> for RawServer {
    fn listen(&self) -> crate::transport::Result<RawListener> {
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
            Ok(RawListener::new(RawSocket::new(fd)))
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
    fn connect(&self) -> crate::transport::Result<RawConnection> {
        unsafe {
            let fd = Fd::new(handle_os_result(socket(AF_INET, SOCK_RAW, PROTOCOL))?);
            handle_os_result(setsockopt(
                fd.value(),
                SOL_SOCKET,
                SO_BINDTODEVICE,
                self.interface.as_ptr() as *const c_void,
                self.interface.len() as socklen_t,
            ))?;
            let socket = RawSocket::new(fd);
            socket.sendto(&[], &self.destination)?;
            Ok(RawConnection::new(socket, self.destination.clone()))
        }
    }
}
