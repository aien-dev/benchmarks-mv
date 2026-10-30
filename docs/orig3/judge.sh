#!/bin/sh
# ORIG-3 judge over selmain E per-record lines (disclosed POSIX awk glue; the records and the
# scorer are .om). Columns: seq fam core M N model rule ref static near oracle c_model c_oracle
# c_rule c_ref c_static c_near. Gated families: 2 (H1) and 3 (H2). Usage: judge.sh MLP AFFINE SHUF UNTR
awk -v crit60=37 -v crit120=70 '
function tot(f, lab,   l, a) {
  while ((getline l < f) > 0) { split(l, a, " "); if (a[2] != 2 && a[2] != 3) continue;
    c = a[3]; n[lab, c]++; m[lab, c] += a[12]; o[lab, c] += a[13]; r[lab, c] += a[14]; rf[lab, c] += a[15]; s[lab, c] += a[16]; nr[lab, c] += a[17];
    if (a[12] < a[14]) wr[lab, c]++; if (a[12] < a[17]) wn[lab, c]++; if (a[12] < a[16]) ws[lab, c]++;
    if (a[12] > a[14]) lr[lab, c]++; if (a[12] > a[17]) ln[lab, c]++; if (a[12] > a[16]) ls[lab, c]++;
    if (a[2] == 4) {}
  } close(f) }
BEGIN { tot(ARGV[1], "mlp"); tot(ARGV[2], "aff"); tot(ARGV[3], "shuf"); tot(ARGV[4], "untr");
  go = 1; nogo = 0;
  for (c = 0; c <= 1; c++) {
    lab = "mlp"; N = n[lab, c]; crit = (N == 60 ? crit60 : (N == 120 ? crit120 : -1));
    best = (s[lab, c] < nr[lab, c] ? s[lab, c] : nr[lab, c]);
    A = (m[lab, c] <= 0.95 * r[lab, c]); B = (m[lab, c] <= rf[lab, c]); C = (m[lab, c] <= m["aff", c]);
    D = (crit > 0 && wr[lab, c] >= crit); E = (m[lab, c] <= best); F = !((m["shuf", c] <= 0.95 * r["shuf", c]) || (m["untr", c] <= 0.95 * r["untr", c]));
    printf "core %d n %d SEALED H1+H2: mlp/oracle %.4f aff/oracle %.4f rule/oracle %.4f ref/oracle %.4f static/oracle %.4f retrieval/oracle %.4f shuffled/oracle %.4f untrained/oracle %.4f\n", c, N, m[lab,c]/o[lab,c], m["aff",c]/o["aff",c], r[lab,c]/o[lab,c], rf[lab,c]/o[lab,c], s[lab,c]/o[lab,c], nr[lab,c]/o[lab,c], m["shuf",c]/o["shuf",c], m["untr",c]/o["untr",c];
    printf "core %d A(<=0.95 rule)=%d [%.3f] B(<=ref)=%d [%.3f] C(<=affine)=%d [%.4f] D(sign vs rule %d/%d >= %d)=%d E(<=min(static,retrieval))=%d [%.4f] F(controls fail A: shuffled %.3f untrained %.3f)=%d\n", c, A, m[lab,c]/r[lab,c], B, m[lab,c]/rf[lab,c], C, m[lab,c]/m["aff",c], wr[lab,c], lr[lab,c], crit, D, E, m[lab,c]/best, m["shuf",c]/r["shuf",c], m["untr",c]/r["untr",c], F;
    printf "core %d reported: sign vs retrieval +%d/-%d, vs static +%d/-%d\n", c, wn[lab,c], ln[lab,c], ws[lab,c], ls[lab,c];
    if (!(A && B && C && D && E && F)) go = 0; if (m[lab, c] > 1.01 * best) nogo = 1;
  }
  v = go ? "GO" : (nogo ? "NO-GO" : "INCONCLUSIVE"); printf "ORIG3_VERDICT=%s\n", v }' "$@"
