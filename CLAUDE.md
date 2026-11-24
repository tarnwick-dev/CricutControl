# CricutControl

Independent control software for Cricut machines we own (Joy Xtra and Maker).
It talks RFCOMM channel 4 itself. It does not run Design Space.

## Crates

- `cricut-proto` — bridge protobuf and command opcodes. Rich NativeModel
  schemas stay as reference under `proto/reference/`.
- `cricut-transport` — Linux RFCOMM (`RfcommTransport`, classic Bluetooth,
  channel 4).
- `cricut-bridge` — SPP frames (`Frame`, 2-byte big-endian header, bit `0x8000`
  means the body is ciphertext) and `MachineSession`.
- `cricut-cli` — `cricut connect <BDADDR>`.

## Session cipher

`JoyXtraCipher` is AES-256-ECB. The host adds a trailing 8-bit checksum before
encryption. `decrypt_body` checks that sum and swaps the first two plaintext
bytes when the first is not zero. `JoyXtraKeyExchange` turns the plaintext
48-byte exchange body into the 32-byte session key.

No key bytes, 33-byte unwrap seed, or materials string go in this repository.
Load them at runtime:

- `SessionKey::load_from_file` — 32 raw bytes or 64 hex chars.
- `load_seed_file` — 33 raw bytes or 66 hex chars.
- `~/.config/cricut/` or `cricut connect --config <dir>` — `materials.txt`,
  `seed` (or `seed.bin` / `seed.hex`), `firmware`, `serial`, optional
  `machine-type`. The first plaintext 48-byte body is the exchange.
- `cricut connect --key <file>` skips the exchange when the session key is
  already known.

`cargo test -p cricut-bridge` covers the synthetic vectors. Set
`CRICUT_SESSION_KEY` to a key file outside the repo to decrypt
`~/Projects/cricut-joyxtra-crypto/captures/joyxtra2.pcapng` and compare it with
`decoded/joyxtra2-first.txt`. The crypto pack is private. Do not copy it into
this tree.

## Gotchas

- The opcode of an encrypted command is byte 2 (`00 LL opcode`). Byte 0 is the
  length prefix, not the opcode.
- A trailing newline in `materials.txt` changes the unwrap window. Keep the
  file exact.
- `SessionKey`'s `Debug` is redacted. Leave it that way.
- Open phase uses `NullCipher`. `MachineSession::establish` installs the real
  cipher after the exchange. The CLI does that itself when it sees the 48-byte
  body, because the open phase comes first.
