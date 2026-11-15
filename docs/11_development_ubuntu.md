# Entwicklungsumgebung unter Ubuntu

Stand: 2026-11-14. Zielsystem dieses Checkouts: Ubuntu 24.04 LTS, x86_64.
Der Python-Teil benötigt Python >=3.11, die Rust-Pakete deklarieren Rust >=1.85
und Edition 2024. Für Entwicklung und CI wird aktuelles Stable mit rustfmt und
Clippy verwendet. Die neue lokale Prüfung steht im
[Repository-Bericht](./12_repository_audit.md).

## Systempakete

Für Rust/Python aus dem Quellbaum:

```bash
sudo apt update
sudo apt install build-essential curl ca-certificates python3-dev python3-venv pkg-config
```

Die ausgelieferten Binär-Wheels benötigen keine Compiler. Der Rust-Kern nutzt
Rust-Bibliotheken für exakte Arithmetik; GMP, GSL und Boost sind keine
Abhängigkeiten des neuen Kerns. CMake, Ninja und Clang sind für die vorhandenen
Builds ebenfalls nicht erforderlich: Rust verwendet Cargo, die C++-Archive
verwenden Autotools und Make.

Nur für die optionalen alten C++-Programme und ihre Probes zusätzlich:

```bash
sudo apt install autoconf automake libtool libboost-dev libgsl-dev libreadline-dev gdb
```

Die Skripte bauen mit G++ und `-std=c++11 -O2 -g -fno-fast-math`. Sie protokollieren
auch die Autotools-Versionen, obwohl sie die mitgelieferten `configure`- und
`Makefile.in`-Dateien verwenden und keine Maintainer-Regeneration benötigen.

## Rust im Benutzerkonto

