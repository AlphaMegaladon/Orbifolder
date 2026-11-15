# Schritt 2.4: ein positiver Tachyonfall und `Excited`

## Gemessenes Ergebnis

[non_susy_tachyonic.json](./fixtures/non_susy_tachyonic.json) enthält **12
projizierte Tachyonfelder**, sechs im Sektor `(1,2,0)` und sechs in `(1,4,0)`.
Es handelt sich um `(8v,1,1)` unter `D4 × A3 × D6` mit drei U(1)-Faktoren;
die Felder enthalten zusammen 96 Darstellungsgewichte. Die nativen Massenwerte
sind `-0.16666666666666666` beziehungsweise `-0.16666666666666674` in der
Normierung des C++-Tachyonlösers. Originales Laden, modulare Invarianz,
Modellkonstruktion und `CheckAnomaly` bestehen.

`StandardConfig` enthält 604 masselose Felder plus die angehängten zwölf
Tachyonfelder, insgesamt 616. Die separate `TachyonicStandardConfig` enthält
dieselben zwölf Tachyonfelder. Diese beiden Ansichten dürfen nicht doppelt
gezählt werden.

Die [Eingabe](./cases/non_susy_tachyonic.txt) verwendet die vorhandene Z6-II-
Geometrie und den unveränderten Z2-Brechungsshift. Nur der geometrische
Einbettungsshift des archivierten Standardmodells wurde ersetzt:

```text
V = (1/6,1/6,1/6,0,1/6,0,0,0 ; 1/2,1/6,0,0,0,0,0,0)
```

Alle Wilsonlinien bleiben null. `V²=7/18` stimmt mit dem geometrischen
Twistnormquadrat überein; `6V` liegt in E8×E8 und das Skalarprodukt mit dem
Brechungsshift ist null. Maßgeblich ist zusätzlich die ausgeführte originale
C++-Validierung. Es wurde genau **ein** weiterer Referenzfall eingefroren.

## Exakte Fundstellen im unveränderten Archiv

