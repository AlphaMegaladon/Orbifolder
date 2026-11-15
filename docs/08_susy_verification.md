# 08 – SUSY-Monomiale und Kopplungen: Phase 3.3c

> Historischer Phasenbericht vom 2026-10-31. Testzahlen und Umgebungsangaben
> beziehen sich auf diesen damaligen Lauf. Für den heutigen Checkout gelten
> der [aktuelle Prüfstand](./12_repository_audit.md) und die
> [Ubuntu-Anleitung](./11_development_ubuntu.md).

**Stand 2026-10-31:** Phase 3.3c einschließlich des SUSY-Auswahlregel-Audits
vom Nutzer abgenommen. Phase 3.3c implementiert und verifiziert; **89 Tests,
fmt und Clippy bestanden**. Originalquellen und sieben Kern-Fixtures bleiben
unverändert. PyO3 und Rayon-Batch-Pipelines sind weiterhin Phase 3.4.

## Typ- und Ordnungsprofil

Die [SUSY-Erweiterung](../crates/orbifolder-core/src/susy/mod.rs) erstellt
`SusySpectrum` ausschließlich aus dem validierten Modell oder seinen gelösten
Sektoren und prüft Theorie sowie tatsächlich erhaltene Superladungen. Ein
fremdes Spektrum kann nicht angehängt werden. Alle vier non-SUSY-Fixtures
ergeben `NotApplicable`. Monomiale akzeptieren nur vorhandene linkschirale
Superfelder. Der gemeinsame
[Computation-Status](../crates/orbifolder-core/src/analysis.rs) wird von der
bisherigen Anomalie-API weiterhin reexportiert.

```rust,ignore
if let Computation::Computed(susy) = SusySpectrum::compute(&group, solver_limits)? {
    let m = Monomial::new(&susy, &[field_a, field_b, field_c])?;
    let reports = analyze_couplings(&susy, &[m], &rules, analysis_limits)?;
    let w = enumerate_superpotential(&susy, &field_pool, 3, &rules, analysis_limits)?;
}
```

Das verifizierte Profil umfasst Monomiale von **Grad eins bis drei**, native
kubische Kopplungskandidaten und vollständige Enumeration innerhalb des
angegebenen Feldpools. Höhere Grade ergeben `UnsupportedOrder`; Ressourcenlimits
liefern Fehler statt abgeschnittener Erfolgsresultate. Ergebnisse besitzen
private Daten und lesende Getter; alle Arbeitsdaten sind aufruflokal.

`Superpotential` speichert symbolische Monomialklassen mit formalen F-Term-
Ableitungen. Kopplungsstärken, Komponenten-Kontraktionsbasen, F-flache VEVs und
numerische D-Term-Lösungen werden nicht berechnet. Holomorphe Gauge-Invarianten
charakterisieren mögliche D-flache Richtungen bei FI=0; die getrennte native
FI-Ladungsprüfung ist kein VEV-Nachweis.

## Exakte Mathematik und Auswahlregeln

[CharacterAlgebra](../crates/orbifolder-core/src/math_core/characters.rs)
verwendet Dynkin-Koordinaten, die inverse Cartan-Matrix als exakte Metrik und
Freudenthal-Rekursion in einer endlichen Normkugel von `lambda + root lattice`.
Gewichtsmultiplizitäten sind ganze Zahlen; ihre Summe wird unabhängig durch
Weyls Dimensionsformel geprüft. Racah–Speiser zerlegt Tensorprodukte mit
verschobenen Weyl-Reflexionen, Vorzeichen und Wandtests. Dualgewichte bestimmen
kubische Tensor-Singuletts.

Adams-Operationen und
`Sym² chi=(chi²+psi² chi)/2`,
`Sym³ chi=(chi³+3 chi psi² chi+2 psi³ chi)/6`
symmetrisieren identische vollständige Feldrepräsentationen. Die
Permutationsspuren werden zuerst über alle Eichfaktoren multipliziert. Der
[Monomialbericht](../crates/orbifolder-core/src/susy/invariants.rs) trennt
`tensor_singlets` und `polynomial_singlets` sowie exakte `sum Q=0` und native
FI-Kompensation.

Der [Raumgruppen-Test](../crates/orbifolder-core/src/geometry/space.rs) entscheidet,
ob das Produkt der Konjugationsklassen die Identität enthält: drei zyklische
Faktoren, endliche Punktgruppen-Konjugationen und die ganzzahlige Bildsumme
`L=sum_i (1-R_i) Lambda`. Smith-Normalform entscheidet über die vollständige
Produkttranslation. Affine Schnitte bleiben erhalten; simultane Konjugation
fixiert die erste Konjugation ohne Verlust. Tests prüfen Konjugations- und
Permutationsinvarianz sowie nichttriviale Resttranslationen.

