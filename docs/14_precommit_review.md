# Portierungsumfang, Python-Paket und erster Commit

Prüfdatum: 2026-11-15, Ubuntu 24.04 x86_64. Dieser Bericht ergänzt die
[Prüfung vom Vortag](./12_repository_audit.md). Neue Ergebnisse und Messwerte
werden hier dem jeweiligen Prüfumfang zugeordnet.

## Funktionsumfang gegenüber den Originalprogrammen

Der Rust-Port ist eine neue Bibliothek für den dokumentierten Rechenkern,
kein vollständiger Ersatz sämtlicher SUSY-/non-SUSY-C++-Funktionen.

| Bereich | Aktueller Rust-/Python-Stand |
| --- | --- |
| Exakte Mathematik, Raumgruppen und Eich-Einbettung | Implementiert mit geprüfter rationaler Arithmetik und expliziten Ressourcenfehlern. Einbettung deckt ein eingeschränktes Profil ab. |
| Masselose Zustände, Felder und Tachyonen | Implementiert, gegen sieben Modelle sowie ergänzende Probes/Invarianten geprüft; keine vollständige massive Stringzustandsturm-Berechnung. |
| Anomalien, kanonische U(1)-Basis und SUSY-FI-Koeffizient | Implementiert; vorgegebene Konsistenzprüfungen und bekannte Legacy-Fehler sind getrennt dokumentiert. |
| SUSY-Monomiale und Auswahlregeln | Grad 1–3, getrennte Tensor-/Polynominvarianten; keine vollständige Portierung aller möglichen höheren Kopplungsordnungen und keine numerischen Amplituden. |
| Formale F-Term-Ableitungen | In Rust vorhanden, keine öffentliche Python-Methode; kein vollständiger F-/D-Flachheits- oder VEV-Löser. |
| Geometrien und Elternkonstruktionen | Drei konkrete Presetfamilien mit sieben Modellreferenzen; strukturierte Eingaben möglich. Spin(32)-Grundbausteine sind vorhanden, vollständige Spin(32)-Spektren nicht durch diese Suite zertifiziert. |
| Torsion, frei wirkende Wilsonlinien und Rototranslations-Einbettungen | Teilweise mathematische Verträge, Einschränkungen bzw. explizite Ablehnung im Rechenprofil; keine allgemeine Legacy-Parität. |
| Zufallsmodelle, Äquivalenz-/Duplikatsuche | Nicht portiert. Batch-Auswertung vorhandener Modelle ersetzt keinen Modellgenerator oder Äquivalenzklassifikator. |
| SM-/PS-/SU(5)-Analyse, Hyperladungsfindung, Generationserkennung | Nicht portiert. Die Rohdarstellungen und Ladungen stehen für eigene Analysen bereit. |
| Betafunktionen und RG-Entwicklung | Nicht implementiert; die C++-Programme besitzen bereits Betafunktionsroutinen. |
| VEV-Konfigurationen, weitere Symmetriebrechung und effektive Massenmatrizen | Kein vollständiger Port der entsprechenden SUSY-C++-Analysen bzw. der VEV-Konfigurationsverwaltung. |
| Legacy-CLI, Skriptinterpreter, Sitzungen und Formatierer | Bewusst nicht als Python-Bibliotheksoberfläche nachgebaut. Modelltextparser ist kein vollständiger CLI-/Sitzungsimport. |
| Python-Ergonomie, deterministische parallele Batches und Summaries | Neue öffentliche API mit eingefrorenen Objekten, exakten Zahlen und Worker-Projektion. |

