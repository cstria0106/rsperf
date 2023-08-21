use std::io::{Read, Write};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("system error: {0}")]
    IO(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TransportMode {
    #[serde(rename = "send")]
    Send,
    #[serde(rename = "receive")]
    Receive,
}

pub trait Server<L: Listener<Conn>, Conn: Connection> {
    fn listen(&self) -> Result<L>;
}

pub trait Listener<Conn: Connection> {
    fn accept(&self) -> Result<Conn>;
}

pub trait Client<Conn: Connection> {
    fn connect(&self) -> Result<Conn>;
}


pub trait SetReadTimeout {
    fn set_read_timeout(&mut self, milliseconds: Option<u64>) -> std::io::Result<()>;
}

pub trait Connection: Read + Write + Clone + Send + SetReadTimeout {
    fn header_size() -> usize;
}