[DiscreteRule](../crates/orbifolder-core/src/susy/selection.rs) fordert exakt
`(sum charge-target)/order` ganzzahlig. R-Operatoren verwenden interne Komponenten
des skalaren `q_sh + oscillator_contribution`; die durch Superladungen bewiesene
Bildzuordnung aus dem Spektrumlöser wird wiederverwendet. Non-R-Operatoren lesen
`(m,n,k,l1,...,l6)` und müssen einen Charakter der Raumgruppe definieren;
Rotationsrelationen und affine Faktorüberträge werden geprüft. Nur der externe
Testadapter bildet Legacy-Achtslot-Operatoren auf das kanonische Schema ab.

Die drei archivierten SUSY-Geometrien aktivieren keine R-Regeln. Das native
Profil reproduziert genau diese Konfiguration; explizite R-Profile ergänzen
die separat durch Probes geprüften Ordnungen. Gamma/W-Summen bleiben gespeichert;
die deaktivierte native Prüfung erhält `NotApplicable`. Nichttriviale Torsion
und zusätzliche unkonfigurierte Phasenregeln sind durch diese Kopplungsreferenzen
nicht zertifiziert.

`native_allowed()` reproduziert C++-Tensor-Kandidaten samt Mindestgrad drei und
Fixed-Torus-Filter. `polynomial_allowed()` prüft symmetrisierte holomorphe
Invarianten und die konfigurierten mathematischen Regeln. Zulässigkeit ist kein
Nachweis einer berechneten Stringamplitude. Der
[Audit](../tests/ground_truth/SUSY_SELECTION_AUDIT.md) dokumentiert diese
Abgrenzung, Wiederholungsfälle und unvollständige native Tensorzerlegungen.

## Ground Truth und Abdeckung

Der [Generator](../tests/ground_truth/generate_couplings.py) verwendet isolierte
Runtime-Verzeichnisse, verifiziert Quellmanifest und Linkeingaben, prüft native
Charakterdimensionen/Basisräume und bestätigt zwei unabhängige Prozesse je
Probe/Modell. Die [Zusatzreferenz](../tests/ground_truth/couplings_oracle.json)
ist über SHA-256 an die drei SUSY-Fixtures und alle Probe-/Runnerquellen gebunden.
Die sieben Kern-Fixtures werden nicht neu geschrieben.

| Modell | Original-Cubics | Native erlaubt | Mit explizitem R-Profil erlaubt | Zusatz-Trials |
| --- | ---: | ---: | ---: | ---: |
| SUSY Z3 | 56 | 15 | 4 | 15 |
| SUSY Z6-II | 56 | 1 | 1 | 15 |
| SUSY Z3×Z3/WL | 56 | 0 | 0 | 13 |

[Integrationstests](../crates/orbifolder-core/tests/susy_ground_truth.rs)
berechnen Modell, Zustände und Spektrum aus reinen Eingaben und vergleichen
alle **211 Trials**. Feld-IDs werden erst danach anhand Quelle, Gewichtsindizes
und skalarer Rechtsladung zugeordnet. Erwartete Auswahlentscheidungen, Gewichte
oder Ladungen gehen nicht in den Solver ein. Die native U(1)-Basis dient allein
der Ausgabekoordinatentransformation beim Vergleich der Ladungssummen.

Verglichen werden alle 168 Original-Cubics samt Monomialexponenten/-feldern,
native Tensor-Invarianz, `CreatePreCouplings`, `AddCoupling`, explizite R-Ladungen
und kontrollierte Space-only-Prüfungen. Zusatzfälle isolieren elf R- und zwei
Raumgruppen-Ablehnungen in Z3 sowie jeweils zwei Eichinvarianz-Ablehnungen in
Z6-II und Z3×Z3. Neun native FI-Ladungsdiagnosen prüfen beide Demand-Varianten.
Alle vier non-SUSY-Abschnitte stimmen mit `NotApplicable` überein.

[Charaktertests](../crates/orbifolder-core/tests/characters_ground_truth.rs)
vergleichen **33 native Darstellungscharaktere**, **62 vollständige Tensorpaare**,
Rust-Irrep-Rekonstruktionen, symmetrische/antisymmetrische Quadrate und
**25 symmetrische Kuben**. Unvollständige native Zerlegungen werden ausdrücklich
gekennzeichnet. [Invariantentests](../crates/orbifolder-core/tests/susy_invariants.rs)
ergänzen SU(2)-Clebsch–Gordan-Propertytests, SU(3)-Dualität und kubische Symmetrie,
BigInt-Dimensionen, falsche Eingaben, gebrochene Ladungen und Raumgruppenfälle.

## Abschließende Prüfungen

```bash
bash tests/ground_truth/build_couplings_probe.sh
python tests/ground_truth/generate_couplings.py
python tests/ground_truth/compare.py --self-test
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

**89 Tests bestanden**: die 75 bisherigen Tests, drei Charakter-Oracle-Tests,
fünf SUSY-/Integritätstests und sechs Invariantentests. `fmt` und Clippy sind
fehlerfrei. Reguläre Rust-Tests benötigen weder C++-Build noch Python.
Der Comparator-Selbsttest bestätigt zusätzlich alle sieben Kern-Fixtures
einschließlich Schema, Referenzen, exakter Metrik und gezielter Beschädigungen.
