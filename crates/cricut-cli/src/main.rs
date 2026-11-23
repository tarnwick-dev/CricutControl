//! `cricut` — commandoregel voor de eigen aansturing.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cricut", version, about = "Eigen aansturing van onze Cricut-machines")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Aangesloten machines zoeken (USB en Bluetooth).
    Scan,
    /// Status van een machine opvragen.
    Status,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let cli = Cli::parse();
    match cli.command {
        Command::Scan => {
            // Wacht op de transportlaag: dan wordt hier een DeviceScanner
            // aangeroepen en de gevonden machines afgedrukt.
            println!("scan: transportlaag nog niet ingevuld");
        }
        Command::Status => {
            println!("status: nog niet ingevuld");
        }
    }
    Ok(())
}
