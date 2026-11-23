# Protocol: hoe we de machines aansturen

Dit is het naslagdocument. Het beschrijft hoe een Cricut-machine bestuurd wordt,
voor zover we dat uit de statische analyse weten, en het markeert precies waar
de kennis ophoudt en een opname van onze eigen machine nodig is. Het deel dat
jij zelf invult (de versleuteling) staat in het hoofdstuk Versleuteling.

Bron: statische analyse van Design Space 10.12.122, in `~/work/cricut-re/`.
Markering per bewering: **(code)** = gezien in de binaries/schema's,
**(afgeleid)** = redenering, **(onbekend)** = nog niet vastgesteld.

> Status van dit document: de hoofdstukken Keten, Commandomodel, Versleuteling
> en Opname staan. De hoofdstukken Transport en Machinecodenamen worden
> ingevuld zodra de transportanalyse klaar is (`~/work/cricut-re/notes/
> native-transport.md`).

---

## 1. De keten

De machine hangt aan de computer via USB of Bluetooth. Design Space praat niet
rechtstreeks met de machine, maar via een hulpproces. Wij nemen dat hulpproces
over.

```
onze software ── USB/Bluetooth ── machine
```

Twee generaties in Design Space (**code**, zie `docs/architectuur.md`):

- **v6 (device-common):** de app stuurt gestructureerde commando's
  (`NativeModel.*`), de helper zet ze om naar de lijn.
- **v7 (device-common-next) + cricut-device-io:** de app bouwt de machine-bytes
  zelf en stuurt ze als ruwe stroom (`SessionDataStreamRequest { handle, bytes
  data }`). De helper vindt en opent het apparaat en geeft bytes door.

Voor ons is de v7-opzet het model: wij bouwen de bytes, het transport geeft ze
door. De `NativeModel`-schema's blijven de beste beschrijving van wát er
bestuurd wordt.

---

## 2. Transport (USB en Bluetooth)

