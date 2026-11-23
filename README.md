# CricutControl

Snelle, eigen aansturing van Cricut-machines (Maker en Joy) als alternatief
voor Cricut Design Space. Bedoeld voor interoperabiliteit met hardware die je
zelf bezit. Zie `docs/juridisch.md` voor de reikwijdte.

## Status

Vroeg. Skelet plus protocoldocumentatie. De machinelaag is nog niet af: het
versleutelde deel van de handshake naar de machine wordt met de hand ingevuld
uit een opname van een eigen machine (zie `docs/protocol.md`, hoofdstuk
Versleuteling). Er staan bewust geen sleutels of uit Design Space overgenomen
code in deze repo.

## Opbouw

Design Space is een Electron-app die praat met losse hulpprocessen
(`CricutDevice`, `CricutDeviceIO`, `CricutVulcanDevice`). Die helper opent het
apparaat (USB of Bluetooth) en stuurt er bytes doorheen. Onze software neemt de
rol van die helper over en praat rechtstreeks met de machine.

```
cricut-proto      protobuf-berichten (bridge + device-sessie); de rijke
                  NativeModel-berichten staan als referentie in proto/reference/
cricut-transport  USB/Bluetooth: apparaat vinden, openen, bytes lezen/schrijven
cricut-bridge     sessie- en commandologica bovenop het transport
cricut-cli        commandoregel: apparaten lijsten, status, een testsnede
```

De protobuf-schema's in `proto/reference/` zijn statisch uit de hulpprocessen
van Design Space 10.12.122 gehaald, puur als beschrijving van het berichtformaat
(interoperabiliteit). Ze worden niet uitgevoerd.

## Bouwen

Rust (stable):

```
cargo build
```

## Bronnen

- `docs/protocol.md`  hoe de machines aangestuurd worden (het naslagdocument)
- `docs/architectuur.md`  hoe Design Space is opgebouwd en wat wij overnemen
- `docs/juridisch.md`  waarom dit privé blijft
