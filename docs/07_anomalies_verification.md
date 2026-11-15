# 07 – Anomalien und kanonische U(1)-Basis: Phase 3.3b

> Historischer Phasenbericht vom 2026-10-31. Testzahlen und Umgebungsangaben
> beziehen sich auf diesen damaligen Lauf. Für den heutigen Checkout gelten
> der [aktuelle Prüfstand](./12_repository_audit.md) und die
> [Ubuntu-Anleitung](./11_development_ubuntu.md).

**Stand 2026-10-31:** Phase 3.3b einschließlich des Anomalie-Audits und der
E6-Klarstellung vom Nutzer abgenommen. Implementiert, sämtliche sieben positiven Modell-
Abgleiche und gezielte Negativtests bestanden. Die freigegebene Zustandsengine
berechnet zuerst das Spektrum aus Modelleingaben; die Anomalie-Engine übernimmt
dieses immutable Ergebnis. Keine erwartete Anomalie oder U(1)-Basis ist
Solver-Eingabe.

## API und Datenvertrag

[anomalies](../crates/orbifolder-core/src/anomalies/mod.rs) exponiert
`representation_indices`, `trace_matter`, `canonical_u1_basis` und
`analyze_anomalies`:

```rust,ignore
let spectrum = calculate_spectrum(&solved, SolverLimits::default())?;
let analysis = analyze_anomalies(&spectrum, AnomalyOptions::default())?;
let report = analysis.report();
let canonical_spectrum = analysis.spectrum();
assert!(report.is_anomaly_free());
```

`AnomalyAnalysis` enthält ein neues Spektrum mit transformierten Ladungen und
einen `AnomalyReport`. Der Bericht trennt rohe und kanonische Spuren,
Basisabbildung/Gram-Matrix, Kac-Moody-Level, FI-Daten, Einzelprüfungen und
Gesamturteil. Ein erfolgreiches `Result` enthält vollständig berechnete
Spuren; eine verletzte physikalische Bedingung ergibt einen Bericht mit
`is_anomaly_free=false`. Form-, Basis-, Level- und nicht unterstützte
Multipletfehler sind typisierte Fehler.

Einzelprüfungen besitzen `Computed(bool)` oder `NotApplicable(reason)` sowie
tatsächliche/erwartete rationale Werte. Nicht anwendbare Prüfungen bedeuten
keine fehlgeschlagene Auswertung. Der Statusvertrag enthält außerdem
`Unsupported`; er gilt nicht als bestandene Prüfung. Alle Core-Rechnungen
verwenden `Rational`/BigInt, private Ergebnisfelder und eigene lokale Caches.
`AnomalyReport` ist `Send + Sync`. Es gibt keine Floats, I/O oder globalen
Arbeitsdaten.

## Darstellungs-Indizes und chirale Zählung

Die [Indexberechnung](../crates/orbifolder-core/src/anomalies/indices.rs)
rekonstruiert das höchste Gewicht aus exakten Dynkin-Labels. Für normierte
ADE-Wurzeln mit Normquadrat zwei gilt

```text
C2(R) = (lambda, lambda+2 rho)/2
T(R)  = dim(R) C2(R)/dim(G).
```

Damit ist `T(SU(N)-Fundamental)=1/2`, `T(SO(2N)-Vektor)=1`.
Der C++-Index `l(R)` ist **2T(R)**. Dimensionen stammen aus der bereits
verifizierten allgemeinen Weyl-Dimensionsformel; es wird keine nach Dimension
indizierte Legacy-Tabelle portiert.

Für SU(N>=3) wird die Dynkin-Kette am lexikographisch kleinsten Endpunkt
orientiert. Partitionszeilen `r_i`, die traceless höchsten Gewichte
`mu_i = r_i - sum(r)/N` und `rho_i=(N+1-2i)/2` liefern das normalisierte
kubische Casimirpolynom `I3(lambda)=sum_i(mu_i+rho_i)^3`:

```text
A(R) = dim(R) I3(lambda)/(N I3(fundamental)).
```

