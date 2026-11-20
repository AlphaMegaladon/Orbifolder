# Abnahme der Scan-API 0.2.0

Gemessen mit dem lokalen Linux-Release-Wheel auf AMD Ryzen 7 5700U. Der
[Rohbericht](../tests/benchmark/release-0.2.0-local.json) enthält den Wheel-Hash,
120 Messungen pro Exportbedingung und getrennte Zeiten für Projektion,
Python-Ausgabe und Voll-JSON.

| Messung | Ergebnis |
| --- | ---: |
| Vollständige Masken gegen den Z3-Korpus aus `shift_w2` | 16/16 identisch, kein Budgetabbruch |
| Erreichte DP-Zustände pro vorbereiteter Tabelle | 1.857–2.144 (Standardlimit 100.000) |
| Echte Timeout-Elternmaske; Maske am ausgeschlossenen Kind | `[true,false,true]`; `[false,false,false]` |
| Native Vorbereitung, Median / p95 | 11,51 ms / 12,94 ms |
| Native wiederholte Abfrage, Median / p95 | 0,49 µs / 0,88 µs |
| Neu gelöste Z3-Referenz / vorbereitete native Abfrage, Quotient der Mediane | 295.674× |
| Bereits gecachte Python-Maske / vorbereitete native Abfrage, Quotient der Mediane | 30,9× |
| Höchstes p95 für den reinen Dict-Export über sieben Referenzen | 2,18 ms |
| Höchstes p95 für Projektion plus Dict über sieben Referenzen | 8,43 ms |
| Projektion plus Dict beim zusätzlichen Drei-Generationen-Kandidaten, p95 | 6,43 ms |

Die Z3-Zeit ist eine gemessene Zeit aus der Erzeugung der fixierten
Referenzmasken; die Cache-Messung verwendet die später erneut befragten Masken.
Das 100×-Ziel gegenüber einer neu gelösten exakten Abfrage ist erreicht.
Gegenüber einem bereits fertigen Python-Cache-Treffer liegt der Gewinn
unter 100×. Beide Vergleiche trennen die Rust-Vorbereitung von der Abfrage.
Keiner der Quotienten ist eine Messung des RL-Gesamtdurchsatzes.

Der Masker normiert Gitter- und Modularbedingungen zu Restklassenpolynomen.
Der Zustand enthält deren konstante Reste und nur diejenigen linearen
Beiträge zu noch offenen Koeffizienten, die durch bereits gewählte
Koeffizienten verändert wurden. In einer blockdiagonalen E8-Basis gibt es
zwischen den Blöcken keine Gram-Kreuzterme. Polynomreste verschwinden nach
ihrem letzten relevanten Schritt, sobald ihre Bedingung geprüft ist. Identische
Reste teilen denselben Zustand. Die Vorbereitung markiert ausgehend von
den zulässigen Endzuständen die erreichbaren Kanten; `mask` folgt danach
nur der vorberechneten Tabelle. Das Budget begrenzt den Aufbau der Tabelle,
nicht die logische Bedeutung eines Maskenbits.

Die kompakte Projektion reproduziert die kanonischen Traces einschließlich
der exakten kubischen Tensoren und die Spektrums-Eingaben der unabhängigen
RL-Klassifikation. Der Bench vergleicht auf allen Referenzen die Typen,
Ladungen und Ausschlusszertifikate. Für `MSSM0` und den zusätzlichen
non-SUSY-Kandidaten prüft er außerdem dieselben Hypercharge-Zeugen,
Chiralitätsbefunde und Rewards. Die Klassifikation des Kompaktpfads
liest keinen Vollsnapshot.

Aus dem Library-Repo, nach isolierter Installation des Wheels unter
`target/release020-test`:

```bash
PYTHONPATH="$PWD/target/release020-test" \
  ../orbifolder_rl/.venv/bin/python tests/benchmark/release_020.py \
  --rl ../orbifolder_rl \
  --wheel target/release020-wheel/orbifolder-0.2.0-cp311-abi3-manylinux_2_34_x86_64.whl \
  --output target/release020-benchmark.json --repeats 120 --check-gates
```

Dieser Lauf überprüft die Bibliothek lokal. Er aktualisiert weder die
gepinnten Abhängigkeiten noch die Checkpoints des laufenden PPO-Versuchs.

Der [Anomaliefilter-Test](../crates/orbifolder-core/src/execution/tests.rs)
entfernt aus dem SUSY-Z3-Spektrum ein geladenes chirales Feld. Mit und ohne
Filter meldet der Prüfer dieselbe Verletzung der kubischen Eichanomalie. Ohne
Filter werden das kanonische Spektrum und die SUSY-Auswertung je einmal
aufgebaut. Mit Filter bleibt der Zähler an beiden Funktionen bei null
zusätzlichen Aufrufen, und der Scan liefert den Grund `anomaly`.

Der [Batchtest](../crates/orbifolder-python/tests/prepared_geometry.rs)
bereitet über die Python-API eine Z3-Geometrie vor und berechnet zwei
verschiedene Einbettungen mit zwei Worker-Threads. Vom Anlegen der Geometrie
bis zur Rückgabe beider Summaries zählt er genau einen Aufruf von
`Geometry::build`.

Mit den beiden Regressionstests bestanden lokal alle 97 Rust-Tests im
Release-Profil und alle 69 Python-Tests gegen ein frisch gebautes, isoliert
installiertes Linux-Wheel. Clippy und der Workspace-Check mit Rust 1.85.0
waren ebenfalls grün. Die Typprüfungen und `twine check --strict` wurden
bei der ursprünglichen Wheel-Abnahme erfolgreich ausgeführt.
