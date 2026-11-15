# 05 – Geometrie, Raumgruppe und Einbettung: Phase 3.2

> Historischer Phasenbericht vom 2026-10-31. Testzahlen und Umgebungsangaben
> beziehen sich auf diesen damaligen Lauf. Für den heutigen Checkout gelten
> der [aktuelle Prüfstand](./12_repository_audit.md) und die
> [Ubuntu-Anleitung](./11_development_ubuntu.md).

**Stand:** implementiert, geprüft und am 2026-10-31 fachlich abgenommen.
Der gezielt isolierte non-SUSY-Wilson-Kongruenzfehler führt außerhalb der sieben
Referenzmodelle zu einem expliziten Oracle-Konflikt. Die verbindliche Entscheidung steht
am Ende dieses Berichts. Es wurden weder Legacy-Quellen noch abgenommene
Modell-Fixtures geändert.

## Implementierter Vertrag

[geometry](../crates/orbifolder-core/src/geometry/mod.rs) besitzt ausschließlich
exakte, immutable Ergebnistypen. `GeometrySpec` und `EmbeddingSpec` sind
Eingabespezifikationen; erfolgreiche Konstruktion ersetzt mutable Statusflags.
`embed` übernimmt eine `Geometry` oder ein `Arc<Geometry>` zur Wiederverwendung
derselben zertifizierten Geometrie für viele Einbettungen. Es gibt keine
Massenlösungen, Oszillatoren, Zustandsprojektionen oder Rust-Spektrumerzeugung.

| Bereich | Exakte Prüfung beziehungsweise Berechnung |
| --- | --- |
| Torusgitter | Ganzzahlige Koordinatenbasis Z⁶ mit positiv definiter rationaler Gram-Matrix G. Rotationsmatrizen sind in dieser Gitterbasis ganzzahlig und erfüllen `RᵀGR=G`; sämtliche rationalen Rechnungen benutzen den hybriden Zahlentyp aus Phase 3.1. |
| Punkt-/Spin-Gruppe | Drei zyklische Faktoren mit Ordnung eins für ungenutzte Slots. Der nichttriviale Spin-Faktor darf geometrisch die Einheitsmatrix besitzen. Vollständige Twistrepräsentanten bleiben erhalten. |
| Twist-/Rotationskonsistenz | Für **jedes** Punktgruppenelement stimmt das charakteristische Polynom der 6D-Rotation exakt mit den räumlichen Twist-Eigenwerten überein. Zyklotomische Polynome und rationale Spuren ersetzen trigonometrische Funktionen. Generatorpotenzen und Kommutatorrelationen werden geprüft. |
| Raumgruppe | `(R,l)(S,t)=(RS,l+Rt)`, exakte Inverse, Konjugation und Kommutation. Alle drei Exponenten bleiben erhalten. Rationale Generatortranslationen sind zulässig, wenn Potenzen und Kommutatoren Gittertranslationen ergeben. |
| Fixbranen | Affine Lösung von `(I−R)f=l`, besondere Lösung und **primitive ganzzahlige** Basis des invarianten Gitters. Nicht vorhandene Fixloci sind ein expliziter Ausgang. Koordinaten werden nicht pauschal modulo eins reduziert. |
| Konjugationsklassen | Smith-Normalform von `I−R`, endlicher Torsionsquotient und Orbits unter der gesamten Punktgruppe. Damit wird die Vollständigkeit des Katalogs berechnet. |
| Zentralisator | Für jede Punktgruppenkomponente h wird `(I−R_g)t=(I−R_h)l_g` als ganzzahlig-affines System gelöst. Ergebnis: endliche Punktgruppen-Nebenklassen plus primitives invariantes Translationsgitter; damit Darstellung der vollständigen unendlichen Gruppe. |
| Wilsonrelationen | Koinvariantenquotient `Z⁶ / Σ_i (I−R_i)Z⁶`. Daraus folgen die erlaubten Ordnungen, Aliasgruppen und sämtliche linearen Relationen. Einbettungen müssen diese Relationen **modulo Eichgitter** erfüllen. |
| Einbettung | Lokaler Shift `Σ m_i V_i + Σ l_a W_a` und lokaler Twist `Σ m_i v_i` für jedes konstruierende Element und jede importierte Zentralisatordarstellung; exakte Ordnung im E8×E8- oder D16⁺-Gitter. |
| Modulare Invarianz | Diagonal `N_i(V_i²−v_i²) ∈ 2Z`, paarweise `gcd(N_i,N_j)(V_i·V_j−v_i·v_j) ∈ 2Z`; Wilsonslots besitzen Twist null und die geometrisch ermittelte Ordnung. Alle 45 Bedingungen der neun Slots werden ausgewertet und ihre rationalen Werte gespeichert. |
| Superladungen | Vier unabhängige CPT-Repräsentanten der SO(8)-Spinoren werden gegen **alle** Twists auf ganzzahlige Skalarprodukte geprüft. Die Anzahl wird nicht aus `TheorySpec` oder einem Statusflag gelesen. |
| Torsion | Exakte alternierende Bicharakter-Daten auf den neun Abelianisierungsslots, geprüft gegen Generatorpotenzen, Rotations-/Translationsrelationen und Kommutatoren. SUSY erhält einen exakten Phasenvertrag. non-SUSY erhält `StoredNotApplied`; eine aktive Projektionsphase wird ausdrücklich abgelehnt. |

