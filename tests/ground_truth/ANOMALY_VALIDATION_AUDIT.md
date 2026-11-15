# Native Anomalieprüfer: bestätigte falsche Erfolgsstatuswerte

Die Zusatzprobes der Phase 3.3b entfernen genau ein chirales Feld aus einer
Kopie der nativen Konfiguration und rufen den unveränderten `CheckAnomaly`
auf. [anomalies_oracle.json](./anomalies_oracle.json) speichert den entfernten
nativen Feldindex, originale Spuren, Originalbericht und Rückgabewert;
je zwei unabhängige Prozesse stimmen exakt überein.

| Modell | Entferntes natives Feld | C++-Urteil | Mathematisches Rust-Urteil |
| --- | ---: | --- | --- |
| SUSY Z3 | 3 | true, native kubische Spur -27 | false, SU(3)^3 verletzt |
| SUSY Z3×Z3/WL | 12 | false | false |
| non-SUSY Z3 | 27 | false | false |
| non-SUSY Z3×Z3/WL | 677 | true, native kubische Spur -1 | false, SU(3)^3 verletzt |
| non-SUSY tachyonisch | 13 | true, native kubische Spur 8 | false, SU(4)^3 verletzt |

Die sieben unveränderten Modell-Fixtures sind in beiden Prüfern erfolgreich.
SUSY/non-SUSY Z6-II besitzen keinen für diese konkrete Probe auswählbaren
SU(N>=3)-geladenen chiralen Zustand; dort wird kein Entfernungsfall behauptet.

## Ursachen im Original

Im [SUSY-Prüfer](../../legacy_cpp/susy_orbifolder/src/orbifolder/corbifold.cpp#L1351)
und im [non-SUSY-Prüfer](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/corbifold.cpp#L1593)
kehrt der Zweig ohne U(1)-Faktoren mit `true` zurück, obwohl zuvor bereits
ein nicht-abelscher oder globaler Anomaliefehler bestimmt worden sein kann.

Der [non-SUSY-Prüfer](../../legacy_cpp/non_susy_orbifolder/src/orbifolder/corbifold.cpp#L1368)
setzt zudem `verbose=false`. Mehrere Zuweisungen
`AllAnomaliesCancel=false` stehen innerhalb `if(verbose)`: SU(N)^3,
ungerade SU(2)-Doublettzahl und verschiedene GS-Universalitätsfehler.
Das ist eine Unterdrückung der Fehlerentscheidung, nicht nur der Ausgabe.
Einige andere Fehlerpfade liegen außerhalb des Guards und liefern weiterhin
`false`; deshalb sind die nativen Ergebnisse der Entfernungsfälle verschieden.

## Verbindlicher Prüfvertrag dieser Phase

Der Nutzer verlangt ausdrücklich das zuverlässige Ablehnen nicht-abelscher
kubischer Anomalien, ungleicher GS-Verhältnisse und zusätzlicher unabhängiger
abelscher Anomalien. Rust prüft jede Bedingung unabhängig von Ausgabeoptionen
und kombiniert alle Urteile. Die dokumentierten nativen Negativ-Erfolgswerte
werden nicht als mathematisch erlaubte Spektren übernommen. Keine Originalquelle
oder abgenommene Fixture wird zur Erzeugung eines gewünschten Urteils gepatcht.