> Wordt ingevuld uit `native-transport.md`. Hier komen per machine:
> USB-klasse (CDC-serieel / vendor-WinUSB / HID), VID en PID, endpoints en
> pakketgroottes; Bluetooth classic (RFCOMM-kanaal) of BLE (service- en
> characteristic-UUID's); en welke helper welk apparaat opent.

Wat publieke bronnen (onbevestigd) zeggen, als vertrekpunt om te toetsen:

- **Maker (v1, "Athena")**: USB CDC-serieel, claim VID `0x20D3` PID `0x0015`,
  bulk OUT `0x02` / IN `0x82`; Bluetooth classic SPP/RFCOMM. **(onbekend tot
  getoetst)**
- **Joy, Joy Xtra, Maker 3**: niets publiek bekend. BLE waarschijnlijk, maar
  UUID's, VID/PID en crypto onbekend. Dit is waar onze analyse nieuw terrein is.

---

## 3. Machinecodenamen

Design Space gebruikt interne codenamen (`PBMachineType` in
`NativeModel.Machines.proto`) **(code)**. Bevestigd: `ATHENA_MT = 11` = Maker
(v1). De overige codenamen voor onze machines (Joy, Joy Xtra, Maker 3) worden
uit de app-strings gekoppeld voordat ze hier als waarheid komen te staan.
Kandidaten in de enum, nog **niet** hard gekoppeld: `CUPID`, `SPRITE`, `SCOUT`,
`RANGER`, `MORPHEUS`, `NEWT`, `RIDDLE`, `GANDALF`. **(onbekend tot getoetst)**

Volledige enum: `proto/reference/v7-device-common-next/
bridge__proto__NativeModel.Machines.proto`.

---

## 4. Commandomodel

Uit `NativeModel.Commands.proto` en `NativeModel.Machines.proto` **(code)**. Dit
is de woordenschat van wat je een machine laat doen. De exacte byte-codering op
de lijn (opcodes, volgorde) komt uit de transportanalyse en een opname; hier
staat de betekenis.

**Beweging**
- `PBDrawVectors` / `PBDrawVector`: de eigenlijke snede/teken-beweging, als
  reeks vectoren. `PBDrawVectorType` geeft het type lijnstuk. De coördinaten en
  eenheid (publieke claim: float32 X/Y in inches) toetsen we op een opname.
- `PBSpeedValues`, `PBSpeedGroup`: snelheid.

**Gereedschap en druk** (dit verklaart "het mes beweegt maar snijdt niet":
tool-neer en druk zijn aparte commando's naast de beweging)
- `PBToolType` (**code**, volledige lijst): `ROLLING_BLADE`, `DRAG_KNIFE`,
  `TANGENTIAL_BLADE`, `PEN_DT`, `STYLUS`, `SCORING_WHEEL`, `PERFORATION_WHEEL`,
  `DEBOSSING_TOOL`, `WAVE_TOOL`, `ENGRAVE_TOOL`, `MIDAS_*`, `DEEP_CUT`,
  `AUTO_CUTOFF_TOOL`, `SCORE_TIP_TOOL`, `ENGRAVE_STATIC_TOOL`.
- `PBToolSetting`, `PBToolSettings`, `PBToolTypeState`.
- Kracht/druk: `NativeModel.Force.proto`. Speling: `NativeModel.Backlash.proto`.
- Kop: `PBHeadType` (`CLAMP_A`, `CLAMP_B`), `PBAvailableHeadType`.

**Machinestatus en besturing**
- `PBSetMachineState`, `PBMachineInterface`, `PBMachineInterfaceState`.
- `PBGetFullStatus`, `PBMachineInformationBridge`, `PBSerialNumber`.
- `PBReadEdgeSensorCommand` / `PBSensorResult` (`PBSensorType`): mat- en
  materiaalsensoren.
- `PBMatloadOptions`: mat laden/lossen.
- `PBSimulateButton` (`PBDeviceButton`), `PBDeviceLEDState`: knoppen en lampjes.
- `PBAck`: bevestiging per commando (**afgeleid**: elk commando wordt geACKt).
- `PBActivateDeactivateCommand`: dit is de "machine locked/geactiveerd"-controle.
  **(afgeleid)** Dit is waarschijnlijk een entitlement-check van de app, geen
  voorwaarde voor beweging. Toetsen op een opname.
- `PBRawCommandRequestResponse`: ruwe passthrough, handig voor experimenteren.

**Materiaal**
- `PBAllMaterialSettings`, `PBMaterialSettingsApi`, `PBMachineMaterial`,
  `PBMaterialTag(s)`, `PBMatSize`, `PBMaterialInset`, `PBMachineMode`.
- **(afgeleid, publieke claim)** materiaalinstellingen (druk, snelheid, aantal
  passes per materiaal) komen uit een online bron van Cricut en worden gecachet
  in het profiel. Voor ons: we kunnen eigen materiaalprofielen maken; we hebben
  Cricuts tabel niet nodig om te bewegen. Toetsen wat de machine echt als input
  eist.

---

## 5. Versleuteling (dit vul jij in)

Dit hoofdstuk is bewust niet door mij uitgewerkt. De versleuteling van de
verbinding met de machine is zowel het filterpunt (het cyberfilter blokkeert het
decompileren ervan) als het juridisch gevoelige punt (zie `docs/juridisch.md`).
Deze repo bevat geen sleutels en geen uit Design Space overgenomen cryptocode.

Wat we er in grote lijnen van weten, puur als kader (**afgeleid uit publieke
bronnen, onbevestigd**):

- De verbinding kent een **open fase** (status lezen, registers, apparaat
  opstarten) en daarna een **versleutelde fase** voor beweging en snijden.
- De overgang is een **sleuteluitwisseling**: de software stuurt een frame, de
  machine antwoordt, en daaruit volgt een sessiesleutel die per verbinding
  verschilt.
- Publieke projecten gebruiken hiervoor sleutelmateriaal dat in Cricuts software
  zit. Dat materiaal overnemen is precies de rode lijn. Voor eigen machines is
  de nette weg: de **handshake van je eigen machine opnemen** en het algoritme
  zelf implementeren op basis van wat je in die opname ziet, zonder Cricuts
  sleuteltabellen te kopiëren.

In de code sluit dit aan op `cricut-bridge`:

- `FrameCodec`: de framing (lengte-header, byte-orde, checksum) rond elk bericht.
- `SessionCipher`: de versleutellaag. `NullCipher` is er al voor de open fase en
  voor replay-tests op een opname. Jouw implementatie komt hierachter, met
  sleutelmateriaal dat **buiten de repo** blijft (zie `.gitignore`: `*.key`,
  `/private/`).

De bouwstenen die je waarschijnlijk nodig hebt (AES, en afhankelijk van de
opname een sleuteluitwisseling) zijn gewone Rust-crates (`aes`, `cipher`, en
voor een eventuele uitwisseling `x25519-dalek` of wat de opname laat zien). Die
keuze hangt af van wat je in de handshake van onze eigen machine ziet.

---

## 6. Wat een opname van onze eigen machine nog moet opleveren

Statische analyse zegt niet wat er echt over de lijn gaat. Met een machine aan
een computer waar je bij kunt (een pc met Design Space, verkeer opnemen met
USBPcap of een BLE-sniffer) willen we vastleggen:

1. **Transport bevestigen**: welke USB-klasse of welk BLE-profiel, VID/PID of
   UUID's, endpoints. Per machine (Maker en Joy verschillen vrijwel zeker).
2. **De open fase**: de exacte bytes van opstart, status, registers, tot aan de
   sleuteluitwisseling. Dit kunnen we direct namaken.
3. **De sleuteluitwisseling**: de frames van de handshake, zodat jij het
   algoritme kunt implementeren zonder Cricuts sleutels te kopiëren.
4. **Een echte snede**: de volgorde van commando's tussen "Make" en klaar
   (beweging, tool-neer, druk, mat laden, voortgang, pauze/stop), en de
   bytecodering van `PBDrawVector` (eenheid, stapgrootte, lijn versus curve).
5. **Of de cloud nodig is op snijmoment**: meekijken of er tussen "Make" en de
   beweging een serververzoek zit dat de machine echt nodig heeft, of dat het
   alleen telemetrie/entitlement is.

Punt 3 en 4 zijn de kern. Zodra die er zijn, kan `cricut-bridge` een echte
snede sturen.
