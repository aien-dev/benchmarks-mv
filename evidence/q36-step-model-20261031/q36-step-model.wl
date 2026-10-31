(* q36-step-model.wl: the fitted step-time model of the Qwen3.6-35B-A3B decode stroke on one GB10 (DRA-64).
   Version q36-step-model/1, 2026-10-31. Changes only through a model-revision issue.
   Needs q36in (inputs.wl, generated from the sha256-pinned files by scripts/extract-inputs.sh) evaluated first.

   t(n) = a + b n + c U(n) + h Ceiling[n/4]   [ms per stroke of n heads]
   U(n) = experts read per layer per stroke = routed-expert union of the n heads + 1 shared expert.
   The a, b, c, h published on Linear DRA-64 are frozen in q36Published; predict[] always uses them.

   Fit procedure (the one that reproduces the published numbers, see report key "fitPublishedProcedure"):
     c = (gate_up + down chip time of the stamped lever-2 eight-head stroke) / U(8)
     h = (lm_head chip time of that stroke) / (its lm_head launches)
     a, b = least squares of  median(n) - c U(n) - h Ceiling[n/4]  over the lever-2 ladder n = 1, 2, 4, 8.

   Use by later lever issues: recordLever["lever-3", n, U, measuredMs] returns predicted, measured, residual and the
   verdict; |residual| > 10 % of the prediction means OPEN_MODEL_REVISION_ISSUE (a new versioned .wl, never an edit). *)

q36ModelVersion = "q36-step-model/1 (DRA-64, 2026-10-31)";

(* ---- frozen model (Linear DRA-64 text) ---- *)
q36Published = <|"a" -> 913/100, "b" -> 329/100, "c" -> 696/1000, "h" -> 481/100, "rms" -> 24/10,
   "U" -> {9, 163/10, 307/10, 556/10}, "Uindep8" -> 584/10, "tokS8measured" -> 955/10, "tokS8indep" -> 933/10|>;
q36Halfunit = <|"a" -> 5/1000, "b" -> 5/1000, "c" -> 5/10000, "h" -> 5/1000, "rms" -> 5/100|>;
q36Ns = {1, 2, 4, 8};
q36CeilingGBs = 273;       (* GB10 LPDDR5X ceiling quoted in motor-chip README L154 *)
q36MaxHeadsQualified = 8;  (* OMEGA_GPU_Q36_MAX_HEADS 8u, omega-mv 474197e4 src/omega_gpu_q36_graph.h L66 *)

q36t[n_, u_, p_Association] := p["a"] + p["b"] n + p["c"] u + p["h"] Ceiling[n/4];
predict[n_, u_] := q36t[n, u, q36Published];
predictTokS[n_, u_] := 1000 n/predict[n, u];
q36Uindep[n_] := 1 + 256 (1 - (1 - 8/256)^n);   (* independent top-8 draws from 256 routed experts, + shared *)
q36RevisionThreshold = 1/10;
recordLever[lever_String, n_, u_, measuredMs_] := Module[{p = predict[n, u], r},
   r = (measuredMs - p)/p;
   <|"lever" -> lever, "n" -> n, "U" -> q36Num[u], "predictedMs" -> q36Num[p], "measuredMs" -> q36Num[measuredMs],
     "residual" -> q36Num[r],
     "verdict" -> If[Abs[r] > q36RevisionThreshold, "OPEN_MODEL_REVISION_ISSUE", "WITHIN_MODEL"],
     "scope" -> If[n > q36MaxHeadsQualified, "architecture projection until OMEGA_GPU_Q36_MAX_HEADS > 8 is qualified", "fitted range"]|>];

(* ---- derived inputs ---- *)
(* The stroke gate's median (and the README ladder) is the upper median: element Floor[len/2]+1 of the sorted list.
   Wolfram's Median averages the middle pair for an even count; both are reported, the fit uses the gate's. *)
