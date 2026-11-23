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
    /// Plan the draw-vector frames for an SVG path (no machine needed).
    ///
    /// Parses the SVG path `d` string, flattens it to polylines in inches, and
    /// shows the planned draw-command frames. The exact wire bytes are the
    /// hypothesis in cricut-proto::wire (to confirm against a decrypted cut).
    Plan {
        /// SVG path `d` string, e.g. "M0,0 L72,0 L72,72 L0,72 Z"
        path_d: String,
        /// Source units: points (72/inch), css (96/inch) or inch.
        #[arg(long, default_value = "points")]
        units: String,
    },
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

fn run_plan(path_d: &str, units: &str) -> anyhow::Result<()> {
    use cricut_bridge::cut::{plan_draw_frames, polylines_to_records};
    use cricut_design::{path_to_polylines, Units};

    let units = match units {
        "points" => Units::POINTS,
        "css" => Units::CSS_PX,
        "inch" | "inches" => Units::INCHES,
        other => anyhow::bail!("unknown units '{other}' (points|css|inch)"),
    };
    let polylines = path_to_polylines(path_d, units, 0.005)?;
    let records = polylines_to_records(&polylines);
    let frames = plan_draw_frames(&polylines);
    println!(
        "{} subpath(s), {} draw records, {} draw frame(s)",
        polylines.len(),
        records.len(),
        frames.len()
    );
    for (i, f) in frames.iter().enumerate() {
        println!("frame {i}: {} bytes  {}", f.len(), hex(&f[..f.len().min(24)]));
    }
    println!("(wire byte layout is unconfirmed; see cricut-proto::wire)");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    match Cli::parse().command {
        Command::Plan { path_d, units } => run_plan(&path_d, &units),
        Command::Connect { bdaddr, channel, seconds } => run_connect(&bdaddr, channel, seconds),
    }
}
