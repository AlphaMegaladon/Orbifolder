# 03 – Verifikations- und Migrationsplan

**Historischer Phase-1-Plan.** Build, Fixture-Erzeugung und die hier beschriebenen
Kernstufen sind inzwischen implementiert. Die folgenden Freigaberegeln und
Zukunftsformulierungen dokumentieren den damaligen Ablauf. Sie sind keine
Behauptung, dass der heutige Checkout noch vor der Implementierung steht.

| Stufe | Implementierter Umfang / Nachweis |
| --- | --- |
| 2 | Optionale C++-Builds, sieben eingefrorene Modellreferenzen und ergänzende Probes; [Oracle-Dokumentation](../tests/ground_truth/README.md). |
| 3.1–3.2 | Exakte Mathematik und Geometrie; Berichte [04](./04_math_core_verification.md) und [05](./05_geometry_verification.md). |
| 3.3a–3.3b | Zustände/Spektren/Tachyonen und Anomalien; Berichte [06](./06_states_and_spectrum_verification.md) und [07](./07_anomalies_verification.md). Betafunktionen und allgemeine Phänomenologieschemata sind nicht implementiert. |
| 3.3c | SUSY-Monomiale, Auswahlregeln und formale F-Term-Ableitungen im Grad-1–3-Profil; [08](./08_susy_verification.md). Keine numerischen VEV-/D-Term-/Massenmatrixrechnungen. |
| 3.4 / Packaging | Rayon, PyO3, Presets, Parser, Summaries und Release-Konfiguration; Berichte [09](./09_final_verification_and_release.md) und [10](./10_packaging_and_repository_hygiene.md). Eine Workflow-Datei belegt keine Veröffentlichung. |

Die tatsächliche [Architektur](./02_rust_architecture.md), der
[aktuelle Prüfstand](./12_repository_audit.md) und das
[Ubuntu-Setup](./11_development_ubuntu.md) ergänzen den ursprünglichen Plan.

## Ursprünglicher Plan

## Freigaben und Abnahmeregel

| Stufe | Ergebnis und Übergangsbedingung |
| --- | --- |
| Phase 1 | Architektur, Zahlen-/Datenkonventionen und dieser Plan werden vom Nutzer freigegeben. |
| Phase 2 | Beide C++-Oracles sind reproduzierbar gebaut; versionierte Referenzen, Extraktion, Vergleichsregeln und Abdeckungsmatrix liegen vor. Der Nutzer gibt die Ground-Truth-Suite frei. |
| Phase 3.1 | `math_core` gegen C++-Referenzen und mathematische Invarianten abnehmen, bevor Geometrie implementiert wird. |
| Phase 3.2 | `geometry` einschließlich Einbettung, drei Faktoren und modularer Invarianz abnehmen, bevor Zustandsphysik implementiert wird. |
| Phase 3.3 | `physics_engine` und darauf aufbauende Phänomenologie schrittweise abnehmen: Zustände/Spektren, Tachyonen, Anomalien, danach SUSY-Kopplungen. |
| Phase 3.4 | PyO3-API, In-Memory-Roundtrips, Nebenläufigkeit und reproduzierbare Performance prüfen; erst danach Bibliotheksabnahme. |

Vor jeder Modulimplementierung müssen die dafür benötigten C++-Referenzen vorliegen. Vor dem nächsten Modul müssen sämtliche vereinbarten Oracle-Vergleiche des aktuellen Moduls bestehen. Zusätzliche Fähigkeiten erhalten zuerst zusätzliche Referenzfälle; fehlende C++-Abdeckung ist keine erfolgreiche Validierung. Vor Freigabe von Phase 1 **und** Phase 2 entstehen keine Rust-Implementierungsdateien.

## Phase 2: C++ als reproduzierbarer Oracle

### Build und Ausführung

