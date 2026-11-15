# Bestätigter dritter Legacy-Befund: feste Tachyon-Suchfenster

Phase 3.3a und dieser Befund sind vom Nutzer offiziell abgenommen.
Vollständige exakte Normschranken bleiben der physikalische Rust-Standard
(`TachyonSearch::Complete`, fachlich „FullNormBound“).

Der [native Tachyonlöser](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/chalfstate.cpp#L568)
zählt das unverschobene q ausschließlich in
`q0=-7..0`, `q1..q3=-7..5` auf. Die Spinorschleife ist auskommentiert.
Die Grenzen hängen nicht vom konstruktiven Twist ab. Ein ganzzahliger,
spinverträglicher Twist-Lift kann damit physikalisch identische Impulse
aus dem Suchfenster verschieben.

[states_oracle.json](./states_oracle.json) und die
[unveränderte Objektdateien verwendende Probe](./probes/states_probe.cpp)
isolieren den Fall `(1,1,0)` mit `v0=(0,1,1,1)`, `v1=(0,1/6,-1/6,0)`.
C++ findet einen rechten Grundzustand mit `mu=-1/3` und zwei Oszillatoranregungen
mit `mu=-1/6`. Bei `v0[1]=25` findet dasselbe native Suchfenster keine dieser
Lösungen. Die vollständige Rust-Suche reproduziert alle drei Gewichte, Massen
und Eigenphasen. Je zwei unabhängige Prozesse stimmen exakt überein.

Das ist ein lokaler kinematischer Gegenbeweis zur Vollständigkeit des Fensters,
kein zusätzlicher Nachweis einer projizierten Modell-Fixture. Das Fenster
bleibt ausschließlich als expliziter Oracle-Vergleichsmodus erhalten. Originale
C++-Quellen und die sieben vollständigen Modell-Fixtures wurden nicht geändert.
