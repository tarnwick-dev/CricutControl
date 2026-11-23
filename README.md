# CricutControl

Fast, independent control software for Cricut cutting machines (Maker and Joy)
as an alternative to Cricut Design Space. Built for interoperability with
hardware you own. See `docs/legal.md` for scope.

## Status

Early. Skeleton plus protocol documentation. The machine layer is not finished:
the encrypted part of the handshake to the machine is filled in by hand from a
capture of your own machine (see `docs/protocol.md`, chapter Encryption). This
repository deliberately contains no keys and no code taken from Design Space.

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
