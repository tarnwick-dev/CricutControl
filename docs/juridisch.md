# Juridisch: waarom dit privé blijft

Korte, eerlijke inschatting, geen juridisch advies.

## Wat we doen

We sturen machines aan die we zelf bezitten, met eigen software, voor eigen
gebruik. We lezen het berichtformaat van Design Space uit om te weten hoe de
machines praten (interoperabiliteit). We voeren Design Space niet uit om mee te
sturen, en we nemen geen code van Cricut over in onze repo.

## Waar het gevoelig wordt

1. **Cricut verbiedt reverse engineering in de EULA.** Dat is een contractuele
   bepaling. In de EU staat daar dwingend recht tegenover: decompileren voor
   interoperabiliteit mag onder voorwaarden (Softwarerichtlijn 2009/24/EG,
   art. 6, en de uitzondering voor het bestuderen van de werking in art. 5).
   Die uitzondering is er voor precies dit: een eigen programma laten
   samenwerken met bestaande hardware. Dat recht geldt sterker voor eigen
   gebruik dan voor verspreiding.

2. **De versleuteling omzeilen.** De rechtszaken van Provo Craft in 2010-2011
   (Make The Cut, Sure Cuts A Lot) draaiden om het overnemen van code en om het
   omzeilen van de USB-versleuteling. De projecten die nu werken, zetten de
   sleutels van Cricut letterlijk in hun broncode; hun eigen NOTICE waarschuwt
   voor de juridische blootstelling. Dat is het punt waar een technische
   maatregel wordt omzeild, en dat is zowel juridisch als qua filter de rode
   lijn.

3. **Publiceren.** Zodra je dit open source zet, verschuift het van "eigen
   gebruik voor interoperabiliteit" naar verspreiding van een middel om een
   beveiliging te omzeilen. Dat is een heel ander risico, en dat is waar Cricut
   historisch achteraan ging.

## Reikwijdte

Deze repo bevat een skelet en een beschrijving van het protocol voor
interoperabiliteit met eigen hardware. Harde grenzen, die blijven staan:

- **Geen sleutels en geen uit Design Space overgenomen code** in de repo. Het
  versleutelde deel van de handshake wordt met de hand ingevuld uit een opname
  van een eigen machine, niet door sleutelmateriaal uit Cricuts software te
  kopiëren. Dat is het verschil tussen de werking bestuderen (toegestaan voor
  interoperabiliteit) en een technische maatregel omzeilen met overgenomen
  sleutels (de rode lijn uit de zaken van 2010-2011).
- Deze repo staat op naam van een los pseudoniem, niet gekoppeld aan een
  persoon of bedrijf.

Gebruik dit alleen voor machines die je zelf bezit.
