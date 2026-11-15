# 09 – Gesamtabnahme: Rayon, PyO3 und Release 0.1.0

> Historischer Phasenbericht vom 2026-10-31. Testzahlen und Umgebungsangaben
> beziehen sich auf diesen damaligen Lauf. Für den heutigen Checkout gelten
> der [aktuelle Prüfstand](./12_repository_audit.md) und die
> [Ubuntu-Anleitung](./11_development_ubuntu.md).

Stand: 2026-10-31. Phasen bis 3.3c einschließlich der dokumentierten Legacy-
Befunde sind vom Nutzer abgenommen. Phase 3.4 ist implementiert. Der komplette
[Prüflauf](../tests/verify_release.sh) besteht mit **93 Rust- und 36 Python-Tests**,
`cargo fmt --all -- --check` und `cargo clippy --workspace --all-targets --locked
-- -D warnings`. Ein installierbares Linux-Wheel wurde gebaut und in einer
separaten virtuellen Umgebung installiert. Auch das installierte Wheel besteht
dieselben **36 Python-Tests** (144,83 s); der Entwicklungs-Prüflauf benötigt
138,70 s für diese Python-Suite. Der Release-Import wurde nachweislich aus
`site-packages` geladen und nicht aus dem Quellbaum.

## Architektur und Garantien

| Baustein | Implementierter Vertrag |
| --- | --- |
| [orbifolder-core](../crates/orbifolder-core/src/lib.rs) | Fachkern einschließlich exakter Arithmetik, Geometrie, Zustände, Anomalien und SUSY-Profile; keine Python-Abhängigkeit. |
| [execution](../crates/orbifolder-core/src/execution.rs) | Echte serielle Ausführung oder expliziter wiederverwendbarer Rayon-Pool; Parallelisierung unabhängiger Modelle, keine globalen/nested Pools. |
| [orbifolder-data](../crates/orbifolder-data/src/lib.rs) | Versioniertes In-Memory-Eingabeformat, embeddingbezogener Textadapter und exakte Ergebnisdarstellung; keine eigene Physik. |
| [orbifolder-python](../crates/orbifolder-python/src/lib.rs) | Dünne PyO3-Fassade mit frozen `Geometry`, `Model`, `Result`, `Field`, `Executor`, `BatchItem`; teure Arbeit über `Python::detach`. |

`detach` ist in PyO3 0.27 der aktuelle Name der GIL-Freigabe, die zuvor
`allow_threads` hieß. Worker erhalten ausschließlich Rust-Daten. Eingaben sind
vor der Freigabe kopiert; Geometrien werden über immutable `Arc<Geometry>` geteilt.
Ein gemeinsam genutzter Geometriekern muss auch die Eingabe- und ID-Konventionen
erfüllen. Ein Python-Worker-Fortschritt während nativer Rechnung wurde gemessen.
Field-Wrapper behalten ihre Daten per `Arc`, auch wenn das übergeordnete Resultat
nicht mehr referenziert wird; exportierte Dictionaries sind unabhängige Kopien.

Rationale Eingaben akzeptieren Python-Ganzzahlen, `Fraction`, `(num,den)`,
`{"num":...,"den":...}` und rationale Ganzzahl-/Bruchstrings. Zahlen werden
gekürzt und vorzeichennormalisiert; Float-Koordinaten werden abgelehnt.
Direkte PyLong/BigInt-Konvertierung umgeht CPythons standardmäßige 4.300-Ziffern-
JSON-Grenze ohne globale Interpreteränderung. Ein `1 << 20000`-Roundtrip und
negative Nenner sind geprüft. `exact_data` liefert wahlweise `Fraction`, Paare
oder Bruch-Dictionaries. Physikergebnisse enthalten keine numerische Float-
Entscheidung; Fließkomma ist im externen Oracle-Vergleich und den Zeitmessungen.

