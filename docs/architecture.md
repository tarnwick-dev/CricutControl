# Architecture: how Design Space works and what we take over

Based on static analysis of Cricut Design Space 10.12.122 (Windows, released
2026-11-16).

## Design Space in layers

```
Electron app (JavaScript, app.asar, 441 MB)
        │  protobuf messages over the helper's stdin/stdout or a named pipe
        ▼
Helper process  (separate .exe per generation)
   device-common/CricutDevice.exe        v6.12.13   (x86, old generation)
   device-common-next/CricutDevice.exe   v7.0.7     (x86, current generation)
   cricut-device-io/CricutDeviceIO.exe   3.18.0-a3  (x64, new I/O layer)
   plugin-vulcan/CricutVulcanDevice.exe  (x86, separate family: heat press)
        │  USB or Bluetooth
        ▼
Machine (Maker, Joy, ...)
```

There are no drivers and no firmware files in the installer. Firmware is fetched
at runtime (not investigated).

## The two protobuf layers

The analysis found two levels of protobuf messages (the schemas are in
`proto/reference/`):

1. **Transport / control** (`bridge/proto/*`, `bridge/proto/device/*`). The
   envelope message is `bridge.proto.Message`. The device side,
   `bridge.proto.device.Message`, has:
   - **Registry**: add, remove and list devices (`RegistryAdd`,
     `RegistryGetAll`, ...). This is how the app knows which machines exist.
   - **Session**: open and close a connection to one device (`SessionOpen`,
     `SessionClose`), keyed by a `DeviceHandle`.
   - **SessionDataStream**: `SessionDataStreamRequest { handle, bytes data }`.
     This is the heart: raw bytes pass through the helper to the machine.

2. **Machine model** (`NativeModel.*`). The rich, structured commands:
   - `NativeModel.Commands`: `PBDrawVectors` / `PBDrawVector` (motion),
     `PBToolSetting`, `PBSetMachineState`, `PBSpeedValues`, `PBGetFullStatus`,
     `PBReadEdgeSensorCommand`, `PBSerialNumber`, `PBActivateDeactivateCommand`
     (the "machine locked" check), `PBRawCommandRequestResponse`, and more.
   - `NativeModel.Machines`: all machine types (`PBMachineType`), tools
     (`PBToolType`), heads, material settings.
   - `NativeModel.Force`, `NativeModel.Backlash`: force/pressure and backlash.

## What this means for us

- In the **new design** (device-common-next, cricut-device-io) the app builds
  the machine bytes itself and sends them as a raw `SessionDataStream`. The
  helper mainly finds and opens the device and passes bytes through. **That is
  exactly the layer we take over in `cricut-transport`.** The actual machine
  commands (vectors, tool, pressure) we build in `cricut-bridge`.
- In the **old design** (device-common v6) the app sent the structured
  `NativeModel` commands and the helper converted them to the wire. Those
  schemas remain the best description of *what* is being controlled.
- Observed vs inferred: that the data stream carries raw machine bytes is
  inferred from the schema (`bytes data`). Exactly where the framing and the
  encryption happen (in the app, in the helper, or in a shared engine) needs a
  real capture to confirm.

## Our layout mirrors this

| Design Space            | CricutControl        |
|-------------------------|----------------------|
| find/open a device      | `cricut-transport`   |
| session + byte stream   | `cricut-bridge`      |
| machine commands        | `cricut-bridge` (+ `cricut-proto` for the NativeModel messages, later) |
| the Electron UI         | `cricut-cli` (for now), later possibly a UI of our own |
