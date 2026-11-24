# Protocol: how we drive the machines

This is the reference document. It describes how a Cricut machine is controlled.
The session cipher is in the Encryption chapter. Key material stays outside
this repository.

Source: static analysis of Design Space 10.12.122. Tag per claim: **(code)** =
seen in the binaries/schemas, **(inferred)** = reasoning, **(unknown)** = not
yet established.

> Status: Chain, Transport, Machine code names, Command model, and Encryption
> are written up. The byte layout of a real cut still comes from a capture
> (see chapter 6).

---

## 1. The chain

The machine connects to the computer over USB or Bluetooth. Design Space does
not talk to the machine directly but through a helper process. We take over that
helper.

```
our software ── USB/Bluetooth ── machine
```

Two generations in Design Space (**code**, see `docs/architecture.md`):

- **v6 (device-common):** the app sends structured commands (`NativeModel.*`),
  the helper converts them to the wire.
- **v7 (device-common-next) + cricut-device-io:** the app builds the machine
  bytes itself and sends them as a raw stream (`SessionDataStreamRequest {
  handle, bytes data }`). The helper finds and opens the device and passes bytes
  through.

For us the v7 design is the model: we build the bytes, the transport passes them
on. The `NativeModel` schemas remain the best description of *what* is driven.

**Key feasibility finding (code):** all four machines this project targets
(Maker, Maker 3, Joy, Joy Xtra) share one command set and encrypt the entire
command stream. The send function passes every command, except the key exchange
itself, through an encryption step. So there is **no plaintext cut path** for
these machines: the encrypted session is required to make them move and cut. The
encryption part (chapter 5) is therefore on the critical path, not a side issue.

**No cloud at cut time (code, JS side).** For our four machines the app makes no
per-cut server call that the machine needs: no server-converted job data, no
per-session key fetched from Cricut. There is a lock check
(`/machine/v2/Machines/machineinfo/{serial}`) that only blocks when the server
explicitly returns "locked" or 403; offline or on any other error the app
continues with a placeholder machine id. The per-session server handshake key
that does exist applies to other models (Joy 2, Joy Xtra 2, Maker 5, Explore 5,
Scout), **not** to our Joy or Joy Xtra. So local control is feasible; the one
remaining unknown is the machine-link encryption (chapter 5).

---

## 2. Transport (USB and Bluetooth)

The helper opens the machine itself, in-process (not through a separate driver).
For the serial path, libusbp looks up the COM port via the registry (`PortName`)
and opens it with `CreateFileA("\\.\COMn")` plus `SetCommTimeouts` **(code)**.
For Bluetooth it uses WinRT: `BluetoothDevice`, `RfcommServiceId`, `StreamSocket`
**(code)**.

Per machine:

- **Maker ("Athena") and Maker 3 ("Scamander")**: USB as a virtual COM port
  (CDC) **(code)**, or Bluetooth classic SPP over RFCOMM. The RFCOMM channel
  comes from SDP at runtime, not from a constant; v7 answers the pairing request
  with a fixed 4-character PIN **(code)**. No BLE.
