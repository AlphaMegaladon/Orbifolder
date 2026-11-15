# Repository-Prüfung vom 2026-11-14

Fortsetzung und aktueller Stand: [Portierungs-/Paketierungsreview vom 2026-11-15](./14_precommit_review.md).

Geprüft wurde der lokale Checkout unter Ubuntu 24.04.5 LTS, x86_64, mit
CPython 3.12.3, rustc/Cargo 1.100.0 und maturin 1.16.0. Gegenstand waren
README, Migrations-/Verifikationsberichte, Manifeste, Workflows, öffentliche
Rust-/Python-Schnittstellen, Test- und Paketierungsskripte sowie die lokale
Werkzeugausstattung. Dies ist eine Konsistenz- und Build-/Testprüfung; die
physikalische Abdeckung bleibt auf die dokumentierten Referenzen begrenzt.

## Befunde und Änderungen

| Befund | Korrektur |
| --- | --- |
| Architektur und Plan behaupteten noch, Rust-Implementierung und Oracle-Aufbau seien künftige Schritte. | Aktuelle Modulzuordnung und Phasenstatus ergänzt; ursprüngliche Entwürfe ausdrücklich als historisch gekennzeichnet. |
| Geplante `theory`-/`pheno`-/`batch`-/`orbifolder-io`-Bausteine entsprachen nicht dem Workspace. | Tatsächliche Crates/Module und fehlende Fähigkeiten in [02](./02_rust_architecture.md) dokumentiert. |
| Historische WSL-Versionen und Testzahlen konnten als heutiger Ubuntu-Befund gelesen werden. | Phasenberichte getrennt vom aktuellen Nachweis eingeordnet; [Dokumentationsindex](./README.md) ergänzt. |
| README begann mit einer PyPI-Installation, obwohl hier nur lokale Release-Konfiguration nachgewiesen ist. | Installation dieses Checkouts vorangestellt; PyPI-Befehle an eine verfügbare Veröffentlichung geknüpft. |
| Formulierung zu SUSY-Presets konnte automatisch vorhandene R-Regeln suggerieren. | Tatsächlich konfigurierte Regeln von separat getesteten expliziten R-Profilen unterschieden. |
| Geometrietest-Doku behauptete Zugriff auf lokale C++-Archive. | Tatsächliches Lesen der gebündelten, hashgeprüften Geometrietexte dokumentiert. |
| `.venv` und `.venv-release` enthielten leere, nicht ausführbare Python-Dateien und alte `/home/mwimmer/...`-Pfade. | Umgebungen gesichert und `.venv` mit `/usr/bin/python3` neu erstellt. |
| Conda-Base liefert Python 3.10.9, das Projekt verlangt >=3.11. | Projektinterpreter 3.12 eingerichtet und in Entwicklungsanleitung/VS-Code-Einstellungen explizit gewählt. |
| Rust fehlte; Editor-Erweiterungen für Rust und C++ fehlten. | Stable-Rust mit Cargo, rustfmt, Clippy, rust-src und rust-analyzer sowie VS-Code-rust-analyzer und Microsoft C/C++ installiert. |
| Aktuelles Clippy beanstandete zwei Indexschleifen. | Äquivalente Iteratoren in `anomalies/traces.rs` und im Smith-Normalform-Propertytest verwendet; keine Formeln oder Auswahlregeln geändert. |
| Release-Prüfskript installierte `dist/*.whl`, also möglicherweise alte/fremde Wheels. | Build und Installation auf ein frisches temporäres Wheel-Verzeichnis begrenzt; erfolgreiche Ausgabe anschließend nach `dist/` kopiert. |
| Linux-Wheel-Reparatur konnte von zufällig vorhandenen Systemwerkzeugen abhängen. | patchelf als Linux-Entwicklungsabhängigkeit und im normalen Linux-CI-Setup ergänzt. |
| Benchmark-Anleitung erwähnte den installierten Python-Teil und den festen C++-Buildpfad nicht. | Beide Voraussetzungen und der historische Messumfang ergänzt. |

## Implementierter Umfang

Das Projekt besteht aus einem exakten Rust-Kern, einem In-Memory-Datenadapter
und PyO3-Bindings mit einer kleinen Python-Fassade. Es berechnet Geometrie,
Einbettung, Zustände, Spektren, Tachyonen und Anomalien. Für SUSY existiert ein
Grad-1–3-Profil für Monomiale und Auswahlregeln. Modell-Batches können seriell
oder in einem expliziten Rayon-Pool laufen. Presets, Legacy-Katalogparser,
Summaries und installierte Typdeklarationen sind vorhanden.

