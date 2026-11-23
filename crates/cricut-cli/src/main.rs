//! `cricut` — command line for the independent control software.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cricut", version, about = "Independent control for Cricut machines")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan for attached machines (USB and Bluetooth).
    Scan,
    /// Query the status of a machine.
    Status,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let cli = Cli::parse();
    match cli.command {
        Command::Scan => {
            // Waiting on the transport layer: a DeviceScanner will be called
            // here and the machines found will be printed.
            println!("scan: transport layer not implemented yet");
        }
        Command::Status => {
            println!("status: not implemented yet");
        }
    }
    Ok(())
}