`A(fundamental)=1`, Konjugation kehrt das Vorzeichen um. Die Orientierung
entspricht dem nativen SU(N)-Endpunktvertrag; alle in den sieben Modellen
verwendeten Indizes stimmen nach exakter Wurzel-/Dynkin-Abbildung überein.

**E6 hat keinen kubischen Eich-Anomalieindex.** Seine primitiven invarianten
Grade sind 2,5,6,8,9,12. Das kubische Invariant der 27 ist ein Tensor auf
Darstellungsvektoren und kein symmetrischer Grad-drei-Tensor auf der Lie-Algebra.
E6, E7, E8 und SO(2N>=8) erhalten daher rohe kubische Spur null und eine klar
nicht anwendbare lokale kubische Prüfung. Quadratische Indizes bleiben
berechnet: native E6-27 `l=6`, E7-56 `l=12`, adjungierte E6/E7/E8
`l=24/36/60`.

Der Core-Spektrumsvertrag führt CPT-Partner explizit. `analyze_anomalies`
zählt daher ein unabhängiges linkshändiges Weyl-Fermion pro links-chiralem
SUSY-Multiplet beziehungsweise non-SUSY-`LeftFermion`. Neutrale links-chirale
Moduli sind enthalten, tragen zu diesen Spuren aber null bei. Rechte
Spektrumsfelder sind die bereits enthaltenen CPT-Partner und werden nicht
nochmals gezählt. Neutrale Gauginos in reellen adjungierten Darstellungen
tragen zu den hier verlangten Anomalien ebenfalls null bei.

Für unabhängig vorgegebene chirale Materie stellt `MatterField` explizit
`Handedness::{Left,Right}` und eine nichtnegative BigInt-Multiplizität bereit.
Rechtshändige unabhängige Fermionen tragen ein Minuszeichen bei; beide
Händigkeiten zählen positiv zur globalen SU(2)-Parität. Halbhypers in der
Spektrumsansicht werden ausdrücklich als nicht unterstützt abgelehnt; die
sieben Referenzmodelle besitzen N=1 beziehungsweise N=0. Die zweifelhafte
native N=2-Bedingung mit der Zahl 244 wird nicht als neue 4D-Prüfung übernommen.

## Vollständige exakte Spuren

Die [Spurberechnung](../crates/orbifolder-core/src/anomalies/traces.rs)
verwendet die gesamte Tensorproduktdimension und pro Eichfaktor die Dimension
aller Zuschauerfaktoren. Sie berechnet

- `gauge_cubic[g]`: Summe der signierten A(R_g) mit Zuschauer-Multiplizität;
- `mixed[i][g]`: Summe Q_i T(R_g) mit Zuschauer-Multiplizität;
- `gravitational[i]`: Summe Q_i mit vollständiger Darstellungsdimension;
- `abelian_cubic(i,j,k)`: den vollständigen symmetrischen Tensor der
  Q_i Q_j Q_k-Spuren, einschließlich drei verschiedener Indizes;
- SU(2)-Doublettzahlen und die allgemeine Witten-Parität
  `sum multiplicity * spectators * 2T(R) mod 2`.

Die Witten-Prüfung erfasst damit auch höhere anomaliegefährdete Isospins,
nicht nur Doubletts. Für andere einfache Faktoren ist sie `NotApplicable`.

## Kanonische U(1)_A-Transformation

Die [Basisrotation](../crates/orbifolder-core/src/anomalies/basis.rs)
arbeitet auch für eine nicht orthogonale rationale Ausgangsbasis. Mit
`G_ij=t_i dot t_j` und `g_i=Tr Q_i` wird exakt `G c=g` gelöst und

```text
F = sum_i c_i t_i
t_A = F/12  (wenn F != 0)
Tr Q_A = 12 ||t_A||².
```

Die erste Richtung ist damit basisunabhängig und übernimmt die native
Normalisierung des chiralen Spurvektors. Das übrige Komplement wird durch
rationales Gram-Schmidt und primitive ganzzahlige Vorzeichennormalisierung
bestimmt. Seine Vektoren sind paarweise orthogonal, orthogonal zu t_A und
spurfrei. Verschwindet F, wird keine anomale Richtung behauptet.

