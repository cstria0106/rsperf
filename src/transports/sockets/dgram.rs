use crate::c::*;
use crate::transport::{Connection, Listener};
use libc::*;
use std::marker::PhantomData;
use std::net::SocketAddrV4;
use std::sync::Arc;

type Result<T> = crate::transport::Result<T>;

/// Socket struct that implements recvfrom sendto
#[derive(Clone)]
pub struct DgramSocket {
    fd: Arc<Fd>,
}

impl DgramSocket {
    pub fn new(fd: Fd) -> Self {
        Self { fd: Arc::new(fd) }
    }

    pub fn recvfrom(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddrV4)> {
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

    pub fn sendto(&self, buf: &[u8], destination: &SocketAddrV4) -> std::io::Result<usize> {
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

    pub fn connect(&self, destination: &SocketAddrV4) -> std::io::Result<()> {
        self.sendto(&[], &destination)?;
        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    pub fn set_timeout(&self, milliseconds: Option<u64>) -> std::io::Result<()> {
        self.fd.set_timeout(milliseconds)
    }
}

pub struct DgramListener<Conn: Connection, ConnFactory: ConnectionFactory<Conn>> {
    socket: DgramSocket,
    connection_factory: ConnFactory,
    phantom_connection: PhantomData<Conn>,
}

impl<Conn: Connection, ConnFactory: ConnectionFactory<Conn>> DgramListener<Conn, ConnFactory> {
    pub fn new(socket: DgramSocket, connection_factory: ConnFactory) -> Self {
        Self {
            socket,
            connection_factory,
            phantom_connection: PhantomData,
        }
    }
}

impl<Conn: Connection, ConnFactory: ConnectionFactory<Conn>> Listener<Conn>
    for DgramListener<Conn, ConnFactory>
{
    fn accept(&self) -> Result<Conn> {
        loop {
            let mut buffer = [0; 1050];
            let (read, address) = self.socket.recvfrom(&mut buffer)?;

            let payload = &buffer[Conn::header_size()..read];

            if payload.len() == 0 {
                break Ok(self
                    .connection_factory
                    .new_connection(self.socket.clone(), address));
            }
        }
    }
}

pub trait ConnectionFactory<Conn: Connection> {
    fn new_connection(&self, socket: DgramSocket, destination: SocketAddrV4) -> Conn;
}
