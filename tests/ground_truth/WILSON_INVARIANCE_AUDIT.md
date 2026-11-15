# Bestätigter Legacy-Fehler: non-SUSY-Wilson-Kongruenzen

Am 2026-10-31 hat der Nutzer die exakte Ablehnung für beide Theorien verbindlich
freigegeben und Phase 3.2 abgenommen. Neben dem [Excited-Befund](./TACHYON_AUDIT.md)
ist dies ein zweiter bestätigter Fehler des non-SUSY-Altprogramms.

## Reproduktion und Ursache

Die [Zusatzreferenz](./geometry_oracle.json) trennt
`legacy_conflicts.non_susy_wl_norm` und `negative_cases.susy_wl_norm`.
Beide verwenden auf Z3 die Wilsonlinien
`W1=W2=(0^8;1/3,1/3,0^6)`, `W3=...=W6=0`.
Ordnung und geometrische Relationen sind erfüllt: `3W` ist eine Wurzel,
`(3W)^2=2`. Dennoch verletzt `3W^2=2/3` die modulare Kongruenz.

Im [non-SUSY-Validator](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/corbifoldgroup.cpp#L1335)
liegt die Fehlerzuweisung `IsModularInvariant=false` im Block
`if(info) if(verbose)`, wobei `verbose=false` ist. Dadurch bleibt bei verletzten
Wilson-Selbst-/Paarbedingungen der Erfolgsstatus erhalten. Der SUSY-Validator
lehnt dieselbe Eingabe ab. Die Zusatzreferenz bestätigt beide Ergebnisse mit
je zwei unabhängigen Prozessen und hält die Herkunft der Binaries fest.

## Verbindlicher Rust-Vertrag

Der Core lehnt diese Eingabe exakt mit
`ModularInvariance { left:3, right:3, value:2/3 }` ab, unabhängig von der Theorie.
Die Regression bewahrt das abweichende native Ergebnis ausdrücklich als
Legacy-Konflikt. Weder Originalquellen noch Modell-Fixtures werden verändert.
Eine Gitter-Ordnungsprüfung ersetzt keine Norm-/Paar-Kongruenzprüfung.