`InputError`, `ModelError`, `ExecutionError` sowie indexierte Batch-Fehler
trennen Eingabekonstruktion, Modellrechnung und Poolkonfiguration. Physikalische
Einbettungsvalidierung erfolgt bei der Rechnung, sodass ungültige Einbettungen
als einzelne Batch-Ausgänge erhalten bleiben. Ressourcenabbrüche sind Fehler;
sie werden nicht als leere vollständige Spektren zurückgegeben.

## Determinismus und In-Memory-Roundtrips

[Rust-Tests](../crates/orbifolder-data/tests/parallel.rs) vergleichen die gesamten
kanonischen Nutzdaten bytegleich: sieben Modelle, ungültige Wilsonlinie und
Ressourcenabbruch, seriell und mit **1, 2, 4, 8 Threads**. Concurrent callers auf
demselben Pool ergeben dieselben Daten. `Send + Sync` ist am Typ geprüft.
[Python-Tests](../tests/python/test_api.py) wiederholen die Poolvergleiche inklusive
Batch-Fehlern, prüfen gleichzeitige Aufrufer, Datenbesitz und frozen Wrapper.

Batch-Ausgaben behalten die Eingabeindizes. Feld-IDs sind positionale IDs innerhalb
der getrennten massless-/tachyon-Namensräume; vollständige Identität ist
`(batch_index, namespace, field_id)`. Zustände liegen stabil unter
`(sector_id, brane_id, namespace, state_id)`. Der Scheduler verändert keine
Gewichts-, Feld- oder Multiplizitätsreihenfolge. Canonical JSON besitzt sortierte
Map-Schlüssel und exakte Ganzzahl-Tokens, keine Zeitstempel oder Adressen.

[Input-only-Referenzbeschreibungen](../tests/python/model_inputs.json) werden aus
den Modell- und archivierten Geometrie-Eingaben reproduziert. Erwartete Spektren,
Ladungen oder Entscheidungen werden nicht in Solver eingespeist. `from_data`
und `from_text` erzeugen identische kanonische Ergebnisse. Der Textadapter erhält
die Geometrie ausdrücklich im Speicher; er öffnet keine im Legacy-Text genannten
Dateien und führt keine CLI-Blöcke aus.

## Abdeckungsmatrix

[Der Python-Oracle-Abgleich](../tests/python/test_oracle.py) prüft Geometrie,
lokale Einbettungen, rechte/linke Massenschalen und Halbzustände, projizierte
Zustände, sämtliche Felder/Quantenzahlen, Anomaliekoeffizienten und FI-Daten.
Nicht-abelsche Faktoren/Dynkin-Konventionen und U(1)-Basen werden erst zum
externen Vergleich exakt ausgerichtet. Alle bisherigen modulweisen C++-Probes
und Invariantentests bleiben grün.

| Fixture | Massless Felder | Tachyonfelder | Anomalieabgleich | SUSY-Cubics |
| --- | ---: | ---: | --- | --- |
| SUSY Z3 | 246 | 0 | bestanden | 56 Trials, 15 native erlaubt |
| SUSY Z6-II | 544 | 0 | bestanden | 56 Trials, 1 native erlaubt |
| SUSY Z3×Z3/WL | 524 | 0 | bestanden | 56 Trials, 0 native erlaubt |
| non-SUSY Z3 | 438 | 0 | bestanden | NotApplicable |
| non-SUSY Z6-II | 832 | 0 | bestanden | NotApplicable |
| non-SUSY Z3×Z3/WL | 1112 | 0 | bestanden | NotApplicable |
| non-SUSY tachyonisch | 604 | 12 | bestanden | NotApplicable |
| Summe | **4300** | **12** | sieben Modelle | 168 Original-Cubics |

