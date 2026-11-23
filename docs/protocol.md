# Protocol: how we drive the machines

This is the reference document. It describes how a Cricut machine is controlled,
as far as we know from static analysis, and it marks exactly where that
knowledge stops and a capture of your own machine is needed. The part you fill
in yourself (the encryption) is in the Encryption chapter.

Source: static analysis of Design Space 10.12.122. Tag per claim: **(code)** =
seen in the binaries/schemas, **(inferred)** = reasoning, **(unknown)** = not
yet established.

> Status: Chain, Transport, Machine code names, Command model, Encryption and
> Capture are filled in from the static analysis. The byte layout of the
> commands and the encryption itself come from a capture (see chapter 6).

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

## 5. Encryption (you fill this in)

This chapter is deliberately not worked out here. The encryption of the
connection to the machine is both the blocking point for automated analysis and
the legally sensitive point (see `docs/legal.md`). This repository contains no
keys and no cryptography code taken from Design Space.

What we know in outline, purely as a frame (**inferred from public sources,
unconfirmed**):

- The connection has an **open phase** (reading status, registers, starting the
  device) and then an **encrypted phase** for motion and cutting.
- The transition is a **key exchange**: the software sends a frame, the machine
  replies, and from that follows a session key that differs per connection.
- Public projects use key material that sits inside Cricut's software for this.
  Copying that material is exactly the line. For machines you own, the clean way
  is to **capture the handshake of your own machine** and implement the
  algorithm yourself from what you see in that capture, without copying Cricut's
  key tables.

In the code this attaches to `cricut-bridge`:

- `FrameCodec`: the framing (length header, byte order, checksum) around each
  message.
- `SessionCipher`: the encryption layer. `NullCipher` already exists for the
  open phase and for replay tests on a capture. Your implementation goes behind
  it, with key material that stays **outside the repo** (see `.gitignore`:
  `*.key`, `/private/`).

The building blocks you will likely need (AES, and depending on the capture a
key exchange) are ordinary Rust crates (`aes`, `cipher`, and for a possible
exchange `x25519-dalek` or whatever the capture shows). That choice depends on
what you see in the handshake of your own machine.

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
