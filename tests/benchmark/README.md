# Reproduzierbare Messung

```bash
cargo build --release --locked -p orbifolder-data --example snapshot
# .venv und Entwicklungsabhängigkeiten vorher einrichten (siehe unten)
.venv/bin/python tests/benchmark/run.py --repeats 3 --build-root "$PWD/tests/ground_truth/.build/ubuntu-24.04" --output tests/benchmark/ubuntu-2026-11-15.json
```

[latest.json](./latest.json) enthält Rohwiederholungen, Messumfang, Compiler,
Hardware, Quellen-/Lock-/Input-/C++-Link-SHA-256 und getrennte Exportkosten.
Die C++-Probes müssen unter `<build-root>/{susy,non_susy}/` gebaut sein. Der
Runner akzeptiert `--build-root` oder `LEGACY_BUILD_ROOT`; ohne beides gilt
`tests/ground_truth/.build/`. Die frischen Ubuntu-Builds entstehen entsprechend
der Entwicklungsanleitung unter `.build/ubuntu-24.04/`. Probes verlinken mit:

```bash
LEGACY_BUILD_ROOT="$PWD/tests/ground_truth/.build/ubuntu-24.04" bash tests/ground_truth/build_probes.sh susy non_susy
```

In `.venv` muss außerdem das aktuelle Release-
Wheel installiert sein, da der Runner auch `import orbifolder` verwendet.
Siehe [Ubuntu-Setup](../../docs/11_development_ubuntu.md).
`latest.json` enthält historische Messungen des dort protokollierten Rechners,
keine aktuelle Geschwindigkeitszusage für diesen Checkout. Runtimekopien und Prüfung der
Ausgaben sind außerhalb der Zeitmessung; die Originalquellen bleiben unverändert.

Die 1000-Auswertungen-Messung wiederholt die sieben bestätigten Eingaben mit
gemeinsamen immutable Geometrien in 32er-Chunks; sie erzeugt keine 1000 neuen
unterschiedlichen physikalischen Modelle. Der
[Release-Bericht](../../docs/09_final_verification_and_release.md) interpretiert
Laufzeiten und Abdeckungsgrenzen.


Die [Ubuntu-Messung vom 2026-11-15](./ubuntu-2026-11-15.json) wurde mit frisch
gebauten C++-Probes auf dem aktuellen Rechner wiederholt. Median: Rust seriell
3,031 s, Rayon/8 Threads 1,149 s, also 2,64× innerhalb des Rust-Workloads.
Der C++-Probe-Batch benötigt 13,183 s bei anderem Messumfang. Vollständiger
Python-Export kostet zusätzlich 9,642 s. Interpretation und Grenzen:
[Portierungs-/Paketierungsreview](../../docs/14_precommit_review.md).