Die Matrix B erfüllt `t_new = B t_old`, `Q_new = B Q_old` und
`G_new = B G_old B^T`. `Spectrum::rebased` transformiert **alle** Feldladungen,
einschließlich CPT- und Tachyonfeldern; Darstellungen, Lokalisation,
Oszillatoren und Massen bleiben als dasselbe Ergebnis erhalten.
Der Vergleich bestätigt die erste anomale Richtung exakt als Vektor.
Im anomaliefreien Komplement werden zulässige Basiswechsel exakt nachgewiesen;
keine fiktive rohe Array-Gleichheit wird erzwungen.

## Green–Schwarz-Prüfungen und FI

Mit `delta_i = Tr Q_i/24`, abelschen Levels `K_ij=2G_ij` und den einfachen
Eich-Leveln k_G werden die basis-kovarianten Relationen geprüft:

```text
Tr[Q_i T_G²] = k_G delta_i
Tr[Q_i Q_j Q_k] = delta_i K_jk + delta_j K_ik + delta_k K_ij.
```

Für die Referenzen gilt k_G=1. Der Aufrufer kann positive ganzzahlige Level
explizit angeben. In der orthogonalen kanonischen Basis folgen insbesondere

```text
Tr Q_A³ = ||t_A||² Tr Q_A / 4
Tr Q_A Q_i² = ||t_i||² Tr Q_A / 12
Tr[Q_A T_G²] = k_G Tr Q_A / 24.
```

Alle G³-Spuren müssen verschwinden. Nicht-anomale Richtungen besitzen
`Tr Q_i=Tr[Q_i T_G²]=Tr Q_i³=Tr Q_A²Q_i=0`; auch kubische Spuren ausschließlich
nicht-anomaler Richtungen sind null. Jede Gleichheit hat einen eigenen
Prüfeintrag. Das Gesamturteil schließt die universelle GS-Kompensation ein;
es verlangt nicht das Verschwinden der rohen Anomalie in der A-Richtung.

Die Anzahl unabhängiger abelscher Anomalierichtungen wird aus dem Rang der
gravitationalen/gemischt-nicht-abelschen Kovektoren geprüft. **Der Rang der
symmetrischen kubischen Tensorscheiben ist dafür ungeeignet:** bereits eine
einzige GS-Richtung erlaubt `C_Aii=C_iAi!=0`. Zusätzliche rein abelsche
Hindernisse werden durch die vollständige Tensor-Faktorisierung abgelehnt.
Die übliche Zuordnung der gemischten abelschen Eichvariation zur A-Richtung
beinhaltet den lokalen Gegentermvertrag; Symmetrie der rohen Spuren allein
zählt keine zusätzlichen anomalen U(1).

`FiData::legacy_d0` ist exakt `Tr Q_A` und reproduziert die native D0/FI-Zahl.
Für SUSY wird zusätzlich der rationale Koeffizient `Tr Q_A/192` gespeichert:
in der deklarierten Konvention
`xi = g_s² M_Pl² (Tr Q_A/192)/pi²`. Kopplung, Massenskala und pi werden als
physikalischer Vorfaktor getrennt gehalten; es wird kein rationaler Wert für
eine irrationale Gesamtgröße erfunden. Non-SUSY bewahrt die native D0-Zahl,
weist die supersymmetrische FI-Auswertung mangels Hilfs-D-Feld jedoch klar
als `NotApplicable` aus.

## Oracle-Ergebnis und Negativfälle

[anomalies_ground_truth.rs](../crates/orbifolder-core/tests/anomalies_ground_truth.rs)
berechnet jedes Modell über die freigegebene Geometrie-/Zustandspipeline.
Alle gespeicherten rationalen `anomalies.coefficients` stimmen überein:
SU(N)³, SU(2)-Doublettzahlen, Tr Q_i, Tr Q_i³, Tr Q_i Q_j² und sämtliche
nativen `l(R)-Q_i`-Spuren. C++-`l=2T` und native Faktor-/Basisordnungen
werden ausschließlich beim Vergleich abgebildet. Jede kanonische Feldladung
und die volle transformierte Gram-Matrix werden zusätzlich exakt bestätigt.

