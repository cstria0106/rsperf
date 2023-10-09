use std::net::{Ipv4Addr, SocketAddrV4};

use libc::*;
use num_traits::Num;

#[derive(Debug, Clone, Copy)]
pub struct Sockaddr {
    inner: sockaddr_in,
}

impl Sockaddr {
    pub fn as_ptr(&self) -> *const sockaddr {
        &self.inner as *const _ as *const sockaddr
    }
}

pub trait IntoC<T> {
    fn into_c(self) -> T;
}

impl IntoC<(Sockaddr, socklen_t)> for &SocketAddrV4 {
    fn into_c(self) -> (Sockaddr, socklen_t) {
        (
            Sockaddr {
                inner: sockaddr_in {
                    sin_family: AF_INET as sa_family_t,
                    sin_port: self.port().to_be(),
                    sin_addr: in_addr {
                        s_addr: u32::from_be_bytes(self.ip().octets()).to_le(),
                    },
                    sin_zero: unsafe { std::mem::zeroed() },
                },
            },
            std::mem::size_of::<sockaddr_in>() as socklen_t,
        )
    }
}

pub trait FromC<T> {
    fn from_c(value: &T) -> Self;
}

impl FromC<sockaddr_in> for SocketAddrV4 {
    fn from_c(address: &sockaddr_in) -> Self {
        let ip_bytes = u32::from_be(address.sin_addr.s_addr).to_ne_bytes();
        let ip = Ipv4Addr::new(ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]);
        let port = u16::from_be(address.sin_port);
        Self::new(ip, port)
    }
}

#[inline]
pub fn handle_os_result<T: Ord + Num>(value: T) -> std::io::Result<T> {
    if value < T::zero() {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(value)
    }
}

pub struct Fd(i32);

impl Fd {
    pub fn new(value: i32) -> Self {
        Self(value)
    }

    pub fn value(&self) -> i32 {
        self.0
    }

    pub fn set_timeout(&self, milliseconds: Option<u64>) -> std::io::Result<()> {
        unsafe {
            if let Some(seconds) = milliseconds {
                let time = timeval {
                    tv_sec: (seconds / 1000) as i64,
                    tv_usec: ((seconds % 1000) * 1000) as i64,
                };
                let len = std::mem::size_of_val(&time) as socklen_t;
                let time = &time as *const timeval as *const c_void;
                handle_os_result(setsockopt(self.value(), SOL_SOCKET, SO_RCVTIMEO, time, len))?;
            } else {
                let time = std::mem::zeroed::<timeval>();
                let len = std::mem::size_of_val(&time) as socklen_t;
                let time = &time as *const timeval as *const c_void;
                handle_os_result(setsockopt(self.value(), SOL_SOCKET, SO_RCVTIMEO, time, len))?;
            }
        }

        Ok(())
    }
}

impl Drop for Fd {
    fn drop(&mut self) {
        unsafe { close(self.value()) };
    }
}
