use std::net::SocketAddrV4;
use std::sync::{Arc, RwLock};
use serde::Deserialize;
use thiserror::Error;
use crate::message::*;
use crate::{message, transport};
use crate::test::{Test, TestData, TestOptions, TestPlan};
use crate::transport::*;
use crate::transports::*;

#[derive(Deserialize)]
pub struct Config {
    /// tcp-server/client, raw-server/client, zero-copy-server/client
    transport: String,
    tcp_server: Option<TcpServerConfig>,
    tcp_client: Option<TcpClientConfig>,
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
    Err(Error::InvalidConfig(format!("The field \"{}\" is required", field)))
}

pub fn run(config: Config, test_options: TestOptions) -> Result<()> {
    match config.transport.as_str() {
        "tcp-server" => match config.tcp_server {
            None => missing_field("tcp_server"),
            Some(tcp_server_config) =>
                Ok(start_server(TcpServer::new(tcp_server_config.address), test_options)?)
        },
        "tcp-client" => match config.client {
            None => missing_field("client_config"),
            Some(client_config) => match config.tcp_client {
                None => missing_field("tcp_client"),
                Some(tcp_client_config) => Ok(start_client(TcpClient::new(tcp_client_config.address), client_config, test_options)?)
            }
        },
        _ => Err(Error::InvalidConfig(format!("Invalid transport value \"{}\"", config.transport))),
    }
}

fn start_server<S: Server<L, Conn>, L: Listener<Conn>, Conn: Connection + 'static>(server: S, test_options: TestOptions) -> Result<()> {
    let mut test_id = 0;
    let listener = server.listen()?;

    loop {
        let connection = listener.accept()?;
        let test_options = test_options.clone();

        if let Err(e) = (move || -> Result<()> {
            let mut reader = MessageReader::new(connection.clone());
            let mut writer = MessageWriter::new(connection.clone());

            let syn = reader.read_until(|m| {
                match m {
                    Message::Syn(syn) => Some(syn),
                    _ => None,
                }
            })?;

            test_id += 1;

            let final_options = syn.options.clone();
            // Send syn ack
            let syn_ack = Message::SynAck(SynAck { test_id, test_plan: final_options.clone() });
            writer.write(syn_ack)?;

            // Start test
            let test = Test::new(TestData::new(test_id, final_options.clone()), test_options.clone());
            match syn.mode {
                TransportMode::Send => start_receiver(connection, test),
                TransportMode::Receive => start_sender(connection, test)
            }?;

            Ok(())
        })() {
            println!("Error: {}", e);
        }
    }
}

fn start_client<C: Client<Conn>, Conn: Connection + 'static>(client: C, client_config: ClientConfig, test_options: TestOptions) -> Result<()> {
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
        _ => None
    })?;

    let test = Test::new(TestData::new(syn_ack.test_id, syn_ack.test_plan), test_options);

    match client_config.mode {
        TransportMode::Send => start_sender(connection, test),
        TransportMode::Receive => start_receiver(connection, test)
    }
}

fn start_sender<Conn: Connection + 'static>(mut connection: Conn, mut test: Test) -> Result<()> {
    let should_stop = Arc::new(RwLock::new(false));

    let receive_thread = {
        let mut message_reader = MessageReader::new(connection.clone());
        let mut message_writer = MessageWriter::new(connection.clone());
        let should_stop = Arc::clone(&should_stop);
        std::thread::spawn(move || -> Result<()> {
            // Wait for Fin
            message_reader.read_until(|m| match m {
                Message::Fin => Some(()),
                _ => None,
            })?;

            // Mark as stop
            *should_stop.write().unwrap() = true;

            // Send FinAck
            message_writer.write(Message::FinAck)?;
            Ok(())
        })
    };

    let buffer = vec![0; test.data.plan.packet_size];

    test.start();
    loop {
        let written = connection.write(&buffer)?;
        test.transferred(written);

        if *should_stop.read().unwrap() {
            break;
        }
    }
    test.finish();

    receive_thread.join().expect("Failed to join receive thread")?;

    Ok(())
}

fn start_receiver<Conn: Connection>(mut connection: Conn, mut test: Test) -> Result<()> {
    let mut reader = MessageReader::new(connection.clone());
    let mut writer = MessageWriter::new(connection.clone());
    let mut buffer = vec![0; Conn::header_size() + test.data.plan.packet_size];
    test.start();
    loop {
        let read = connection.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        test.transferred(read);

        // Send Fin if time is over
        if test.elapsed().as_secs_f64() > test.data.plan.duration {
            writer.write(Message::Fin)?;
            break;
        }
    }
    test.finish();

    // Wait for FinAck (graceful disconnection)
    reader.read_until_timeout(
        |m| match m {
            Message::FinAck => Some(()),
            _ => None
        },
        1000,
    )?;

    Ok(())
}