Quellen für die ursprünglichen Fähigkeiten sind die
[SUSY-Arbeit](https://arxiv.org/html/1110.5229v1), die
[non-SUSY-Arbeit](https://arxiv.org/html/2504.20137v1) sowie die lokal separat
bereitgestellten Klassen `CAnalyseModel`, `CRandomModel`, `CMonomial`,
`CYukawaCouplings`, `CMassMatrix` und die CLI-Implementierungen.
Die Archive selbst bleiben vom Repository ausgeschlossen.

Die sieben Referenzen sind keine fest eingebauten Spektrumsantworten. Der Core
rechnet aus Eingaben; die unabhängigen erwarteten Ausgaben werden erst im Test
verglichen. Die begrenzte Referenzabdeckung beweist keine Vollständigkeit des
Eingaberaums und keine automatische Gültigkeit aller erweiterten Profile.

## Korrektheit und Performance

Die dokumentierten Verbesserungen betreffen konkrete Verträge und
Regressionen: rationale Entscheidungen statt Float-Toleranzen, BigInt-Promotion,
vollständige Tachyon-Normschranken im unterstützten Profil sowie korrigierte
Wilson-, Anomalie- und SUSY-Invariantenprüfungen. Das belegt keine pauschale
Fehlerfreiheit oder Überlegenheit jeder einzelnen Funktion.

Der Benchmark vergleicht zwei unterschiedliche Arbeitsumfänge: Rust löst
vorbereitete In-Memory-Modelle einschließlich Anomalien, C++ wird pro Modell
als Probe-Prozess gestartet und lädt Eingaben, rechnet zusätzliche Cubic-Proben
und schreibt JSON. Rust-Exportkosten werden getrennt gemessen. Der Quotient
dieser Zeiten ist daher kein isolierter Algorithmus-Speedup.

Neue Messung auf diesem Rechner: AMD Ryzen 7 5700U, Ubuntu 24.04 x86_64,
Rust 1.100.0, Release mit Overflow-Checks, drei Wiederholungen. Keine parallel
laufenden eigenen Test-Suiten während der Messung; normale Workstation-
Hintergrundlast bleibt möglich. Rohdaten und vollständige Provenienz stehen in
[ubuntu-2026-11-15.json](../tests/benchmark/ubuntu-2026-11-15.json).

| Ausführung der sieben Modelle | Median in Sekunden | Gegen Rust seriell |
| --- | ---: | ---: |
| Rust seriell | 3,031 | 1,00× |
| Rayon 1 Thread | 3,067 | 0,99× |
| Rayon 2 Threads | 1,767 | 1,72× |
| Rayon 4 Threads | 1,469 | 2,06× |
| Rayon 8 Threads | 1,149 | 2,64× |
| C++-Probe-Batch, anderer Messumfang | 13,183 | kein direkter Algorithmus-Speedup |

1000 Auswertungen in 32er-Chunks mit acht Threads: **110,198 s**, etwa
**9,07 Auswertungen/s**, 614.356 masselose Feldobjekte. Der Batch wiederholt
die sieben Referenzeingaben; er umfasst keine 1000 unterschiedlichen Modelle.
Die gewöhnliche Sieben-Modell-Rechnung ergibt weiterhin 4.300 masselose und
zwölf tachyonische Felder.

Der kanonische JSON-Voll-Export der sieben Modelle benötigt separat **6,019 s**
für 31.340.497 Byte. Die Python-Dictionary-Konvertierung aller Vollresultate
benötigt **9,642 s**. Das maximale RSS des gesamten Rust-Benchmarkprozesses
beträgt 1.193.208 KiB (rund 1,14 GiB), einschließlich Voll-Export; dies ist keine
isolierte Solver-Speichermessung. Große Voll-Exporte können den Rechenzeitgewinn
aufzehren. Die vorhandene Summary-API vermeidet diese vollständige Ausgabe,
wurde hier aber nicht als eigener Geschwindigkeitsbenchmark gemessen.

Belastbar ist damit die gemessene Parallelbeschleunigung für diese Eingaben.
Eine allgemeine Überlegenheit gegenüber jedem C++-Arbeitsablauf ist nicht
nachgewiesen. Ein zusätzlicher Vergleich mit identischem Physikumfang und
identischen Ein-/Ausgabegrenzen bleibt sinnvoll.

## Paketierung und Repository

Die Veröffentlichung als GPL-3.0-or-later-Beta kann den nachgewiesenen
Funktionsumfang bereitstellen. Feature-Parität mit den Originalprogrammen ist
keine Voraussetzung für einen ersten Commit, muss aber ehrlich abgegrenzt sein.

- `Cargo.lock`, Quellcode, Tests und eingefrorene Referenzen gehören in Git.
  Die README verwendet absolute Repository-Links, damit ihre Doku-Verweise
  auch in der PyPI-Darstellung zum Quellrepository führen. `ROADMAP.md` wird
  zusätzlich ins Quellpaket aufgenommen.
- `.vscode/extensions.json` und `.vscode/tasks.json` enthalten gemeinsame
  Empfehlungen/Arbeitsabläufe und bleiben versioniert. `.vscode/settings.json`
  und sonstige lokale Workspace-Dateien werden ignoriert; die vorhandene
  lokale Interpretereinstellung bleibt auf diesem Rechner erhalten.
- Venvs, `target/`, `dist/`, Oracle-Builds, Caches und separate C++-Archive
  bleiben ignoriert. Lokale `.env`-Dateien werden ebenfalls ignoriert;
  eine bewusste `.env.example` darf versioniert werden.
- Künftige physikalische Erweiterungen sind in [ROADMAP.md](../ROADMAP.md)
  festgehalten, getrennt von aktuell vorhandenen Fähigkeiten.

Beim Check der deklarierten Rust-Mindestversion 1.85.0 wurde ein temporärer
Referenzwert in `susy_invariants.rs` nicht akzeptiert (`E0716`). Eine explizite
Bindung behebt ausschließlich diesen Test-Kompatibilitätsfehler. Danach
besteht `cargo +1.85.0 check --workspace --all-targets --locked`; die betroffene
sechs Tests umfassende Suite, Formatierung und striktes Clippy bestehen unter
Stable. Ein zusätzlicher CI-Job prüft künftig die deklarierte Mindestversion.

Abgeschlossene Prüfungen dieses Durchlaufs:

| Prüfung | Ergebnis |
| --- | --- |
| Formatierung / striktes Clippy | Bestanden. |
| Frisches entpacktes sdist, ohne Legacy-Verzeichnis | **95 Rust-Tests bestanden**, eigener neuer Cargo-Target-Pfad. |
| Installiertes ABI3-Wheel unter CPython 3.12.3 | **59 Tests bestanden** in 293,98 s, außerhalb des Checkouts mit gesperrten Quellbaumzugriffen. |
| mypy / stubtest / Pyright | Strict-Client, drei Runtime-Module und alle absichtlich falschen Aufrufe geprüft; VerifyTypes 95,8 % mit dokumentierter externer Fraction-Grenze. |
| Rust 1.85.0 | Workspace einschließlich aller Test-/Example-Targets mit `--locked` kompilierbar. |
| GitHub-Workflows | actionlint 1.7.12 fehlerfrei; zusätzlicher MSRV-Job vorhanden. |
| Git-Hygiene | Gemeinsame VS-Code-Dateien sichtbar, lokale Settings/Builds/Venvs/Archive ignoriert; Credential-Musterprüfung ohne Treffer. |
| Dasselbe ABI3-Wheel unter CPython 3.11.16 | **59 Tests bestanden** in 312,66 s; keine Rust-/C++-Installation in dieser Python-Umgebung erforderlich. |
| Dasselbe ABI3-Wheel unter CPython 3.14.7 | **59 Tests bestanden** in 308,62 s; Quellbaumzugriffe des Testprozesses gesperrt. |
| Unabhängiger sdist→Wheel-Build | `pip wheel --no-deps ...tar.gz` erfolgreich in Build-Isolation. |
| Inhalte / Metadaten | `check_distributions.py` und `twine check --strict` für Wheel und sdist bestanden; Roadmap und MSRV-Fix im sdist, absolute README-Links im Wheel. |
| Frisch installiertes sdist-Wheel | Zwei isolierte Ressourcen-/Parser-Tests bestanden (57 übrige Tests bewusst nicht wiederholt); die vollständige Physik-/API-Suite wurde zuvor auf dem Release-Wheel geprüft. |
| Benchmark | Frische C++-Probes und Rust-Release-Example, drei Wiederholungen plus 1000er-Batch erfolgreich; Rohdaten/Quell-/Lock-/Objekthashes gespeichert. |

Lokale Tests ersetzen keinen tatsächlichen GitHub-Multi-OS-Lauf. Vor PyPI-Publishing sind der eigene
Projektname/Trusted Publisher und das GitHub-Environment `pypi` einzurichten.


## Konkrete Vorbereitung des ersten Commits

GitHub-Repository geprüft: `AlphaMegaladon/Orbifolder`, öffentlich, leer,
Standardbranch `main`. Lokal ist der Branch noch `master`, es gibt keine
Commits und keinen Remote. Nach Review kann der Nutzer den ersten Commit
und die Verbindung beispielsweise so erstellen:

```bash
git branch -m main
git remote add origin https://github.com/AlphaMegaladon/Orbifolder.git
git add .
git diff --cached --stat
git commit -m "Initial Rust/Python orbifold engine"
git push -u origin main
```

Diese Kommandos sind eine Anleitung; diese Prüfung führt sie nicht aus.
`git add --dry-run .` und `git check-ignore` prüfen vorher den vorgesehenen
Dateiumfang. Die Fixtures sind reguläre kleine JSON-Dateien; die größte liegt
bei rund 5,35 MiB, der gesamte Quell-/Referenzbestand bei rund 21 MiB. Dafür
ist kein Git LFS erforderlich. Ein Scan der Git-sichtbaren Dateien auf
Private-Key-Blöcke, bekannte GitHub-Tokenformate und AWS-Zugriffsschlüssel
fand keine Treffer. Das ist eine begrenzte Musterprüfung, keine Garantie für
beliebige Geheimnisse oder personenbezogene Inhalte.


## Urteil vor dem ersten Commit

**Für einen ersten Commit als klar begrenzte Python/Rust-Beta bereit.** Der
implementierte Kern ist getestet und paketierbar, die fehlende Legacy-Parität
wird direkt in README und dieser Matrix offengelegt. Für eine Behauptung
„vollständiger Ersatz beider Originalprogramme“ oder „universell schneller“
reicht der Stand ausdrücklich nicht.

Das lokale Linux-Wheel ist praktisch geprüft. Windows, macOS und Linux ARM64
sind bislang über den Release-Workflow vorgesehen; vor einer plattformweiten
Veröffentlichung muss ein tatsächlicher manueller Workflow-Lauf ohne Publish
auf GitHub bestehen. Der erste normale Branch-Push startet die CI; erst ein
Release-Tag oder ein ausdrücklich aktivierter manueller Publish-Lauf startet
nach erfolgreichen Prüfungen den PyPI-Publish-Job.