q36GateMedian[x_List] := Sort[x][[Floor[Length[x]/2] + 1]];
q36MedMs[in_] := Table[q36GateMedian[in["ladderNs" <> ToString[k]]]/10^6, {k, q36Ns}];
q36MedMsMean[in_] := Table[Median[in["ladderNs" <> ToString[k]]]/10^6, {k, q36Ns}];
q36UfromCells[cells_] := 1 + Mean[cells];
q36ExpertsNs[in_] := in["stampsKindNs"]["gate_up"] + in["stampsKindNs"]["down"];
q36Fit[ms_, u_, in_] := Module[{c, h, r, ab, fitted},
   c = (q36ExpertsNs[in]/10^6)/u[[4]];
   h = (in["stampsKindNs"]["lm_head"]/10^6)/in["stampsKindLaunches"]["lm_head"];
   r = ms - c u - h Ceiling[q36Ns/4];
   ab = LeastSquares[Transpose[{ConstantArray[1, 4], q36Ns}], r];
   fitted = ab[[1]] + ab[[2]] q36Ns + c u + h Ceiling[q36Ns/4];
   <|"a" -> ab[[1]], "b" -> ab[[2]], "c" -> c, "h" -> h, "rms" -> Sqrt[Mean[(ms - fitted)^2]],
     "residualsMs" -> ms - fitted|>];
q36FreeFit[ms_, u_] := Module[{x = LinearSolve[Transpose[{ConstantArray[1, 4], q36Ns, u, Ceiling[q36Ns/4]}], ms]},
   <|"a" -> x[[1]], "b" -> x[[2]], "c" -> x[[3]], "h" -> x[[4]],
     "note" -> "four parameters through four points: an interpolation, RMS 0 by construction"|>];