Die [Smith-Implementierung](../crates/orbifolder-core/src/geometry/integer.rs)
liefert ganzzahlige Zertifikate `UAV=D`. Property-Tests prüfen die Diagonalform,
Teilbarkeit, `det(U),det(V)=±1`, primitive Kerne und ganzzahlige Lösbarkeit.
Die Enumeration besitzt explizite Grenzen und liefert bei Überschreitung einen
Fehler, kein unvollständiges leeres Resultat.

## Herkunft der Eingaben und Vermeidung eines zirkulären Tests

Der [Testadapter](../crates/orbifolder-core/tests/common/geometry_input.rs) liest
den gespeicherten rationalen Modelltext und die darin referenzierte Geometrie
aus [geometry_inputs.json](../tests/ground_truth/geometry_inputs.json). Dieses
Bündel enthält die originalen Geometrietexte samt SHA-256; lokale C++-Archive
sind dafür nicht erforderlich. Der Hash muss der Eingabe-Provenienz entsprechen. Er liest
**keine erwarteten Sektoren oder Zentralisatoren aus den Ergebnisabschnitten**
als Rust-Eingabe.

Die Koordinate `1.7320508076` wird über die bereits geprüfte Q(√3)-Interpretation
der drei Geometriefamilien normalisiert. G entsteht durch symbolische
Skalarprodukte der Eingabebasis; die rationalen Rotationen werden aus ihren
exakten Twistwinkeln in dieser Basis gebildet. Unbekannte Basisformen werden
vom Testadapter nicht gerundet oder still rekonstruiert.

Die ursprünglichen Geometriedateien enthalten feste Repräsentanten und endliche
Zentralisatorlisten als **Eingabekatalog**. Die Auswahl und Reihenfolge dieser
Repräsentanten wird für den Listenvergleich erhalten. Unabhängig davon muss
Rust beweisen:

1. Jeder Repräsentant besitzt den berechneten Fixlocus.
2. Es gibt genau einen Repräsentanten je selbst berechnetem Punktgruppenorbit
   des ganzzahligen Torsionsquotienten; keine Duplikate oder fehlenden Klassen.
3. Jeder Zentralisatoreintrag kommutiert exakt mit dem konstruierenden Element.
4. Die importierte Liste erzeugt den **vollständigen** berechneten Zentralisator.
   Dazu wird die endliche Punktgruppenabbildung enumeriert. Schreier-Produkte
   liefern die erzeugten reinen Translationen; ganzzahlige Bildtests weisen die
   Gleichheit mit dem primitiven invarianten Gitter nach.

Ohne Eingabekatalog erzeugt die öffentliche API eigene deterministische
Repräsentanten und Zentralisatorgeneratoren. Diese müssen nicht dieselbe
drucktechnische Basiswahl wie der C++-Katalog besitzen.

## Nachgewiesene sieben Modellfälle

[geometry.rs](../crates/orbifolder-core/tests/geometry.rs) vergleicht Sektoren,
konstruierende Elemente und Zentralisatorlisten elementweise, nicht nur deren
Anzahlen. Hinzu kommen rationale Rotationsmatrizen, Twists, Gram-Matrizen,
Wilsonordnungen/-aliasgruppen, sämtliche lokalen Shifts/Twists, Superladungen
und die erfolgreichen nativen Statuswerte.

| Modell | Sektoren | Fixbranen-Repräsentanten | Erhaltene Superladungen |
| --- | ---: | ---: | ---: |
| SUSY Z3 | 3 | 55 | 1 |
| SUSY Z6-II | 6 | 45 | 1 |
| SUSY Z3×Z3 mit WL | 9 | 109 | 1 |
| non-SUSY Z3 | 6 | 110 | 0 |
| non-SUSY Z6-II | 12 | 90 | 0 |
| non-SUSY Z3×Z3 mit WL | 18 | 218 | 0 |
| non-SUSY tachyonisch | 12 | 90 | 0 |

