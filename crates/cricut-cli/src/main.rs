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
    /// Connect to a machine over classic Bluetooth RFCOMM and read frames.
    ///
    /// The machine must already be paired (bluetoothctl). Reads and prints the
    /// frames the machine sends, decoded by the SPP codec. Encrypted frames are
    /// shown as such (they decode once the cipher is implemented).
    Connect {
        /// BD_ADDR of the machine, e.g. AA:BB:CC:DD:EE:FF
        bdaddr: String,
        /// RFCOMM channel (4 for the Joy Xtra).
        #[arg(long, default_value_t = 4)]
        channel: u8,
        /// How many seconds to read before exiting.
        #[arg(long, default_value_t = 10)]
        seconds: u64,
    },
}

#[cfg(target_os = "linux")]
fn run_connect(bdaddr: &str, channel: u8, seconds: u64) -> anyhow::Result<()> {
    use cricut_bridge::MachineSession;
    use cricut_transport::RfcommTransport;
    use std::time::{Duration, Instant};

    println!("connecting to {bdaddr} on RFCOMM channel {channel} ...");
    let transport = RfcommTransport::connect(bdaddr, channel)?;
    let mut session = MachineSession::new(transport);
    println!("connected; reading frames for {seconds}s (Ctrl-C to stop)");

    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        match session.recv() {
            Ok((body, encrypted)) => {
                let tag = if encrypted { "ENC" } else { "   " };
                let op = cricut_proto::Opcode::from_body(&body)
                    .map(|o| format!("{o:?}"))
                    .unwrap_or_else(|| format!("0x{:02x}", body.first().copied().unwrap_or(0)));
                println!("{tag} {:>18} {}", op, hex(&body));
            }
            Err(cricut_bridge::BridgeError::Transport(
                cricut_transport::TransportError::Timeout(_),
            )) => continue,
            Err(e) => {
                eprintln!("recv: {e}");
                break;
            }
        }
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn run_connect(_: &str, _: u8, _: u64) -> anyhow::Result<()> {
    anyhow::bail!("RFCOMM transport is only implemented on Linux")
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    match Cli::parse().command {
        Command::Connect { bdaddr, channel, seconds } => run_connect(&bdaddr, channel, seconds),
    }
}
