# C++-Ground-Truth: sieben Fixtures, Schritte 2.2–2.4

## Optionale Originalarchive lokal bereitstellen

Das öffentliche Repository, Wheel und sdist enthalten **keinen fremden
C++-Quellcode**. Rust-/Python-Tests und die installierte Bibliothek benötigen
die Archive nicht; alle verifizierten Eingaben und JSON-Referenzen sind enthalten.
Die hier veröffentlichten C++-Probes sind unsere eigenen Adapter.

Für eine erneute native Oracle-Extraktion werden die Originale separat benötigt:

- SUSY: **orbifolder v1.2.1**, vom
  [HepForge-Projekt](https://orbifolder.hepforge.org/), Downloadbereich
  [HepForge downloads](https://www.hepforge.org/downloads/orbifolder/).
  Das Archiv so entpacken, dass `legacy_cpp/susy_orbifolder/configure` und
  `legacy_cpp/susy_orbifolder/src/orbifolder/` existieren.
- non-SUSY: [StringsIFUNAM/nonSUSYorbifolder](https://github.com/StringsIFUNAM/nonSUSYorbifolder).
  Ein Checkout/Archiv gehört nach `legacy_cpp/non_susy_orbifolder/`;
  dort müssen `configure`, `src/`, `Geometry/` und `Models/` liegen.

```bash
mkdir -p legacy_cpp
git clone https://github.com/StringsIFUNAM/nonSUSYorbifolder.git legacy_cpp/non_susy_orbifolder
# SUSY-v1.2.1-Archiv nach legacy_cpp/susy_orbifolder entpacken.
bash tests/ground_truth/build_legacy.sh susy
bash tests/ground_truth/build_legacy.sh non_susy
python tests/ground_truth/generate.py --build-probes --output tests/ground_truth/.build/review
```

`/legacy_cpp/` und `.build/` werden ignoriert. Für bytegleiche Provenienz müssen
Archivinhalt und Dateihashes mit den eingefrorenen Manifesten übereinstimmen;
ein aktueller Git-Checkout kann abweichen. Nicht passende Hashes rechtfertigen
kein automatisches Überschreiben der sieben freigegebenen Kern-Fixtures.
Systemabhängigkeiten und isolierter Build: [BUILD.md](./BUILD.md).
Lizenz, Papers und korrekte non-SUSY-arXiv-ID **2504.20137**:
[THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md).

[geometry_inputs.json](./geometry_inputs.json) enthält ausschließlich die sechs
hashgeprüften geometrischen Texteingaben. So bleiben die ursprünglichen
Geometrieparser-Tests unabhängig von lokalen Archiven und prüfen weiterhin
dieselben Eingabehashes. Die Wheel-Presets enthalten die daraus bereits in
Phase 3.4 exportierten exakten strukturierten Eingaben und Auswahlregeln;
keine erwarteten Spektren werden in die Berechnung eingeschleust.
[bundle_inputs.py](./bundle_inputs.py) erzeugt diese Eingabebündel reproduzierbar
aus den separat bereitgestellten Archiven und den gespeicherten Modelleingaben.

Die beiden Probes laden und berechnen sieben konkrete Modelle mit den unveränderten
C++-Klassen. [oracle.cpp](./probes/oracle.cpp) wird zweimal kompiliert und gegen
die bestehenden 31 SUSY- beziehungsweise 28 non-SUSY-Objektdateien und die drei
Bibliotheken `io`, `linalg`, `groupTheory` gelinkt. `main.o` wird ausgeschlossen.
Build-Voraussetzungen und technische Build-Anpassungen: [BUILD.md](./BUILD.md).

## Reproduzieren

Python >= 3.10 mit den Abhängigkeiten aus [requirements.txt](./requirements.txt).
Aus dem Repository-Hauptverzeichnis, bei vorhandenen Legacy-Builds:

```bash
python3 tests/ground_truth/generate.py --build-probes
python3 tests/ground_truth/compare.py --self-test
```

[generate.py](./generate.py) funktioniert auch mit Windows-Python in einem
WSL-UNC-Workspace und führt die C++-Programme dann über `wsl.exe` aus.
`LEGACY_BUILD_ROOT` beziehungsweise `--build-root` wählen vorhandene isolierte
Builds; `--case susy_z3` beschränkt den Lauf auf einen Fall. Ein neuer Probe-Build
ist nach Änderungen an Probe oder Link-Skript erforderlich.

Vorhandene [Referenzen](./fixtures/) werden regulär nur geprüft. Änderungen an
Referenzen erfordern ein ausdrückliches `--overwrite`; zum Review empfiehlt sich
zunächst eine Ausgabe in ein separates Arbeitsverzeichnis:

```bash
python3 tests/ground_truth/generate.py --output tests/ground_truth/.build/review
python3 tests/ground_truth/compare.py tests/ground_truth/fixtures/susy_z3.json tests/ground_truth/.build/review/susy_z3.json
```

Der strikte Vergleich schließt die Provenienz ein. Ein anderer Build oder eine
geänderte Suite kann daher auch bei gleichen Physikdaten eine Differenz liefern.
Der Comparator implementiert noch keine Basisumrechnung und keinen Vergleich
exakter Rust-Ergebnisse mit gerundeten C++-Doubles.
Der Rust-Integrationsadapter führt diese exakte Basisabbildung durch; siehe
[Zustands-/Spektrumbericht](../../docs/06_states_and_spectrum_verification.md).

## Gemessene Referenzen

| Fixture | Sektoren | Felder | Rechte Tachyonkandidaten | Projizierte Tachyonfelder | Erlaubte kubische Sample-Kopplungen |
| --- | ---: | ---: | ---: | ---: | ---: |
| [susy_z3](./fixtures/susy_z3.json) | 3 | 246 | 0 | 0 | 15 |
| [susy_z6_ii](./fixtures/susy_z6_ii.json) | 6 | 544 | 0 | 0 | 1 |
| [non_susy_z3](./fixtures/non_susy_z3.json) | 6 | 438 | 1 | 0 | nicht anwendbar |
| [non_susy_z6_ii](./fixtures/non_susy_z6_ii.json) | 12 | 832 | 3 | 0 | nicht anwendbar |
| [susy_z3xz3_wl](./fixtures/susy_z3xz3_wl.json) | 9 | 524 | 0 | 0 | 0 |
| [non_susy_z3xz3_wl](./fixtures/non_susy_z3xz3_wl.json) | 18 | 1112 | 7 | 0 | nicht anwendbar |
| [non_susy_tachyonic](./fixtures/non_susy_tachyonic.json) | 12 | 616 | 3 | 12 | nicht anwendbar |

Alle sieben originalen `CheckAnomaly`-Aufrufe liefern `true`. Die Feldzahlen zählen
die tatsächlichen `SConfig::Fields` einschließlich der nativen CPT-/Multiplet-
Konvention; sie sind keine Summe der Darstellungsdimensionen. Rechte
Tachyonkandidaten zählen Gewichte vor den abschließenden Projektionen.
Beim positiven Fall enthält `StandardConfig` 604 masselose und zwölf angehängte
Tachyonfelder. Quelle, deterministische Fehlklassifikation und exakte
Legacy-Fundstellen: [TACHYON_AUDIT.md](./TACHYON_AUDIT.md).

Die [Eingaben](./cases/) sind vollständig gespeichert und werden bei der
Generierung nicht überschrieben:

- SUSY Z3/Z6-II: Standard-Einbettung, sechs verschwindende Wilsonlinien.
- non-SUSY Z3/Z6-II: archivierte Standardmodelle einschließlich Brechungsshift.
- SUSY Z3×Z3: erstes vollständiges Modell `MSSM0` aus dem archivierten
  [77-Modell-Katalog](../../legacy_cpp/susy_orbifolder/Z3xZ3_1_1-1WL-77PatricksModels.txt).
- non-SUSY Z3×Z3: archiviertes Modell mit `WL1=WL2`, erste acht Komponenten null,
  zweite acht Komponenten `(2/3,-1/3,-1/3,0,0,0,0,0)`; übrige Wilsonlinien null.
  Die originale C++-Validierung bestätigt diese Einbettung.
- Positiver non-SUSY-Tachyonfall: dieselbe Z6-II-Geometrie, gültige Variation
  ausschließlich des geometrischen Einbettungsshifts; zwölf projizierte
  Grundzustandstachyonfelder. Die vollständige Eingabe ist gespeichert.

## Zusatzreferenz für Phase 3.3a

[states_probe.cpp](./probes/states_probe.cpp) ruft ausschließlich native
Sektorklassen auf und liest `Excited` niemals aus. Der Fall
`v0=(0,1,1,1)`, `v1=(0,1/6,-1/6,0)`, Sektor `(1,1,0)`, enthält einen rechten
Grundzustandstachyon mit `mu=-1/3` und zwei rechte Anregungen mit `mu=-1/6`.
Mit `v0[1]=25` verliert das native feste Suchfenster alle drei Lösungen.
Die vollständigen Rust-Normschranken reproduzieren auch nach diesem Lift
die ursprünglichen Gewichte und Massen. Dies sind lokale Kinematiktests,
keine zusätzlichen vollständigen Modell-Fixtures und kein Nachweis ihrer
abschließenden Zentralisatorprojektion.

[states_oracle.json](./states_oracle.json) enthält beide nativen Ergebnisse
mit zwei unabhängigen Prozessen und vollständigen Link-/Quellhashes:

```bash
bash tests/ground_truth/build_states_probe.sh
python3 tests/ground_truth/generate_states.py
```

Die sieben freigegebenen Modellreferenzen bleiben eingefroren. Der bestätigte
non-SUSY-Wilsonfehler und die verbindliche Nutzerentscheidung sind separat im
[Wilson-Audit](./WILSON_INVARIANCE_AUDIT.md) dokumentiert.

## Daten und Herkunft

Für Phase 3.3b ergänzt [anomalies_oracle.json](./anomalies_oracle.json) die
nativen quadratischen/kubischen Darstellungs-Indizes und 90 `CheckAnomaly`-
Aufrufe für lineare U(1)-Polarisationsproben. Die Spuren stammen aus den
Originalmethoden, nicht aus eigenen C++-Anomaliesummen. Je zwei Prozesse,
Link-/Quellhashes und gebundene Modell-Fixture-Hashes sichern die Herkunft.

```bash
bash tests/ground_truth/build_anomalies_probes.sh
python3 tests/ground_truth/generate_anomalies.py
```

Einzelne chirale Feldentfernungen weisen falsche native Erfolgsstatuswerte
nach: [Anomalie-Audit](./ANOMALY_VALIDATION_AUDIT.md).
Der [Verifikationsbericht](../../docs/07_anomalies_verification.md) trennt die
positiven Oracle-Gleichheiten von diesen negativen Legacy-Konflikten.

Der Vertrag ist [schema.json](./schema.json), Version 1. Native rationale Zahlen
stehen als reduzierte JSON-Integer-Paare `{"num": ..., "den": ...}`; Doubles
werden mit `max_digits10` exportiert. Der Export rationalisiert keine Doubles.
JSON-NaN, Infinity, doppelte Schlüssel und nichtfinite Zahlen sind unzulässig.
Die Fixtures speichern die native Reihenfolge und vollständige Gewichtslisten.

| Abschnitt | Tatsächlich erfasste Daten |
| --- | --- |
| `math` | Native rationale Grundoperationen, 480 E8×E8-Elternwurzeln, drei SO(8)-Klassifikationsbeispiele. |
| `geometry` | Torusbasis, Twists, Faktorordnungen, Generatoren, Konstruktionsklassen, Zentralisatoren, WL-Relationen, diskrete Symmetrieoperatoren und rationale Rotationsaktionen. |
| `orbifold_group` | Shifts/Wilsonlinien, lokale Elemente und Zentralisatoren, Torsionsparameter, Superladungen und Prüfstatus. |
| `gauge_group` | Faktoren, einfache Wurzeln, U(1)-Generatoren/Basis, FI-Information. |
| `sectors` | Öffentliche masselose Lösungen, Oszillatoren, Halbzustände, Fixbranen und projizierte Zustände; non-SUSY zusätzlich rechte Tachyonlösungen samt Massen und beide Tachyon-Zustandscontainer. |
| `spectrum`, `tachyonic_spectrum` | Native Felder, Darstellungen/Höchstgewichte, U(1)- und diskrete Ladungen, Lokalisation und überprüfbare Verweise auf Gewichte/Zustände. |
| `anomalies` | Originaler Gesamtstatus, vollständiger Originalbericht, daraus gelesene native rationale/integer Spuren sowie Eichdaten nach dem Aufruf; Eichdaten vor dem Aufruf stehen separat. |
| `susy_analysis` | Echter, begrenzter Test von `CMonomial` und `CYukawaCouplings::AddCoupling`; non-SUSY explizit `not_applicable`. |

`CheckAnomaly` schreibt teilweise nach `cout`, teilweise nach `CPrint.out`; beide
Kanäle werden gemeinsam erfasst. Die Koeffizienten werden aus den von der
Originalrechnung ausgegebenen Integer-/Bruchwerten gelesen, nicht aus eigenen
Python-Anomaliesummen. Die native Anomalieroutine selbst wandelt zuvor
Double-Ladungen mit `D2RatHugeInt` um; diese Herkunft macht die Koeffizienten
nicht zu unabhängig exakt hergeleiteten Eingangsdaten. Nicht ausgegebene
Koeffiziententypen sind durch diesen Oracle nicht abgedeckt.

Provenienz enthält Modell-/Geometrie-, Archivmanifest-, C++-Quellmanifest-,
Binary-, Probe-/Runner-/Comparator-/Schema-Hashes, die vollständige Menge der
gelinkten Objekt-/Bibliotheksdateien sowie Compiler-/Paket-/Python-Versionen.
Die Erzeugung prüft die archivierten Dateien, C++-Buildkopien und Laufzeittabellen
gegen die gespeicherten Manifeste. Der Prozess verwendet `LC_ALL=C`, `TZ=UTC`,
Seed 0 und standardmäßig 180 Sekunden Limit je C++-Aufruf.

Pro Fall läuft zusätzlich das unveränderte Original-CLI mit derselben Eingabe;
Ladebestätigung und Exitcode werden geprüft. Die vollständigen strukturierten
Daten stammen aus der Probe. Zwei eigenständige Probe-Prozesse müssen exakt
übereinstimmen. Rohdaten und CLI-/Probe-Protokolle liegen unter
`.build/<Variante>/runs/<Fall>/` und sind nicht versioniert.

## Exakte Geometrie-Normalisierung

Die native Torusbasis bleibt als beobachteter Double-Wert erhalten. Zusätzlich
kennzeichnet `geometry.exact_metric` eine kuratierte symbolische Interpretation:
Die archivierte Koordinate `1.7320508076` wird als gerundetes `sqrt(3)` gelesen.
Das ist eine dokumentierte Normalisierung, kein nativ rationaler C++-Messwert
und keine allgemeine rationale Rekonstruktion aus Floats.

Für Z3 und Z3×Z3 bestehen die drei Ebenen aus `(2,0),(-1,sqrt(3))`; ihre
Gram-Blöcke sind `[[4,-2],[-2,4]]`. Bei Z6-II besitzen die ersten beiden Ebenen
diese Blöcke; die dritte besitzt die orthogonalen Basisvektoren
`(sqrt(3),0),(0,sqrt(3))` mit Gram-Block `diag(3,3)`.

Der Validator prüft die symbolische Zuordnung komponentenweise gegen die
native Basis mit der festgelegten Schranke `4e-11` aus der Dezimalrundung von
`sqrt(3)`. Positive Definitheit und `R^T G R = G` für sämtliche nativen
rationalen Generatoraktionen werden exakt mit `Fraction` geprüft; die
Gram-Matrix wird zusätzlich exakt aus den symbolischen Basiskoordinaten in
`Q(sqrt(3))` bestätigt. Diese
Schranke wird ausschließlich für die Interpretation der gerundeten
Kartesischen Eingabebasis verwendet; Fixture-Vergleiche bleiben strikt.

## Abdeckungsgrenzen und Legacy-Befunde

- Der SUSY-Test untersucht alle 56 kubischen Kombinationen mit Wiederholung
  aus sechs links-chiralen Feldern; zuerst werden verschiedene `q_sh` gewählt.
  Er speichert Gauge-Invarianz, Monomialdaten und die originale Zahl erlaubter
  Kopplungen. Das ist kein vollständiges Superpotential des Modells.
- non-SUSY `CreateRepresentations` hat die Zuweisung von `CField::q_sh`
  auskommentiert. Daher steht dort `null`; rechte Gewichte und gruppierte
  `q_charges` bleiben im verknüpften Zustand vollständig erhalten.
- Der parameterisierte non-SUSY-`CHalfState`-Konstruktor initialisiert `Excited`
  nicht. Ein Wiederholungstest hat wechselnde Speicherwerte aufgedeckt. Das
  undefinierte Zusatzfeld wird nicht ausgelesen/exportiert; Oszillatorbelegungen
  und tatsächliche Zustandscontainer werden verglichen. Da der Legacy-Code
  das Feld bei der Tachyonklassifikation selbst liest, ist die Klassifikation
  positiv überlebender Tachyonen damit noch nicht zuverlässig validiert.
  Im siebten Fixture sind zwölf falsche `Excited`-Labels bei leeren rechten
  Oszillatorlisten nachgewiesen; zehn zusätzliche Prozesse liefern identische
  Rohdaten. Es wurde kein Physik-Patch vorgenommen. Der explizite Comparator-
  Modus `--tachyon-physics` isoliert nur dieses unzuverlässige Enum-Bit anhand
  der tatsächlichen rechten Belegung; der Standardvergleich bleibt strikt.
- Die sechs Basisfälle enthalten null projizierte Tachyonfelder; der siebte
  enthält zwölf. Ein positiver wirklich rechts angeregter Tachyonfall fehlt
  weiterhin. Der native rechte
  Tachyonlöser sucht mit `q0=-7..0`, `q1..q3=-7..5` und Schwelle `1e-5`;
  weitere linke Suchschritte bleiben die unveränderten Legacy-Algorithmen.
  Die Ergebnisse begründen keine allgemeine Tachyonfreiheit außerhalb dieses
  Rechenprofils. Nicht öffentlich zugängliche linke Tachyonlösungen werden
  nur erfasst, soweit ein projizierter Zustand sie referenziert.
- Spin(32), nichttriviale Torsion, frei wirkende Wilsonlinien, große rationale
  Grenzfälle und höhere SUSY-Analysen benötigen vor ihrer Rust-Implementierung
  zusätzliche Referenzen gemäß dem
  [Migrationsplan](../../docs/03_verification_and_migration_plan.md).

## Selbsttest

[compare.py](./compare.py) validiert Schema und semantische Verweise, reduzierte
Brüche, Dimensionen und den deklarierten Kopplungsumfang. Es vergleicht alle
Werte einschließlich Doubles ohne Vergleichstoleranz und nennt den ersten
abweichenden JSON-Pfad. Der Selbsttest erwartet genau die sechs Kern-Fixtures
und den einen positiven Tachyonfall,
prüft sie und verwirft gezielte Beschädigungen: fehlende Abschnitte, Nenner null,
ungültige Zustandsverweise, falsche Feldzähler und veränderte Quantenzahlen.
Eine veränderte Gram-Matrix muss ebenfalls scheitern. NaN und doppelte Schlüssel
werden gesondert geprüft.
Für den positiven Fall werden außerdem die Isolation des falschen Legacy-
Anregungsbits und die Ablehnung einer veränderten Oszillatorbelegung geprüft.

Die sieben Kern-Fixtures sind abgenommen; Phasen bis 3.3b wurden ebenfalls
freigegeben. Für Phase 3.3c ergänzt
[couplings_oracle.json](./couplings_oracle.json) die unveränderten Kern-Fixtures
um native Charaktere, vollständige Tensor-Basisräume sowie isolierte R-/SG-
Prüfungen. [generate_couplings.py](./generate_couplings.py) friert jeweils zwei
unabhängige Prozesse ein. Build und Reproduktion:

```bash
bash tests/ground_truth/build_couplings_probe.sh
python tests/ground_truth/generate_couplings.py
```

Der [SUSY-Audit](./SUSY_SELECTION_AUDIT.md) dokumentiert die Abgrenzung zwischen
nativen Tensor-Kandidaten und symmetrisierten Superfeld-Polynomen sowie den
nachgewiesenen Fehler der nativen Tensorzerlegung. Der
[Verifikationsbericht](../../docs/08_susy_verification.md) nennt die genaue
Abdeckung und die grünen Rust-Prüfungen.
