# 06 – Zustände und Spektren: Phase 3.3a

> Historischer Phasenbericht vom 2026-10-31. Testzahlen und Umgebungsangaben
> beziehen sich auf diesen damaligen Lauf. Für den heutigen Checkout gelten
> der [aktuelle Prüfstand](./12_repository_audit.md) und die
> [Ubuntu-Anleitung](./11_development_ubuntu.md).

**Stand 2026-10-31:** Phase 3.3a einschließlich des dritten Legacy-Befunds
vom Nutzer abgenommen. Implementiert und gegen alle sieben freigegebenen
Modellreferenzen verifiziert. Die Rechnung beginnt bei den rationalen
Modelleingaben und den archivierten Geometrie-Eingabedateien. Erwartete
Zustände, Eichbasen und Spektren werden ausschließlich zum Vergleich gelesen.

## Implementierte Pipeline

Der [Core](../crates/orbifolder-core/src/physics_engine/mod.rs) bietet:

```rust,ignore
let group = embed(geometry, embedding, theory)?;
let solved = solve_sectors(&group, TachyonSearch::Complete, SolverLimits::default())?;
let spectrum = calculate_spectrum(&solved, SolverLimits::default())?;
let massless = spectrum.fields();
let tachyons = spectrum.tachyonic().fields();
```

Die Ergebnisse haben private Felder und lesende Getter. `SolvedSectors` besitzt
die zugehörige immutable Einbettung; bei der Feldbildung kann keine andere
Einbettung untergeschoben werden. `SectorKinematics` trennt wiederverwendbare
Rechtssektorrechnungen von der lokalen Eichprojektion. Linksseitige Massenlösungen
werden innerhalb eines Aufrufs nach exakt gleichem Shift und Normziel gecacht.
Es gibt weder globale Arbeitsdaten noch I/O, Python-Referenzen oder Floats im
Core. `SolvedSectors` und `Spectrum` sind nachweislich `Send + Sync` und können
zwischen unabhängigen Modellaufträgen geteilt werden. Ein Rayon-Scheduler gehört
weiterhin zur späteren Batch-/API-Schicht.

## Exakte Energien, Oszillatoren und Massenlösungen

Für `eta_i = v_i mod 1` gilt

```text
delta = 1/2 sum_i eta_i (1-eta_i)
a_L = -1 + delta,  a_R = -1/2 + delta
mu = 1/2 ||p+V||² + N_L + a_L = 1/2 ||q+v||² + N_R + a_R.
```

Negative Twist-Lifts werden mit euklidischem Modulo reduziert. Die
[Oszillatoren](../crates/orbifolder-core/src/physics_engine/oscillator.rs)
besitzen exakte Frequenzen, komplexe Richtung und Konjugationsbit.
Eine explizite DFS zählt Fock-Zustände mit Wiederholungen und `N <= mu-a` auf,
ohne rekursive Aufrufstapel. Die Oszillator-Indizes behalten die native
Reihenfolge. Der Katalog folgt dem nativen Materie-/Gravitationsvertrag:
Die dort auskommentierten 16 Cartan-Oszillatoren werden nicht zusätzlich
als eigene Felder erzeugt; auch die native Auswahl untwisted/twisted wird
beibehalten. Frequenzen oberhalb des Energiebudgets können im Katalog stehen,
gelangen aber nie in eine Anregung.

Die [Normsuche](../crates/orbifolder-core/src/physics_engine/shell.rs) arbeitet
generisch über Dimension und Gitterklasse, mit vollständigem Branch-and-Bound.
E8×E8 besitzt zwei unabhängige ganz-/halbzahlige D8-Blöcke; D16+ einen
gemeinsamen D16-Block. D8×D8 und rechte SO(8)-V/S-Klassen werden mit derselben
Enumeration behandelt. Parität wird bereits an Blockgrenzen geprüft.

