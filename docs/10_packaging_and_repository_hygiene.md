# Phase 4: Packaging, Ergonomie und Repository-Hygiene

> Historischer Phasenbericht vom 2026-10-31. Testzahlen und Umgebungsangaben
> beziehen sich auf diesen damaligen Lauf. Für den heutigen Checkout gelten
> der [aktuelle Prüfstand](./12_repository_audit.md) und die
> [Ubuntu-Anleitung](./11_development_ubuntu.md).

Autor: Martin Wimmer. Paket/Version: `orbifolder 0.1.0`, Python >= 3.11,
ABI3 mit PyO3 0.27.2, GPL-3.0-or-later.

## Öffentlicher Inhalt und Provenienz

336 `:Zone.Identifier`-Dateien wurden gelöscht. Die beiden lokalen
C++-Originalverzeichnisse wurden vollständig aus dem Git-Index entfernt und
über `/legacy_cpp/` ignoriert. Ihre eigentlichen Quelldateien bleiben lokal
erhalten. Ignoriert sind außerdem Venvs, Rust-Zielverzeichnis, Wheels,
Oracle-Builds und Python-Caches. Es wurde kein Commit, Tag oder Push erstellt.

Die sieben freigegebenen Kern-Fixtures bleiben unverändert. Die regulären
Rust-Tests benötigen die C++-Archive nun nicht mehr: Die sechs geometrischen
Texteingaben liegen mit Originalhashes in
[geometry_inputs.json](../tests/ground_truth/geometry_inputs.json).
Der bisherige Testadapter rekonstruiert daraus weiterhin die exakten Gram-
Matrizen in Q(sqrt(3)) und prüft die ursprünglichen Eingabehashes.

Die strukturierten [Presets](../python/orbifolder/presets/) enthalten ausschließlich
Modelleingaben, einschließlich der bereits verifizierten Vertreter-/Zentralisator-
Konvention und SUSY-Auswahlregeln. Erwartete Zustände oder Spektren sind kein
Bestandteil dieser Berechnungseingaben. Jedes Preset enthält getrennte SUSY-
und non-SUSY-Generatoren; im non-SUSY-Fall gehört der Brechungsgenerator dazu.
[bundle_inputs.py](../tests/ground_truth/bundle_inputs.py) erzeugt die Bündel
reproduzierbar, ohne die sieben Oracle-Ausgaben zu ändern.

[Oracle-README](../tests/ground_truth/README.md) dokumentiert HepForge v1.2.1
und StringsIFUNAM/nonSUSYorbifolder sowie das optionale lokale Archivlayout.
[Lizenz](../LICENSE), [Attribution](../THIRD_PARTY_NOTICES.md) und
[CITATION.cff](../CITATION.cff) werden mitverteilt.
Die korrekte non-SUSY-Paper-ID **2504.20137** ist durch die originale README
und arXiv bestätigt; die zunächst angegebene ID 2505.00083 bezeichnet einen
anderen, astrophysikalischen Artikel.

## API-Verträge

- `Geometry.available_presets()` liefert `Z3`, `Z6-II`, `Z3xZ3`;
  `from_preset` akzeptiert zusätzlich `Z6_II`. `theory_name="non_susy"` wählt
  die Variante. Dies sind konkrete Referenzgitter, keine vollständige
  Klassifikation aller Torusgitter für diese Punktgruppen.
- `parse_legacy_models(text, geometry=...)` liest mehrere vollständige
  Modellblöcke. `geometry` ist ein Presetname oder eine unveränderliche Geometry.
  `Geometry.evaluate_legacy_text` liefert berechnete BatchItems. `SpaceGroup:`
  wird niemals geöffnet, CLI-Anweisungen niemals ausgeführt.
- `load_legacy_file(path, ...)` übernimmt ausschließlich am Python-Rand das
  Lesen einer Datei. Der Rust-Core bleibt vollständig frei von Dateisystem-I/O.
  Ein Katalog muss dieselbe explizite Geometrie benutzen; unbekannte optionale
  Physikabschnitte werden nicht stillschweigend als Standardeinbettung gelesen.
