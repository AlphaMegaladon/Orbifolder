# 04 – Implementierung und Verifikation von `math_core`

> Historischer Phasenbericht vom 2026-10-31. Testzahlen und Umgebungsangaben
> beziehen sich auf diesen damaligen Lauf. Für den heutigen Checkout gelten
> der [aktuelle Prüfstand](./12_repository_audit.md) und die
> [Ubuntu-Anleitung](./11_development_ubuntu.md).

Phase 1 und 2 sind freigegeben. Dieser Nachweis betrifft ausschließlich
[orbifolder-core::math_core](../crates/orbifolder-core/src/math_core/mod.rs).
Raumgruppen, Zustandslöser, Anomalieberechnung, Rayon-Batches und PyO3 gehören
weiterhin zu den nachfolgenden Phasen.

## Öffentliche mathematische Verträge

| Baustein | Implementierter Vertrag |
| --- | --- |
| [Rational, Phase](../crates/orbifolder-core/src/math_core/rational.rs) | Private kanonische `i128`-Zähler-/Nennerpaare; geprüfte Zwischenprodukte; exakter `BigRational`-/BigInt-Fallback. Automatische Rückkehr zum kleinen Backend, sobald das Ergebnis passt. Nenner positiv, Null `0/1`, sichere Behandlung von `i128::MIN`, exakter Vergleich, floor und Modulo eins auch für negative Zahlen. |
| [Vector, Matrix](../crates/orbifolder-core/src/math_core/linear.rs) | Immutable Vektoren mit const-Dimension; Aliase für 4/6/16 Komponenten. Formgeprüfte rechteckige Matrizen, Skalarprodukte, bilineare Formen, RREF, Rang, Determinante, Inverse, affine Lösungsräume und Nullraum. Positive Definitheit durch rationale LDL-Elimination. |
| [GaugeLattice, DnClass](../crates/orbifolder-core/src/math_core/lattice.rs) | E8×E8, D16+ = Spin(32)/Z2, D8×D8; D-gerade-Nebenklassen 0/V/S/C, Nebenklassenaddition, Äquivalenz und exakte kleinste Gitterordnung eines rationalen Vektors. Keine komponentenweise Modulo-Reduktion von Einbettungen. |
| [RootSystem, LieFactor](../crates/orbifolder-core/src/math_core/lie.rs) | Norm-zwei-Wurzeln, einfache Wurzeln einer expliziten regulären positiven Kammer, ADE-Zerlegung, vollständiger Weyl-Abschluss, Cartan-Matrizen, ganzzahlige Dynkinlabels, Fundamentalgewichte und eindeutiges dominantes Gewicht einer vorgegebenen irreduziblen Gewichtsmenge. |
| Weyl-Reflexion und Wurzelfilter | Exakt `v − 2(v·α)/(α·α) α`; überlebende Wurzeln erfüllen sämtliche vorgegebenen rationalen Kongruenzen `α·V ∈ Z`. Dies implementiert keine Sektorprojektion oder modulare Invarianz. |

Es gibt keinen Float-Typ im Core und keine Eingabe über Datei- oder Python-APIs.
Alle Typen sind `Send + Sync`; Vektoren, Matrizen, Faktordaten und Lösungsräume
exponieren ausschließlich lesende Zugriffe. Lokale Eliminationspuffer bleiben
innerhalb reiner Funktionen. Der Core verbietet `unsafe_code`, Compilerwarnungen
und Clippy-`all`-Warnungen. Release-Builds behalten Overflow-Checks bei.

Der schnelle rationale Pfad verwendet bewusst eigene gekürzte `i128`-Paare,
damit auch interne Operationen keine ungeprüften `Ratio<i128>`-Überläufe
auslösen. Die Promotion erfolgt pro überlaufender Operation; eine vollständige
Modellstufe muss dafür nicht neu gerechnet werden. Standardmäßig gibt es kein
BigInt-Bitlimit; Fehler bei Nenner null, Division durch null, nichtintegralen
Dynkinlabels, ungültigen Dimensionen und singulären/inkonsistenten Systemen
bleiben explizit. Weyl-Enumeration besitzt ein explizites Größenlimit.

## Zusätzliche native mathematische Referenzen

Die sieben freigegebenen [Modell-Fixtures](../tests/ground_truth/fixtures/) wurden
nicht verändert. Ihnen fehlen direkte Cartan-Funktionsresultate, Spin(32)-
Mitgliedschaftstests und große rationale Operanden. Diese Lücke schließt separat
[math_oracle.json](../tests/ground_truth/math_oracle.json), keine achte Modell-Fixture.

[math_probe.cpp](../tests/ground_truth/probes/math_probe.cpp) wird je Variante
gegen die vorhandenen originalen Objekte ohne `main.o` gelinkt. Es ruft native
Gittertests, den Spin(32)-Wurzellöser, `calculateCartanMatrix`, `findDynkinLabels`
und rationale `CHugeInt`-Arithmetik mit einem Operanden `2^160/3` auf. Je Modell
stimmen zwei unabhängige Probe-Prozesse überein. Provenienz enthält Quell-,
Binary-, Objekt-/Bibliotheks-, Compiler-/Build- und Generatornachweise. SHA-256
bindet die ergänzenden Ergebnisse an genau die sieben früheren Fixture-Dateien.
Die Legacy-Quellen bleiben unverändert.