Ist nach j Koordinaten das verbleibende Normbudget B, so muss jede weitere
Koordinate `(p_i+V_i)² <= B` erfüllen. Das endliche Suchintervall wird aus
`floor(sqrt(floor(B)))+1` exakt eingeschlossen und jeder Kandidat rational
nachgeprüft. Ein großer ganzzahliger Shift wird als BigInt abgespalten;
die Schleife läuft nur über das kleine Restintervall. Es gibt keine feste
Impulsschranke und keine Float-Toleranz. Überschrittene Such-/Ergebnisbudgets
liefern `PhysicsError::ResourceLimit`, kein erfolgreiches Teilspektrum.

## Tachyonen und Excited

`TachyonSearch::LegacyWindow` beschränkt das **unverschobene** rechte q auf
die nativen ganzzahligen Fenster `q0=-7..0`, `q1..q3=-7..5`.
`Complete` sucht dagegen alle zulässigen SO(8)-Klassen mit
`||q+v||² < -2(N_R+a_R)`. Unterschiedliche negative Massen werden vor der
linksseitigen Niveaumatchung getrennt. Auch rechts angeregte Zustände laufen
durch dieselben Phasen- und Projektionsfunktionen wie masselose Zustände.
Die native Nullmassen-Normgrenze bleibt im rechten `Excitation::target`
als Referenzkonvention erhalten; die tatsächliche negative Masse steht separat
in `tachyon_masses` und anschließend im Feld.

Jedes Feld bewahrt beide tatsächlichen Oszillatorbelegungen. Ein Tachyon gilt
als angeregt, wenn links oder rechts mindestens ein Oszillator besetzt ist.
Das native uninitialisierte `CHalfState::Excited` wird nicht nachgebildet.
Bei den zwölf projizierten Referenztachyonen sind **beide** Belegungen leer.
Der Vergleich nutzt damit dieselbe physikalische Normalisierung wie
`compare.py --tachyon-physics`; der strikte Legacy-Enum bleibt im eingefrorenen
JSON unverändert. Quelle: [Excited-Audit](../tests/ground_truth/TACHYON_AUDIT.md).

Zusätzlich zu den sieben Modellen beobachtet eine
[kleine native Sektor-Probe](../tests/ground_truth/probes/states_probe.cpp)
den Sektor `(1,1,0)` für `v0=(0,1,1,1)`, `v1=(0,1/6,-1/6,0)`:

| Lokale Kinematik | C++-Fenster | Vollständige Rust-Suche |
| --- | --- | --- |
| Ursprünglicher Lift | Grundzustand `mu=-1/3`, zwei Anregungen `mu=-1/6` | Identische Gewichte, Oszillatoren, Massen und Eigenphasen |
| `v0[1]=25` | Keine rechte Tachyonlösung | Dieselben drei physikalischen Lösungen wie vor dem ganzzahligen Lift |

Diese [Zusatzreferenz](../tests/ground_truth/states_oracle.json) wurde in zwei
unabhängigen Prozessen gegen unveränderte Objektdateien erzeugt. Sie prüft
lokale Kinematik und den Verlust durch die Legacy-Suchgrenze; sie behauptet
keine zusätzliche vollständige projizierte Modellreferenz.

## Phasen, Projektionen und Felder

[Zustandsbildung](../crates/orbifolder-core/src/physics_engine/states.rs)
gruppiert Massenlösungen nach exakten Eigenphasen modulo eins. Links gilt

```text
phi_L(g,h) = p_sh · V_h - 1/2 (V_g · V_h - v_g · v_h)
             + oscillator_phase_L(h) + torsion(g,h) mod 1.
```

Die rechte Phase wird aus q_sh und den rechten Oszillatoren nach der nativen
Theoriekonvention berechnet; non-SUSY verwendet das negative
Oszillatorvorzeichen. Die geprüften SUSY-Massenlösungen besitzen `N_R=0`;
ein nicht beobachteter SUSY-Zweig mit rechten Anregungen wird hierdurch
nicht als unabhängig oraclevalidiert ausgewiesen.
Die lokale Zentralisatorpräsentation ist bereits in Phase 3.2 als vollständige
Erzeugermenge zertifiziert. Projektion verlangt `phi_L=phi_R` für alle ihre
Elemente. Die Generatoren der ganzen Raumgruppe liefern danach
`gamma(h) = phi_R(h)-phi_L(h) mod 1`. Drei zyklische Faktoren werden direkt
verwendet; es gibt keinen verlorenen non-SUSY-m-Index.