Die sieben eingefrorenen C++-Modelle decken **4.300 masselose Felder und zwölf
Tachyonfelder** ab. Der rohe Legacy-Export zählt beim tachyonischen Modell
616 Felder einschließlich der zwölf Tachyonen; die neue getrennte Sicht zählt
604 masselose plus zwölf tachyonische Felder. Das ist keine zusätzliche
616-Felder-Massless-Referenz. Die bekannte unzuverlässige `Excited`-Kennzeichnung
wird nur für den dokumentierten Vergleich anhand der Oszillatorbelegung
normalisiert; die Original-Fixtures bleiben erhalten.

Nicht implementiert sind unter anderem Betafunktionen, allgemeine
SM-/PS-/SU(5)-Klassifikation, Zufallsmodellgenerierung, numerische VEV-/D-Term-
und Massenmatrixrechnungen sowie NumPy-Merkmalsexporte. Rust bietet formale
F-Term-Ableitungen, die Python-Fassade dafür keine öffentliche Methode.
Vollständige Spin(32)-Spektren, nichttriviale Torsions-Kopplungsprofile,
zusätzliche Torusgitter und höhere Kopplungsordnungen sind durch die sieben
Modelle nicht zertifiziert. Ein voller Port aller Legacy-CLI-Funktionen wird
nicht behauptet.

## Aktuelle Prüfungen

| Prüfung | Ergebnis auf diesem Ubuntu |
| --- | --- |
| `cargo fmt --all -- --check` | Bestanden. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Bestanden mit Rust 1.100.0. |
| `cargo test --workspace --locked` | **95 bestanden**, einschließlich seriell/1/2/4/8 Threads, Fehlern und parallelen Aufrufern. |
| Installiertes Release-Wheel / Python | **59 Tests bestanden** in 257,71 s; Import aus `.venv/.../site-packages`, Ausführung außerhalb des Checkouts mit gesperrtem Quellbaumzugriff. |
| README-Beispiele | Quickstart und exakter SUSY-/Kopplungsaufruf ausgeführt; 246 masselose Felder, keine Tachyonen im Z3-Standardfall. |
| `tests/check_typing.py` | mypy Strict, stubtest für drei Module und Pyright fehlerfrei; beide Checker verwerfen alle drei absichtlich ungültigen Aufrufe. VerifyTypes: 95,8 %, verbleibende externe Fraction-/stdlib-Grenze wie im Packaging-Bericht. |
| `compare.py --self-test --tachyon-physics` | Alle sieben Fixtures: Schema, Referenzen, exakte Metrik, Selbstvergleich und je sechs Korruptionsprüfungen bestanden; zwölf dokumentierte Legacy-Tachyonlabel-Abweichungen bestätigt. |
| Wheel-/sdist-Inhalte und `twine check --strict` | Bestanden; Presets, Stubs, Typmarker und Notices vorhanden, keine Legacy-Archive oder Build-Caches enthalten. |
| `cargo doc --workspace --no-deps --locked` | API-Dokumentation für alle drei Crates erfolgreich erzeugt. |
| Konfiguration und Verweise | TOML/VS-Code-JSON/Workflow-YAML lesbar, Paketversion 0.1.0 in Cargo/Python/CFF konsistent; lokale Doku-Verweise geprüft, optionale Archiv-/Buildlinks gesondert berücksichtigt. |
| `python -m pip check` | Keine widersprüchlichen Paketanforderungen. |
| Frische C++-Builds mit GCC/G++ 13.3 | SUSY und non-SUSY unter `tests/ground_truth/.build/ubuntu-24.04/` erfolgreich gebaut; beide CLI- und Modell-Smoke-Tests bestanden (MSSM0 bzw. Z3_1_1), Quellmanifeste unverändert. |

Die vorherigen WSL-Laufzeiten aus Bericht 09/10 sind historische Messungen und
werden nicht als neue Benchmarkwerte übernommen.

## Werkzeuge und lokale Grenzen

Die neue `.venv` enthält die Entwicklungswerkzeuge aus `pyproject.toml`,
einschließlich pytest, jsonschema, maturin/patchelf, mypy, Pyright, Twine und
PyYAML. Systemweit waren GCC/G++ 13.3, Make, GDB 15.1, Boost, GSL, Autoconf,
Automake und Libtool bereits vorhanden. CMake, Ninja und Clang werden von den
vorhandenen Builds nicht benötigt und wurden daher nicht ergänzt.

Beim Erstcheck fehlten `libreadline-dev`, `pkg-config` und `python3-venv`.
Der Nutzer hat diese inzwischen installiert; alle drei Pakete sind laut dpkg
vollständig eingerichtet. Der dabei verwendete Befehl war:

```bash
sudo apt install libreadline-dev pkg-config python3-venv
```

