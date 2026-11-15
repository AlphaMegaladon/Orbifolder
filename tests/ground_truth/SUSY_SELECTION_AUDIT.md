# SUSY-Auswahlregeln: native Kandidaten und Polynom-Invarianten

Stand: 2026-10-31. Originalquellen und sieben Kern-Fixtures bleiben unverändert.
Die zusätzliche [Referenz](./couplings_oracle.json) bindet zwei unabhängige
Prozesse je Probe/Modell sowie Quellen, Linkeingaben und Fixtures über SHA-256.

## Inaktive R- und Gamma-Prüfungen

Die drei archivierten SUSY-Geometrien enthalten nur `new non-R symmetry`.
[CreatePreCouplings](../../legacy_cpp/susy_orbifolder/src/orbifolder/cyukawacouplings.cpp#L143)
prüft R-Regeln nur bei konfigurierten Operatoren; die Gamma-Prüfung davor ist
auskommentiert. Dies ist eine Abdeckungsgrenze des Referenzprofils.

Die [Zusatz-Probe](./probes/couplings_probe.cpp) aktiviert kontrolliert
`R_i=-(q_sh,i+OsciContribution_i)`, Ziel 1 modulo `(3,3,3)` beziehungsweise
`(6,3,2)`. Der native
[GetDiscreteCharge](../../legacy_cpp/susy_orbifolder/src/orbifolder/cfield.cpp#L117)
liest im skalaren Superfeldbild die internen Komponenten 1–3. Rust reproduziert
alle einzelnen rationalen Ladungen und Entscheidungen. Bei den ursprünglichen
Z3-Trials reduziert dies 15 Kandidaten auf vier: elf isolierte R-Ablehnungen.
Gamma/W-Summen werden gespeichert und die deaktivierte Prüfung ausgewiesen.

## Tensor-Singulett und holomorphes Polynom

[AddCoupling](../../legacy_cpp/susy_orbifolder/src/orbifolder/cyukawacouplings.cpp#L515)
prüft das vollständige Tensorprodukt. Der nachgelagerte
[Wiederholungsfilter](../../legacy_cpp/susy_orbifolder/src/orbifolder/cyukawacouplings.cpp#L1552)
fragt bei unbekannten Kontraktionen
[interaktiv](../../legacy_cpp/susy_orbifolder/src/orbifolder/cyukawacouplings.cpp#L1801)
`Coupling zero? (y/n)`; er liefert keinen automatisch entscheidenden Oracle.

Z3-Feld 3 hat `(27bar,3,1)` und `q_sh=(0,-1,0,0)`. Der native Trial `[3,3,3]`
enthält ein Tensor-Singulett und wird gezählt. Die E6-Kontraktion ist symmetrisch,
die SU(3)-Kontraktion antisymmetrisch: Das Polynom aus identischen kommutierenden
Superfeldern verschwindet. Rust berichtet getrennt `tensor_singlets=1` und
`polynomial_singlets=0` und erhält die native Kandidatenreferenz.

Z3×Z3-Feld 250 hat zwei Dublettfaktoren und sonst nicht-abelsche Singuletts.
In seinem Quadrat ist `epsilon × epsilon` symmetrisch: Beide Singulettzahlen
sind eins. Die U(1)-Summe ist jedoch nicht null. Symmetrisierung muss auf der
gesamten Produktdarstellung erfolgen, nicht einzeln für jeden Lie-Faktor.

## Belegte unvollständige native Tensorzerlegung

[DecomposeIntoIrreps](../../legacy_cpp/susy_orbifolder/src/orbifolder/cgaugeinvariance.cpp#L1055)
ist bereits mit `Does not work!!!` kommentiert. Die neue Probe weist verlorene
Gewichte nach, etwa beim Z3-E6-Paar mit Höchstgewichten
`[1,0,0,0,0,0] × [1,0,0,0,0,0]`. Die Routine meldet
[Weight from Freudenthal algorithm not found](../../legacy_cpp/susy_orbifolder/src/orbifolder/cgaugeinvariance.cpp#L1149).
Zudem wird der
[rekursive Rückgabewert](../../legacy_cpp/susy_orbifolder/src/orbifolder/cgaugeinvariance.cpp#L1156)
verworfen. Äußerer Erfolg allein zertifiziert deshalb keine vollständige
Zerlegung. Die genaue Ursache aller verlorenen Gewichte wird hier nicht
behauptet; Originalquellen werden nicht korrigiert.

| Modell | Tensorpaare | Unvollständige native Zerlegungen |
| --- | ---: | ---: |
| SUSY Z3 | 17 | 8 |
| SUSY Z6-II | 7 | 2 |
| SUSY Z3×Z3/WL | 38 | 24 |

Die [Charakter-Probe](./probes/characters_probe.cpp) prüft vollständige native
Freudenthal-Charaktere gegen die Felddimensionen und bildet daraus vollständige
Tensor-, symmetrische und antisymmetrische Gewichts-Basisräume. Native
Zerlegungen werden nur bei exakt gleichem Gewichtscharakter exportiert;
andernfalls steht `native_decomposition_complete=false`. Die fehlerhaften
Dimensionstabellen der Zerlegungsroutine werden nicht als Wahrheit verwendet.

Rust rekonstruiert sämtliche 62 vollständigen Tensorcharaktere aus seiner
exakten Racah–Speiser-Zerlegung. Die Referenz sind erhaltene native Gewichte
und native Invarianzentscheidungen, nicht unvollständige Teilzerlegungen.

## Weitere native Verträge

- [CMonomial](../../legacy_cpp/susy_orbifolder/src/orbifolder/cmonomial.cpp#L120)
  akzeptiert eventuell eine nichtverschwindende erste U(1)-Ladung zur
  FI-Kompensation. Rust trennt diesen Test von `sum Q=0`. Ladungen −1, 0, +1
  bei FI=1 prüfen beide Varianten in jedem SUSY-Modell.
- [AddCoupling](../../legacy_cpp/susy_orbifolder/src/orbifolder/cyukawacouplings.cpp#L518)
  liefert unter Grad drei immer null. Das ist eine API-Suchgrenze, keine
  mathematische Aussage über lineare oder quadratische Terme.
- Der [Fixed-Torus-Filter](../../legacy_cpp/susy_orbifolder/src/orbifolder/cyukawacouplings.cpp#L659)
  verbietet eine gemeinsame invariante Ebene ohne untwisted Feld. Der Bericht
  trennt ihn von der vollständigen mathematischen Raumgruppenregel.

Aus Auswahlregel-Zulässigkeit folgt keine berechnete nichtverschwindende
Stringamplitude. Beide Prüfprofile bleiben ausdrücklich unterscheidbar.
