use std::fmt::{Display, Formatter};
use clap::*;

#[derive(Parser, ValueEnum, Clone, Debug)]
pub enum FormatType {
    Json,
    Pretty,
}

impl Display for FormatType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatType::Json => f.write_str("json"),
            FormatType::Pretty => f.write_str("pretty")
        }
    }
}

#[derive(Parser, Debug)]
pub struct Command {
    #[arg(short, long)]
    pub config: Option<String>,

    #[arg(short = 'f', long = "format", default_value_t = FormatType::Pretty)]
    pub format: FormatType,
}

pub fn parse() -> Command {
    Command::parse()
}
