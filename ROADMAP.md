# Mögliche Erweiterungen

Festgehalten am 2026-11-15 aus der Diskussion über die physikalische Abdeckung.
Dies ist eine Sammlung zukünftiger Arbeitsbereiche, keine Zusage bereits
implementierter Funktionen oder eines vollständigen Ports der C++-Programme.
Der aktuelle Umfang steht im [Portierungs- und Paketierungsreview](./docs/14_precommit_review.md).

## Nächste mögliche Arbeitspakete

1. **Phänomenologie:** Standardmodell-/PS-/SU(5)-Einbettungen, Hyperladungswahl,
   chirale Generationen, exotische Felder und Betafunktionen nachvollziehbar
   bestimmen. Bestehende C++-Routinen können zusätzliche Referenzen liefern.
2. **Symbolische SUSY-Analyse:** Kopplungsprofile schrittweise erweitern,
   symbolische Superpotentiale/Massenmatrizen und F-Term-Ableitungen auch in
   Python bereitstellen. Nicht berechnete Koeffizienten symbolisch erhalten.
3. **Vorgegebene Vakuumerwartungswerte:** Ungebrochene Eichsymmetrie und
   Massenmatrixstruktur für explizit angegebene VEVs untersuchen; Annahmen zu
   Kählerpotential, Normierung und F-/D-Bedingungen dokumentieren.
4. **Breitere Eingaben:** Weitere Geometrien und vollständige Spin(32)-Spektren
   durch unabhängige Referenzen absichern; Torsion, Rototranslations-Einbettungen
   und zusätzliche Wilsonlinienprofile ausdrücklich abgrenzen und erweitern.
5. **Modellsuche:** Geseedete Erzeugung gültiger Einbettungen, Äquivalenztests,
   Duplikaterkennung und begrenzte Scans. Ein endlicher Scan ist keine
   vollständige Klassifikation aller heterotischen Kompaktifizierungen.

## Größere Forschungs- und Implementierungsschritte

- Numerische Stringamplituden/Yukawa-Kopplungen, Feldnormierungen und die
  Abhängigkeit von geometrischen Moduli berechnen.
- Moduli- und Vakuumstabilisierung, SUSY-Brechung sowie Quantenkorrekturen
  untersuchen; eingegebene Parameter von dynamisch bestimmten Größen trennen.
- Renormierungsgruppenentwicklung und unter diesen Voraussetzungen Massen und
  Mischungen bei niedrigen Energien bestimmen.
- Andere Kompaktifizierungsklassen wie glatte Calabi–Yau-Räume mit Eichbündeln,
  asymmetrische Orbifolds oder allgemeinere Flusshintergründe benötigen eigene
  Eingabemodelle und Rechenverfahren.

F-/D-Term- und Superpotentialanalysen beziehen sich auf SUSY-Konstruktionen.
Ein non-SUSY-Potential benötigt eine eigenständige Beschreibung. Ein nach
Auswahlregeln erlaubter Term ist noch keine numerisch berechnete Amplitude.

Für jedes neue Profil zuerst Umfang, Referenzen und Negativfälle festlegen.
Die sieben bestehenden Modell-Fixtures bleiben als Regression erhalten.
Performance künftig zusätzlich mit identischem Physikumfang und vergleichbaren
Ein-/Ausgabegrenzen zwischen C++ und Rust messen; Skalierung und Exportkosten
weiter getrennt berichten.