q36Rms[ms_, u_] := Sqrt[Mean[(ms - Table[predict[q36Ns[[i]], u[[i]]], {i, 4}])^2]];
q36Reproduced[fit_] := AllTrue[{"a", "b", "c", "h", "rms"}, Abs[fit[#] - q36Published[#]] <= q36Halfunit[#] &];
q36Num[x_] := N[Round[x, 1/10^6]];   (* display only: exact arithmetic throughout, rounded to 1e-6 at output *)
q36NumAssoc[a_Association] := Map[If[NumericQ[#] || VectorQ[#, NumericQ], q36Num[#], #] &, a];

q36Report[in_] := Module[{ms, uLinear, uMatched, uQuoted, uIndep, fitLinear, fitMatched, fitIndep, stampMed,
    u8stamp, bw, bwRow, checks, proj, variants, pinnedMeans},
   ms = q36MedMs[in];
   stampMed = q36GateMedian[in["stampsNs"]]/10^6;
   uLinear = q36Published["U"];
   uMatched = q36UfromCells /@ {in["cells[ladder1-p1]"], in["cells[ladder2-p1p2]"], in["cells[ladder4-p1234]"],
      in["cells[ladder8-setb]"]};
   uIndep = q36Uindep /@ q36Ns;
   (* the README quotes behind the Linear U: 4 chat prompts (29.7), mean of 8-prompt sets a and b (54.7, 54.5) *)
   uQuoted = <|"U4 from 4chat" -> q36UfromCells[in["cells[4chat]"]],
     "U8 from mean(set a, set b)" -> 1 + Mean[{Mean[in["cells[8a]"]], Mean[in["cells[ladder8-setb]"]]}],
     "U2 candidate code+p2" -> q36UfromCells[in["cells[pair-code-p2]"]],
     "U2 candidate code+review" -> q36UfromCells[in["cells[pair-code-review]"]],
     "U2 of the ladder pair p1+p2" -> uMatched[[2]]|>;
   pinnedMeans = <|"4p" -> {Mean[in["cells[ladder4-p1234]"]], in["pinnedUnionMean[4p]"]},
     "4chat" -> {Mean[in["cells[4chat]"]], in["pinnedUnionMean[4chat]"]},
     "8prompts" -> {Mean[in["cells[8a]"]], in["pinnedUnionMean[8prompts]"]},
     "8prompts-b" -> {Mean[in["cells[ladder8-setb]"]], in["pinnedUnionMean[8prompts-b]"]}|>;
   fitLinear = q36Fit[ms, uLinear, in];
   fitMatched = q36Fit[ms, uMatched, in];
   fitIndep = q36Fit[ms, uIndep, in];
   (* per-class achieved bandwidth on the stamped lever-2 stroke (set b heads; U from the ladder-8 set-b union).
      Stamps cost chip time: these are stamped-timeline rates, not speed receipts. bytes/ns = GB/s. *)
   u8stamp = uMatched[[4]];
   bwRow[name_, bytes_, kinds_List] := Module[{ns = Total[in["stampsKindNs"] /@ kinds]},
     {name, q36Num[bytes], Total[in["stampsKindLaunches"] /@ kinds], q36Num[ns/10^6], q36Num[bytes/ns],
      q36Num[100 (bytes/ns)/q36CeilingGBs]}];
   bw = {{"class", "bytes per stroke", "launches", "ms", "GB/s", "% of 273 GB/s"},
     bwRow["experts gate_up+down (U8 set b)", in["bytes[layers]"] u8stamp in["bytes[expert]"], {"gate_up", "down"}],
     bwRow["experts gate_up", in["bytes[layers]"] u8stamp in["bytes[gate_up]"], {"gate_up"}],
     bwRow["experts down", in["bytes[layers]"] u8stamp in["bytes[down]"], {"down"}],
     bwRow["lm_head bf16 (2 four-head reads)", in["stampsKindLaunches"]["lm_head"] in["bytes[lm_head]"], {"lm_head"}],
     bwRow["mix (FP8 + scales)", in["bytes[mix]"], {"mix"}],
     bwRow["o_proj (FP8 + scales)", in["bytes[o_proj]"], {"o_proj"}],
     bwRow["router (bf16 256x2048)", in["bytes[router]"], {"router"}],
     bwRow["ab (bf16 in_proj_a,b)", in["bytes[ab]"], {"ab"}],
     bwRow["superseded: experts at 768-wide 3x2048x768 B", in["bytes[layers]"] u8stamp 3*2048*768, {"gate_up", "down"}]};
   variants = <|
     "measured union (Linear U8 = 55.6)" -> <|"U8" -> 556/10, "tMs" -> q36Num[predict[8, 556/10]],
        "tokS" -> q36Num[predictTokS[8, 556/10]]|>,
     "independent-draw union (U8 = 1 + 256(1-(31/32)^8))" -> <|"U8" -> q36Num[q36Uindep[8]],
        "tMs" -> q36Num[predict[8, q36Uindep[8]]], "tokS" -> q36Num[predictTokS[8, q36Uindep[8]]]|>,
     "independent-draw union, Linear rounding (U8 = 58.4)" -> <|"U8" -> 584/10, "tMs" -> q36Num[predict[8, 584/10]],
        "tokS" -> q36Num[predictTokS[8, 584/10]]|>,
     "ladder-matched union (U8 set b)" -> <|"U8" -> q36Num[u8stamp], "tMs" -> q36Num[predict[8, u8stamp]],
        "tokS" -> q36Num[predictTokS[8, u8stamp]]|>|>;
   proj = Prepend[Table[{n, q36Num[q36Uindep[n]], q36Num[predict[n, q36Uindep[n]]], q36Num[predictTokS[n, q36Uindep[n]]],
       If[n > q36MaxHeadsQualified, "architecture projection until OMEGA_GPU_Q36_MAX_HEADS > 8 is qualified",
        "fitted range (lever-2 ladder)"]}, {n, {1, 2, 4, 8, 16, 32, 64}}],
     {"n", "U independent draws", "t ms", "tok/s", "status"}];
   checks = <|
     "ladderMediansMatchReadme" -> AllTrue[Abs[ms - in["readmeLadderMs"]], # <= 5/10000 &],
     "ladderMediansMatchFileField" -> (ms == Table[in["ladderFileMedianNs" <> ToString[k]], {k, q36Ns}]/10^6),
     "stampsMedianMatchesLog" -> (Abs[stampMed - in["stampsLogMedianMs"]] <= 5/10000),
     "ladderHeadsAre1248" -> ({in["ladderHeads1"], in["ladderHeads2"], in["ladderHeads4"], in["ladderHeads8"]} == q36Ns),
     "pinnedUnionMeansRederived" -> AllTrue[Values[pinnedMeans], Abs[#[[1]] - #[[2]]] <= 10^-9 &],
     "expertBytesContract" -> (in["bytes[expert]"] == 3*2048*512 + 3*16*4*2 == 3146112 &&
        in["bytes[shared]"] == in["bytes[expert]"] && in["bytes[gate_up]"] + in["bytes[down]"] == in["bytes[expert]"] &&
        in["bytes[routed_expert_tensor_sets_layer0]"] == 256 && in["bytes[layers]"] == 40),
     "lmHeadBytes" -> (in["bytes[lm_head]"] == 248320*2048*2),
     "mixBytesMatchReadme0944GB" -> (Round[in["bytes[mix]"]/10^9, 1/1000] == 944/1000),
     "fitPublishedReproducedLinearU" -> q36Reproduced[fitLinear],
     "linearUMatchesPinnedUnions" -> (Round[uQuoted["U4 from 4chat"], 1/10] == uLinear[[3]] &&
        Round[uQuoted["U8 from mean(set a, set b)"], 1/10] == uLinear[[4]] && uMatched[[1]] == uLinear[[1]]),
     "contractT8" -> (predict[8, 556/10] == 837676/10000 && Round[predictTokS[8, 556/10], 1/100] == 9550/100),
     "variantIndep" -> (Round[q36Uindep[8], 1/10] == 584/10 && Round[predictTokS[8, q36Uindep[8]], 1/10] == 933/10),
     "weightClassesBelowCeiling" -> AllTrue[Most[Rest[bw]][[All, 5]], # <= q36CeilingGBs &]|>;
   <|"version" -> q36ModelVersion,
     "checks" -> checks,
     "allChecksPass" -> AllTrue[Values[checks], TrueQ],
     "ladderMediansMs (gate upper median)" -> q36Num[ms], "readmeLadderMs" -> in["readmeLadderMs"],
     "ladderMediansMs (Wolfram Median, mean of middle pair)" -> q36Num[q36MedMsMean[in]],
     "stampsMedianMs" -> q36Num[stampMed],
     "U" -> <|"linear (frozen)" -> q36Num[uLinear], "ladder-matched" -> q36Num[uMatched],
        "independent draws" -> q36Num[uIndep], "quoted-sources" -> q36NumAssoc[uQuoted],
        "ladder-matched minus linear" -> q36Num[uMatched - uLinear]|>,
     "pinnedUnionMeans {wolfram, file}" -> Map[q36Num, pinnedMeans],
     "fitPublishedProcedure (linear U)" -> q36NumAssoc[fitLinear],
     "fitPublishedProcedure (ladder-matched U)" -> q36NumAssoc[fitMatched],
     "fitPublishedProcedure (independent-draw U)" -> q36NumAssoc[fitIndep],
     "freeFit (linear U)" -> q36NumAssoc[q36FreeFit[ms, uLinear]],
     "publishedModelRmsMs (linear U)" -> q36Num[q36Rms[ms, uLinear]],
     "publishedModelResidualsMs (linear U)" -> q36Num[ms - Table[predict[q36Ns[[i]], uLinear[[i]]], {i, 4}]],
     "variantsN8" -> variants,
     "bandwidth (stamped lever-2 stroke)" -> bw,
     "projection" -> proj,
     "revisionRuleExamples" -> {recordLever["example within", 8, 556/10, 105/100 predict[8, 556/10]],
        recordLever["example breach", 8, 556/10, 115/100 predict[8, 556/10]]}|>];

q36Report[q36in]