Installation nach der [offiziellen Rust-Anleitung](https://rust-lang.org/install):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --profile minimal --component rustfmt --component clippy --component rust-src --component rust-analyzer
source "$HOME/.cargo/env"
rustc --version
cargo --version
```

Bei bereits installiertem rustup reichen `rustup update stable` und
`rustup component add rustfmt clippy rust-src rust-analyzer`. Dafür sind keine
Root-Rechte nötig. Nach der erstmaligen Installation VS Code neu starten, damit
auch dessen Prozesse `~/.cargo/bin` im Suchpfad finden.

## Projektumgebung statt Conda-Base

Alle folgenden Kommandos laufen im Repository-Hauptverzeichnis:

```bash
source "$HOME/.cargo/env"
/usr/bin/python3 --version
/usr/bin/python3 -m venv .venv
source .venv/bin/activate
python --version
python -m pip install '.[dev]'
```

`/usr/bin/python3` ist auf Ubuntu 24.04 Python 3.12. Im hier vorhandenen Conda-
`(base)` ist `python3` dagegen Python 3.10.9 und damit zu alt. Die Aktivierung
der Projektumgebung setzt deren Python vor Conda in den Suchpfad. Conda muss
dafür weder entfernt noch systemweit umkonfiguriert werden.

Als Alternative kann das bereits vorhandene uv die Umgebung ohne Ubuntus
`python3-venv` anlegen und pip mitbringen; siehe
[uv-Dokumentation](https://docs.astral.sh/uv/pip/environments/):

```bash
uv venv --python /usr/bin/python3 --seed .venv
source .venv/bin/activate
python -m pip install '.[dev]'
```

`.[dev]` installiert das lokale Paket, maturin, pytest, jsonschema, mypy,
Pyright, Twine, PyYAML und unter Linux patchelf für die Wheel-Reparatur. Nach
Rust-Änderungen die native Erweiterung erneut bauen:

```bash
maturin develop --release --locked
```

Virtuelle Umgebungen sind nicht portabel. Bei einem kopierten Checkout eine
defekte `.venv` zuerst unter einem anderen Namen außerhalb des Repositorys
sichern und anschließend neu erstellen. Auch `target/`, `dist/` und
`tests/ground_truth/.build/` sind lokale Build-Produkte, keine Nachweise eines
erfolgreichen Builds auf diesem Rechner.

## Prüfungen

```bash
bash tests/verify_release.sh
python tests/check_typing.py
python tests/ground_truth/compare.py --self-test --tachyon-physics
```

`verify_release.sh` führt Formatprüfung, striktes Clippy, alle Rust-Tests, einen
Release-Wheel-Build und die Python-Suite gegen das installierte Wheel aus. Es
wählt `.venv` und deren Python ausdrücklich aus. Das neu gebaute Wheel wird
zuerst in einem separaten temporären Verzeichnis installiert und nach bestandenem
Test nach `dist/` kopiert. Bereits vorhandene Wheels aus anderen Builds werden
nicht zur Installation ausgewählt.

`run_installed_tests.py` führt die Python-Tests außerhalb des Checkouts aus und
verbietet dem Kindprozess Dateizugriffe auf den Quellbaum. Rust-Tests lesen die
mitgelieferten Referenzen; dafür müssen die alten C++-Programme nicht gebaut
sein. `cargo test --workspace` umfasst auch das PyO3-Paket und benötigt einen
kompatiblen Python-Interpreter. Zum isolierten Testen des Fachkerns genügt:

```bash
cargo test --locked -p orbifolder-core
```

Paketierung zusätzlich prüfen:

```bash
maturin sdist --out dist
python tests/check_distributions.py
python -m twine check --strict dist/*
python tests/verify_sdist.py dist/orbifolder-0.1.0.tar.gz
```

Der letzte Befehl baut und testet Rust erneut aus dem entpackten Quellpaket in
einem frischen Target-Verzeichnis. Das ist eine zusätzliche Release-Prüfung;
Versionsnummern in Pfaden müssen zur aktuellen Paketversion passen.

## Optionale C++-Referenzprogramme

Zuerst die Archive entsprechend dem [Oracle-README](../tests/ground_truth/README.md)
unter `legacy_cpp/` bereitstellen. Für eine von einem anderen Rechner kopierte
Build-Struktur ein neues absolutes Arbeitsverzeichnis wählen:

```bash
export LEGACY_BUILD_ROOT="$PWD/tests/ground_truth/.build/ubuntu-24.04"
bash tests/ground_truth/build_legacy.sh susy
bash tests/ground_truth/smoke_legacy.sh susy
bash tests/ground_truth/build_legacy.sh non_susy
bash tests/ground_truth/smoke_legacy.sh non_susy
```

Ein bereits verwendetes Verzeichnis ist für weitere Builds auf demselben
Rechner geeignet, solange das gespeicherte Ursprungsmanifest stimmt. Die
Skripte prüfen bytegleiche C++-Quellen und verändern die Archive nicht.
Referenzen nicht im normalen Entwicklungslauf neu erzeugen; einen neuen
Oracle-Lauf zuerst in ein Review-Verzeichnis schreiben. Details und Grenzen
stehen in [BUILD.md](../tests/ground_truth/BUILD.md).

## VS Code

Die Workspace-Empfehlungen enthalten rust-analyzer, Microsoft Python/Pylance
und Microsoft C/C++. Manuell installieren:

```bash
code --install-extension rust-lang.rust-analyzer
code --install-extension ms-vscode.cpptools
```

Die lokale, ignorierte `.vscode/settings.json` kann `.venv/bin/python` vorschlagen
und über `rust-analyzer.cargo.extraEnv.PYO3_PYTHON` an Rust-Builds übergeben.
Die gemeinsamen Tasks und Erweiterungsempfehlungen werden versioniert, lokale
Interpretereinstellungen dagegen nicht. Falls VS Code bereits einen anderen
Interpreter gespeichert hat, über **Python: Select Interpreter** ausdrücklich
die Projektumgebung auswählen. Die Tasks bieten Rust-Tests, Wheel-Build,
Gesamtprüfung und die optionalen Legacy-Builds. Die Linux-Pfade in den
Workspace-Einstellungen bei Windows-nativer Entwicklung entsprechend anpassen.