Für diskrete Torsion bleibt der freigegebene exakte Bicharaktervertrag gültig.
Non-SUSY `StoredNotApplied` trägt entsprechend dem nativen ungenutzten Pfad
keine Projektionsphase bei. Die sieben Modelle besitzen keine aktive Torsion;
sie validieren daher keine neuen aktiven Legacy-Torsionspfade.

Die [Feldbildung](../crates/orbifolder-core/src/physics_engine/spectrum.rs)
berechnet überlebende Eichwurzeln aus Shifts und Wilsonlinien, wählt eine feste
Weylkammer und gruppiert linke Gewichte nach ihrem dominanten Weylvertreter.
Damit entstehen irreduzible Ladungs-/Darstellungsgruppen ohne kopierte
Representationstabellen. Dynkin-Labels sind exakt ganzzahlig; Dimensionen
folgen aus der allgemeinen Weyl-Dimensionsformel mit BigInt-Ergebnis.
Darstellungsdimensionen sind positiv, Konjugation steckt in den Dynkin-Labels.
Ein negatives C++-Dimensionslabel ist daher ein Darstellungskonventionszeichen,
keine negative mathematische Dimension.

`Multiplet` unterscheidet SUSY-Multiplets, non-SUSY-Teilchenarten und Tachyonen.
Die SUSY-Gruppierung verwendet die aus Twists berechneten Superladungen.
N=1 und N=2 werden abgebildet; N>2 wird ausdrücklich als nicht unterstützt
zurückgewiesen. Die sieben Modelle prüfen N=1 beziehungsweise N=0.
Rechte Gewichte, Ursprungsklasse und nach dem Brechungstwist verschobene
SO(8)-Klasse bleiben erhalten, einschließlich V/S/C und der Nullklasse.
Non-SUSY übernimmt nicht das leere native Feld-`q_sh`, sondern bewahrt die
tatsächlichen rechten Gewichte. Neutralen untwisted Zuständen werden die
nativen Modulus-/Gravitationsrollen zugeordnet.

## Exakte Basisabbildung ohne Anomalieberechnung

Der Core wählt seine U(1)-Generatoren deterministisch als rationale RREF-Basis
des orthogonalen Komplements der einfachen Wurzeln. Optional kann der Aufrufer
eine vollständige rationale Basis in-memory vorgeben; Rang und Orthogonalität
werden geprüft. Er berechnet keine aus einem chiralen Spurvektor bestimmte
„anomale erste U(1)-Richtung“.

Der Integrationsadapter ordnet die **berechneten** einfachen Wurzeln durch
Identität der Wurzelvektoren der nativen Faktor-/Dynkin-Reihenfolge zu.
Für native Generatoren T und die berechneten Generatoren U löst er exakt
`U_i = sum_j A_ij T_j` und verlangt `Q_U = A Q_T`. Zusätzlich wird jede
native Ladung direkt als `p_highest · T_j` bestätigt. Native U(1)-Generatoren
und Faktorenreihenfolgen gehen dabei ausschließlich in die Assertions ein,
nie in einen Core-Solver. Der Nachweis ist basisabgeglichen; er behauptet
keine rohe JSON-Array-Gleichheit zweier verschiedener Basiskonventionen.

Alle C++-Doubles werden nur im Testadapter eindeutig rational rekonstruiert.
Für diese Eingaben reichen Nenner 12 für verschobene Gewichte und Ladungen,
72 für bilineare Phasen/Energien (`2*lcm(2,3,6)^2`). Die Kandidaten müssen
zusätzlich innerhalb des dokumentierten IEEE-Rundungsbudgets liegen; fehlende
oder mehrdeutige Rekonstruktion scheitert. Im Core gibt es keinen solchen
Rationalisierungsschritt.

