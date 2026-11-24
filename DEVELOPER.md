# Developer: the session cipher

`JoyXtraCipher` and `JoyXtraKeyExchange` are implemented in
`crates/cricut-bridge/src/cipher.rs`. `cargo test -p cricut-bridge` runs the
synthetic checks (the same vectors as `tools/session_cipher.py selftest` in the
private pack) and, when the pack is present, checks encrypt and exchange
against that Python reference.

Set `CRICUT_SESSION_KEY` to a 32-byte key file outside the repo to also decrypt
`joyxtra2.pcapng` and compare the first commands with `decoded/joyxtra2-first.txt`.
Optional overrides: `CRICUT_CRYPTO_PACK`, `CRICUT_JOYXTRA_PCAP`,
`CRICUT_JOYXTRA_DECODED`.

Live: `cricut connect <BDADDR> --key <file>` decrypts with a key you already
have. Without `--key`, a config directory supplies the exchange inputs
(`--config`, or `~/.config/cricut` when `materials.txt` is there). The first
plaintext 48-byte body is the exchange. Files in that directory, read exactly:

- `materials.txt` — the materials text, including any trailing newline
- `seed`, `seed.bin`, or `seed.hex` — 33 raw bytes, or 66 hex chars
- `firmware` — decimal firmware, for example `8.618`
- `serial` — machine serial, used when the matching record asks for it
- `machine-type` — optional, default `0x13`

The whole stack builds. With the cipher installed, this software can read and
write a machine's encrypted command stream.

```
cargo build        # builds the workspace
cargo test         # codec tests and the cipher checks
```

## What is done

- `cricut-proto`   — the bridge protobuf types and the command opcodes
  (`Opcode`), plus the full reference schemas under `proto/reference/`.
- `cricut-transport` — a working Linux RFCOMM transport (`RfcommTransport`):
  opens classic-Bluetooth channel 4 to a paired machine, read/write.
- `cricut-bridge`  — the SPP frame codec (`Frame`, `FrameReader`: the 2-byte
  header with the encrypted-flag bit), `MachineSession` (open phase, key
  exchange, encrypted phase), and `JoyXtraCipher`. `NullCipher` is the open
  phase.
- `cricut-cli`     — `cricut connect <BDADDR>` opens the link and prints the
  frames the machine sends. `--key` or `--config` decrypts the encrypted phase.

## What the cipher does

Everything is in **`crates/cricut-bridge/src/cipher.rs`**:

1. `JoyXtraCipher::encrypt_body` / `decrypt_body` — the per-body session cipher.
2. `JoyXtraKeyExchange::derive_session_key` — derive the session key from the
   machine's key-exchange body.

Follow the algorithm, addresses and reference implementation in the private
crypto pack
(`~/Projects/cricut-joyxtra-crypto/`):

- `CLAUDE.md` + `docs/session-cipher.md` — the exact algorithm and the plugin
  addresses.
- `tools/session_cipher.py` — a verified reference (encrypt, decrypt, exchange).
- `captures/joyxtra2.pcapng` + `decoded/joyxtra2-first.txt` — test vectors.

## Hard rule: no key material in the repo

No key bytes, no 33-byte unwrap seed, and no materials (`EncryptionGet`) string
go into this repository. The `aes` (and whatever else) crate is fine; the
*material* is not. Load it at runtime:

- `SessionKey::load_from_file(path)` reads a 32-byte or 64-hex-char key from a
  file outside the repo.
- `KeyExchangeInputs` carries the materials text / firmware version / serial /
  seed, supplied at runtime (e.g. from `~/.config/cricut/…`, which is
  `.gitignore`d). Never hard-code them.

`SessionKey`'s `Debug` is redacted on purpose. Keep it that way.

## How to verify

1. Unit: with a known session key, `decrypt_body` on the encrypted frames of
   `joyxtra2.pcapng` must reproduce `decoded/joyxtra2-first.txt`. Mirror
   `tools/session_cipher.py selftest` as a Rust test, gated behind an env var
   pointing at an external key file so nothing secret is committed.
2. Live (optional): pair a Joy Xtra to a Linux Bluetooth host (the office Pi
   works), then `cargo run -p cricut-cli -- connect <BDADDR>`. You should see the
   plaintext open-phase frames (ResetAes, Register, GetMachineStatus,
   GetInterfaceState). Wiring the cipher lets the encrypted frames decode too.

## After the cipher works

`MachineSession::establish(...)` installs the cipher after the key exchange; from
there `send(body, true)` / `recv()` handle encryption. The next build step (ours)
is the command layer: emit `DrawVectorFloat`/`Move4D`/`ToolSetting`/`LoadMat`
from an SVG path, using the `NativeModel` schemas. Ping us when the cipher lands.
