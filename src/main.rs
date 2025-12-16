use payments_engine::{Format, PaymentsEngine, Result};

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::process;

use clap::{Parser, ValueHint};
use log::{error, info};

#[derive(Parser)]
#[command(author, version)]
struct Arguments {
    #[arg(value_name = "INPUT", value_hint = ValueHint::FilePath)]
    input: PathBuf,
}

fn main() {
    env_logger::init();

    if let Err(e) = run() {
        error!("{e}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let Arguments { input } = Arguments::parse();

    let mut engine = PaymentsEngine::new();
    let reader = BufReader::new(File::open(&input)?);

    info!("Processing transactions from {}", input.display());

    engine.process_reader(reader, Format::Csv)?;
    engine.export_accounts(std::io::stdout(), Format::Csv)?;

    Ok(())
}
