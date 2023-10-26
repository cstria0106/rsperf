mod c;
mod cli;
mod message;
mod program;
mod test;
mod test_format;
mod transport;
mod transports;

use crate::program::Config;
use crate::test::TestOptions;
use crate::test_format::{Format, FormattedTestPrinter, Json, Pretty};

use std::fs::File;
use std::io::{stdin, Read};

type Result = std::result::Result<(), Box<dyn snafu::Error>>;

fn start_handle_signals() {
    ctrlc::set_handler(|| {
        std::process::exit(0);
    })
    .unwrap();
}

fn start<R: Read, F: Format + Clone + 'static>(reader: R, format: F) -> Result {
    let mut deserializer = serde_json::Deserializer::from_reader(reader).into_iter();
    let printer = FormattedTestPrinter::new(format);
    loop {
        // Read config from stream
        let config: Config = if let Some(config) = deserializer.next() {
            config?
        } else {
            break;
        };

        program::run(config, TestOptions::new(1.0, printer.clone()))?;
    }

    Ok(())
}

fn start_from_file<F: Format + Clone + 'static>(path: &str, format: F) -> Result {
    eprintln!("Press enter to start");
    let mut buf = [];
    _ = stdin().read(&mut buf);
    eprintln!("Started");
    start(File::open(path)?, format)
}

fn start_from_stdin<F: Format + Clone + 'static>(format: F) -> Result {
    start(stdin(), format)
}

fn start_cli() -> Result {
    let command = cli::parse();
    match command.format {
        cli::FormatType::Json => {
            if let Some(config) = command.config {
                start_from_file(&config, Json)
            } else {
                start_from_stdin(Json)
            }
        }
        cli::FormatType::Pretty => {
            if let Some(config) = command.config {
                start_from_file(&config, Pretty)
            } else {
                start_from_stdin(Pretty)
            }
        }
    }
}

fn main() {
    start_handle_signals();

    if let Err(e) = start_cli() {
        eprintln!("error: {}", e);
    }
}