- `Result.summary()`, `Executor.compute_summaries` und
  `Geometry.evaluate_batch_summaries` geben nur Kennzahlen aus. Im Batch wird
  die Projektion innerhalb des Rust-Workers ausgeführt; das vollständige
  Ergebnis wird vor dem nächsten Modell verworfen. Die Speicherhaltung der
  vollständigen berechneten Spektren wächst mit den aktiven Workern, nicht
  mit der gesamten Modellzahl. Eingaben und Zusammenfassungen bleiben geordnet.
- Geladene Singuletts sind masselose Felder mit ausschließlich trivialen
  nicht-abelschen Darstellungen und mindestens einer von null verschiedenen
  U(1)-Ladung. Wie bei den detaillierten Feldzahlen zählen vorhandene CPT-
  Partner separat. FI-Koeffizienten bleiben rationale Zahlen; non-SUSY zeigt
  `not_applicable`, abgeschaltete Anomalien `not_requested` und keinen
  erfundenen erfolgreichen Anomaliestatus.

## Reproduzierbare lokale Prüfung

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
maturin build --release --locked --out dist
maturin sdist --out dist
python tests/check_distributions.py
python -m twine check --strict dist/*
python tests/verify_sdist.py dist/orbifolder-0.1.0.tar.gz
python -m venv .venv-release
.venv-release/bin/python -m pip install --force-reinstall dist/*.whl pytest jsonschema
.venv-release/bin/python tests/run_installed_tests.py
python tests/check_typing.py
```

Der letzte Typprüfungsbefehl benötigt die Entwicklungsabhängigkeiten und das
installierte Wheel in der ausführenden Umgebung. Die Seitendeklarationen
beschreiben Konstruktoren, eingefrorene Klassen, Modell-/Geometrieeingaben,
exakte Zahlen, Felder, Anomalieberichte, Kopplungen und diskriminierte
Status-/Batch-Zusammenfassungen. Private TypedDicts erfinden keine öffentlichen
Runtime-Klassen. `stubtest` vergleicht auch die native Erweiterung und deren
tatsächlich vorhandene Signaturen.

`run_installed_tests.py` kopiert nur die Tests und JSON-Referenzen in ein
temporäres Verzeichnis. Der isolierte Kindprozess startet dort und verweigert
per Audit-Hook Dateizugriffe auf den ursprünglichen Quellbaum; nur sein
installiertes Interpreter-/Venv-Verzeichnis ist ausgenommen. Damit bleiben
Preset-, Parser-, Datei-, Zusammenfassungs- und alle sieben Physikvergleiche
unabhängig vom Checkout.

## GitHub-/PyPI-Release

[release.yml](../.github/workflows/release.yml) verwendet
`PyO3/maturin-action@v1`. Native Runner bauen Linux x86_64/aarch64,
Windows x86_64 und macOS x86_64/aarch64. Jedes ABI3-Wheel wird installiert und
mit den Python-Tests unter **3.11 und 3.14** außerhalb des Checkouts geprüft.
Ein weiterer Job baut das sdist, prüft seinen Inhalt und baut/testet daraus
erneut ein Wheel. [verify_sdist.py](../tests/verify_sdist.py) entpackt es zudem
in ein temporäres Verzeichnis und führt dort alle Rust-Tests mit vollständig
neuem Target-Verzeichnis aus: gecachte Test-Binaries mit eincompiliertem altem
`CARGO_MANIFEST_DIR` können so keine Fixture-Dateien aus dem Checkout lesen.
Rust-Fmt, Clippy und Tests sind vorgeschaltet.

Vor der ersten Veröffentlichung extern einzurichten:

1. Eigenes GitHub-Repository/Remote anlegen und die sauberen Dateien committen.
2. PyPI-Projekt oder Pending Trusted Publisher mit Name `orbifolder`, dem
   tatsächlichen GitHub-Owner/Repository, Workflow `release.yml` und Environment
   `pypi` anlegen. Das GitHub-Environment `pypi` entsprechend konfigurieren.
3. Versionsnummern in den drei Cargo-Paketen, Python-Metadaten und CFF konsistent
   halten; Cargo.lock nachführen. Tag `v0.1.0` muss zur Paketversion passen.
4. Zunächst `workflow_dispatch` ohne Publish verwenden; danach bewusst Tag
   pushen oder einen manuellen Lauf mit explizitem Publish starten.

Nur der abschließende Publish-Job erhält `id-token: write`; es werden keine
PyPI-Tokens im Repository benötigt. Alle Builds und Wheel-Tests müssen vorher
erfolgreich sein. Manuelle Läufe veröffentlichen standardmäßig nichts.
Eine tatsächliche Veröffentlichung oder ein entfernter Multi-OS-Lauf wurde
lokal nicht durchgeführt.

## Prüfresultate

Lokale Umgebung: Linux/WSL x86_64, CPython 3.14.4, rustc 1.99.1,
maturin 1.16.0. **Alle 95 Rust-Tests** bestehen auch im separat entpackten
sdist mit vollständig frischem Target-Verzeichnis und ohne Legacy-Verzeichnis;
`fmt` und striktes Clippy sind grün.

| Prüfung | Lokaler Befund |
| --- | --- |
| Finales installiertes Release-Wheel, Repo-Dateizugriffe gesperrt | **59 Python-Tests grün** in 218,68 s, einschließlich aller sieben Oracle-Modelle, der neuen Parser-Negativfälle und Presets/Summaries. |
| Neue Ergonomieprüfungen | **23 Tests**: Presets aller sieben Fälle, Ressourcen, Kataloge/Dateien, Auswahlregeln, Summary-Vollständigkeit, Fehler und 1/2/4/8 Threads. |
| Originale Legacy-Dateien | Alle **77 SUSY-Katalogmodelle** (`MSSM0` bis `MSSM1076`) und die originale non-SUSY-Z3-Datei direkt geladen; dies ist ein Parser-/Eingabetest, kein zusätzlicher 77-Modell-Spektrumsnachweis. |
| Unabhängiger sdist→Wheel-Neuaufbau | `pip wheel --no-cache-dir --no-deps ...tar.gz` erfolgreich in temporärer Build-Isolation; separat installiert, **23 Ergonomietests grün** in 51,83 s, ebenfalls mit gesperrtem Quellbaumzugriff. |
| Wheel-/sdist-Inhalte | Presets, Stubs, Marker, Lizenz/Attribution/CFF vorhanden; keine Legacy-Archive, Venvs, Caches oder Buildprodukte im sdist. Sieben Fixtures und Geometrieeingaben im sdist vorhanden. |
| PyPI-Metadaten/README | `twine check --strict` für Wheel und sdist erfolgreich. |
| Runtime-/Stub-Vertrag | `mypy.stubtest orbifolder`: **drei Module ohne Fehler**. |
| Statische Nutzung | mypy `--strict --python-version 3.11` und Pyright Strict ohne Fehler; beide verwerfen alle drei absichtlich falschen API-Aufrufe. |
| Ungefiltertes VerifyTypes | **95,8 %**, 46/48 bekannte öffentliche Symbole. Die zwei Grenzen sind der erhaltene Reexport `Fraction` und der Fraction-enthaltende Rückgabebaum von `exact_data`: Pyrights gebündelte `numbers`-/`fractions`-/`decimal`-stdlib-Deklarationen sind teilweise unbekannt. Keine erfundene Ersatzklasse, kein `Any` und keine Checker-Unterdrückungen in den Stubs. |
| Release-Workflows | `actionlint 1.7.12` ohne Befund; tatsächliche entfernte Multi-OS-Ausführung bleibt der GitHub-Pipeline vorbehalten. |
| Zitierbarkeit | CFF gegen offizielles CFF-Schema validiert; non-SUSY-Referenz korrigiert und quellengeprüft. |
| Git-Hygiene | Null Zone-Identifier, keine Legacy-Dateien im Index; `git add --dry-run .` enthält ausschließlich **154 saubere Projektdateien**. Ungeborener `master`, weiterhin kein Commit und leerer Index; die eigenen Dateien sind bereit zum Hinzufügen. |

Die Vollständigkeitsmessung ist keine Einschränkung der exakten Berechnungen
oder der bestandenen Strict-Client-Prüfungen; sie benennt eine verbleibende
Grenze in den externen Standardbibliotheks-Typdeklarationen. Die Pylance-MCP-
Werkzeuge waren in dieser Sitzung nicht verfügbar; native Rust-Signaturen,
installierte Laufzeitschnittstelle, `stubtest`, mypy und Pyright liefern hier
die überprüften Fakten.
