# CricutControl

Fast, independent control software for Cricut cutting machines (Maker and Joy)
as an alternative to Cricut Design Space. Built for interoperability with
hardware you own. See `docs/legal.md` for scope.

## Status

The stack is in place and builds: the SPP frame codec, a Linux RFCOMM transport
(classic Bluetooth, channel 4), the session (open phase, key-exchange hook,
encrypted phase) and the command opcodes. The one slot left is the session
cipher — see `DEVELOPER.md`. Everything works today with `NullCipher` for the
plaintext open phase.

This repository deliberately contains no keys, no unwrap seed, no materials
string, no captures, and no code taken from Design Space. Key material is loaded
at runtime from outside version control.

## Layout

Design Space is an Electron app that talks to separate helper processes
(`CricutDevice`, `CricutDeviceIO`, `CricutVulcanDevice`). Such a helper opens
the device (USB or Bluetooth) and streams bytes to it. This software takes over
the role of that helper and talks to the machine directly.

```
cricut-proto      protobuf messages (bridge + device session); the rich
                  NativeModel messages are kept as reference in proto/reference/
cricut-transport  USB/Bluetooth: find a device, open it, read/write bytes
cricut-bridge     session and command logic on top of the transport
cricut-cli        command line: list devices, status, a test cut
```

The protobuf schemas in `proto/reference/` were recovered statically from the
Design Space 10.12.122 helper processes, purely as a description of the message
format (interoperability). They are not executed.

## Build

Rust (stable):

```
cargo build
```

## Documentation

- `docs/protocol.md`      how the machines are driven (the reference document)
- `docs/architecture.md`  how Design Space is built and what we take over
- `docs/legal.md`         scope and limits