Die zwölf Tachyonfelder entsprechen dem angenommenen `--tachyon-physics`-
Vertrag. Ausschließlich das nachgewiesene uninitialisierte Legacy-Excited-Bit
wird anhand echter Oszillatorindizes normalisiert. Vollständige Normschranken
bleiben mit `TachyonSearch::Complete` / Python `tachyon_search="complete"` Standard.
Rohfelder, Gewichte, Anregungen und Multiplizitäten werden dabei nicht verändert.

SUSY bleibt beim freigegebenen Grad-1–3-Profil. Native Tensor-Kandidaten und
symmetrisierte Polynom-Invarianten werden separat berichtet. Konfigurierte
R-Regeln sind exakt; die archivierten drei SUSY-Geometrien konfigurieren selbst
keine R-Operatoren. Die deaktivierte Gamma/W-Prüfung bleibt ausdrücklich
ausgewiesen. Numerische Kopplungsstärken, F-/D-flache VEVs, höhere Kopplungsordnungen,
weitere Geometrien, vollständige Spin(32)-Spektrum-Referenzen und nichttriviale
Torsions-Kopplungsreferenzen sind nicht durch diese sieben Modelle zertifiziert.

## Bestätigte Legacy-Befunde

| Befund | Verbindliche Rust-Behandlung / Nachweis |
| --- | --- |
| Uninitialisiertes `CHalfState::Excited` | Tatsächliche Anregungen entscheiden; [Tachyon-Audit](../tests/ground_truth/TACHYON_AUDIT.md). |
| non-SUSY akzeptiert `3 W²=2/3` trotz verletzter modularer Invarianz | Exakte Ablehnung für beide Theorien; [Wilson-Audit](../tests/ground_truth/WILSON_INVARIANCE_AUDIT.md). |
| Feste Tachyon-Suchfenster bei ganzzahligem Twist-Lift | Vollständige Normschranken statt abgeschnittener Suche; [Window-Audit](../tests/ground_truth/TACHYON_WINDOW_AUDIT.md). |
| non-SUSY-Fehlerstatus gekoppelt an `verbose` | Bewertung unabhängig von Ausgabe; [Anomalie-Audit](../tests/ground_truth/ANOMALY_VALIDATION_AUDIT.md). |
| SUSY-`number_of_U1s==0`-Frührücksprung überspringt nicht-abelsche Fehler | Sämtliche Anomalieprüfungen ausführen; derselbe Anomalie-Audit. |
| Unvollständige native Tensorzerlegung | Vollständige native Charaktere und exakte Rust-Zerlegung; [SUSY-Audit](../tests/ground_truth/SUSY_SELECTION_AUDIT.md). |
| Kandidatenzähler symmetrisiert identische Superfelder nicht automatisch | Getrennte Tensor-/Polynomberichte; derselbe SUSY-Audit. |

Zusätzliche Klarstellungen: E6 besitzt keinen symmetrischen kubischen Lie-
Anomalietensor; dessen kubische Anomalie ist null. `CField::q_sh` ist im non-SUSY-
Exportpfad nicht gesetzt, die zugrunde liegenden Rechtsgewichte bleiben erhalten.
Native FI-kompensierende Monomiale sind von strikter Eichinvarianz getrennt.
Unterschiedliche Torusbasen mit irrationalen Komponenten werden über bewiesene
exakte Gram-Matrizen/Gitteraktionen behandelt. Die sieben Kern-Fixtures und
Originalquellen bleiben unverändert; ihre SHA-Bindungen werden weiterhin geprüft.

## Gemessene Performance

[Rohdaten und Provenienz](../tests/benchmark/latest.json), erzeugt durch den
[Runner](../tests/benchmark/run.py): AMD Ryzen 5 PRO 220, 12 logische CPUs,
WSL2/Linux x86_64, Rust **1.99.1**, CPython **3.14.4**, Release-Build mit
Overflow-Prüfungen. Drei Wiederholungen, kein verworfener Warm-up. Werte sind
Median-Wandzeiten für die sieben Modelle; Poolstart/Eingabeaufbau sind bei Rust
außerhalb der Messung.