| Fundstelle | Verhalten |
| --- | --- |
| [chalfstate.cpp:31](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/chalfstate.cpp#L31) | Der parameterisierte Konstruktor initialisiert ausschließlich `Index` und `Type`; `Excited` fehlt. |
| [chalfstate.cpp:53](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/chalfstate.cpp#L53) | Der Default-Konstruktor initialisiert dagegen `Excited(0)`. |
| [csector.cpp:814](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/csector.cpp#L814) | `with_excitation` wird korrekt aus `Excitation.OscillatorIndices.size() != 0` bestimmt. |
| [csector.cpp:884](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/csector.cpp#L884) | Parameterisierten `new_RightMover` erzeugen; nur bei `with_excitation` folgt `Excited = 1` in Zeile 886. Der Grundzustandspfad setzt keinen Wert. |
| [cfixedpoint.cpp:834](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/cfixedpoint.cpp#L834) | Projektion legt Grundzustände in `TachyonicInvariantStates` ab. |
| [cfixedpoint.cpp:895](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/cfixedpoint.cpp#L895) | Der gesonderte Projektionspfad legt angeregte Zustände in `ExcitedTachyonicInvariantStates` ab. Die Auswahl erfolgt über die Positionen der rechten Halbzustände, nicht über `RM.Excited`. |
| [cfixedpoint.cpp:579](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/cfixedpoint.cpp#L579) und [607](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/cfixedpoint.cpp#L607) | Beide Container rufen anschließend `TachyonicFindSUSYMultiplets` auf. |
| [cstate.cpp:581](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/cstate.cpp#L581) | **Hier wird `RM.Excited == 0` gelesen:** daraus entstehen `Tachyon/bTachyon` oder `Excited_Tachyon/Excited_bTachyon`. |
| [corbifold.cpp:820](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/corbifold.cpp#L820) und [842](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/corbifold.cpp#L842) | Beide Repräsentationspfade schreiben in dieselbe `TachyonicStandardConfig`. Angeregte Zustandsindizes erhalten einen Offset um die Grundzustandsanzahl, siehe Zeile 840. |
| [corbifold.cpp:359](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/corbifold.cpp#L359) | Tachyonfelder werden zusätzlich an `StandardConfig.Fields` angehängt. |

`TachyonicExcitedConfig` existiert in dieser archivierten Codebasis nicht
([Konfigurationen in corbifold.h](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/corbifold.h#L117)).
Der uninitialisierte Wert steuert im untersuchten Pfad die Multiplet-Enums,
nicht die Wahl einer separaten Konfiguration oder eines Projektionscontainers.

## Prozessvergleich und Einordnung

Die reguläre Erzeugung verwendet zwei unabhängige Probe-Prozesse. Zusätzlich
prüft [audit_tachyons.py](./audit_tachyons.py) zehn Prozesse mit
`MALLOC_PERTURB_ = 0,1,85,170,255`, jede Einstellung zweimal. **Alle vollständigen
Rohdatensätze stimmen mit dem eingefrorenen Fixture überein.**

Alle zwölf Felder verweisen auf Grundzustandscontainer und leere rechte
Oszillatorlisten; auch ihre linken Listen sind leer. Trotzdem lauten die nativen
Enums sechsmal `Excited_bTachyon` und sechsmal `Excited_Tachyon`. Beobachtet
wurde somit eine **deterministische Fehlklassifikation**, kein Wechsel zwischen
Prozessen oder Konfigurationen. Die Allocator-Einstellungen garantieren nicht,
dass uninitialisierter Stack-Speicher variiert; stabile Läufe beseitigen das
undefinierte Verhalten nicht.

## Comparator und Wiederholung

[compare.py](./compare.py) löst Tachyonfelder anhand ihrer nativen
`source_index`-Verweise auf: Grundzustände zuerst, dann angeregte Zustände mit
dem Offset aus der Originalroutine. Die Anregung folgt ausschließlich aus
`tachyonic_right_solutions[right.solution_index].solution.excitation.oscillators`.
Linke Anregungen werden getrennt erhalten. Container und Oszillatorbelegung
müssen übereinstimmen; Gewichte, Massen, Ladungen und Rückverweise bleiben
streng geprüft. Die nach `StandardConfig` kopierten Felder werden samt Offset
gegen die separate Tachyonkonfiguration geprüft.

Der Standardvergleich bleibt vollständig strikt, einschließlich der falschen
Legacy-Enums. Bei künftigen Rohlabel-Schwankungen scheitert die reguläre
Fixture-Erzeugung weiterhin. Der ausdrücklich gewählte Modus
`--tachyon-physics` ersetzt ausschließlich das unzuverlässige Anregungsbit der
Tachyon-Multiplet-Enums in einer Vergleichskopie durch die Oszillatorableitung;
Konjugation, Rohdateien und alle übrigen Daten bleiben erhalten. Eine dadurch
ausgeblendete Rohdifferenz wird ausgegeben. Dieser Modus friert keine
abweichenden Oracles automatisch ein.

Der Selbsttest beweist, dass ein Enum-Wechsel im strikten Vergleich auffällt
und im expliziten Physikmodus isoliert wird. Eine manipulierte rechte
Oszillatorbelegung wird verworfen. Ein nativ überlebender **wirklich rechts
angeregter** Tachyonfall ist damit weiterhin nicht abgedeckt.

```bash
python3 tests/ground_truth/generate.py --case non_susy_tachyonic
python3 tests/ground_truth/audit_tachyons.py
python3 tests/ground_truth/compare.py --self-test
python3 tests/ground_truth/compare.py --tachyon-physics EXPECTED.json ACTUAL.json
```

Ausführungsdaten und Hashes der zehn Läufe liegen lokal unter
`.build/non_susy/runs/non_susy_tachyonic/audit-summary.json`.
Die sechs früheren Fixtures und sämtliche Legacy-Quellen bleiben unverändert;
der neue Fall speichert seine eigene aktuelle Generator-/Comparator-Provenienz.
Ein möglicher Konstruktor-Fix wäre separat fachlich zu prüfen und ist kein Teil
dieses Ground-Truth-Snapshots.