| Modell | Native D0/FI | Rust-GS-Gesamturteil |
| --- | ---: | --- |
| SUSY Z3 | 0 | true |
| SUSY Z6-II | 0 | true |
| SUSY Z3×Z3/WL | 960 | true |
| non-SUSY Z3 | 20736 | true |
| non-SUSY Z6-II | 9216 | true |
| non-SUSY Z3×Z3/WL | 29376 | true |
| non-SUSY tachyonisch | 960 | true |

[anomalies_oracle.json](../tests/ground_truth/anomalies_oracle.json) ergänzt
die originalen Indexmethoden und 90 native `CheckAnomaly`-Polarisationsaufrufe
für Q_i+Q_j+Q_k. Die Probe berechnet keine eigene Anomaliesumme. Diese
Beobachtungen prüfen die kubischen Tensorwerte mit drei verschiedenen
U(1)-Indizes, welche der normale Bericht nicht einzeln ausgibt. Je zwei
eigenständige Prozesse stimmen überein. Quell-/Linkhashes und unveränderte
Modell-Fixture-Hashes sind gebunden. Originalquellen und sieben abgenommene
Modell-Fixtures wurden nicht verändert.

[anomaly_invariants.rs](../crates/orbifolder-core/tests/anomaly_invariants.rs)
isoliert die verlangten Ablehnungen:

1. Ein SU(3)-Fundamental ohne chiral ausgleichenden Partner verletzt allein
   die nicht-abelsche kubische Prüfung; unabhängige Gegenhändigkeit oder
   konjugierte linke Darstellung hebt sie auf.
2. Zwei SU(3)-Faktoren mit verschiedenen gemischten U(1)-Spuren bestehen
   ihre G³-Prüfungen, scheitern aber jeweils an der GS-Universalität.
3. Zwei unabhängige gemischte Kovektoren bleiben nach Rotation erkennbar;
   die zweite Richtung ist gravitational spurfrei, hat aber nichtverschwindende
   G²-Spuren und wird einzeln abgelehnt.

Weitere Tests prüfen rein abelsche Anomalien ohne gravitationalen Partner,
eine gültige einzelne GS-Richtung trotz nichtverschwindender symmetrischer
Tensorscheiben, Witten-Parität für Doubletts und höhere Isospins,
E7/E8-Indizes sowie falsche Eingabeformen, Basen und Level.

Native Einzel-Feldentfernungen weisen zusätzlich falsche C++-Erfolgsstatuswerte
nach. Beispielsweise liefert SUSY Z3 bei `Tr SU(3)³=-27` weiterhin true;
non-SUSY Z3×Z3/WL bei -1 und der tachyonische Fall bei `Tr SU(4)³=8` ebenso.
Rust lehnt alle ab. Ursachen sind ein früher true-Rücksprung ohne U(1) und
non-SUSY-Fehlerzuweisungen unter `verbose=false`. Der
[Anomalie-Audit](../tests/ground_truth/ANOMALY_VALIDATION_AUDIT.md) trennt diese
belegten Legacy-Konflikte von den sieben bestandenen positiven Gleichheiten.
Die strikte Ablehnung folgt dem ausdrücklichen Negativtest-Auftrag des Nutzers.

## Abschließende Prüfungen und Abgrenzung

```bash
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

**75 Tests bestanden**: sämtliche 59 vorherigen Tests sowie acht neue
Oracle-/Integritätstests und acht Anomalie-Invariantentests. `fmt` und Clippy
sind fehlerfrei. Die regulären Rust-Tests benötigen keinen laufenden C++-Build
und kein Python. Die ergänzende Probe/Generierung ist separat reproduzierbar.

Die abgenommene vollständige Tachyon-Normsuche bleibt Standard; der
[dritte Legacy-Befund](../tests/ground_truth/TACHYON_WINDOW_AUDIT.md) ist
ausdrücklich dokumentiert. Diskrete/R-Anomalien, neue N=2-Sonderprüfungen,
SUSY-Superpotentiale und PyO3-Bindings sind nicht Teil dieser Implementierung.