1. Quellen, Geometrie-/Modelldaten und Tensorprodukttabellen mit SHA-256 erfassen. Bereits vorhandene Binärdateien sind kein Nachweis eines Builds aus diesem Stand.
2. Beide Varianten in getrennten Build-Verzeichnissen beziehungsweise isolierten Quellkopien bauen. Die archivierten Legacy-Quellen bleiben unverändert. Ausgangspunkte sind [SUSY configure.ac](../legacy_cpp/susy_orbifolder/configure.ac) und [non-SUSY configure.ac](../legacy_cpp/non_susy_orbifolder/configure.ac); diese deklarieren GSL/Boost, non-SUSY zusätzlich readline. Die Ziele heißen [orbifolder](../legacy_cpp/susy_orbifolder/src/orbifolder/Makefile.am) und [nonSUSYorbifolder](../legacy_cpp/non_susy_orbifolder/src/orbifolder/Makefile.am).
3. Compiler, C++-Standard, vollständige Flags, Bibliotheksversionen, Plattform und Binary-Hash protokollieren. Kein `fast-math`. Falls der Build Anpassungen benötigt: isolierter, dokumentierter Build-Patch mit Hash und Prüfung, dass keine mathematischen Ausdrücke oder Auswahlbedingungen geändert wurden.
4. Ein Python-Runner führt deterministische Modellprogramme und C++-Probes als getrennte Prozesse aus. Pro Fall: festgelegtes Arbeitsverzeichnis, Datenpfade, Locale, Seed soweit relevant, Timeout und Exitcode; Rohdaten, stdout und stderr erhalten eigene Dateien.
5. Zusätzlich zum Exitcode müssen Laden, Prüfstatus und angeforderte Rechenstufen bestätigt sein. Fehlgeschlagene oder abgebrochene Rechnungen liefern keinen gültigen leeren Referenzdatensatz. Jeden deterministischen Fall mindestens zweimal unabhängig erzeugen und kanonische Nutzdaten vergleichen.

Geplante Ablage unter `tests/ground_truth/`:

```text
schema/           versionierter Fixture- und Eingabevertrag
cases/            explizite Modelle, Geometriezuordnungen und Rechenprofile
probes/           kleine C++-Programme gegen die Originalimplementierung
fixtures/         eingefrorene strukturierte Referenzwerte
generate.py       Builds/Prozesse orchestrieren, Rohdaten erfassen, JSON erzeugen
compare.py        Konventionen abgleichen und feldgenaue Differenzen melden
```

Build-Produkte und Rohprotokolle liegen separat als reproduzierbare Arbeits-/CI-Artefakte. Die Fixture-Erzeugung benötigt keine Rust-Bibliothek.

### Strukturierte Extraktion

