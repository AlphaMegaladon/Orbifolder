# Dokumentation

Für den aktuellen Checkout zuerst diese Dokumente lesen:

- [PyPI-Veröffentlichung und GitHub-Environment](./15_pypi_publishing.md):
  konkrete Publisher-Felder, Protection Rules, Testlauf und Release-Runner.
- [Portierungs- und Paketierungsreview vom 2026-11-15](./14_precommit_review.md):
  Funktionsvergleich, aktuelle Prüfungen, Benchmarkgrenzen und erster Commit.
- [Künftige Erweiterungen](../ROADMAP.md): besprochene nächste Arbeitspakete und
  größere Forschungsfragen.
- [Ubuntu-Entwicklungsumgebung](./11_development_ubuntu.md): Installation,
  Python-/Conda-Auswahl, Rust, C++, Editor und ausführbare Prüfkommandos.
- [Repository-Prüfung vom 2026-11-14](./12_repository_audit.md): tatsächlicher
  Implementierungsstand, neue Prüfergebnisse und verbleibende Grenzen.
- [Rust-Architektur](./02_rust_architecture.md): aktuelle Modulzuordnung mit
  Abweichungen vom ursprünglichen Entwurf.
- [Paketierung und Release-Verfahren](./10_packaging_and_repository_hygiene.md):
  Artefaktprüfung und Voraussetzungen für eine Veröffentlichung.
- [C++-Oracle](../tests/ground_truth/README.md) und
  [Build-Anleitung](../tests/ground_truth/BUILD.md): optionale Reproduktion der
  Referenzen. Die regulären Tests verwenden die gespeicherten JSON-Daten.

Die nummerierten Berichte 01 und 03–10 dokumentieren die jeweiligen
Migrationsphasen. Ihre Testzahlen, Betriebssysteme, Laufzeiten und Aussagen über
damals folgende Schritte sind historische Angaben. Sie ersetzen keine aktuelle
Prüfung und behaupten keine vollständige Portierung sämtlicher Legacy-Funktionen.

| Bericht | Inhalt |
| --- | --- |
| [01](./01_domain_model.md) | Domänenmodell der originalen C++-Programme |
| [03](./03_verification_and_migration_plan.md) | Ursprünglicher Migrationsplan und heutige Zuordnung |
| [04](./04_math_core_verification.md) | Exakte Arithmetik, Gitter und Lie-Algebren |
| [05](./05_geometry_verification.md) | Geometrie, Raumgruppe und Einbettung |
| [06](./06_states_and_spectrum_verification.md) | Zustände, Spektren und Tachyonen |
| [07](./07_anomalies_verification.md) | Anomalien und U(1)-Basis |
| [08](./08_susy_verification.md) | SUSY-Invarianten und Auswahlregeln |
| [09](./09_final_verification_and_release.md) | Rayon, Python und historische Benchmarks |
| [10](./10_packaging_and_repository_hygiene.md) | Paketierung, Typdeklarationen und Release-Workflow |

Links nach `legacy_cpp/` und auf generierte Build-Produkte setzen lokale,
ignorierte Dateien voraus. Die C++-Archive werden nicht mit dem öffentlichen
Quellpaket verteilt.
