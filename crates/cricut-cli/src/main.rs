//! `cricut` — command line for the independent control software.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cricut",
    version,
    about = "Independent control for Cricut machines"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Connect to a machine over classic Bluetooth RFCOMM and read frames.
    ///
    /// The machine must already be paired (bluetoothctl). Plaintext open-phase
    /// frames are printed as they arrive. With `--key`, or with a config
    /// directory that holds the key-exchange inputs, encrypted frames are
    /// decrypted too.
    Connect {
        /// BD_ADDR of the machine, e.g. AA:BB:CC:DD:EE:FF
        bdaddr: String,
        /// RFCOMM channel (4 for the Joy Xtra).
        #[arg(long, default_value_t = 4)]
        channel: u8,
        /// How many seconds to read before exiting.
        #[arg(long, default_value_t = 10)]
        seconds: u64,
        /// Already-derived session key file (32 raw bytes or 64 hex chars).
        /// Skips the key exchange.
        #[arg(long)]
        key: Option<PathBuf>,
        /// Directory with materials.txt, a seed file, firmware, and serial.
        /// Defaults to ~/.config/cricut when that directory has materials.txt.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Device type for the materials record. Default 0x13 (Joy Xtra).
        #[arg(long, value_parser = parse_u32_auto)]
        machine_type: Option<u32>,
    },
}

fn parse_u32_auto(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).map_err(|e| e.to_string())
    } else {
        s.parse::<u32>().map_err(|e| e.to_string())
    }
}

fn auto_config_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let dir = PathBuf::from(home).join(".config/cricut");
    dir.join("materials.txt").is_file().then_some(dir)
}

fn load_config_dir(
    dir: &Path,
    machine_type: Option<u32>,
) -> anyhow::Result<cricut_bridge::KeyExchangeInputs> {
    let materials = std::fs::read(dir.join("materials.txt"))
        .map_err(|e| anyhow::anyhow!("reading {}/materials.txt: {e}", dir.display()))?;
    let materials_text =
        String::from_utf8(materials).map_err(|_| anyhow::anyhow!("materials.txt is not utf-8"))?;
    let seed_path = ["seed", "seed.bin", "seed.hex"]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
        .ok_or_else(|| anyhow::anyhow!("no seed, seed.bin, or seed.hex in {}", dir.display()))?;
    let seed = cricut_bridge::load_seed_file(&seed_path)?;
    let firmware = std::fs::read_to_string(dir.join("firmware"))
        .map_err(|e| anyhow::anyhow!("reading {}/firmware: {e}", dir.display()))?;
    let serial = std::fs::read_to_string(dir.join("serial")).unwrap_or_default();
    let from_file = if dir.join("machine-type").is_file() {
        let text = std::fs::read_to_string(dir.join("machine-type"))?;
        Some(parse_u32_auto(text.trim()).map_err(|e| anyhow::anyhow!(e))?)
    } else {
        None
    };
    Ok(cricut_bridge::KeyExchangeInputs {
        materials_text: Some(materials_text),
        firmware_version: Some(firmware.trim().to_string()),
        serial: Some(serial.trim().to_string()),
        unwrap_seed: Some(seed),
        machine_type: machine_type.or(from_file),
    })
}

#[cfg(target_os = "linux")]
fn run_connect(
    bdaddr: &str,
    channel: u8,
    seconds: u64,
    key: Option<PathBuf>,
    config: Option<PathBuf>,
    machine_type: Option<u32>,
) -> anyhow::Result<()> {
    use cricut_bridge::{
        JoyXtraCipher, JoyXtraKeyExchange, KeyExchange, MachineSession, SessionKey,
    };
    use cricut_transport::RfcommTransport;
    use std::time::{Duration, Instant};

    println!("connecting to {bdaddr} on RFCOMM channel {channel} ...");
    let transport = RfcommTransport::connect(bdaddr, channel)?;
    let mut session = MachineSession::new(transport);
    let mut exchange = None;
    let mut ready = false;
    if let Some(path) = key {
        let loaded = SessionKey::load_from_file(&path)?;
        session.set_cipher(Box::new(JoyXtraCipher::new(loaded)));
        ready = true;
        eprintln!("using session key from {}", path.display());
    } else if let Some(dir) = config.or_else(auto_config_dir) {
        let inputs = load_config_dir(&dir, machine_type)?;
        exchange = Some(JoyXtraKeyExchange { inputs });
        eprintln!("key exchange inputs loaded from {}", dir.display());
    }
    println!("connected; reading frames for {seconds}s (Ctrl-C to stop)");

    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        match session.recv() {
            Ok((body, encrypted)) => {
                if !ready {
                    if encrypted {
                        eprintln!("encrypted frame before the session key was installed");
                    } else if body.len() == 48 {
                        if let Some(kx) = exchange.as_mut() {
                            let derived = kx.derive_session_key(&body)?;
                            session.set_cipher(Box::new(JoyXtraCipher::new(derived)));
                            ready = true;
                            eprintln!("session key installed");
                        }
                    }
                }
                let tag = if encrypted { "ENC" } else { "   " };
                let op_at = if ready && encrypted && body.len() >= 3 && body[0] == 0 {
                    2
                } else {
                    0
                };
                let op_byte = body.get(op_at).copied().unwrap_or(0);
                let op = cricut_proto::Opcode::from_body(&[op_byte])
                    .map(|o| format!("{o:?}"))
                    .unwrap_or_else(|| format!("0x{op_byte:02x}"));
                println!("{tag} {op:>18} {}", hex(&body));
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
fn run_connect(
    _: &str,
    _: u8,
    _: u64,
    _: Option<PathBuf>,
    _: Option<PathBuf>,
    _: Option<u32>,
) -> anyhow::Result<()> {
    anyhow::bail!("RFCOMM transport is only implemented on Linux")
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    match Cli::parse().command {
        Command::Connect {
            bdaddr,
            channel,
            seconds,
            key,
            config,
            machine_type,
        } => run_connect(&bdaddr, channel, seconds, key, config, machine_type),
    }
}