Die normalen [CPrint-Ausgaben](../legacy_cpp/susy_orbifolder/src/orbifolder/cprint.cpp#L61) beginnen mit geringer Druckpräzision; `PrintRational` rationalisiert zudem bereits berechnete Doubles. Ein CLI-Spektrumsdump allein genügt deshalb nicht als vollständiger Zahlen-Oracle.

C++-Probes nutzen originale Klassen und öffentliche Methoden: Massenlösungen, Halbzustandsgewichte, Lokalisation, [Felder](../legacy_cpp/susy_orbifolder/src/orbifolder/cfield.h), Eichwurzel-/U(1)-Basis und beide [non-SUSY-Konfigurationen](../legacy_cpp/non_susy_orbifolder/src/orbifolder/corbifold.h) werden vor einer verdichtenden Darstellung ausgegeben. Dieselbe Eingabe läuft zusätzlich durch das Original-Binary; Spektrum, Zustandszahlen und Status müssen mit dem Probe-Lauf übereinstimmen.

Native rationale Werte werden als Zähler/Nenner exportiert. Bei Doubles werden der Rundlauf-Dezimalwert mit `max_digits10` und die IEEE-Bitdarstellung gespeichert; ein gedruckter oder nachträglich rationalisierter Wert bleibt als solcher gekennzeichnet. Die Extraktion verändert keine Massen-, Gitter- oder Projektionsentscheidung.

Wo öffentliche APIs Zwischenwerte nicht zugänglich machen, kann eine isolierte Build-Kopie ausschließlich Ausgabeinstrumentierung erhalten. Jeder Patch wird separat gespeichert; Original- und instrumentierter Lauf müssen in den vorhandenen physikalischen Ausgaben übereinstimmen. Ein nachimplementierter Python- oder Rust-Algorithmus erzeugt keine C++-Ground-Truth.

Für Anomalien zuerst die vorhandene Methode [CheckAnomaly](../legacy_cpp/non_susy_orbifolder/src/orbifolder/corbifold.cpp#L1366) mit ausführlicher Ausgabe erfassen: Rückgabewert, tatsächlich berechnete Spuren/Koeffizienten und Konfiguration vor/nach dem Aufruf. Nicht ausgegebene Koeffizienten benötigen Ausgabeinstrumentierung an den Originalberechnungen. Bloßes Nachrechnen aus den extrahierten Feldern prüft die Konsistenz des Exports, ersetzt aber keinen unabhängigen C++-Anomalie-Oracle.

### Fixture-Vertrag

| JSON-Bereich | Pflichtinhalt |
| --- | --- |
| `schema_version`, `case_id`, `capabilities` | Datenversion, stabiler Fallname, tatsächlich ausgeführte Stufen und Vollständigkeit jeder Stufe. |
| `oracle` | SUSY/non-SUSY-Quellvariante, Quell-/Daten-/Binary-Hashes, Buildumgebung, Instrumentierungs-Patch, Runner-Version und genaue Aufrufe. |
| `input` | Vollständige rationale Twists/Shifts/Wilsonlinien, Konstruktion, Faktorordnungen und Importabbildung, Torsion, exakte Geometriezuordnung, Such-/Analyseprofil und gegebenenfalls Seed. |
| `conventions` | Gitter-/Wurzelbasis, einfache Wurzeln und Faktorordnung, U(1)-Generatoren samt Normquadraten, Händigkeit/CPT, Multiplet- gegenüber Komponentenzählung, Phasen- und Massennormierung. |
| `math`, `geometry` | Für Modulprobes: Operanden/Resultate, Gitterklassen, Wurzeln/Cartan-Daten, Gruppenoperationen, Fixbranen, Zentralisatoren, Relationen und Invarianzstatus. |
| `states`, `spectrum` | Sektor `(m,n,k)`, Translation, Oszillatoren, `p_sh`/`q_sh`, Rechtsklasse, Eigen-/Gamma-Phasen, Projektionsstatus, Massenparameter, Darstellung mit Höchstgewichten, Ladungen und Multiplizitäten. Tachyonen und angeregte Tachyonen bleiben unterscheidbar. |
| `anomalies` | Gemessene quadratische/kubische und gemischte Spuren, globale Prüfungen, U(1)-/FI-Daten, Green–Schwarz-/Universalitätsbewertung und C++-Gesamtstatus; Abdeckung je Koeffiziententyp. |
| `susy_analysis` | Multipletzuordnung, Monomiale/Kopplungen samt Ordnung, Auswahlregeln, gegebenenfalls F-/D-Term- und Massenmatrixdaten; `not_applicable`, `unsupported` und `not_requested` sind explizite Zustände. |
| `outcome`, `raw_evidence` | Lade-/Prüfstatus, vollständiger Abschluss oder Fehler/Limit, Verweise auf Rohdaten und integritätsgesicherte Protokolle. |

Exakte Brüche verwenden `{ "numerator": "…", "denominator": "…" }`; Doubles bleiben gekennzeichnete Oracle-Beobachtungen. Ein aus Double-Rationalisierung entstandener Bruch erhält diese Herkunft. Unvollständige oder unzugängliche Daten werden niemals durch Null oder eine leere Liste ersetzt.

## Referenzfälle und nachzuweisende Abdeckung

Die Tabelle beschreibt zu erzeugende Fälle; **noch keine** Spektrumszahl oder Tachyonfreiheit gilt als gemessen.

| Fallfamilie | Eingabe und Zweck |
| --- | --- |
| SUSY-Z₃ und Z₆-II | [Z₃-Geometrie](../legacy_cpp/susy_orbifolder/Geometry/Geometry_Z3_1_1.txt) und [Z₆-II-Geometrie](../legacy_cpp/susy_orbifolder/Geometry/Geometry_Z6-II_1_1.txt), E8×E8-Standard-Einbettung `V=(v₁,v₂,v₃,0¹³)`, Wilsonlinien null. Baseline für exakte Massen, Sektoren, Felder, Ladungen und Anomalien. |
| non-SUSY-Z₃ und Z₆-II | Vorhandene [Z₃-](../legacy_cpp/non_susy_orbifolder/Models/ZN_models/modelZ3_1_1.txt) und [Z₆-II-Modelle](../legacy_cpp/non_susy_orbifolder/Models/ZN_models/modelZ6-II_1_1.txt) einschließlich Brechungsshift und zusätzlichem Spin-Twist. Vollständige masselose und tachyonische Auswertung anfordern. |
| Zwei beziehungsweise drei Faktoren | Einen bestätigten SUSY-Fall aus den [Z₃×Z₃-Modellen mit Wilsonlinie](../legacy_cpp/susy_orbifolder/Z3xZ3_1_1-1WL-77PatricksModels.txt) und ein [non-SUSY-Z₃×Z₃-Modell](../legacy_cpp/non_susy_orbifolder/Models/ZNxZM_models/modelZ3xZ3_1_1.txt) einfrieren. Faktorabbildung, nichttriviale Wilsonlinie und alle drei Exponenten prüfen. |
| Tachyonische und tachyonfreie Fälle | Durch deterministische C++-Auswertung konkrete Eingaben mit überlebenden Grundzustands-/angeregten Tachyonen sowie mindestens einen vollständig geprüften tachyonfreien Fall identifizieren. Solche positiven Fälle sind bisher nicht belegt. |
| Mathematik-Probes | Rationale Arithmetik einschließlich großer Ganzzahlen, Phase modulo eins, E8-/Spin(32)-Gitterdaten, SO(8)-V/S/C, Wurzeln und Cartan-/Dynkin-Daten aus Originalroutinen erfassen; D₈-Daten aus tatsächlich überlebenden SO(16)-Wurzeln ableiten. |
| SUSY-Kopplungen | Für einen geeigneten bestätigten SUSY-Fall erlaubte **und** verbotene Monomiale/Kopplungen bis zu einer festgelegten niedrigen Ordnung erfassen; vorhandene [Yukawa-](../legacy_cpp/susy_orbifolder/src/orbifolder/cyukawacouplings.h) und [Monomialroutinen](../legacy_cpp/susy_orbifolder/src/orbifolder/cmonomial.h) verwenden. |
| Fehler und Grenzfälle | Verletzte modulare Invarianz oder Wilsonlinienrelationen, negative Phasen, Grenzwerte der Massenbedingung und exakte Nullentscheidungen. Eingabevertragsprüfungen wie Nenner null werden gesondert von physikalischen Oracle-Vergleichen ausgewiesen. |

Weitere Geometrien, Spin(32)-Spektren, nichtverschwindende Torsion, frei wirkende Wilsonlinien und zusätzliche SUSY-Analysen benötigen vor Freigabe ihrer Implementierung passende Referenzen. Eine Headerdeklaration belegt keinen aktiven Rechenpfad: Im [non-SUSY-Projektionscode](../legacy_cpp/non_susy_orbifolder/src/orbifolder/cfixedpoint.cpp#L1114) sind beispielsweise Torsionsbeiträge teilweise auskommentiert. Abdeckung wird aus ausgeführten Originalpfaden ermittelt.

Der [C++-Tachyonlöser](../legacy_cpp/non_susy_orbifolder/src/orbifolder/chalfstate.cpp#L533) verwendet feste Suchfenster. Diese Grenzen sind pro Fall zu dokumentieren und gegen hergeleitete Normschranken zu prüfen. Aus einem leeren, möglicherweise begrenzten Suchergebnis folgt noch keine allgemeine Tachyonfreiheit. Für ein nicht vollständig belegtes Suchprofil bleibt die entsprechende Fähigkeit gesperrt.

Die exakte Interpretation der näherungsweisen Torusbasen ist ein eigener Phase-2-Nachweis: rationale Rotationsmatrizen, gegebenenfalls Gram-Matrix, Generatorrelationen und Fixpunktdaten werden gegen die Legacy-Geometrie geprüft. Ein bloßes Runden von `1.7320508076` oder eine unbelegte rationale Rekonstruktion erfüllt diesen Nachweis nicht.

## Vergleichsregeln: exakter Core, beobachteter C++-Wert

1. **Gemeinsame Konvention herstellen.** Varianten anhand ihrer tatsächlichen Konstruktion vergleichen. SUSY `(k_alt,l_alt)` explizit nach `(m,n,0)` abbilden; non-SUSY behält `(m,n,k)`. Wurzel-/Faktorreihenfolge, U(1)-Basis und CPT-/Multipletzählung mit gespeicherten Abbildungen ausrichten. Eine gleiche Geometriebezeichnung allein bezeichnet nicht dasselbe physikalische Modell.
2. **Kanonisieren.** Rationale Werte kürzen, skalare Phasen modulo eins normalisieren, Datensätze mit stabilen physikalischen Schlüsseln sortieren und Multiplizitäten erhalten. Lokalisation, Darstellungskonjugation oder Gamma-Phasen dürfen bei der Sortierung nicht verloren gehen.
3. **Diskrete Entscheidungen exakt vergleichen.** Zustandszahlen, SO(8)-Klassen, Händigkeit, erlaubte Kopplungen, Projektionsentscheidungen, Tachyonanzahl und Invarianzstatus besitzen keine Toleranz. C++-native rationale Werte werden nach geklärter Herkunft direkt verglichen.
4. **Doubles ausschließlich im Comparator behandeln.** Einen exakten Bruch nur dann rekonstruieren, wenn eine fachlich belegte Nennergrenze und das Fehlerintervall einen eindeutigen Kandidaten ergeben. Andernfalls gilt ein dokumentiertes feldbezogenes Fehlerbudget, etwa `|x_cpp − x_exact| ≤ atol_f + rtol_f·|x_exact|`; bei Phasen zählt der zirkuläre Abstand modulo eins. Das Budget stammt aus Druckpräzision und charakterisierten C++-Operationen; ein globales `10⁻⁴` oder `10⁻⁵` wird nicht eingeführt. Rust-Grenzentscheidungen bleiben exakt.
5. **Basisabhängige Größen gemeinsam transformieren.** Ein U(1)-Basiswechsel betrifft alle Ladungen, Normdaten und Anomaliekoeffizienten; einzelne Einträge dürfen nicht unabhängig passend skaliert werden. Ist die Basisumrechnung numerisch/algebraisch, wird dies im externen Vergleich ausgewiesen. Höchstgewichte und gemeinsame eingebettete Gewichte sichern die Darstellungszuordnung.
6. **Anomalien fachlich auswerten.** Rohe Spuren/Koeffizienten und die C++-Bewertung einschließlich Green–Schwarz getrennt vergleichen. `CheckAnomaly=true` bedeutet nicht, dass jede U(1)-Spur null sein muss.

Bei einer Differenz werden Eingabe, erste abweichende Pipeline-Stufe, rationale Rust-Werte, rohe C++-Werte, verwendete Basisabbildung und relevante C++-Schwelle protokolliert. Der Fall wird minimiert und als Regression erhalten. Auch ein plausibler C++-Toleranzeffekt bleibt ein ungelöster Oracle-Konflikt: weder größere Vergleichstoleranzen für diskrete Entscheidungen noch stilles Überschreiben der Referenz sind zulässig. Eine Abweichung vom C++-Verhalten benötigt nach Ursachenklärung eine ausdrückliche fachliche Entscheidung des Nutzers.

Die Referenzwerte bleiben schreibgeschützt im normalen Testlauf. Regeneration ist eine separate Aktion mit Quell-/Schema-Version und prüfbarem Datendiff; fehlende oder abweichende Fixtures werden im Testlauf nicht automatisch neu erzeugt.

## Phase 3: Bottom-up-Migration und Prüfungen

| Modulschritt | Oracle-Abnahme | Ergänzende Invarianten/Integration |
| --- | --- | --- |
| 1. `math_core` | Originale rationale Resultate, Gitterzugehörigkeit/Nebenklassen, Wurzelmengen, Cartan-Matrizen und Dynkinlabels. | Normalform, negative Modulo-Phasen, arithmetischer Überlauf mit BigInt-Wiederholung, Weyl-Involution und Normerhaltung. Ein eigener C++-Weyl-API-Aufruf ist bisher nicht belegt: Weyl-Ergebnisse deshalb mit den C++-Wurzelmengen/Dynkindaten abgleichen und die direkte/indirekte Abdeckung ausweisen. |
| 2. `geometry` | Produkte/Inversen, Konstruktionsklassen, Fixbranen, Zentralisatoren, lokale Shifts/Twists, Wilsonlinienrelationen und modulare Invarianz; SUSY und drei non-SUSY-Faktoren. | Assoziativität, rationale Gitteraktionen, Spin-Lift des geometrisch trivialen Faktors, Rototranslationsrelationen und unveränderliche Eingaben. |
| 3a. Zustände/Spektren | Unprojizierte Gewichte und Oszillatoren, Eigenphasen, Projektionen, vollständige Felder/Quantenzahlen, Grundzustands- und angeregte Tachyonen mit Masse. | Exakte Massengleichheit, vollständige hergeleitete Suchschranken, CPT-/Chiralitätszählung, dieselben Generatoren für beide Theorien; Ressourcenabbruch als eigener Ausgang. |
| 3b. Anomalien/Phänomenologie | Originalspuren und Status, Betafunktionen und vereinbarte Phänomenologieschemata; Referenzen jeweils vor Implementierung ergänzen. | Anomalieberechnung aus unabhängigen Fermionen; konsistenter Basiswechsel; keine Doppelzählung durch Multipletansichten. |
| 3c. SUSY-Erweiterung | Erlaubte/verbotene Monomiale, Kopplungen, F-/D-Term- und Massenmatrixdaten für das vereinbarte Ordnungs-/Hintergrundprofil. | Exakte Auswahlregeln, stabile Superfeld-IDs; `NotApplicable` für non-SUSY und explizite Abdeckung endlicher Kopplungsordnungen. |
| 4. `pyo3_api` | Dieselben Fixtures durch Rust- und Python-Einstieg; keine erneute Physikimplementierung im Binding. | Exakte Rational-Roundtrips, immutable Wrapper, getrennte Fehler pro Batch-Eingabe, GIL-Freigabe, Datenbesitz und unterstützte Python-Runtimes. |

Mathematische Eigenschaftstests ergänzen den differentiellen C++-Abgleich. Ein bestandener Eigenschaftstest ersetzt keine erforderliche Oracle-Referenz. Neue öffentliche Fähigkeiten werden erst nach bestandener Abnahme in der dokumentierten Abdeckungsmatrix aktiviert.

## Determinismus, Leistung und Abschluss

Serielle und parallele Ausführung müssen für denselben Eingabesatz identische kanonische Nutzdaten und Fehler liefern. Mit unterschiedlichen Poolgrößen werden Reihenfolge, stabile IDs, Feldmultiplizitäten, BigInt-Wiederholungen und unveränderte gemeinsame Geometriedaten geprüft. Zufallsfälle besitzen gespeicherte Seeds und werden als konkrete Modelle eingefroren; der Scheduler bestimmt weder Seed noch Physik.

Performance wird erst nach fachlicher Gleichheit gemessen: Einzelmodell, gemeinsamer Geometriekern mit vielen Einbettungen und ein repräsentativer Batch von mindestens 1.000 Modellen. Erfasst werden Laufzeitverteilung, Durchsatz, Speichermaximum, Kandidatenzahlen und BigInt-Anteil bei festgehaltenen Limits, Threads, Hardware und Buildprofilen. Core-Rechenzeit und Python-/JSON-/CLI-Konvertierung werden getrennt ausgewiesen; Beschleunigung wird gemessen, nicht vorausgesetzt.

Die späteren CI-Prüfungen umfassen Rust-Formatierung/Lints, modulweise Oracle-Tests, serielle/parallele Regressionen und Python-Integration. Die vorberechneten Fixtures müssen regulär ohne lokale C++-Installation nutzbar sein; eine separate Oracle-Regenerationsprüfung bestätigt ihre Herkunft mit dem festgelegten C++-Build.

Abgeschlossen ist eine Migrationsstufe erst mit reproduzierbar bestandenen Vergleichen, dokumentiertem Fähigkeitsumfang und unverändert gültigen früheren Referenzen. Die Bibliotheksabnahme verlangt zusätzlich stabile In-Memory-Schnittstellen, versionierte Datenkonventionen und dokumentierte verbleibende Grenzen. Legacy-CLI, Prozessverwaltung und Formatierung sind keine Voraussetzungen für die Freigabe des mathematisch-physikalischen Rust-Kerns.