- **Joy ("Cupid") and Joy Xtra ("CupidXL", family MARS)**: marked
  **Bluetooth-only** in the family table **(code)**, so classic SPP/RFCOMM, no
  USB path. BLE exists in Design Space only in `cricut-device-io` for the
  Bifrost generation; no BLE UUIDs were found for our four machines.

  Confirmed live on a Joy Xtra over SDP **(observed on a real device)**: it is
  classic Bluetooth BR/EDR (an SDP server with a PnP Information record, not pure
  BLE). The command channel is a Serial Port Profile service ("Telink Serial Port
  Service", chip vendor Telink Semiconductor) on **L2CAP + RFCOMM channel 4**.
  A second RFCOMM service on channel 7 is iAP2 (Apple accessory protocol, UUID
  `00000000-deca-fade-deca-deafdecacaff`) for iOS hosts. PnP record:
  SpecificationID `0x0102`, VendorID `0x0792`, ProductID `0x0002`. So a Joy Xtra
  is driven over RFCOMM channel 4; pairing and the encrypted session on that
  channel are the remaining pieces (chapter 5).

  Confirmed on real hardware by capturing a cut (HCI capture, dissected to
  RFCOMM): the session is L2CAP -> SDP (find Serial Port) -> RFCOMM channel 4
  (DLCI 0x08), then the Cricut SPP protocol. The SPP framing matches the
  Maker-family scheme below: a 2-byte big-endian header where the top bit is an
  encrypted flag and the low 15 bits are the body length, a plaintext setup
  phase, then 16-byte encrypted blocks. This framing was not previously
  documented for the Joy Xtra.

**Not derivable from the code (unknown):** VID/PID, the interface (CDC or HID),
endpoints and HID report sizes are in no binary; fill these with `lsusb -v` or
Device Manager. The RFCOMM channel, and whether the Joy exposes BLE after all,
come from an SDP browse and a GATT listing of your own machine.

**Which helper (code):** all four are `communicationType CNV2` and go through
`device-common` (v6) or `device-common-next` (v7), selected by the flag
`DH_MACHINES_USE_BRIDGE_NEXT`. Not through `cricut-device-io` (that is for the
Bifrost/ODM generation) and not through `vulcan` (the AutoPress heat press).

### Helper IPC (for reference, if we ever emulate the helper)

- **v7 (device-common-next):** Electron opens a named pipe
  `\\.\pipe\cricut-bridge-<uuid>` and launches the helper with `--pipe <path>`.
  Frame: a 14-byte header (`BRDG`, version 1, flags, payload length u32 LE,
  CRC-32C u32 LE) plus a `bridge.proto.Message` **(code)**.
- **v6 (device-common):** stdio, launched with `--new-comm-lib`, u32 LE length;
  payload encrypted in Electron.
- **cricut-device-io:** stdio, u32 LE length plus a
  `cricut.device_io.cli.Message` **(code)**. (Not for our machines.)

---

## 3. Machine code names

Design Space uses internal code names (`PBMachineType` in
`NativeModel.Machines.proto`) **(code)**. Mapping to retail names, from name
prefixes, family images and flag text:

| Code name     | Retail name   | Basis        |
|---------------|---------------|--------------|
| ATHENA        | Maker         | observed     |
| SCAMANDER     | Maker 3       | observed     |
| CUPID         | Joy           | observed     |
| CUPIDXL (MARS)| Joy Xtra      | observed     |
| VOLDEMORT     | Explore 3     | observed     |
| MORPHEUS      | Venture       | observed     |
| DUMBLEDORE    | Maker 5       | observed     |
| GANDALF       | Explore 5     | observed     |
| PSYCHE        | Joy 2         | observed     |
| EROS          | Joy Xtra 2    | observed     |
| NEWT          | Maker 4       | inferred (image filenames) |
| RIDDLE        | Explore 4     | inferred (image filenames) |

`SPRITE`, `RANGER`, `TITAN`, `VOYAGER`, `ZEUS`: present in the enum, no retail
mapping found (unknown). Full enum:
`proto/reference/v7-device-common-next/bridge__proto__NativeModel.Machines.proto`.

---

## 4. Command model

From `NativeModel.Commands.proto` and `NativeModel.Machines.proto` **(code)**.
This is the vocabulary of what you make a machine do. The exact byte encoding on
the wire comes from a capture; here is the meaning.

Each command object exposes its opcode byte via a vtable slot **(code)**; the
table is the same in v6 and v7. Observed opcodes include: `SetMachineState`
`0x30`, `Move4D` `0xF1`, `DrawVectorFloat` `0x42`, `LoadMat` `0x84`. The sender
passes every command except the AES handshake ids through the encryption step,
so the plaintext bodies are not visible statically.

**Motion**
- `PBDrawVectors` / `PBDrawVector`: the actual cut/draw motion, as a sequence of
  vectors. `PBDrawVectorType` gives the segment type. Coordinates and unit
  (public claim: float32 X/Y in inches) to be confirmed on a capture.
- `PBSpeedValues`, `PBSpeedGroup`: speed.

**Tool and pressure** (this explains "the blade moves but does not cut":
tool-down and pressure are separate commands next to the motion)
- `PBToolType` (**code**, full list): `ROLLING_BLADE`, `DRAG_KNIFE`,
  `TANGENTIAL_BLADE`, `PEN_DT`, `STYLUS`, `SCORING_WHEEL`, `PERFORATION_WHEEL`,
  `DEBOSSING_TOOL`, `WAVE_TOOL`, `ENGRAVE_TOOL`, `MIDAS_*`, `DEEP_CUT`,
  `AUTO_CUTOFF_TOOL`, `SCORE_TIP_TOOL`, `ENGRAVE_STATIC_TOOL`.
- `PBToolSetting`, `PBToolSettings`, `PBToolTypeState`.
- Force/pressure: `NativeModel.Force.proto`. Backlash: `NativeModel.Backlash.proto`.
- Head: `PBHeadType` (`CLAMP_A`, `CLAMP_B`), `PBAvailableHeadType`.

**Machine state and control**
- `PBSetMachineState`, `PBMachineInterface`, `PBMachineInterfaceState`.
- `PBGetFullStatus`, `PBMachineInformationBridge`, `PBSerialNumber`.
- `PBReadEdgeSensorCommand` / `PBSensorResult` (`PBSensorType`): mat and material
  sensors.
- `PBMatloadOptions`: load/unload the mat.
- `PBSimulateButton` (`PBDeviceButton`), `PBDeviceLEDState`: buttons and LEDs.
- `PBAck`: acknowledgement per command (**inferred**: each command is ACKed).
- `PBActivateDeactivateCommand`: the "machine locked/activated" check.
  **(inferred)** This is probably an entitlement check in the app, not a
  precondition for motion. To be confirmed on a capture.
- `PBRawCommandRequestResponse`: raw passthrough, useful for experimenting.

**Material**
- `PBAllMaterialSettings`, `PBMaterialSettingsApi`, `PBMachineMaterial`,
  `PBMaterialTag(s)`, `PBMatSize`, `PBMaterialInset`, `PBMachineMode`.
- **(inferred, public claim)** material settings (pressure, speed, passes per
  material) come from a Cricut online source and are cached in the profile. For
  us: we can make our own material profiles; we do not need Cricut's table to
  move. Confirm what the machine really requires as input.

---

### Cut flow (code, JS side)

Maker and Joy use the same message sequence between "Make" and the finished cut
(Joy is Bluetooth-only, single-head, matless by default):

1. Feature flags and log level, then a start-session message carrying the user
   settings as `authData` and the selected device.
2. The helper reports connected devices; the app selects one.
3. The helper asks for a machine summary; the app answers (after its server
   checks, which are soft, see chapter 1).
4. The device opens.
5. Material selection (mat size, mode, margins).
6. The helper reads the pressure table from the machine.
7. The tool list, with pressure, passes and speeds per tool.
8. The path data.
9. Load, Go, progress, tool-change, pause, unload, complete. Cancel is a
   close-session message; there are virtual Load/Go/Pause buttons.

### Design data format (code, JS side)

The design leaves the JS as **SVG path strings in points (72 per inch)**, with a
transform per layer and sizes in inches. No machine units or machine commands
leave the JS; the native helper converts paths to the motion stream. By default
the paths are encrypted locally (by `CricutPathOperation.exe`); a feature flag
sends them in plain text and the helper accepts both. For us this is convenient:
we control the vector data and feed paths, not pre-encoded machine commands.

### App-to-helper channel (reference, not the machine link)

Separate from the machine-link encryption, the app encrypts the channel to the
helper:

- **v6:** stdin/stdout, 4-byte LE length + `PBCommonBridge`; requests
  AES-256-ECB (zero padding), replies plain. The key derives from a server value
  in the user settings, masked with SHA-512 of user id + app name/version, XORed
  with a 64-byte salt in `main.js` (**inferred** that the helper holds a matching
  constant).
- **v7:** named pipe, `BRDG` header + ChaCha20-Poly1305; a random 32-byte
  session key is handed to the helper in the env var `CRICUT_BRIDGE_MESSAGE_KEY`.
  No server input.

Because we replace the helper, we do not need this channel; it only matters if
we ever emulate the helper instead. `config.bin` is a dead end: AES-256-CBC with
a key derived from an embedded string, and it only holds a build manifest
(environment, version, git commit), no device keys.

## 5. Encryption

The connection has a plaintext open phase and then an encrypted phase. The
transition is one 48-byte plaintext body. The session key is different every
connection. This repository contains no keys, no unwrap seed, and no materials
string (see `docs/legal.md`). Those are loaded at runtime.

Command and response bodies are AES-256-ECB under that 32-byte key. There is no
IV. Each block is independent.

The host seals a body before encryption:

1. Round the length down to a multiple of 16 (`n = len & !0x0f`).
2. Allocate `n + 16` zero bytes and copy the body into the front.
3. Store the 8-bit sum of every preceding byte in the last byte.
4. AES-256-ECB encrypt that buffer.

Decrypt is the inverse. The length must be a positive multiple of 16. Trailing
`0x00` bytes are dropped when the last byte is `0x00`. The low 8 bits of the
sum of every byte except the last must equal the last byte. When the first
byte is not zero, the first two bytes are swapped before the command parser
sees the body. Host commands already start with `0x00`.

The 32-byte session key is taken from the decrypted 48-byte exchange body, at
the offset in the first plaintext byte. The wrap key for that decrypt is built
from the materials record for this machine: device type (Joy Xtra is `0x13`),
firmware bounds stored as two float32 values inside one decimal integer, even
or odd bytes of the record's key material, walked forward or backward, and on
the primary match five bytes of the serial when the record says so. The
materials text itself is unwrapped with a 33-byte seed supplied at runtime.
`JoyXtraCipher` and `JoyXtraKeyExchange` implement this. `NullCipher` remains
the open phase. Key files stay outside version control (see `.gitignore`:
`*.key`, `/private/`).

---

## 6. What a capture of your own machine still has to provide

Static analysis does not tell you what actually goes over the wire. With a
machine attached to a computer you can access (a PC with Design Space, capturing
traffic with USBPcap or a BLE sniffer) we want to record:

1. **Confirm the transport**: which USB class or which BLE profile, VID/PID or
   UUIDs, endpoints. Per machine (Maker and Joy almost certainly differ).
2. **The open phase**: the exact bytes of startup, status, registers, up to the
   key exchange. We can reproduce this directly.
3. **The key exchange**: the handshake frames, so you can implement the
   algorithm without copying Cricut's keys.
4. **A real cut**: the order of commands between "Make" and done (motion,
   tool-down, pressure, mat load, progress, pause/cancel), and the byte encoding
   of `PBDrawVector` (unit, step size, line vs curve).
5. **Whether the cloud is needed at cut time**: watching whether there is a
   server request between "Make" and the motion that the machine actually needs,
   or whether it is only telemetry/entitlement.

Points 3 and 4 are the core. Once those exist, `cricut-bridge` can send a real
cut.

**The single open feasibility question.** Both the JS side and the native side
point to the same unknown: what the native helper does with the user-settings
`authData`, and whether any of it feeds the machine-link encryption key, or
whether that key is a constant inside the helper. If it is a constant (or
derivable locally), local control needs no account at all. If the machine-link
key is seeded from a server-provided settings value, you need to have signed in
once so that value is cached. A capture of the handshake (point 3) answers this
directly.
