use std::io::{Read, Write};
use std::net::SocketAddrV4;
use std::sync::Arc;
use libc::*;
use crate::c::{handle_os_result, IntoC};
use crate::transport::{Server, Client, Listener, Connection, SetReadTimeout};

type Result<T> = crate::transport::Result<T>;

struct Fd(i32);

impl Fd {
    fn value(&self) -> i32 {
        self.0
    }
}

impl Drop for Fd {
    fn drop(&mut self) {
        unsafe { close(self.value()) };
    }
}

#[derive(Clone)]
pub struct TcpConnection {
    fd: Arc<Fd>,
}

impl TcpConnection {
    fn new(fd: Fd) -> Self {
        Self { fd: Arc::new(fd) }
    }
}


impl Read for TcpConnection {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        unsafe {
            Ok(handle_os_result(recv(self.fd.value(), buffer.as_mut_ptr() as *mut c_void, buffer.len(), MSG_NOSIGNAL))? as usize)
        }
    }
}

impl Write for TcpConnection {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        unsafe {
            Ok(handle_os_result(send(self.fd.value(), buffer.as_ptr() as *const c_void, buffer.len(), MSG_NOSIGNAL))? as usize)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SetReadTimeout for TcpConnection {
    fn set_read_timeout(&mut self, milliseconds: Option<u64>) -> std::io::Result<()> {
        unsafe {
            if let Some(seconds) = milliseconds {
                let time = timeval { tv_sec: (seconds / 1000) as i64, tv_usec: ((seconds % 1000) * 1000) as i64 };
                let len = std::mem::size_of_val(&time) as socklen_t;
                let time = &time as *const timeval as *const c_void;
                handle_os_result(setsockopt(self.fd.value(), SOL_SOCKET, SO_RCVTIMEO, time, len))?;
            } else {
                let time = std::mem::zeroed::<timeval>();
                let len = std::mem::size_of_val(&time) as socklen_t;
                let time = &time as *const timeval as *const c_void;
                handle_os_result(setsockopt(self.fd.value(), SOL_SOCKET, SO_RCVTIMEO, time, len))?;
            }
        }

        Ok(())
    }
}

impl Connection for TcpConnection {
    fn header_size() -> usize {
        0
    }
}

pub struct TcpClient {
    address: SocketAddrV4,
}

impl TcpClient {
    pub fn new(address: SocketAddrV4) -> Self {
        Self { address }
    }
}

impl Client<TcpConnection> for TcpClient {
    fn connect(&self) -> Result<TcpConnection> {
        unsafe {
            // 1. Create socket
            let fd = handle_os_result(socket(AF_INET, SOCK_STREAM, 0))?;

            // 2. Connect
            let (address, length) = self.address.into_c();
            handle_os_result(connect(fd, address.as_ptr(), length))?;

            // 3. Return
            Ok(TcpConnection::new(Fd(fd)))
        }
    }
}

pub struct TcpListener {
    fd: Fd,
}

impl TcpListener {
    fn new(fd: Fd) -> Self {
        Self { fd }
    }
}

impl Listener<TcpConnection> for TcpListener {
    fn accept(&self) -> Result<TcpConnection> {
        unsafe {
            let mut address = std::mem::zeroed::<sockaddr_in>();
            let mut address_length: socklen_t = 0;
            let fd = handle_os_result(accept(self.fd.value(), &mut address as *mut sockaddr_in as *mut sockaddr, &mut address_length as *mut socklen_t))?;
            Ok(TcpConnection::new(Fd(fd)))
        }
    }
}

pub struct TcpServer {
    address: SocketAddrV4,
}

impl TcpServer {
    pub fn new(address: SocketAddrV4) -> Self {
        Self { address }
    }
}

impl Server<TcpListener, TcpConnection> for TcpServer {
    fn listen(&self) -> Result<TcpListener> {
        unsafe {
            // 1. Create socket
            let fd = handle_os_result(socket(AF_INET, SOCK_STREAM, 0))?;

            // 2. Set options
            handle_os_result(
                setsockopt(
                    fd,
                    SOL_SOCKET,
                    SO_REUSEADDR,
                    &1 as *const i32 as *const c_void,
                    std::mem::size_of::<i32>() as u32,
                )
            )?;

            // 3. Bind
            let (address, address_length) = self.address.into_c();
            handle_os_result(bind(fd, address.as_ptr(), address_length))?;

            // 4. Listen
            handle_os_result(listen(fd, 0))?;

            // 5. Return
            Ok(TcpListener::new(Fd(fd)))
        }
    }
}