Die neue Python-Umgebung konnte mit vorhandenem uv bereits ohne diese
Root-Installation erstellt werden. Mit den jetzt verfügbaren Systempaketen
sind beide C++-Programme in einem frischen Build-Verzeichnis gebaut und per
CLI-/Modell-Smoke-Test geprüft. Die sieben eingefrorenen Oracle-Fixtures wurden
dafür nicht neu erzeugt. Anleitung: [Ubuntu-Setup](./11_development_ubuntu.md).

Die ungültigen Umgebungen und die vor der Bearbeitung gesicherten Dokumente
liegen lokal unter `~/.cache/orbifolder-audit/2026-11-14/`; dort werden auch die
Prüfprotokolle abgelegt. Das Git-Repository hat weiterhin keine Commits und
keinen Remote; sämtliche Projektdateien sind untracked. Ein lokaler erfolgreicher
Testlauf belegt keinen entfernten GitHub-Multi-OS-Lauf oder PyPI-Publish.
Der Repository-Link ist inzwischen in `pyproject.toml`, `CITATION.cff` und
allen drei Cargo-Paketen hinterlegt. Die frühere Cargo-Warnung über fehlende
Projekt-URLs ist damit behoben.
Rust 1.85 ist die deklarierte Mindestversion, nicht die in diesem Durchlauf
getestete Toolchain. Andere Betriebssysteme/Python-Versionen und historische
Performancezahlen werden durch diesen Ubuntu-Lauf nicht erneut zertifiziert.


## Repository-Metadaten und Action-Versionen

Die Einträge in `[project.urls]` und `repository-code` sind korrekt platziert.
`CITATION.cff` besteht die Validierung gegen das offizielle CFF-Schema 1.2.0;
Version und Lizenz stimmen mit den Paketmetadaten überein. Ein Release-Datum
oder DOI wird erst eingetragen, wenn eine tatsächliche Veröffentlichung vorliegt.
Die URL `https://github.com/AlphaMegaladon/orbifolder` liefert beim anonymen Abruf
noch HTTP 404; das kann ein privates oder noch nicht angelegtes Repository sein.
Die URL ist dadurch nicht syntaktisch falsch. Die Metadaten konfigurieren
keinen Git-Remote und keinen PyPI Trusted Publisher.

Am 2026-11-14 gegen die offiziellen GitHub-Releases geprüft und aktualisiert:

| Action | Workflow-Referenz | Aktuell veröffentlichte Version |
| --- | --- | --- |
| [checkout](https://github.com/actions/checkout/releases/tag/v7.0.1) | `actions/checkout@v7` | 7.0.1 |
| [setup-python](https://github.com/actions/setup-python/releases/tag/v7.1.0) | `actions/setup-python@v7` | 7.1.0 |
| [upload-artifact](https://github.com/actions/upload-artifact/releases/tag/v7.0.2) | `actions/upload-artifact@v7` | 7.0.2 |
| [download-artifact](https://github.com/actions/download-artifact/releases/tag/v8.0.2) | `actions/download-artifact@v8` | 8.0.2 |
| [maturin-action](https://github.com/PyO3/maturin-action/releases/tag/v1.51.0) | `PyO3/maturin-action@v1` | 1.51.0 |
| [PyPI-Publish](https://github.com/pypa/gh-action-pypi-publish/releases/tag/v1.14.3) | `pypa/gh-action-pypi-publish@release/v1` | 1.14.3 |

Die Major-Tags folgen Aktualisierungen innerhalb derselben Action-Hauptversion;
auf eine neue Hauptversion wechseln sie nicht automatisch. Beide Workflow-Dateien
bestehen actionlint 1.7.12. Die normalen CI-Berechtigungen sind auf
`contents: read` begrenzt; der Release-Publish-Job besitzt weiterhin seinen
separaten OIDC-Vertrag.

`setup-python` verwendet nun ausdrücklich `check-latest: true`, also die neueste
Patchversion innerhalb der angeforderten Reihen 3.11 und 3.14. Diese Reihen
prüfen Mindestversion und bisher verifizierte obere Grenze des Projekts.
[Python 3.15](https://www.python.org/downloads/) ist inzwischen verfügbar,
aber noch nicht Bestandteil der verifizierten Matrix. Das Action-Update
behauptet keine zusätzliche Python-3.15-Zertifizierung.
`dtolnay/rust-toolchain@stable` wählt aktuelles Stable-Rust. Dagegen erhalten
`--locked`-Aufrufe genau die Versionen aus `Cargo.lock`, nicht automatisch die
neuesten Crate-Versionen. Python-Werkzeuge werden innerhalb der angegebenen
Versionsgrenzen aufgelöst; der PyPI-Buildvertrag für maturin bleibt `>=1.10,<2`.
