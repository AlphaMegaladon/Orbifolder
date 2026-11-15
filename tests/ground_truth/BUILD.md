# Legacy-Builds – Schritt 2.1

Historischer Build-Nachweis vom 2026-10-30 unter WSL. Die folgende Versionstabelle und die erfolgreichen Laufzeittests beziehen sich auf diesen damaligen Rechner, nicht auf die aktuelle Ubuntu-Installation. Aktuelles Setup und Paketliste: [Ubuntu-Entwicklungsanleitung](../../docs/11_development_ubuntu.md); heutige Ergebnisse: [Repository-Prüfung](../../docs/12_repository_audit.md).

Für einen frischen Ubuntu-Rechner werden `build-essential`, `autoconf`, `automake`, `libtool`, `libboost-dev`, `libgsl-dev` und `libreadline-dev` benötigt. Vorhandene Build-Verzeichnisse aus WSL nicht auf einem anderen Rechner weiterverwenden; ein frisches absolutes `LEGACY_BUILD_ROOT` wählen.

## Verifizierte Umgebung

| Bestandteil | Version |
| --- | --- |
| System | Ubuntu 26.04.1 LTS, x86_64, WSL2 |
| Compiler | GCC/G++ 15.2.0 |
| Make | 4.4.1 |
| Autoconf / Automake / Libtool | 2.72 / 1.18.1 / 2.5.4 |
| Boost | 1.90 |
| GSL | 2.8 |
| readline | 8.3 |
| glibc | 2.43 |

Build-Flags: `-std=c++11 -O2 -g -fno-fast-math`, zwei parallele Compilerprozesse. Vollständige Paketversionen, Include-Pfade und Ausführungsumgebung stehen pro Variante in den generierten Build-Metadaten.

## Reproduzieren

Aus dem Repository-Hauptverzeichnis unter Linux (einschließlich WSL):

```bash
bash tests/ground_truth/build_legacy.sh susy
bash tests/ground_truth/smoke_legacy.sh susy
bash tests/ground_truth/build_legacy.sh non_susy
bash tests/ground_truth/smoke_legacy.sh non_susy
```

[build_legacy.sh](./build_legacy.sh) erzeugt getrennte Quellkopien und Build-Verzeichnisse unter `.build/<Variante>/source` beziehungsweise `.build/<Variante>/build`. [smoke_legacy.sh](./smoke_legacy.sh) führt die Originalprogramme aus. `BUILD_JOBS` steuert die Compilerparallelität; `LEGACY_BUILD_ROOT` kann ein anderes absolutes Arbeitsverzeichnis vorgeben. Die [VS-Code-Tasks](../../.vscode/tasks.json) enthalten dieselben WSL-Build-Aufrufe.

Vorhandene Arbeitskopien werden nur wiederverwendet, wenn ihr gespeichertes Ursprungsmanifest weiterhin zum Archiv passt. Für einen vollständigen Neuaufbau kann ein bisher unbenutztes `LEGACY_BUILD_ROOT` gewählt werden. Build-Produkte und Rohprotokolle sind [vom Versionsverzeichnis ausgeschlossen](./.gitignore); [Shellskripte behalten LF-Zeilenenden](../../.gitattributes).

## Notwendige Build-Anpassungen

