# Developer: implement the session cipher

The whole stack is here and builds. The one slot left is the cipher. When it is
filled in, this software can read and write a machine's encrypted command stream.

```
cargo build        # builds the workspace
cargo test         # codec tests pass; add your cipher tests
```

## What is done

- `cricut-proto`   — the bridge protobuf types and the command opcodes
  (`Opcode`), plus the full reference schemas under `proto/reference/`.
- `cricut-transport` — a working Linux RFCOMM transport (`RfcommTransport`):
  opens classic-Bluetooth channel 4 to a paired machine, read/write.
- `cricut-bridge`  — the SPP frame codec (`Frame`, `FrameReader`: the 2-byte
  header with the encrypted-flag bit), and `MachineSession` (open phase, key
  exchange hook, encrypted phase). All of this works with `NullCipher`.
- `cricut-cli`     — `cricut connect <BDADDR>` opens the link and prints the
  frames the machine sends, decoded.

## What you implement

Everything is in **`crates/cricut-bridge/src/cipher.rs`**, marked `DEVELOPER`:

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
