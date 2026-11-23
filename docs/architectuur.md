# Architectuur: hoe Design Space werkt en wat wij overnemen

Gebaseerd op statische analyse van Cricut Design Space 10.12.122 (Windows,
uitgebracht 16-11-2026). Analysebestanden: `~/work/cricut-re/`.

## Design Space in lagen

```
Electron-app (JavaScript, app.asar, 441 MB)
        │  protobuf-berichten over stdin/stdout van het hulpproces
        ▼
Hulpproces  (los .exe per generatie)
   device-common/CricutDevice.exe        v6.12.13   (x86, oude generatie)
   device-common-next/CricutDevice.exe   v7.0.7     (x86, huidige generatie)
   cricut-device-io/CricutDeviceIO.exe   3.18.0-a3  (x64, nieuwe I/O-laag)
   plugin-vulcan/CricutVulcanDevice.exe  (x86, aparte machinefamilie: warmtepers)
        │  USB of Bluetooth
        ▼
Machine (Maker, Joy, ...)
```

Er zitten geen drivers en geen firmwarebestanden in het installatiepakket.
Firmware wordt op het moment zelf opgehaald (niet onderzocht).

## De twee protobuf-lagen

De analyse vond twee niveaus van protobuf-berichten (de schema's staan in
`proto/reference/`):

1. **Transport / besturing** (`bridge/proto/*`, `bridge/proto/device/*`). Het
   omhullende bericht is `bridge.proto.Message`. De apparaatkant,
   `bridge.proto.device.Message`, heeft:
   - **Registry**: apparaten toevoegen, verwijderen, opvragen (`RegistryAdd`,
     `RegistryGetAll`, ...). Zo weet de app welke machines er zijn.
   - **Session**: een verbinding met één apparaat openen en sluiten
     (`SessionOpen`, `SessionClose`), met een `DeviceHandle` als sleutel.
   - **SessionDataStream**: `SessionDataStreamRequest { handle, bytes data }`.
     Dit is het hart: ruwe bytes gaan door de helper heen naar de machine.

2. **Machinemodel** (`NativeModel.*`). De rijke, gestructureerde commando's:
   - `NativeModel.Commands`: `PBDrawVectors` / `PBDrawVector` (de beweging),
     `PBToolSetting`, `PBSetMachineState`, `PBSpeedValues`, `PBGetFullStatus`,
     `PBReadEdgeSensorCommand`, `PBSerialNumber`, `PBActivateDeactivateCommand`
     (de "machine locked"-controle), `PBRawCommandRequestResponse`, en meer.
   - `NativeModel.Machines`: alle machinetypes (`PBMachineType`), gereedschappen
     (`PBToolType`), koppen, materiaalinstellingen.
   - `NativeModel.Force`, `NativeModel.Backlash`: kracht/druk en speling.

## Wat dit betekent voor ons

- In de **nieuwe opzet** (device-common-next, cricut-device-io) bouwt de app de
  machine-bytes zelf en stuurt ze als ruwe `SessionDataStream`. De helper is dan
  vooral: apparaten vinden, openen, bytes door-geven. **Dat is precies de laag
  die wij overnemen in `cricut-transport`.** De eigenlijke machinecommando's
  (vectoren, tool, druk) bouwen wij in `cricut-bridge`.
- In de **oude opzet** (device-common v6) stuurde de app de gestructureerde
  `NativeModel`-commando's en zette de helper die om naar de lijn. Die schema's
  blijven de beste beschrijving van wát er bestuurd wordt.
- Observed vs inferred: dat de datastream ruwe machine-bytes draagt, is afgeleid
  uit het schema (`bytes data`). Waar precies de framing en de versleuteling
  gebeuren (in de app, in de helper, of in een gedeelde engine), moet de
  transportanalyse en uiteindelijk een echte opname bevestigen.

## Onze indeling spiegelt dit

| Design Space            | CricutControl        |
|-------------------------|----------------------|
| apparaat vinden/openen  | `cricut-transport`   |
| sessie + bytes-stroom   | `cricut-bridge`      |
| machinecommando's       | `cricut-bridge` (+ `cricut-proto` voor de NativeModel-berichten, later) |
| de Electron-UI          | `cricut-cli` (voorlopig), later evt. een eigen UI |
