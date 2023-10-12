use crate::message::*;
use crate::test::{Test, TestData, TestOptions, TestPlan};
use crate::transport::*;
use crate::transports::*;
use crate::{message, transport};
use serde::Deserialize;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[derive(Deserialize)]
pub struct Config {
    /// tcp-server/client, raw-server/client, zero-copy-server/client
    transport: String,
    tcp_server: Option<TcpServerConfig>,
    tcp_client: Option<TcpClientConfig>,
    raw_server: Option<RawServerConfig>,
    raw_client: Option<RawClientConfig>,
    client: Option<ClientConfig>,
}

#[derive(Deserialize)]
struct TcpServerConfig {
    address: SocketAddrV4,
}

#[derive(Deserialize)]
struct TcpClientConfig {
    address: SocketAddrV4,
}

#[derive(Deserialize)]
struct RawServerConfig {
    interface: String,
}

#[derive(Deserialize)]
struct RawClientConfig {
    interface: String,
    address: Ipv4Addr,
}

#[derive(Deserialize)]
struct ClientConfig {
    mode: TransportMode,
    test_plan: TestPlan,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("system error: {0}")]
    IO(#[from] std::io::Error),
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    #[error("transport error: {0}")]
    Transport(#[from] transport::Error),
    #[error("message error: {0}")]
    Message(#[from] message::Error),
}

type Result<T> = std::result::Result<T, Error>;

fn missing_field(field: &'static str) -> Result<()> {
    Err(Error::InvalidConfig(format!(
        "The field \"{}\" is required",
        field
    )))
}

pub fn run(config: Config, test_options: TestOptions) -> Result<()> {
    match config.transport.as_str() {
        "tcp-server" => match config.tcp_server {
            None => missing_field("tcp_server"),
            Some(tcp_server_config) => Ok(start_server(
                TcpServer::new(tcp_server_config.address),
                test_options,
            )?),
        },
        "tcp-client" => match config.client {
            None => missing_field("client_config"),
            Some(client_config) => match config.tcp_client {
                None => missing_field("tcp_client"),
                Some(tcp_client_config) => Ok(start_client(
                    TcpClient::new(tcp_client_config.address),
                    client_config,
                    test_options,
                )?),
            },
        },
        "raw-server" => match config.raw_server {
            None => missing_field("raw_server"),
            Some(raw_server_config) => Ok(start_server(
                RawServer::new(raw_server_config.interface),
                test_options,
            )?),
        },
        "raw-client" => match config.client {
            None => missing_field("client_config"),
            Some(client_config) => match config.raw_client {
                None => missing_field("raw_client"),
                Some(raw_client_config) => Ok(start_client(
                    RawClient::new(
                        raw_client_config.interface,
                        SocketAddrV4::new(raw_client_config.address, 0),
                    ),
                    client_config,
                    test_options,
                )?),
            },
        },
        _ => Err(Error::InvalidConfig(format!(
            "Invalid transport value \"{}\"",
            config.transport
        ))),
    }
}

fn start_server<S: Server<L, Conn>, L: Listener<Conn>, Conn: Connection + 'static>(
    server: S,
    test_options: TestOptions,
) -> Result<()> {
    let mut test_id = 0;
    let listener = server.listen()?;

    loop {
        let connection = listener.accept()?;
        let test_options = test_options.clone();

        if let Err(e) = (move || -> Result<()> {
            let mut reader = MessageReader::new(connection.clone());
            let mut writer = MessageWriter::new(connection.clone());

            let syn = reader.read_until(|m| match m {
                Message::Syn(syn) => Some(syn),
                _ => None,
            })?;

            test_id += 1;

            let final_options = syn.options.clone();
            // Send syn ack
            let syn_ack = Message::SynAck(SynAck {
                test_id,
                test_plan: final_options.clone(),
            });
            writer.write(syn_ack)?;

            // Start test
            let test = Test::new(
                TestData::new(test_id, final_options.clone()),
                test_options.clone(),
            );
            match syn.mode {
                TransportMode::Send => start_receiver(connection, test),
                TransportMode::Receive => start_sender(connection, test),
            }?;

            Ok(())
        })() {
            eprintln!("error: {}", e);
        }
    }
}

fn start_client<C: Client<Conn>, Conn: Connection + 'static>(
    client: C,
    client_config: ClientConfig,
    test_options: TestOptions,
) -> Result<()> {
    let connection = client.connect()?;
    let mut reader = MessageReader::new(connection.clone());
    let mut writer = MessageWriter::new(connection.clone());

    // Send Syn
    writer.write(Message::Syn(Syn {
        mode: client_config.mode.clone(),
        options: client_config.test_plan.clone(),
    }))?;

    // Wait for SynAck
    let syn_ack = reader.read_until(|m| match m {
        Message::SynAck(syn_ack) => Some(syn_ack),
        _ => None,
    })?;

    let test = Test::new(
        TestData::new(syn_ack.test_id, syn_ack.test_plan),
        test_options,
    );

    match client_config.mode {
        TransportMode::Send => start_sender(connection, test),
        TransportMode::Receive => start_receiver(connection, test),
    }
}

fn start_sender<Conn: Connection + 'static>(mut connection: Conn, mut test: Test) -> Result<()> {
    println!("Sender started");
    let buffer = vec![0; test.data.plan.packet_size];
    test.start();
    loop {
        let written = match connection.write(&buffer) {
            Ok(written) => written,
            Err(e) => match e.kind() {
                std::io::ErrorKind::ConnectionReset => break,
                _ => {
                    if let Some(raw_error) = e.raw_os_error() {
                        // No buffer space available
                        if raw_error == 105 {
                            continue;
                        } else {
                            return Err(Error::IO(e));
                        }
                    } else {
                        return Err(Error::IO(e));
                    }
                }
            },
        };
        test.transferred(written);

        // Break if time is over
        if test.elapsed().as_secs_f64() > test.data.plan.duration {
            break;
        }
    }
    test.finish();

    Ok(())
}

fn start_receiver<Conn: Connection>(mut connection: Conn, mut test: Test) -> Result<()> {
    println!("Receiver started");
    let header_size = Conn::header_size();
    let mut buffer = vec![0; header_size + test.data.plan.packet_size];
    test.start();
    loop {
        let read = connection.read(&mut buffer)?;
        if read == header_size {
            break;
        }
        test.transferred(read);
    }
    test.finish();

    Ok(())
}
