# 01 – Domänenmodell der C++-Bestände

Basis: Header aus [SUSY](../legacy_cpp/susy_orbifolder/src/) und [non-SUSY](../legacy_cpp/non_susy_orbifolder/src/), ergänzt um gezielte Implementierungslektüre für die Formeln. Historische statische Bestandsaufnahme aus Phase 1. Die anschließend umgesetzte Laufzeitvalidierung ist in den Berichten 04–10 dokumentiert; der [aktuelle Prüfstand](./12_repository_audit.md) beschreibt diesen Checkout.

## Verzeichnisstruktur

| Bereich | Verantwortung |
| --- | --- |
| [SUSY: src/orbifolder/](../legacy_cpp/susy_orbifolder/src/orbifolder/) · [non-SUSY](../legacy_cpp/non_susy_orbifolder/src/orbifolder/) | Geometrie, Eich-Einbettung, Stringzustände, Phänomenologie **und** CLI/Formatierung. |
| [SUSY: src/linalg/](../legacy_cpp/susy_orbifolder/src/linalg/) · [non-SUSY](../legacy_cpp/non_susy_orbifolder/src/linalg/) | Generische Vektor-/Matrixarithmetik, Gleichungssysteme, Diophantik, numerische Vergleiche. |
| [SUSY: src/groupTheory/](../legacy_cpp/susy_orbifolder/src/groupTheory/) · [non-SUSY](../legacy_cpp/non_susy_orbifolder/src/groupTheory/) | Wurzeln, Lie-Algebren, Darstellungen und Gewichtsmultiplizitäten. |
| [SUSY: src/io/](../legacy_cpp/susy_orbifolder/src/io/) · [non-SUSY](../legacy_cpp/non_susy_orbifolder/src/io/) | Parser und Ausgabeoperatoren; enthält zusätzlich mathematische Typ-Aliase. |
| [SUSY: src/examples/](../legacy_cpp/susy_orbifolder/src/examples/) · [non-SUSY](../legacy_cpp/non_susy_orbifolder/src/examples/) | Beispielprogramme zur Modellberechnung. |
| [SUSY: Geometry/](../legacy_cpp/susy_orbifolder/Geometry/) · [non-SUSY](../legacy_cpp/non_susy_orbifolder/Geometry/) | Serialisierte Raumgruppen-/Torusdaten. |
| [SUSY: GaugeInvariance/](../legacy_cpp/susy_orbifolder/GaugeInvariance/) · [non-SUSY](../legacy_cpp/non_susy_orbifolder/GaugeInvariance/) | Tabellen bekannter invarianter/nichtinvarianter Tensorprodukte. |

Zusätzlich enthält non-SUSY [Models/](../legacy_cpp/non_susy_orbifolder/Models/) und [doc/](../legacy_cpp/non_susy_orbifolder/doc/). Autotools-Dateien und die Verzeichnisse [Config/](../legacy_cpp/non_susy_orbifolder/Config/) und [m4/](../legacy_cpp/non_susy_orbifolder/m4/) gehören zur Build-Infrastruktur.

## Mathematik und Geometrie

Bei gemeinsamen Typen verweisen die folgenden Links auf die SUSY-Deklaration; Abweichungen sind ausdrücklich benannt.