## Ergebnis des Abgleichs

[states_ground_truth.rs](../crates/orbifolder-core/tests/states_ground_truth.rs)
berechnet jedes Modell zweimal, mit beiden Suchprofilen, und verlangt für
diese sieben Eingaben identische Sektorergebnisse. Verglichen werden sämtliche
exportierten Nullpunktsenergien, Oszillatorkataloge, masselosen Rechts-/Links-
Massenlösungen samt Reihenfolge und Belegung, Halbzustandsgruppierungen und
Eigenphasen, alle projizierten Zustände, Gamma-/Oszillatorbeiträge sowie die
rechten Tachyonkandidaten mit Massen und projizierten Tachyonzuständen.

Bei den Feldern werden Quelle/Lokalisation, vollständige linke Gewichtsindizes,
rechte Ladungsgruppen, SUSY-/Teilchen-/physikalische Tachyonart, sämtliche
Dynkin-Labels, Dimensionen und U(1)-Ladungen geprüft. Beide Massenformeln werden
für jedes Feld exakt nachgerechnet. Die separat gespeicherten Tachyonfelder
müssen die angehängten nativen Objekte bis auf deren Containerindex duplizieren.

| Modell | Masselose Felder | Tachyonfelder |
| --- | ---: | ---: |
| SUSY Z3 | 246 | 0 |
| SUSY Z6-II | 544 | 0 |
| SUSY Z3×Z3 mit Wilsonlinie | 524 | 0 |
| non-SUSY Z3 | 438 | 0 |
| non-SUSY Z6-II | 832 | 0 |
| non-SUSY Z3×Z3 mit Wilsonlinie | 1112 | 0 |
| non-SUSY tachyonisch | 604 | 12 |
| **Summe** | **4300** | **12** |

Die genannten 4312 sind die nativen StandardConfig-Objekte **einschließlich**
der zwölf angehängten Tachyonfelder. Rust führt beide Spektren getrennt.
CLI-Feldnamen/-Nummern und zusätzliche diskrete Auswahlregeln sind keine
neuen berechneten Observable dieser Stufe.

[physics_invariants.rs](../crates/orbifolder-core/tests/physics_invariants.rs)
ergänzt negative Twist-Lifts, wiederholte Oszillatoren am exakten Budgetrand,
alle Elternwurzelmengen, Ablehnung benachbarter Normziele ohne Float-Toleranz,
10^55-große Gitter-Lifts, Dimensionen jenseits jeder kleinen Tabelle,
Ressourcen-/Formfehler und einen Property-Test gegen eine unabhängige
kartesische SO(8)-Aufzählung. Frühere Arithmetik-, Weyl-, Geometrie- und
Einbettungsregressionen bleiben erhalten.

## Abschließende Prüfungen

```bash
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

59 Tests, einschließlich acht neuer nativer Zustands-/Spektrumtests und sieben
neuer mathematischer Invariantentests, sind grün. `fmt` und `clippy` sind
warnungsfrei. Die normalen Rust-Tests benötigen weder C++ noch Python.
Zusätzlich besteht `compare.py --self-test --tachyon-physics` für alle sieben
unveränderten JSON-Fixtures: Schema, Referenzen, exakte Metrik,
Selbstvergleich, sechs Korruptionsprüfungen je Modell und der Excited-Audit.
Dieser Phase-3.3a-Bericht prüft keine Anomalien, FI-Terme, Superpotentiale,
Monomiale oder PyO3. Die spätere Anomalie-Implementierung wird separat im
[Phase-3.3b-Bericht](./07_anomalies_verification.md) verifiziert.
Der verbindlich entschiedene zweite non-SUSY-Altcode-Fehler ist im
[Wilson-Audit](../tests/ground_truth/WILSON_INVARIANCE_AUDIT.md) dokumentiert;
die exakte Ablehnung für beide Theorien bleibt aktiv.
