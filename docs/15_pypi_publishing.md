# PyPI-Veröffentlichung und GitHub-Environment

Stand: 2026-11-16. Repository: `AlphaMegaladon/Orbifolder`,
Workflow: `.github/workflows/release.yml`, Environment: `pypi`.

## Einmalige Einrichtung

1. PyPI-Konto anlegen, E-Mail bestätigen und Zwei-Faktor-Authentifizierung
   einrichten.
2. In PyPI unter Account → Publishing einen GitHub Pending Trusted Publisher
   mit diesen Angaben registrieren:

   | Feld | Wert |
   | --- | --- |
   | PyPI project name | `orbifolder` |
   | Repository owner | `AlphaMegaladon` |
   | Repository name | `Orbifolder` |
   | Workflow filename | `release.yml` |
   | Environment name | `pypi` |

3. Auf GitHub unter Settings → Environments das Environment `pypi` anlegen.
   Der Workflow nutzt OIDC mit `id-token: write`; ein PyPI-API-Token als
   GitHub-Secret ist dafür nicht erforderlich.

Ein Pending Publisher erstellt das Projekt beim ersten erfolgreichen Upload
und reserviert den Paketnamen vorher nicht.
[PyPI-Anleitung](https://docs.pypi.org/trusted-publishers/creating-a-project-through-oidc/).

## Empfohlene Environment-Einstellungen

Für den aktuellen Ablauf mit Tag-Releases und optionalem manuellem Publish
von `main`:

| Einstellung | Empfehlung |
| --- | --- |
| Deployment branches and tags | `Selected branches and tags`: eine Branch-Regel `main` und eine getrennte Tag-Regel `v*`. |
| Required reviewers | Den eigenen GitHub-Account `AlphaMegaladon` eintragen, wenn vor jedem Upload ein zusätzlicher Freigabeklick gewünscht ist. |
| Prevent self-review | Bei nur einem Maintainer ausgeschaltet lassen, damit der Auslöser seinen eigenen Publish-Lauf freigeben kann. |
| Wait timer | Ausgeschaltet; für diesen Ablauf nicht erforderlich. |
| Allow administrators to bypass configured protection rules | Ausschalten, wenn auch der eigene Admin-Account die Freigaberegel einhalten soll. |
| Environment secrets | Für das vorhandene Trusted Publishing keine PyPI-Zugangsdaten eintragen. |

Die Branch-Regel `main` ist nur für einen ausdrücklich aktivierten manuellen
Publish nötig. Wer ausschließlich Tags veröffentlichen möchte, kann nur die
Tag-Regel `v*` erlauben. Regeln für Branches und Tags werden in der GitHub-UI
getrennt angelegt. Details:
[GitHub-Environment-Regeln](https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments).

Die Freigabe betrifft ausschließlich den Publish-Job. Ein manueller Testlauf
mit `publish=false` überspringt diesen Job und benötigt daher keine
Environment-Freigabe. Review-Regeln sind eine zusätzliche Entscheidung des
Maintainers und keine technische Voraussetzung von Trusted Publishing.

## Testlauf und tatsächliche Veröffentlichung

Nach Push der gewünschten Änderungen unter Actions den Workflow
`Build, test and publish distributions` auf `main` neu starten. Den
Publish-Schalter zunächst ausgeschaltet lassen. Ein Re-run eines älteren
Laufs verwendet dessen bisherigen Commit und damit dessen alte Workflow-Datei;
für einen Workflow-Fix einen neuen manuellen Lauf auf dem aktualisierten
`main` anlegen.

Nach erfolgreichen Builds und Tests kann der zur Paketversion passende Tag
gepusht werden, für Version 0.1.0 beispielsweise:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Der Tag startet den Workflow und nach erfolgreichen Prüfungen den Publish-Job.
Wenn Required reviewers konfiguriert sind, dort anschließend die Freigabe
erteilen. Versionsprüfung, Wheel-/sdist-Metadaten und Trusted Publishing sind
bereits im Workflow enthalten.

## Runner-Korrektur nach dem ersten Testlauf

Im [ersten manuellen Lauf](https://github.com/AlphaMegaladon/Orbifolder/actions/runs/43776625562)
bestanden Quality, sdist und die Wheels für Linux x86_64/ARM64, Windows x86_64
und macOS Intel. Der macOS-ARM64-Job wurde mit nicht unterstützter
`macos-14`-Runner-Konfiguration abgebrochen. GitHub hat macOS-14-Runner am
2026-11-02 eingestellt. Der Apple-Silicon-Job verwendet deshalb nun
`macos-15`; Intel bleibt bei `macos-15-intel`.
[Verfügbare Runner](https://github.com/actions/runner-images#available-images).

Die Hinweise zur Migration von `ubuntu-latest` auf Ubuntu 26.04 sind keine
Build-Fehler. CI, Quality, sdist und Publish verwenden jetzt ausdrücklich
`ubuntu-24.04`, passend zur bereits geprüften Linux-Basis. Rust Stable und
die neuesten Patchversionen innerhalb der gewählten Python-Reihen werden
weiterhin separat durch die Toolchain-Actions ausgewählt.

Der [zweite manuelle Lauf](https://github.com/AlphaMegaladon/Orbifolder/actions/runs/43779722809)
auf Commit `141eac6` bestand anschließend Quality, sdist und alle fünf damals
konfigurierten Wheel-Jobs, einschließlich beider macOS-Architekturen. Publish
wurde wie vorgesehen übersprungen. Der Hinweis auf begrenzte macOS-ARM64-
Kapazität beschreibt mögliche Wartezeiten und verhinderte den Build nicht.

## Plattformen und Wheel-Abdeckung

Die Release-Matrix enthält sechs native Builds. Jeder Job installiert sein
Wheel und führt die isolierten Python-Tests unter CPython 3.11 und 3.14 aus.
`setup-python` verwendet standardmäßig die Architektur des jeweiligen Runners;
der Windows-ARM64-Job verwendet deshalb native ARM64-Interpreter.

| System | Architektur | Runner | Rust-Target |
| --- | --- | --- | --- |
| Linux/glibc | x86_64 | `ubuntu-24.04` | `x86_64-unknown-linux-gnu` |
| Linux/glibc | ARM64 | `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` |
| Windows | x86_64 | `windows-latest` | `x86_64-pc-windows-msvc` |
| Windows | ARM64 | `windows-11-arm` | `aarch64-pc-windows-msvc` |
| macOS | Intel x86_64 | `macos-15-intel` | `x86_64-apple-darwin` |
| macOS | Apple Silicon ARM64 | `macos-15` | `aarch64-apple-darwin` |

Windows ARM64 wurde am 2026-11-16 ergänzt. Der
[dritte manuelle Lauf](https://github.com/AlphaMegaladon/Orbifolder/actions/runs/43782095269)
auf Commit `2e97d4e` bestand Quality, sdist und alle sechs nativen Wheel-Jobs,
einschließlich Windows ARM64 mit Installationstests unter CPython 3.11 und
3.14. Publish wurde für diesen Testlauf wie vorgesehen übersprungen.
Python 3.11 und 3.14 sind im
[Python-Versionsmanifest](https://github.com/actions/python-versions/blob/main/versions-manifest.json)
für Windows ARM64 vorhanden. Die
[Runner-Dokumentation](https://github.com/actions/runner-images#available-images)
führt `windows-11-arm` als nativen ARM64-Runner.

Ein Wheel gilt für eine Kombination aus Python-/ABI-Anforderungen,
Betriebssystem-Kompatibilität und Prozessorarchitektur, nicht nur für die
Betriebssystemversion des Build-Runners. `manylinux` deckt kompatible
glibc-basierte Linux-Distributionen ab. macOS-Wheels tragen eine
Mindestversion; ein Build auf einem neuen macOS bedeutet daher nicht
automatisch, dass nur dieses macOS unterstützt wird. ABI3 vermeidet separate
Builds für jede kompatible CPython-Version. Maßgeblich sind die tatsächlichen
Wheel-Tags und die erfolgreichen Laufzeittests.
[Wheel-Kompatibilität](https://packaging.python.org/en/latest/specifications/platform-compatibility-tags/).

Diese Matrix ist nicht vollständig für alle Plattformen. Nicht enthalten sind
beispielsweise Linux mit musl (Alpine), 32-Bit-x86/ARMv7, RISC-V, POWER,
IBM Z sowie andere Betriebssysteme wie FreeBSD. musl ist eine andere
Systembibliothek, keine zusätzliche Prozessorarchitektur. Auch andere
Python-Implementierungen oder Free-Threaded-CPython sind durch diese ABI3-Tests
nicht abgedeckt. Weitere Ziele benötigen eigene Build- und Laufzeittests.

Fehlt ein kompatibles Wheel, kann `pip` aus dem sdist lokal bauen. Dafür sind
eine passende Rust-Toolchain, Plattform-Build-Werkzeuge und unterstützte
Abhängigkeiten erforderlich. Dieser Rückfall garantiert nicht, dass der Build
auf einer bislang ungetesteten Plattform gelingt. Für den ersten Release sind
die sechs aufgeführten Ziele der vorgesehene Umfang; zusätzliche Builds können
bei konkretem Bedarf folgen.