Insgesamt werden 717 Fixbranen-Repräsentanten geprüft. Die Zahl der nativen
Fixpunkt-/Fixlocus-Flags und ihre kartesischen Koordinaten stammt zusätzlich
aus echten `SG_FixedPoint`-Aufrufen. Rust löst stattdessen das exakte rationale
Fixgleichungssystem. Nur für den abschließenden Vergleich mit diesen gerundeten
kartesischen C++-Koordinaten verwendet der Testadapter ein explizites Budget
aus der √3-Eingaberundung und den trigonometrischen Rundungen. Alle Gruppen-,
Gitter-, Phasen- und Gültigkeitsentscheidungen bleiben exakt.

## Ergänzende C++-Probes und Negativfälle

[geometry_probe.cpp](../tests/ground_truth/probes/geometry_probe.cpp) lädt nur
`COrbifoldGroup`. Es konstruiert **kein** `COrbifold` und ruft keinen Massen- oder
Zustandslöser auf. Es liefert Fixloci, tatsächlich gespeicherte modulare
Vorfaktoren und Raumgruppenprodukte. Die Probes linken vorhandene originale
Objekte ohne `main.o`; die Originalquellen bleiben unverändert.

[geometry_oracle.json](../tests/ground_truth/geometry_oracle.json) ist ein
separater, integritätsgebundener Zusatz zu den sieben unveränderten Fixtures:

- sieben reguläre Modellfälle;
- zwei zusätzliche akzeptierte Spin(32)/Z2-Einbettungen, je eine pro Variante;
- elf tatsächlich abgelehnte Eingaben: Shiftordnung, Shiftnorm,
  Shiftpaar-Kongruenz, Wilsonordnung/-relationen und eine SUSY-Wilsonnorm;
- ein ausdrücklich getrennter non-SUSY-Oracle-Konflikt zur Wilsonnorm.

Jede dieser 21 Eingaben wurde zweimal unabhängig ausgeführt. Modelltext,
Quell-/Binary-/Objekt-/Bibliotheksnachweise, Compiler-/Builddaten und
Generatorhash sind gespeichert. Die Tests binden den Zusatz per SHA-256 an
die sieben alten Modell-Fixtures und die Probe-/Generatorquellen.

```bash
bash tests/ground_truth/build_geometry_probes.sh
python3 tests/ground_truth/generate_geometry.py
```

Reguläre Wiederholung überschreibt keine Referenz. Eine Änderung erfordert
einen geprüften Diff und explizites `--overwrite`.

## Weitere beobachtete Legacy-Grenzen