| Ausführung | Sekunden | Rust-Speedup gegen seriell |
| --- | ---: | ---: |
| Rust seriell | 2,237 | 1,00× |
| Rayon 1 Thread | 2,220 | 1,01× |
| Rayon 2 Threads | 1,238 | 1,81× |
| Rayon 4 Threads | 1,052 | 2,13× |
| Rayon 8 Threads | 0,953 | **2,35×** |
| C++-Probe-Batch | 10,045 | anderer Messumfang |

Rust misst Einbettung, vollständigen Sector-/Spectrum-Löser und Anomalien;
SUSY-Spektrumsansichten werden erzeugt, Kopplungskandidaten hier nicht enumeriert.
C++ misst frische Prozesse mit Modell-/Geometrieladen, nativer Rechnung,
zusätzlichen Probe-/Cubic-Prüfungen und JSON-Schreiben. Kopieren isolierter
Runtime-Verzeichnisse und anschließende Validierung sind ungemessen. Der
C++-Wert ist deshalb eine Probe-Laufzeit, kein isolierter Algorithmus-Speedup.

**1000 Auswertungen**, die sieben Eingaben zyklisch wiederholt mit geteilten
Geometrien, 8 Threads und 32er-Chunks: **113,094 s**, etwa **8,84 Modelle/s**,
614356 massless Feldobjekte. Der gemessene maximale RSS des gesamten Rust-
Benchmarkprozesses ist 1318676 KiB, etwa **1,26 GiB**. Das umfasst auch den
Vollsnapshot-Export und ist kein getrenntes Speichermessergebnis einzelner Stufen.

Kanonischer Voll-Export aller sieben Resultate: **6,058 s**, **31340497 Byte**.
Übergabe aller vollständigen Resultate an Python-Dictionaries: **7,608 s**.
Für selektive Workflows vermeiden Field-Getter den Export aller Massenschalen.
Die Exporte sind ein relevanter Kostenanteil und nicht in den Rust-Rechenzeiten
versteckt. Von 68800 zurückgegebenen Höchstgewichtskoordinaten verwenden null den
BigInt-Backendpfad; dies zählt Ergebniswerte, keine arithmetischen Operationen.

## Reproduktion und Release-Artefakt

```bash
/usr/bin/python3 -m venv .venv
.venv/bin/python -m pip install 'maturin[patchelf]>=1.10,<2' pytest jsonschema
export PATH="$HOME/.cargo/bin:$PWD/.venv/bin:/usr/bin:/bin"
export VIRTUAL_ENV="$PWD/.venv"
bash tests/verify_release.sh
cargo build --release --locked -p orbifolder-data --example snapshot
.venv/bin/python tests/benchmark/run.py --repeats 3
maturin build --release --locked --out dist
```

Reguläre Rust-/Python-Tests benötigen keine C++-Binaries. Nur die separate
Benchmark-Reproduktion braucht die bestehenden unveränderten C++-Builds.
[CI-Konfiguration](../.github/workflows/ci.yml) reproduziert den normalen Linux-
Prüflauf; ein entfernter CI-Lauf wurde in dieser Session nicht ausgeführt.

Das erzeugte Wheel ist
[orbifolder-0.1.0-cp311-abi3-manylinux_2_34_x86_64.whl](../dist/orbifolder-0.1.0-cp311-abi3-manylinux_2_34_x86_64.whl).
Es benötigt keinen C++-Backendprozess. ABI3 deklariert Python ≥3.11; tatsächlich
zertifiziert wurde hier **CPython 3.14.4 auf Linux/WSL x86_64**. Weitere Python-
Runtimes, freie GIL-Varianten und Windows-native Wheels wurden nicht getestet.
Der lokale Editor verwendet weiterhin Windows-Python 3.13; sämtliche
Linux-Prüfungen liefen mit dem ausdrücklich verifizierten WSL-Interpreter.