Reproduktion bei bereits vorhandenen Legacy-Builds:

```bash
bash tests/ground_truth/build_math_probes.sh
python3 tests/ground_truth/generate_math.py
```

Die Erzeugung überschreibt regulär keine Referenz. Ein anderer Build oder ein
geänderter Generator erfordert einen geprüften Diff und explizites `--overwrite`.
Historische C++-Header erzeugen weiterhin Deprecation-Warnungen; sie wurden nicht
für diesen Schritt umgeschrieben.

## Sieben unabhängige Modell-Abgleiche

[ground_truth.rs](../crates/orbifolder-core/tests/ground_truth.rs) prüft je Modell:

1. Rationale Grundoperationen und die vollständige Menge der 480 E8×E8-
   Elternwurzeln gegen den nativen Export.
2. SO(8)-Beispiele einschließlich der unterschiedlichen SUSY-/non-SUSY-Enums.
3. Einbettungen aus den gespeicherten **rationalen Modelltexten** gegen die
   beobachteten C++-Werte; Wurzelfilterung aus Shifts und allen Wilsonlinien.
4. Die vollständige überlebende Wurzelmenge gegen die nativen untwisted
   Halbzustände und deren Eigenphasen.
5. Die unabhängig ermittelten einfachen Wurzeln und ADE-Faktoren. Die Kammer
   `(23,6,5,4,3,2,1,0)` je E8 ist eine Adapterkonvention. Erst nach exakter
   Mengengleichheit werden Knoten anhand identischer Wurzelvektoren auf die
   C++-Reihenfolge permutiert. Keine Wurzel wird durch eine passende ersetzt.
6. Cartan-Matrizen und Dynkinlabels aller 480 Elternwurzeln pro Faktor gegen
   die zusätzlichen **nativen Funktionsaufrufe**.
7. Höchstgewichte und U(1)-Ladungen sämtlicher 4.312 gespeicherter Feldobjekte
   aus deren nativen Gewichtslisten. Die 616 Felder des positiven Falls
   enthalten die zwölf angehängten Tachyonfelder. Dieser Test wertet bereits
   vorhandene Gewichte aus und erzeugt keine Rust-Zustände.
8. Orthogonalität der nativen U(1)-Generatoren zum Wurzelsystem, Gesamtrang 16,
   positive Gram-Matrizen, Inversion und `R^T G R = G`. Die rationale
   Generatorpotenz zur gespeicherten Ordnung ist jeweils die Einheitsmatrix.

| Modell | Überlebende Eichwurzeln | Halbeinfacher Rang | Geprüfte Feldobjekte |
| --- | ---: | ---: | ---: |
| SUSY Z3 | 318 | 16 | 246 |
| SUSY Z6-II | 312 | 14 | 544 |
| non-SUSY Z3 | 158 | 15 | 438 |
| non-SUSY Z6-II | 152 | 13 | 832 |
| SUSY Z3×Z3 mit WL | 18 | 7 | 524 |
| non-SUSY Z3×Z3 mit WL | 86 | 12 | 1112 |
| non-SUSY tachyonisch | 96 | 13 | 616 |

Nur der Testadapter rekonstruiert rationale Werte aus beobachteten C++-Doubles.
Seine Nennergrenze ist `2*lcm(Eingabenenner)` für verschobene Gewichte/Ladungen,
zwei für Elternwurzeln und einfache Wurzeln sowie eins für die nativen
U(1)-Generatoren dieser Fälle. Ein Kandidat muss innerhalb eines Rundungsbudgets
von `256*EPSILON*(1+|x|)` liegen und unter der jeweiligen Grenze **eindeutig**
sein. Fehlende oder mehrdeutige Kandidaten lassen den Test scheitern. Die Zahl
256 begrenzt hier konservativ die elementaren Operationen der gespeicherten
niedrigen Linearkombinationen; sie ist keine Toleranz für physikalische
Entscheidungen. Nach der Adapterkonvertierung sind alle Vergleiche exakt.

Der SO(8)-Core unterscheidet Klasse 0 und ungültige Gewichte. Die ursprünglichen
SO(8)-Selektoren geben für beide `NO_LATTICE=0` zurück; diese Zusammenfassung
geschieht ausschließlich im Testadapter. D8-Wurzeln werden zusätzlich indirekt
über den nativen D8-Eichfaktor geprüft, E7 über den vollständig verifizierten
E8-Wurzelsatz und dessen orthogonales Teilsystem. Weyl-Involution und
Normerhaltung sind mathematische Invariantentests, kein behaupteter direkter
C++-Weyl-Funktionsaufruf. Spin(32)-Spektren sind nicht Gegenstand dieses Schritts.

## Prüfungen ausführen

```bash
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Zusätzlich zum Oracle-Abgleich prüfen fokussierte Tests und Proptest unter
anderem Overflow-Promotion/-Demotion, Kreuzkürzung, Vergleiche großer Brüche,
negative Phasen, Weyl-Involution/Normerhaltung, Nebenklassenaddition,
Matrix-Inversion/-Lösungsräume und ungültige Eingaben. Die normalen Tests
benötigen nur Cargo und die gespeicherten Referenzdateien. Verifiziert unter
WSL/Linux mit Rust/Cargo 1.99.1; deklarierter Mindestcompiler ist Rust 1.85.