| Kernstruktur | Fachlicher Inhalt |
| --- | --- |
| [CVector](../legacy_cpp/susy_orbifolder/src/orbifolder/cvector.h) | Dynamischer `vector<double>` mit Skalarprodukten und Normoperationen; übernimmt auch rationale Eingaben in diese Gleitkommadarstellung. |
| [CLinAlg](../legacy_cpp/susy_orbifolder/src/orbifolder/clinalg.h), [CHugeInt](../legacy_cpp/susy_orbifolder/src/orbifolder/chugeint.h) | Rationale lineare Algebra: Kerne, Zeilenreduktion, Gram–Schmidt, ganzzahlige Lösungen und Ungleichungen. `boost::rational<T>` unterstützt unter anderem den eigenen Ganzzahltyp mit variabler Ziffernzahl. |
| [CLatticeVector](../legacy_cpp/susy_orbifolder/src/orbifolder/clatticevector.h) | Gitterzugehörigkeit: E8-Blöcke, Spin(32)/Z₂ und rechtsbewegte SO(8)-Klassen. [non-SUSY](../legacy_cpp/non_susy_orbifolder/src/orbifolder/clatticevector.h) ergänzt `From_G8_Lattice` und unterscheidet `SO8_V`, `SO8_S`, `SO8_C`. |
| [CLatticeElement, CSpaceGroupElement](../legacy_cpp/susy_orbifolder/src/orbifolder/cspacegroupelement.h) | Translation mit **sechs rationalen** Koeffizienten plus Rotationsexponenten `(k,l)`. [non-SUSY](../legacy_cpp/non_susy_orbifolder/src/orbifolder/cspacegroupelement.h) verwendet `(m,n,k)`. `CLatticeElement` und `CLatticeVector` beschreiben verschiedene Gitterräume. |
| [CTwistVector](../legacy_cpp/susy_orbifolder/src/orbifolder/ctwist.h) | Vier Rotationsphasen in Lichtkegel-Konvention, Twistordnung sowie linke/rechte Nullpunktsenergie `a_L`, `a_R`. |
| [CSpaceGroup](../legacy_cpp/susy_orbifolder/src/orbifolder/cspacegroup.h) | T⁶-Basis in drei komplexen Koordinaten, rationale Twistmatrizen, Generatoren, Konstruktions-/Zentralisatorelemente nach Sektoren, Fixpunkte/-tori, Wilsonlinien-Relationen und diskrete/modulare Symmetriedaten. [non-SUSY](../legacy_cpp/non_susy_orbifolder/src/orbifolder/cspacegroup.h) besitzt einen dritten zyklischen Faktor. |
| [CShiftVector](../legacy_cpp/susy_orbifolder/src/orbifolder/cshiftvector.h), [CWilsonLine](../legacy_cpp/susy_orbifolder/src/orbifolder/cwilsonline.h), [CWilsonLines](../legacy_cpp/susy_orbifolder/src/orbifolder/cwilsonlines.h) | 16-komponentige Eichverschiebungen für Rotationen beziehungsweise Translationen; die Wilsonlinien-Sammlung ordnet den sechs Torusrichtungen je eine Linie zu. Ordnungen beziehen sich auf das gewählte Eichgitter. |
| [COrbifoldGroup](../legacy_cpp/susy_orbifolder/src/orbifolder/corbifoldgroup.h), [COrbifoldGroupElement](../legacy_cpp/susy_orbifolder/src/orbifolder/corbifoldgroupelement.h) | Raumgruppe **mit** Eich-Einbettung: Shifts, Wilsonlinien, rationale diskrete Torsionsparameter, modulare Invarianz und invariante Superladungen. Ein Gruppenelement bündelt Raumgruppenelement, lokalen Shift und lokalen Twist. |
| [gaugeGroupFactor, gaugeGroup, weightsystem, dynkinDiagram](../legacy_cpp/susy_orbifolder/src/groupTheory/groupTheory.hpp) | Einfache Wurzeln, Rang, ADE-Algebra, U(1)-Richtungen und Dynkin-/Wurzellabel. `dynkin` erzeugt Darstellungsgewichte; `freudenthal` berechnet ihre Multiplizitäten. Derselbe Header deklariert E8-/E8×E8-Wurzeldaten und eine Spin(32)-Basis. |

Die gemeinsame [Gitterkennung](../legacy_cpp/non_susy_orbifolder/src/orbifolder/cvector.h) lautet `E8xE8`, `Spin32` oder `SO8`; ein Wert `SO16xSO16` existiert nicht. Auch ein mitgeliefertes [non-SUSY-Z₃-Modell](../legacy_cpp/non_susy_orbifolder/Models/ZN_models/modelZ3_1_1.txt) verwendet `Lattice:E8xE8`. Gitterkennung und physikalische Theorie dürfen daher nicht gleichgesetzt werden.