| Befund | Konsequenz für die Verifikation |
| --- | --- |
| [non-SUSY SG_Multiply](../legacy_cpp/non_susy_orbifolder/src/orbifolder/cspacegroup.cpp#L3333) setzt nur n und k, nicht m. | Direkte Probe-Produkte bestätigen den Verlust von m. Rust erhält alle drei Exponenten; Assoziativität, Inverse und Spin-Identität werden unabhängig geprüft. Bei den non-SUSY-Probe-Produkten werden die korrekten Translationen/n/k verglichen und das fehlende m ausdrücklich als Defekt geprüft. Die vollständigen geladenen Modelllabels behalten m. |
| [non-SUSY SG_Commute](../legacy_cpp/non_susy_orbifolder/src/orbifolder/cspacegroup.cpp#L2998) benutzt die Indizes `[n][n]` beziehungsweise `[k][k]`. | Statischer Quellenbefund; dieser Helfer wurde für die drei Faktoren nicht als Rechen-Oracle aufgerufen. Die geladenen Eingabekataloge werden gegen die unabhängig gelösten ganzzahligen Kommutatorgleichungen geprüft. |
| [non-SUSY Vorfaktortabelle](../legacy_cpp/non_susy_orbifolder/src/orbifolder/cspacegroup.cpp#L2057) erzeugt neun Zeilen mit acht Spalten; die Schleifen schreiben neun Spalten. | Die native Ausgabe bestätigt die gespeicherte Form 9×8. Es wird keine imaginäre neunte Spalte gelesen. Rust berechnet eine vollständige 9×9-Tabelle; jede tatsächlich gespeicherte native Zelle wird exakt verglichen. Stabilität der alten Läufe beseitigt das undefinierte C++-Verhalten nicht. |
| C++-Wilson-Aliasprüfung vergleicht gewählte Vektorrepräsentanten direkt. | Rust prüft die vom Auftrag verlangte Äquivalenz modulo Eichgitter. Ein gesonderter Unit-Test akzeptiert unterschiedliche, aber eichgitteräquivalente Wilsonvektoren, sofern auch die modularen Bedingungen bestehen. |

Keiner dieser Befunde wurde durch Änderungen an der Originalquelle kaschiert.

## Bewusste Abdeckungsgrenzen

- Die allgemeinen Raumgruppenoperationen unterstützen rationale affine
  Generatortranslationen. Fixloci und frei wirkende Beispiele werden geprüft.
  `embed` deckt derzeit reine Twist-Lifts ab; nichttriviale Rototranslations-
  Einbettungen und nichtverschwindende Wilsonlinien in kontinuierlichen freien
  Richtungen liefern `UnsupportedEmbedding`. Dafür fehlen separate
  Narain-/Oracle-Referenzen. Frei wirkende Zusatz-Wilsonlinien sind kein
  behaupteter Teil dieses Einbettungsprofils.
- Die Torsionsdaten beschreiben zertifizierte alternierende Bicharaktere auf der
  Abelianisierung, keine behauptete vollständige Klassifikation von H². Für die
  Legacy-SUSY-Konvention entspricht `a` dem Punktpaar (m,n), `b/c` den gemischten
  Punkt-/Translationsslots und `d` dem 6×6-Translationsblock. Die sieben
  Modellfälle verwenden Torsion null. Nichttriviale Phasen werden durch exakte
  Quantisierungs- und Bicharaktertests geprüft, nicht als bereits verifizierte
  SUSY-Zustandsprojektionen ausgegeben.
- Die non-SUSY-Torsionsprojektion ist im [Original auskommentiert](../legacy_cpp/non_susy_orbifolder/src/orbifolder/cfixedpoint.cpp#L1114).
  Die API meldet bei aktiv angeforderten solchen Projektionsphasen ausdrücklich
  `UnsupportedTorsionProjection`.
- Diskrete R-/Nicht-R-Auswahlregeln und Spektren sind spätere Phasen; sie wurden
  nicht als versteckte Nebenrechnung implementiert.

## Fachlicher Freigabepunkt: non-SUSY-Wilson-Kongruenzen

Die Zusatzreferenz isoliert folgende Eingabe auf der Z3-Geometrie:

```text
W1 = W2 = (0,0,0,0,0,0,0,0 ; 1/3,1/3,0,0,0,0,0,0)
W3 = W4 = W5 = W6 = 0
```

Die Ordnung drei, die geometrischen Aliasrelationen und sämtliche gemischten
Shiftprodukte sind erfüllt. Dagegen ist `3W1² = 2/3` kein Element von 2Z.

| Prüfer | Gemessenes Verhalten |
| --- | --- |
| SUSY-C++ | Laden wird abgelehnt. |
| non-SUSY-C++ | Laden wird akzeptiert, in beiden unabhängigen Prozessen. |
| Vorliegende exakte Rust-Implementierung | Ablehnung: `ModularInvariance { left:3, right:3, value:2/3 }`. |

Ursache ist der [non-SUSY-Wilsonzweig](../legacy_cpp/non_susy_orbifolder/src/orbifolder/corbifoldgroup.cpp#L1335):
`IsModularInvariant=false` liegt unter `if(info) if(verbose)`, während `verbose`
in dieser Funktion false ist. Die Wilson-Selbst-/Paar-Kongruenzen können dadurch
verletzungsfrei gemeldet werden. Der C++-SUSY-Zweig und die gemeinsame
mathematische Bedingung stimmen mit der strikten Rust-Ablehnung überein.

**Verbindliche Nutzerentscheidung (2026-10-31):** Die exakten Wilson-Kongruenzen
gelten für beide Rust-Theorien; die nachweislich unvollständige non-SUSY-
Validierung ist ein bestätigter Legacy-Fehler. Das ist
eine bewusst offengelegte Verhaltensabweichung, kein bestandener differentieller
Gleichheitstest. Die Regression hält beide nativen Ergebnisse und die rationale
Rust-Ablehnung getrennt fest.

Der freigegebene [Migrationsplan](./03_verification_and_migration_plan.md) verlangt:
> Eine Abweichung vom C++-Verhalten benötigt nach Ursachenklärung eine ausdrückliche fachliche Entscheidung des Nutzers.

Diese Entscheidung und die Abnahme von Phase 3.2 sind ausdrücklich erfolgt.
Es wurde kein automatischer Legacy-Kompatibilitäts-
Bypass eingebaut und keine alte Referenz überschrieben.

## Prüfstand

```bash
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

44 Tests einschließlich aller früheren `math_core`-Tests und sechs
Property-Test-Familien. Zusätzliche Invarianten prüfen drei zyklische Faktoren,
Spin-Lifts, Assoziativität, Inverse, primitive Fix-Torus-Gitter, zentrale
Translationsgitter, affine Generatorpotenzen, fehlerhafte Metriken/Charaktere,
unvollständige Kataloge, Ressourcenlimits und quantisierte Torsionsphasen.
Die sieben positiven Modell-Abgleiche und alle elf gemeinsamen Ablehnungsfälle
stimmen mit dem Oracle überein; der eine Wilson-Konflikt bleibt ausdrücklich
als dokumentierter, fachlich entschiedener Legacy-Fehler erhalten.
Keine Aussage über neue Rust-Spektren.