| Befund | Maßnahme ausschließlich im Build bzw. in der Arbeitskopie |
| --- | --- |
| Relative Include-Pfade zeigen beim separaten Build auf Verzeichnisse ohne Quellheader. | Die vier Quellverzeichnisse werden über explizite `CPPFLAGS` eingebunden. |
| Das alte Libtool erkennt beim Konfigurieren Bash-Fähigkeiten wie `+=`, wird sonst aber über `/bin/sh` gestartet. | `CONFIG_SHELL=/bin/bash` und `make SHELL=/bin/bash` verwenden konsistent dieselbe Shell. |
| non-SUSY liefert Build-Skripte/Vorlagen mit CRLF; `configure` scheitert damit bereits syntaktisch. | Nur Build-Dateien in der Arbeitskopie werden nach LF normalisiert. `normalized-build-files.txt` führt die betroffenen Dateien auf; C++-Dateien bleiben unverändert. |
| Zeitstempel beziehungsweise das fehlende non-SUSY-`aclocal.m4` lösen eine Regeneration mit historischem `aclocal-1.16` aus. | Die mitgelieferten `configure`-/`Makefile.in`-/Header-Vorlagen werden über GNU-Make-`--old-file` vor automatischer Maintainer-Regeneration geschützt. `configure` wird bei jedem Build ausdrücklich ausgeführt. |
| Das SUSY-Wurzelziel enthält Quelldaten-Wildcards wie `*.txt`, die im separaten Build-Verzeichnis nicht auflösbar sind. | Die originalen Codeziele werden in Abhängigkeitsreihenfolge gebaut: `io`, `linalg`, `groupTheory`, `orbifolder`. Für Laufzeittests dient die Quellkopie mit den mitgelieferten Geometrie-/Modelldaten als Arbeitsverzeichnis. |
| Der Compilerstandard soll unabhängig vom GCC-Default feststehen. | Explizit C++11 entsprechend der non-SUSY-Konfiguration; kein `fast-math` und kein `-fpermissive`. |

**Kein C++-Quellcode-Patch war erforderlich.** Sämtliche 70 SUSY- und 64 non-SUSY-C++-Quell-/Headerdateien einschließlich der Beispiele stimmen bytegenau mit ihren Archivvorlagen überein. Verglichen werden SHA-256-Manifeste vor beziehungsweise nach dem Build. Zusätzlich bestätigt ein vollständiges Archivmanifest, dass die ursprünglichen Legacy-Verzeichnisse nicht verändert wurden; `Zone.Identifier`-Begleitdateien sind von diesem Manifest ausgenommen.

## Erfolgreiche Laufzeittests

| Programm | CLI-Test | Modelltest |
| --- | --- | --- |
| [SUSY-Binary](./.build/susy/build/src/orbifolder/orbifolder) | Startbanner, `help`, `exit orbifolder`; Exitcode 0. | Erstes vollständiges Modell `MSSM0` aus den archivierten [Z₃×Z₃-Modellen mit Wilsonlinie](../../legacy_cpp/susy_orbifolder/Z3xZ3_1_1-1WL-77PatricksModels.txt) geladen; Ladebestätigung und Exitcode 0. |
| [non-SUSY-Binary](./.build/non_susy/build/src/orbifolder/nonSUSYorbifolder) | Startbanner, `help`, `exit orbifolder`; Exitcode 0. | Archiviertes [Z₃-Modell](../../legacy_cpp/non_susy_orbifolder/Models/ZN_models/modelZ3_1_1.txt) als `Z3_1_1` geladen; Ladebestätigung und Exitcode 0. |

`ldd` findet bei beiden Programmen sämtliche Laufzeitbibliotheken. CLI-Tests haben ein Limit von 30 Sekunden, Modelltests von 120 Sekunden; ein Timeout oder Fehlercode lässt das Skript scheitern. Die Modelltests benutzen das unveränderte CLI einschließlich dessen Modellkonstruktion; sie ersetzen noch keine strukturierte Spektrums-/Anomalievalidierung aus Schritt 2.2/2.3.

## Nachweise pro Variante

Unter `.build/susy/` und `.build/non_susy/` entstehen:

- `environment.txt`: Werkzeug-/Paketversionen, Compilerflags und Build-Konfiguration.
- `original.sha256`, `original-after.sha256`: unveränderte Archivdaten.
- `cpp-original.sha256`, `cpp-built.sha256`: bytegleiche C++-Dateien.
- `binary.sha256`: Hash des tatsächlich gebauten Programms.
- `normalized-build-files.txt`: ausschließlich technische Zeilenendekorrekturen.
- `logs/configure.log`, `logs/make-*.log`: Konfiguration und originale Compiler-/Linkeraufrufe.
- `logs/ldd.log`, `logs/smoke-cli.log`, `logs/smoke-model.log`: Laufzeitnachweise.

Die Binärhashes identifizieren den konkreten Build. Wegen `-g` eingebettete absolute Pfade erlauben keine Behauptung ortsunabhängiger Bitreproduzierbarkeit.