## Zustandskette und Spektrum

| Kernstruktur | Inhalt und Abhängigkeiten |
| --- | --- |
| [COrbifoldCore](../legacy_cpp/susy_orbifolder/src/orbifolder/corbifoldcore.h) | Erzeugt Sektoren und Rechtsbeweger unabhängig von der Wahl der Eich-Shifts/Wilsonlinien; schreibt in einen übergebenen Sektorvektor. |
| [CModedOscillator](../legacy_cpp/susy_orbifolder/src/orbifolder/coscillator.h), [S_OscillatorExcitation](../legacy_cpp/susy_orbifolder/src/orbifolder/chalfstate.h) | Links-/Rechtsbewegung, Modenfrequenz, komplexe Richtung und Transformationsphase; Anregung als Oszillatorindizes, Besetzungsbeitrag und gespeicherter Massenbedingung. |
| [CMasslessHalfState, CHalfState](../legacy_cpp/susy_orbifolder/src/orbifolder/chalfstate.h) | Erst Gewichte, die die Massenbedingung erfüllen; anschließend Gruppierung nach Zentralisator-Eigenwerten. `CHalfState` speichert Indizes auf diese Lösungen und Gewichte. Ein Typ `CMasslessStates` ist in den untersuchten Headern nicht deklariert. |
| [CSector](../legacy_cpp/susy_orbifolder/src/orbifolder/csector.h), [CFixedBrane](../legacy_cpp/susy_orbifolder/src/orbifolder/cfixedpoint.h) | Sektor: lokaler Twist, Oszillatoren, Rechtsbeweger und Fixbranen. Fixbrane: Konstruktions-Raumgruppenelement, lokale Linksbeweger und zentralisatorinvariante Links-/Rechts-Kombinationen. |
| [CState](../legacy_cpp/susy_orbifolder/src/orbifolder/cstate.h) | Paar von Halbzuständen mit Oszillatorbeitrag, Gamma-Phasen, Rechtsladungen und Multipletzuordnung; erzeugt Eichdarstellungen und verweist auf resultierende Felder. |
| [CField](../legacy_cpp/susy_orbifolder/src/orbifolder/cfield.h) | Feld-/Darstellungsebene: Multiplet, Darstellungsdimensionen **und** Dynkin-Höchstgewichte, U(1)-Ladungen, Gewichtsindizes, `q_sh`, Lokalisation und Oszillator-/Gamma-Daten; zusätzlich Labels, VEVs und akzidentelle Ladungen. Sektor/Fixbrane/Zustand werden über einen Indexpfad referenziert. |
| [CTachyonHalfState](../legacy_cpp/non_susy_orbifolder/src/orbifolder/chalfstate.h), [Tachyon-Sektoren](../legacy_cpp/non_susy_orbifolder/src/orbifolder/csector.h), [Tachyon-Fixbranen](../legacy_cpp/non_susy_orbifolder/src/orbifolder/cfixedpoint.h) | non-SUSY ergänzt tachyonische Massenlösungen, nach Massenniveau geordnete Anregungen sowie getrennte Grundzustands-/angeregte Tachyon-Zustände. Die [Multipletkennung](../legacy_cpp/non_susy_orbifolder/src/orbifolder/cstate.h) enthält zusätzlich Bosonen, chirale Fermionen und Tachyonen. |

Die zugrunde liegenden Operationen sind:

1. **Raumgruppenprodukt:** `(R,t)(R′,t′) = (RR′, t + Rt′)`; Zentralisatoren bestimmen die zulässigen Projektionsbedingungen. Siehe [SG_Multiply](../legacy_cpp/susy_orbifolder/src/orbifolder/cspacegroup.cpp#L3247).
2. **Lokale Einbettung:** `v_g = Σᵢ kᵢ vᵢ`, `V_g = Σᵢ kᵢ Vᵢ + Σα nα Aα`; daraus `q_sh = q + v_g`, `p_sh = p + V_g`. Siehe [SUSY](../legacy_cpp/susy_orbifolder/src/orbifolder/corbifoldgroup.cpp#L447) und [non-SUSY](../legacy_cpp/non_susy_orbifolder/src/orbifolder/corbifoldgroup.cpp#L978).
3. **Masselosigkeit:** `½ p_sh² + N_L + a_L = 0` und `½ q_sh² + N_R + a_R = 0`. Das Feld `S_OscillatorExcitation.ZeroPointEnergy` speichert dabei den Sollwert `−2(a + N)` für das Gewichtsquadrat, nicht die reine Nullpunktsenergie. Siehe [Anregungserzeugung](../legacy_cpp/susy_orbifolder/src/orbifolder/csector.cpp#L315) und [Massenlöser](../legacy_cpp/susy_orbifolder/src/orbifolder/chalfstate.cpp#L149).
4. **Projektion und Darstellung:** Zentralisatorinvariante Kombinationen werden nach Superladungen beziehungsweise Teilchenart klassifiziert und zu Eichdarstellungen zusammengefasst.

## Modell, Auswertung und Sitzungszustand

| Struktur | Einordnung |
| --- | --- |
| [COrbifold, SConfig, SSymmetryGroup](../legacy_cpp/susy_orbifolder/src/orbifolder/corbifold.h) | `COrbifold` besitzt Einbettung, Sektoren und Standardkonfiguration. `SConfig` enthält Eichgruppe/Felder sowie VEV-/Auswertungsdaten; `SSymmetryGroup` ergänzt die Eichgruppe um beobachtbare Faktoren, anomale U(1)-Kennzeichnung und FI-Term. non-SUSY besitzt außerdem eine [TachyonicStandardConfig](../legacy_cpp/non_susy_orbifolder/src/orbifolder/corbifold.h). |
| [CSpectrum (SUSY)](../legacy_cpp/susy_orbifolder/src/orbifolder/cspectrum.h), [CSpectrum (non-SUSY)](../legacy_cpp/non_susy_orbifolder/src/orbifolder/cspectrum.h) | Verdichtete Vergleichsdaten mit Darstellungen und Multiplizitäten; non-SUSY verwendet eigene `SSpectrum`-/`SRepresentation`-Strukturen. [CInequivalentModels](../legacy_cpp/susy_orbifolder/src/orbifolder/cinequivalentspectra.h) verwaltet Vergleiche. Diese Daten ersetzen nicht die vollständigen Feldquantenzahlen. |
| [CGaugeIndices, CGaugeInvariance](../legacy_cpp/susy_orbifolder/src/orbifolder/cgaugeinvariance.h) | Quadratische/kubische Indizes beziehungsweise Singulett- und Tensorproduktprüfungen. Anomalieprüfungen liegen an [COrbifold](../legacy_cpp/susy_orbifolder/src/orbifolder/corbifold.h): Rückgabe `bool`, veränderliche Konfiguration und Ausgabe über `CPrint`; kein eigener strukturierter Anomaliebefund. |
| [CAnalyseModel](../legacy_cpp/susy_orbifolder/src/orbifolder/canalysemodel.h), [PhenoScheme](../legacy_cpp/non_susy_orbifolder/src/orbifolder/canalysemodel.h) | Phänomenologische Einordnung, SM-/PS-/SU(5)-Schemata, Generationen, Hyperladungen, Betafunktionen und Symmetriebrechung; mit Label-/Dateiverwaltung vermischt. |
| [CMonomial](../legacy_cpp/susy_orbifolder/src/orbifolder/cmonomial.h), [CYukawaCouplings](../legacy_cpp/susy_orbifolder/src/orbifolder/cyukawacouplings.h), [CMassMatrix](../legacy_cpp/susy_orbifolder/src/orbifolder/cmassmatrix.h) | SUSY-Erweiterung: eichinvariante Monomiale/D-flache Richtungen, Kopplungen und effektive Massenmatrizen. Die SUSY-Konfiguration hält außerdem Superpotentialkopplungen und F-Terme; diese drei Header fehlen im non-SUSY-Baum. |

**Für spätere Referenzdaten:** Vollständige Spektren aus den Feldern samt Gewichten/Lokalisation und der zugehörigen Wurzel-/U(1)-Basis erfassen. Darstellungsdimensionen, gedruckte Labels oder ein Vergleichsspektrum allein bestimmen die Quantenzahlen nicht vollständig.

## Grenze zwischen Fachkern und I/O

| Abtrennbarer Bereich | Konkreter Befund |
| --- | --- |
| Mathematik | Gitterarithmetik, rationale lineare Algebra, Wurzel-/Darstellungsalgorithmen und Raumgruppenoperationen sind fachliche Rechenkerne. Die bestehenden Klassen enthalten dennoch Mutation und teils Diagnostikausgaben. |
| Physik | Massenbedingungen, Projektion, modulare Invarianz, Anomalien und Kopplungsregeln benötigen strukturierte Modelldaten. Laden über `LoadSpaceGroup`, `LoadOrbifoldGroup`, `LoadTensorProducts` sowie Diagnostikausgaben in `CheckAnomaly` gehören an die I/O-Grenze. |
| Darstellung und Bedienung | [CPrint](../legacy_cpp/susy_orbifolder/src/orbifolder/cprint.h) formatiert Text/Mathematica/LaTeX; [CPrompt](../legacy_cpp/susy_orbifolder/src/orbifolder/cprompt.h) hält Modelle, Befehle, Auswahl und Sitzung. [main.cpp](../legacy_cpp/susy_orbifolder/src/orbifolder/main.cpp) ist der CLI-Einstieg. |
| Persistenz und gemischte Hilfen | [io.hpp](../legacy_cpp/susy_orbifolder/src/io/io.hpp) enthält Parser/Ausgabe **und** mathematische Typdefinitionen; [globalfunctions.h](../legacy_cpp/susy_orbifolder/src/orbifolder/globalfunctions.h) mischt ggT/kgV, Rationalisierung, Feldvergleiche und String-/Dateifunktionen. Dateigrenzen entsprechen somit nicht Fachgrenzen. |
| Suche und Prozesssteuerung | [CRandomModel](../legacy_cpp/susy_orbifolder/src/orbifolder/crandommodel.h) organisiert zufällige Einbettungen; [CVector::CreateRandomVector](../legacy_cpp/susy_orbifolder/src/orbifolder/cvector.cpp#L591) verwendet `rand()`. [CPrompt](../legacy_cpp/susy_orbifolder/src/orbifolder/cprompt.cpp#L13) verwendet globale CLI-Daten und startet Prozesse mit `fork()`; selbst `SConfig` enthält Prozessmetadaten (`PID`). |

**Zustand und Zahlenkonventionen:** `CheckStatus`, `Reset`-/`Access*`-Methoden und veränderliche Zwischenergebnisse bilden einen zustandsabhängigen Objektgraphen. Thread-Sicherheit folgt daraus nicht. Rationale Translationen/Twistmatrizen stehen Gleitkommaimpulsen, Shifts, Eigenphasen und U(1)-Ladungen gegenüber; Vergleiche verwenden unterschiedliche Toleranzen, etwa [10⁻⁵ in linalg](../legacy_cpp/susy_orbifolder/src/linalg/linalg.hpp#L11) und [10⁻⁴ beim Twist-Update](../legacy_cpp/susy_orbifolder/src/orbifolder/ctwist.cpp#L208). Diese Konventionen sind beim späteren Oracle-Abgleich ausdrücklich zu prüfen.
